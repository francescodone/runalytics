use std::{
    marker::PhantomData,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
};

use aws_lc_rs::digest::{digest, SHA256};
use serde::{Deserialize, Serialize};
use tauri::{
    ipc::{CommandArg, CommandItem, InvokeBody, InvokeError},
    Emitter, Manager,
};

use crate::{
    commands::Settings,
    error::{AppError, Result},
    providers::{coros::CorosProvider, Provider},
    secure_store,
    state::AppState,
    store::Store,
    types::ProviderKind,
};

const SIGNED_OUT: &str = "signed-out";
const COROS_SERVICE: &str = "com.fradone.runalytics.coros";
const PROFILE_FILES: &[&str] = &[
    "settings.json",
    "plan-sync.json",
    "calendar-publishing.json",
];

#[cfg(test)]
mod tests;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSession {
    pub profile_key: Option<String>,
    pub legacy_available: bool,
    pub legacy_profile: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Identity {
    account_id: String,
    legacy_profile: bool,
}

pub struct ProfileManager {
    root: PathBuf,
    active: RwLock<Arc<AppState>>,
    gate: Arc<tokio::sync::RwLock<()>>,
    switching: tokio::sync::Mutex<()>,
}

pub struct State<'r, T> {
    value: Arc<T>,
    lease: Arc<tokio::sync::OwnedRwLockReadGuard<()>>,
    lifetime: PhantomData<&'r T>,
}
impl<T> Deref for State<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}
impl<T> State<'_, T> {
    pub fn inner(&self) -> &T {
        &self.value
    }
    pub fn owned(&self) -> State<'static, T> {
        State {
            value: self.value.clone(),
            lease: self.lease.clone(),
            lifetime: PhantomData,
        }
    }
}
impl<T> Clone for State<'_, T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            lease: self.lease.clone(),
            lifetime: PhantomData,
        }
    }
}
impl<'r, 'de: 'r, R: tauri::Runtime> CommandArg<'de, R> for State<'r, AppState> {
    fn from_command(command: CommandItem<'de, R>) -> std::result::Result<Self, InvokeError> {
        let manager = command
            .message
            .webview_ref()
            .try_state::<ProfileManager>()
            .ok_or_else(|| InvokeError::from("Profile manager unavailable"))?;
        let scope = match command.message.payload() {
            InvokeBody::Json(value) => match value.get("profileScope") {
                Some(value) => Some(
                    value
                        .as_str()
                        .ok_or_else(|| InvokeError::from("Invalid profile scope"))?,
                ),
                None => None,
            },
            _ => None,
        };
        manager
            .capture(scope)
            .map_err(|error| InvokeError::from(error.to_string()))
    }
}

pub fn profile_key(account_id: &str) -> String {
    digest(&SHA256, account_id.as_bytes())
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn default_settings() -> Settings {
    Settings {
        pmtiles_path: None,
        last_provider: None,
        theme: None,
        palette: Some("amethyst".into()),
        llm_enabled: false,
        llm_endpoint: None,
        llm_model: None,
        thresholds: Default::default(),
    }
}

fn save_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    std::fs::rename(temporary, path)?;
    Ok(())
}

fn open_state(root: &Path, key: &str) -> Result<Arc<AppState>> {
    let directory = root.join("profiles").join(key);
    std::fs::create_dir_all(&directory)?;
    let mut store = Store::open(&directory.join("runalytics.sqlite"))?;
    store.credential_scope = Some(key.to_string());
    crate::calendar::adopt_naming(&store)?;
    let settings_path = directory.join("settings.json");
    let settings = match std::fs::read(&settings_path) {
        Ok(bytes) => serde_json::from_slice(&bytes)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => default_settings(),
        Err(error) => return Err(error.into()),
    };
    if !settings_path.exists() {
        save_json(&settings_path, &settings)?;
    }
    let providers = crate::providers::build_providers(key);
    Ok(Arc::new(AppState {
        store,
        settings: Mutex::new(settings),
        providers,
        data_dir: directory,
        syncing: Default::default(),
        sync_lock: Default::default(),
        calendar_lock: Default::default(),
    }))
}

pub(crate) fn credential_account_id(scope: Option<&str>) -> Result<Option<String>> {
    let value = match secure_store::entry_scoped(scope, COROS_SERVICE, "oauth")?.get_password() {
        Ok(value) => value,
        Err(keyring_core::Error::NoEntry) => return Ok(None),
        Err(_) => {
            return Err(AppError::Other(
                "Could not read the COROS login identity".into(),
            ))
        }
    };
    let stored: serde_json::Value = serde_json::from_str(&value)?;
    let account = stored
        .get("token_response")
        .and_then(|token| token.get("access_token"))
        .and_then(|token| token.as_str())
        .and_then(crate::plan_sync::jwt_sub);
    account.map(Some).ok_or_else(|| {
        AppError::Other("COROS did not provide a stable login ID. No profile was changed.".into())
    })
}

impl ProfileManager {
    pub fn open(root: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&root)?;
        let selected = root.join("active-profile.json");
        let key: Option<String> = if selected.exists() {
            serde_json::from_slice(&std::fs::read(&selected)?)?
        } else {
            let identity = credential_account_id(None)?;
            if let Some(account) = identity {
                let key = profile_key(&account);
                let manager = Self::empty(root.clone())?;
                manager.prepare_profile(&account, true)?;
                manager.copy_login(&key)?;
                save_json(&selected, &Some(&key))?;
                Some(key)
            } else {
                save_json(&selected, &Option::<String>::None)?;
                None
            }
        };
        let mut manager = Self::empty(root)?;
        if let Some(key) = key {
            let identity = manager.identity(&key)?;
            if profile_key(&identity.account_id) != key {
                return Err(AppError::Other(
                    "The saved profile identity is invalid".into(),
                ));
            }
            manager.active = RwLock::new(open_state(&manager.root, &key)?);
        }
        Ok(manager)
    }

    fn empty(root: PathBuf) -> Result<Self> {
        let active = open_state(&root, SIGNED_OUT)?;
        Ok(Self {
            root,
            active: RwLock::new(active),
            gate: Arc::new(Default::default()),
            switching: Default::default(),
        })
    }

    pub fn capture(&self, expected: Option<&str>) -> Result<State<'static, AppState>> {
        let lease = self
            .gate
            .clone()
            .try_read_owned()
            .map_err(|_| AppError::Other("The profile is changing. Please wait.".into()))?;
        let active = self
            .active
            .read()
            .map_err(|_| AppError::Other("Profile unavailable".into()))?
            .clone();
        let key = active
            .store
            .credential_scope
            .as_deref()
            .unwrap_or(SIGNED_OUT);
        if expected.is_some_and(|expected| expected != key) {
            return Err(AppError::Other(
                "The COROS profile changed. Reopen the current profile before continuing.".into(),
            ));
        }
        Ok(State {
            value: active,
            lease: Arc::new(lease),
            lifetime: PhantomData,
        })
    }

    fn identity(&self, key: &str) -> Result<Identity> {
        if key.len() != 64 || !key.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(AppError::Other("Invalid profile key".into()));
        }
        Ok(serde_json::from_slice(&std::fs::read(
            self.root.join("profiles").join(key).join("profile.json"),
        )?)?)
    }

    pub fn session(&self) -> Result<AccountSession> {
        let active = self
            .active
            .read()
            .map_err(|_| AppError::Other("Profile unavailable".into()))?;
        let key = active
            .store
            .credential_scope
            .as_deref()
            .filter(|key| *key != SIGNED_OUT);
        let legacy_profile = key
            .map(|key| self.identity(key).map(|identity| identity.legacy_profile))
            .transpose()?
            .unwrap_or(false);
        Ok(AccountSession {
            profile_key: key.map(str::to_owned),
            legacy_available: key.is_some()
                && self.legacy_unclaimed()
                && self.profile_empty(&active)?,
            legacy_profile,
        })
    }

    fn legacy_unclaimed(&self) -> bool {
        if !self.root.join("runalytics.sqlite").exists()
            || self.root.join("legacy-profile-owner.json").exists()
        {
            return false;
        }
        let Ok(profiles) = std::fs::read_dir(self.root.join("profiles")) else {
            return false;
        };
        !profiles.filter_map(std::result::Result::ok).any(|profile| {
            let name = profile.file_name();
            let key = name.to_string_lossy();
            key.len() == 64
                && self
                    .identity(&key)
                    .map(|identity| identity.legacy_profile)
                    .unwrap_or(true)
        })
    }

    fn profile_empty(&self, state: &AppState) -> Result<bool> {
        let connection = state.store.conn.lock().unwrap();
        for table in [
            "training_plans",
            "activities",
            "health_daily",
            "fitness_assessment",
            "training_load",
            "pain_entries",
            "sync_state",
            "plan_proposals",
        ] {
            let count: i64 =
                connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                    row.get(0)
                })?;
            if count != 0 {
                return Ok(false);
            }
        }
        drop(connection);
        if crate::strava::is_configured(&state.store)? {
            return Ok(false);
        }
        for (service, user) in [
            (
                secure_store::GITHUB_SERVICE,
                secure_store::GITHUB_TOKEN_USER,
            ),
            (
                secure_store::LEGACY_PLAN_SERVICE,
                secure_store::LEGACY_PLAN_TOKEN_USER,
            ),
        ] {
            match secure_store::entry_scoped(
                state.store.credential_scope.as_deref(),
                service,
                user,
            )?
            .get_password()
            {
                Ok(_) => return Ok(false),
                Err(keyring_core::Error::NoEntry) => {}
                Err(_) => {
                    return Err(AppError::Other(
                        "Could not check saved profile connections".into(),
                    ))
                }
            }
        }
        Ok(serde_json::to_value(&*state.settings.lock().unwrap())?
            == serde_json::to_value(default_settings())?
            && !PROFILE_FILES[1..]
                .iter()
                .any(|file| state.data_dir.join(file).exists()))
    }

    fn prepare_profile(&self, account: &str, known_legacy: bool) -> Result<()> {
        let key = profile_key(account);
        let destination = self.root.join("profiles").join(&key);
        if destination.join("profile.json").exists() {
            return Ok(());
        }
        let staging = self
            .root
            .join("profiles")
            .join(format!("prepare-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&staging)?;
        let adopt = known_legacy && self.legacy_unclaimed();
        let prepared = (|| {
            if adopt {
                self.copy_legacy(&staging)?;
                self.copy_legacy_credentials(&key)?;
            }
            save_json(
                &staging.join("profile.json"),
                &Identity {
                    account_id: account.into(),
                    legacy_profile: adopt,
                },
            )?;
            std::fs::rename(&staging, &destination)?;
            if adopt {
                save_json(&self.root.join("legacy-profile-owner.json"), &key)?;
            }
            Ok(())
        })();
        if prepared.is_err() {
            let _ = std::fs::remove_dir_all(&staging);
        }
        prepared
    }

    fn copy_legacy(&self, destination: &Path) -> Result<()> {
        let database = destination.join("runalytics.sqlite");
        if database.exists() {
            return Err(AppError::Other(
                "A profile database already exists; no data was overwritten".into(),
            ));
        }
        let connection = rusqlite::Connection::open(self.root.join("runalytics.sqlite"))?;
        connection.execute("VACUUM INTO ?1", [database.to_string_lossy().as_ref()])?;
        for file in PROFILE_FILES {
            let source = self.root.join(file);
            if source.exists() {
                std::fs::copy(source, destination.join(file))?;
            }
        }
        Ok(())
    }

    fn copy_legacy_credentials(&self, key: &str) -> Result<()> {
        let mut entries = vec![
            (
                secure_store::GITHUB_SERVICE,
                secure_store::GITHUB_TOKEN_USER.to_string(),
            ),
            (
                secure_store::LEGACY_PLAN_SERVICE,
                secure_store::LEGACY_PLAN_TOKEN_USER.to_string(),
            ),
            ("com.fradone.runalytics.strava", "credentials".to_string()),
        ];
        let calendar = crate::calendar_publish::load(&self.root)?;
        if calendar.has_password {
            entries.push(("com.fradone.runalytics.calendar", calendar.upload_url));
        }
        for (service, user) in entries {
            match secure_store::entry(service, &user)?.get_password() {
                Ok(secret) => {
                    secure_store::entry_scoped(Some(key), service, &user)?.set_password(&secret)?
                }
                Err(keyring_core::Error::NoEntry) => {}
                Err(error) => {
                    return Err(AppError::Other(format!(
                        "Could not migrate a saved connection: {error}"
                    )))
                }
            }
        }
        Ok(())
    }

    fn copy_login(&self, key: &str) -> Result<()> {
        let source = secure_store::entry(COROS_SERVICE, "oauth")?;
        let secret = source.get_password()?;
        secure_store::entry_scoped(Some(key), COROS_SERVICE, "oauth")?.set_password(&secret)?;
        source.delete_credential()?;
        Ok(())
    }

    fn import_legacy(&self, expected_key: &str) -> Result<AccountSession> {
        let active = self.active.read().unwrap().clone();
        if active.store.credential_scope.as_deref() != Some(expected_key)
            || !self.legacy_unclaimed()
            || !self.profile_empty(&active)?
        {
            return Err(AppError::Other("Previous local data cannot be imported into this profile. No saved profile was overwritten.".into()));
        }
        let identity = self.identity(expected_key)?;
        let staging = self
            .root
            .join("profiles")
            .join(format!("import-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&staging)?;
        let prepared = (|| {
            self.copy_legacy(&staging)?;
            save_json(
                &staging.join("profile.json"),
                &Identity {
                    legacy_profile: true,
                    ..identity
                },
            )?;
            self.copy_legacy_credentials(expected_key)?;
            Ok::<(), AppError>(())
        })();
        if let Err(error) = prepared {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(error);
        }
        let directory = active.data_dir.clone();
        let backup = directory.with_extension(format!("empty-{}", uuid::Uuid::new_v4()));
        // Close the old SQLite/WAL handles before replacing the empty profile directory.
        *self.active.write().unwrap() = open_state(&self.root, SIGNED_OUT)?;
        drop(active);
        let installed = (|| {
            std::fs::rename(&directory, &backup)?;
            if let Err(error) = std::fs::rename(&staging, &directory) {
                let _ = std::fs::rename(&backup, &directory);
                return Err(error.into());
            }
            Ok::<(), AppError>(())
        })();
        self.select(Some(expected_key))?;
        installed?;
        save_json(&self.root.join("legacy-profile-owner.json"), &expected_key)?;
        let _ = std::fs::remove_dir_all(backup);
        self.session()
    }

    fn select(&self, key: Option<&str>) -> Result<()> {
        let next = open_state(&self.root, key.unwrap_or(SIGNED_OUT))?;
        save_json(&self.root.join("active-profile.json"), &key)?;
        *self
            .active
            .write()
            .map_err(|_| AppError::Other("Profile unavailable".into()))? = next;
        Ok(())
    }
}

#[tauri::command]
pub fn account_session(manager: tauri::State<'_, ProfileManager>) -> Result<AccountSession> {
    manager.session()
}

#[tauri::command]
pub async fn connect_provider(
    app: tauri::AppHandle,
    manager: tauri::State<'_, ProfileManager>,
    provider: ProviderKind,
    profile_scope: Option<String>,
) -> Result<String> {
    if provider != ProviderKind::Coros {
        return Err(AppError::Other("Profiles require a COROS login".into()));
    }
    let _switch = manager
        .switching
        .try_lock()
        .map_err(|_| AppError::Other("A profile change is already running".into()))?;
    let before = manager
        .session()?
        .profile_key
        .unwrap_or_else(|| SIGNED_OUT.into());
    if profile_scope
        .as_deref()
        .is_some_and(|scope| scope != before)
    {
        return Err(AppError::Other(
            "The COROS profile changed. Sign in again.".into(),
        ));
    }
    let login = CorosProvider::new();
    login.connect().await?;
    let account = credential_account_id(None)?
        .ok_or_else(|| AppError::Other("COROS did not identify this account".into()))?;
    let _gate = manager.gate.write().await;
    manager.prepare_profile(&account, false)?;
    let key = profile_key(&account);
    manager.copy_login(&key)?;
    manager.select(Some(&key))?;
    let _ = app.emit("account://changed", manager.session()?);
    Ok("COROS account connected".into())
}

#[tauri::command]
pub async fn disconnect_provider(
    app: tauri::AppHandle,
    manager: tauri::State<'_, ProfileManager>,
    provider: ProviderKind,
    profile_scope: Option<String>,
) -> Result<()> {
    let _switch = manager
        .switching
        .try_lock()
        .map_err(|_| AppError::Other("A profile change is already running".into()))?;
    let _gate = manager.gate.write().await;
    let active = manager
        .active
        .read()
        .map_err(|_| AppError::Other("Profile unavailable".into()))?
        .clone();
    if profile_scope
        .as_deref()
        .is_some_and(|scope| Some(scope) != active.store.credential_scope.as_deref())
    {
        return Err(AppError::Other(
            "The COROS profile changed. Reopen it before logging out.".into(),
        ));
    }
    let connected = active
        .providers
        .iter()
        .find(|candidate| candidate.kind() == provider)
        .ok_or_else(|| AppError::Other("Provider not available".into()))?;
    connected.disconnect().await?;
    manager.select(None)?;
    let _ = app.emit("account://changed", manager.session()?);
    Ok(())
}

#[tauri::command]
pub async fn import_legacy_profile(
    app: tauri::AppHandle,
    manager: tauri::State<'_, ProfileManager>,
    expected_profile_key: String,
    profile_scope: Option<String>,
) -> Result<AccountSession> {
    let _switch = manager
        .switching
        .try_lock()
        .map_err(|_| AppError::Other("A profile change is already running".into()))?;
    let _gate = manager.gate.write().await;
    if profile_scope
        .as_deref()
        .is_some_and(|scope| scope != expected_profile_key)
    {
        return Err(AppError::Other(
            "The COROS profile changed. Reopen the current profile.".into(),
        ));
    }
    let result = manager.import_legacy(&expected_profile_key);
    if let Ok(session) = manager.session() {
        let _ = app.emit("account://changed", session);
    }
    result
}
