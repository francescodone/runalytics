//! Cross-device plan sync over a private GitHub gist.
//!
//! The problem this solves: a plan created on the desktop is invisible on the
//! phone, because each device has its own SQLite database and nothing ever
//! travelled between them. Reading the plan back from COROS is not an option —
//! `queryTrainingPlanDetails` only reports *which days have a course*, not the
//! kind, title, distance or pace bands the plan actually prescribes.
//!
//! # How a device finds the other one
//!
//! Neither device is told a URL. Both derive the same anonymous *sync code*
//! from the COROS account id (`sub` in the OAuth access token), and the code is
//! the only thing that identifies the channel:
//!
//! ```text
//! code = hex(sha256("runalytics.plan.sync.v1" || sub))[..16]
//! ```
//!
//! The desktop creates a **secret** gist whose description is
//! `runalytics-plan:<code>`; the phone lists its own gists and picks the one
//! whose description matches. That is why there is no hardcoded URL and no
//! shared directory file: a directory gist would need a single writer and
//! would advertise who owns a plan, and for one runner's own two devices the
//! "map of user id to gist" has exactly one entry anyway.
//!
//! # Why the payload is encrypted
//!
//! A secret gist is secret only for as long as nobody makes it public, and a
//! plan is somebody's schedule, injuries-adjacent load, and daily routine. The
//! blob is therefore sealed with AES-256-GCM under a key derived from the same
//! `sub`. Reading a plan now needs two independent credentials from two
//! services: the gist body (which needs the runner's GitHub token or the
//! random gist id) *and* the COROS account id.
//!
//! # Direction
//!
//! One-way: desktop publishes, phone imports. The desktop is where plans are
//! created and where the COROS push happens, so making it the single writer
//! removes merge conflicts entirely. `rev` (the plan's creation timestamp)
//! guards against a pull overwriting a newer local plan.

use std::time::Duration;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};
use crate::state::AppState;
use crate::types::{PlanWorkout, TrainingPlan};

/// Version of the on-disk/on-gist wire format. Bump when the DTO shape changes
/// in a way older apps cannot read.
const WIRE_VERSION: u32 = 1;
/// Domain separation for both the sync code and the encryption key, so neither
/// can collide with any other hash or KDF this app might do with the same `sub`.
const PROTOCOL: &[u8] = b"runalytics.plan.sync.v1";
/// Gist file holding the sealed blob.
const BLOB_FILE: &str = "plan.json";
/// Gist description prefix that marks a gist as ours.
const MARKER: &str = "runalytics-plan:";
/// Keychain service for the GitHub token used to read/write the gist.
const KEYRING_SERVICE: &str = "com.fradone.runalytics.plan";
const KEYRING_TOKEN_USER: &str = "github_token";
/// Where the COROS OAuth blob lives — the source of the account id.
const COROS_SERVICE: &str = "com.fradone.runalytics.coros";
const COROS_USER: &str = "oauth";
/// A plan is ~40 workouts; this bounds a pathological gist response.
const MAX_BLOB_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PlanSync {
    pub enabled: bool,
    /// Gist the blob lives in. Discovered automatically; shown so the runner
    /// can recognise the channel they are about to trust.
    pub gist_id: Option<String>,
    pub has_token: bool,
    /// True on the machine that writes. Set from the platform, not the user.
    pub can_publish: bool,
    pub last_pushed: Option<String>,
    pub last_pulled: Option<String>,
    /// Revision currently installed on this device, so a pull never downgrades.
    pub installed_rev: Option<i64>,
    pub last_error: Option<String>,
}

// ---- wire format -----------------------------------------------------------

/// What travels in the gist. Deliberately its own DTOs rather than
/// [`TrainingPlan`]/[`PlanWorkout`]: those are `Serialize`-only IPC shapes, and
/// a persisted wire format needs a version and the freedom to change without
/// touching the in-app types.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlanBlob {
    v: u32,
    /// Monotonic-ish identity for "is this newer than mine".
    rev: i64,
    /// Seconds since epoch when this blob was built, for the UI.
    synced_at: i64,
    plan: PlanWire,
    workouts: Vec<WorkoutWire>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlanWire {
    id: String,
    distance: crate::types::RaceDistance,
    #[serde(default = "crate::types::PlanKind::default_race")]
    kind: crate::types::PlanKind,
    target_date: NaiveDate,
    status: crate::types::PlanStatus,
    weekly_frequency: u32,
    longrun_day: u32,
    increase: crate::types::IncreaseMethod,
    threshold_sec_km: f64,
    created_at: DateTime<Utc>,
    #[serde(default)]
    pushed_id: Option<String>,
    #[serde(default)]
    baseline_weekly_km: Option<f64>,
    #[serde(default)]
    quality_offset_sec_km: f64,
    #[serde(default)]
    quality_offset_from: Option<NaiveDate>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkoutWire {
    id: String,
    plan_id: String,
    date: NaiveDate,
    kind: crate::types::WorkoutKind,
    title: String,
    #[serde(default)]
    distance_m: Option<f64>,
    #[serde(default)]
    duration_min: Option<f64>,
    #[serde(default)]
    pace_lo_s_km: Option<i64>,
    #[serde(default)]
    pace_hi_s_km: Option<i64>,
}

impl PlanWire {
    fn from_plan(p: &TrainingPlan) -> Self {
        PlanWire {
            id: p.id.clone(),
            distance: p.distance,
            kind: p.kind,
            target_date: p.target_date,
            status: p.status,
            weekly_frequency: p.weekly_frequency,
            longrun_day: p.longrun_day,
            increase: p.increase,
            threshold_sec_km: p.threshold_sec_km,
            created_at: p.created_at,
            pushed_id: p.pushed_id.clone(),
            baseline_weekly_km: p.baseline_weekly_km,
            quality_offset_sec_km: p.quality_offset_sec_km,
            quality_offset_from: p.quality_offset_from,
        }
    }

    fn to_plan(&self) -> TrainingPlan {
        TrainingPlan {
            id: self.id.clone(),
            distance: self.distance,
            kind: self.kind,
            target_date: self.target_date,
            status: self.status,
            weekly_frequency: self.weekly_frequency,
            longrun_day: self.longrun_day,
            increase: self.increase,
            threshold_sec_km: self.threshold_sec_km,
            created_at: self.created_at,
            pushed_id: self.pushed_id.clone(),
            baseline_weekly_km: self.baseline_weekly_km,
            quality_offset_sec_km: self.quality_offset_sec_km,
            quality_offset_from: self.quality_offset_from,
        }
    }
}

impl WorkoutWire {
    fn from_workout(w: &PlanWorkout) -> Self {
        WorkoutWire {
            id: w.id.clone(),
            plan_id: w.plan_id.clone(),
            date: w.date,
            kind: w.kind,
            title: w.title.clone(),
            distance_m: w.distance_m,
            duration_min: w.duration_min,
            pace_lo_s_km: w.pace_lo_s_km,
            pace_hi_s_km: w.pace_hi_s_km,
        }
    }

    /// `done` is deliberately not part of the wire format: completion is
    /// derived locally by matching synced activities, and copying another
    /// device's flag would fight that matcher. This re-applies *this* device's
    /// answer for workouts it has already graded.
    fn to_workout(&self, done_here: bool) -> PlanWorkout {
        PlanWorkout {
            id: self.id.clone(),
            plan_id: self.plan_id.clone(),
            date: self.date,
            kind: self.kind,
            title: self.title.clone(),
            distance_m: self.distance_m,
            duration_min: self.duration_min,
            pace_lo_s_km: self.pace_lo_s_km,
            pace_hi_s_km: self.pace_hi_s_km,
            done: done_here,
        }
    }
}

// ---- config persistence ----------------------------------------------------

/// What the interface is allowed to change. Every other field on [`PlanSync`]
/// is either derived from the platform or recorded by the backend, so accepting
/// a whole `PlanSync` from the frontend would let a save wipe the channel id,
/// the timestamps, and the revision guard that stops a downgrade.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSyncPatch {
    pub enabled: bool,
}

pub fn load(directory: &std::path::Path) -> Result<PlanSync> {
    match std::fs::read(directory.join("plan-sync.json")) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(PlanSync::default()),
        Err(error) => Err(error.into()),
    }
}

fn save(directory: &std::path::Path, config: &PlanSync) -> Result<()> {
    let tmp = directory.join("plan-sync.json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(config)?)?;
    std::fs::rename(tmp, directory.join("plan-sync.json"))?;
    Ok(())
}

fn token_entry() -> Result<keyring_core::Entry> {
    crate::secure_store::entry(KEYRING_SERVICE, KEYRING_TOKEN_USER)
        .map_err(|_| AppError::Other("Plan sync Keychain entry unavailable".into()))
}

fn stored_token() -> Option<String> {
    token_entry().ok()?.get_password().ok()
}

// ---- identity --------------------------------------------------------------

/// The COROS account id, read from the OAuth blob already in the Keychain.
///
/// Parsed from the JWT payload without any network call, so sync works offline
/// and never needs a fresh token. Returns `None` when COROS has never been
/// connected on this device, which is the app's honest signal that there is no
/// shared identity to sync against yet.
fn account_id() -> Option<String> {
    let blob = crate::secure_store::entry(COROS_SERVICE, COROS_USER)
        .ok()?
        .get_password()
        .ok()?;
    let stored: serde_json::Value = serde_json::from_str(&blob).ok()?;
    let access = stored
        .get("token_response")?
        .get("access_token")?
        .as_str()?;
    jwt_sub(access)
}

/// The `sub` claim of a JWT, verified against nothing — it is only ever used
/// here as an opaque, stable identifier, and the token came from our own
/// Keychain. Signature verification would need the COROS JWKS and buys nothing
/// for a value we never use to authorise anything.
fn jwt_sub(token: &str) -> Option<String> {
    let payload = token.split('.').nth(1)?;
    let json = base64url_decode(payload).ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&json).ok()?;
    let sub = claims.get("sub")?.as_str()?;
    (!sub.is_empty()).then(|| sub.to_string())
}

/// The anonymous channel code. 16 hex chars is 64 bits — far more than enough
/// to make a collision between two runners' accounts impossible, and short
/// enough to read on a phone screen when something goes wrong.
fn sync_code(sub: &str) -> String {
    let mut data = PROTOCOL.to_vec();
    data.extend_from_slice(sub.as_bytes());
    let digest = aws_lc_rs::digest::digest(&aws_lc_rs::digest::SHA256, &data);
    digest
        .as_ref()
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// AES key for the blob, from the same `sub`. HKDF-Extract then a single
/// Expand block, which for HKDF-SHA256 yields exactly the 32 bytes AES-256
/// needs.
fn blob_key(sub: &str) -> Result<[u8; 32]> {
    use aws_lc_rs::hkdf;
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, PROTOCOL);
    let prk = salt.extract(sub.as_bytes());
    let info = [b"plan-blob.aes-256-gcm".as_slice()];
    let okm = prk
        .expand(&info, hkdf::HKDF_SHA256)
        .map_err(|_| AppError::Other("Could not derive the plan encryption key".into()))?;
    let mut key = [0u8; 32];
    okm.fill(&mut key)
        .map_err(|_| AppError::Other("Could not derive the plan encryption key".into()))?;
    Ok(key)
}

/// Seal a blob into the text stored in the gist: `v1.<b64(nonce)>.<b64(ciphertext||tag)>`.
fn seal(sub: &str, plaintext: &[u8]) -> Result<String> {
    use aws_lc_rs::aead::{Aad, RandomizedNonceKey, AES_256_GCM};
    let key = blob_key(sub)?;
    let sealer = RandomizedNonceKey::new(&AES_256_GCM, &key)
        .map_err(|_| AppError::Other("Could not encrypt the plan".into()))?;
    let mut in_out = plaintext.to_vec();
    let nonce = sealer
        .seal_in_place_append_tag(Aad::empty(), &mut in_out)
        .map_err(|_| AppError::Other("Could not encrypt the plan".into()))?;
    Ok(format!(
        "v1.{}.{}",
        base64url_encode(nonce.as_ref()),
        base64url_encode(&in_out)
    ))
}

fn open(sub: &str, sealed: &str) -> Result<Vec<u8>> {
    use aws_lc_rs::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
    let mut parts = sealed.split('.');
    let (Some(version), Some(nonce_b64), Some(body_b64)) = (parts.next(), parts.next(), parts.next())
    else {
        return Err(AppError::Other(
            "The synced plan is not in a format this version understands".into(),
        ));
    };
    if version != "v1" {
        return Err(AppError::Other(format!(
            "The synced plan uses format {version}, which this app cannot read"
        )));
    }
    if parts.next().is_some() {
        return Err(AppError::Other("The synced plan is malformed".into()));
    }
    let nonce_bytes = base64url_decode(nonce_b64)?;
    let mut in_out = base64url_decode(body_b64)?;
    if in_out.len() > MAX_BLOB_BYTES {
        return Err(AppError::Other("The synced plan is unexpectedly large".into()));
    }
    let key = blob_key(sub)?;
    let opener = LessSafeKey::new(
        UnboundKey::new(&AES_256_GCM, &key)
            .map_err(|_| AppError::Other("Could not decrypt the plan".into()))?,
    );
    let nonce = Nonce::try_assume_unique_for_key(&nonce_bytes)
        .map_err(|_| AppError::Other("Could not decrypt the plan".into()))?;
    let plain = opener
        .open_in_place(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| {
            AppError::Other(
                "The synced plan could not be decrypted — is this the same COROS account?"
                    .into(),
            )
        })?;
    Ok(plain.to_vec())
}

// ---- GitHub ---------------------------------------------------------------

fn github_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| AppError::Other("Could not start the network client".into()))
}

fn github_headers(request: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    request
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "Runalytics")
        .bearer_auth(token)
}

/// The gist we are looking for, among the ones this token can see. Matching on
/// the description rather than the name means neither device stores a URL.
async fn find_gist(client: &reqwest::Client, token: &str, code: &str) -> Result<Option<String>> {
    let want = format!("{MARKER}{code}");
    for page in 1..=3u32 {
        // Plain /gists is "the authenticated user's gists" when sent with a
        // token — there is no /user/gists route, and GitHub answers 404.
        let url = format!("https://api.github.com/gists?per_page=100&page={page}");
        let response = github_headers(client.get(&url), token)
            .send()
            .await
            .map_err(|_| AppError::Other("Could not reach GitHub".into()))?;
        if !response.status().is_success() {
            return Err(AppError::Other(format!(
                "GitHub returned HTTP {} while looking for the synced plan",
                response.status().as_u16()
            )));
        }
        let gists: serde_json::Value = response
            .json()
            .await
            .map_err(|_| AppError::Other("GitHub returned an unreadable response".into()))?;
        let Some(list) = gists.as_array() else {
            return Ok(None);
        };
        for gist in list {
            if gist.get("description").and_then(|d| d.as_str()) == Some(want.as_str()) {
                if let Some(id) = gist.get("id").and_then(|i| i.as_str()) {
                    return Ok(Some(id.to_string()));
                }
            }
        }
        // Fewer than a full page means there is no next page.
        if list.len() < 100 {
            return Ok(None);
        }
    }
    Ok(None)
}

async fn create_gist(
    client: &reqwest::Client,
    token: &str,
    code: &str,
    body: &str,
) -> Result<String> {
    let mut files = serde_json::Map::new();
    files.insert(BLOB_FILE.into(), serde_json::json!({ "content": body }));
    let response = github_headers(
        client.post("https://api.github.com/gists").json(&serde_json::json!({
            "description": format!("{MARKER}{code}"),
            // Secret, not public: the payload is encrypted, but there is no
            // reason for a plan to be world-readable even so.
            "public": false,
            "files": files,
        })),
        token,
    )
    .send()
    .await
    .map_err(|_| AppError::Other("Could not create the plan gist on GitHub".into()))?;
    if !response.status().is_success() {
        return Err(AppError::Other(format!(
            "GitHub refused to create the plan gist (HTTP {})",
            response.status().as_u16()
        )));
    }
    let created: serde_json::Value = response
        .json()
        .await
        .map_err(|_| AppError::Other("GitHub returned an unreadable response".into()))?;
    created
        .get("id")
        .and_then(|id| id.as_str())
        .map(str::to_string)
        .ok_or_else(|| AppError::Other("GitHub did not return the new gist id".into()))
}

async fn patch_gist(
    client: &reqwest::Client,
    token: &str,
    gist_id: &str,
    body: &str,
) -> Result<()> {
    let mut files = serde_json::Map::new();
    files.insert(BLOB_FILE.into(), serde_json::json!({ "content": body }));
    let response = github_headers(
        client
            .patch(format!("https://api.github.com/gists/{gist_id}"))
            .json(&serde_json::json!({ "files": files })),
        token,
    )
    .send()
    .await
    .map_err(|_| AppError::Other("Could not update the plan gist on GitHub".into()))?;
    if !response.status().is_success() {
        return Err(AppError::Other(format!(
            "GitHub refused to update the plan gist (HTTP {})",
            response.status().as_u16()
        )));
    }
    Ok(())
}

/// The sealed text stored in the gist. The gist API returns small files inline,
/// which avoids a second request and the CDN cache the raw URL sits behind.
async fn fetch_blob(client: &reqwest::Client, token: &str, gist_id: &str) -> Result<String> {
    let url = format!("https://api.github.com/gists/{gist_id}");
    let response = github_headers(client.get(&url), token)
        .send()
        .await
        .map_err(|_| AppError::Other("Could not reach GitHub".into()))?;
    if !response.status().is_success() {
        return Err(AppError::Other(format!(
            "GitHub returned HTTP {} for the synced plan",
            response.status().as_u16()
        )));
    }
    let gist: serde_json::Value = response
        .json()
        .await
        .map_err(|_| AppError::Other("GitHub returned an unreadable response".into()))?;
    let file = &gist["files"][BLOB_FILE];
    if file["truncated"] == true {
        let raw = file["raw_url"]
            .as_str()
            .ok_or_else(|| AppError::Other("The synced plan has no download URL".into()))?;
        let body = github_headers(client.get(raw), token)
            .send()
            .await
            .map_err(|_| AppError::Other("Could not download the synced plan".into()))?
            .error_for_status()
            .map_err(|_| AppError::Other("Could not download the synced plan".into()))?
            .text()
            .await
            .map_err(|_| AppError::Other("Could not download the synced plan".into()))?;
        return Ok(body);
    }
    file["content"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| AppError::Other("The synced plan gist has no plan file".into()))
}

// ---- publish / pull --------------------------------------------------------

/// Build, seal and upload the active plan. Called automatically after any plan
/// change on the desktop, and by the explicit button in Settings.
///
/// Silent by design when sync is not configured: the plan-change call sites are
/// on the hot path of creating a plan, and a runner who has never opened this
/// setting must never see an error from it.
pub async fn push_if_enabled(state: &AppState) {
    let config = load(&state.data_dir).unwrap_or_default();
    if !config.enabled || !config.can_publish {
        return;
    }
    match push(state, false).await {
        Ok(_) => {}
        Err(error) => {
            eprintln!("[plan-sync] automatic push failed: {error}");
            let mut retry = config;
            retry.last_error = Some(error.to_string());
            let _ = save(&state.data_dir, &retry);
        }
    }
}

/// Seal and upload the current plan, persisting the resulting config. `explicit`
/// is true when the runner pressed the button, which is the only time a missing
/// prerequisite is worth an error.
pub async fn push(state: &AppState, explicit: bool) -> Result<PlanSync> {
    let result = push_inner(state, explicit).await;
    match result {
        Ok(config) => {
            save(&state.data_dir, &config)?;
            Ok(config)
        }
        Err(error) => Err(error),
    }
}

async fn push_inner(state: &AppState, explicit: bool) -> Result<PlanSync> {
    let mut config = load(&state.data_dir)?;
    config.can_publish = !mobile();
    if !config.enabled {
        if explicit {
            return Err(AppError::Other("Plan sync is switched off".into()));
        }
        return Ok(config);
    }
    let token = stored_token().ok_or_else(|| {
        AppError::Other("Add your GitHub token in Settings → Plan sync to publish a plan".into())
    })?;
    let sub = account_id().ok_or_else(|| {
        AppError::Other("Connect COROS on both devices first — it is how they find each other".into())
    })?;

    let Some((plan, workouts)) = state.store.current_plan()? else {
        if explicit {
            return Err(AppError::Other("There is no active plan to sync".into()));
        }
        // Nothing to publish is not an error; leave the previous blob alone.
        return Ok(config);
    };

    let code = sync_code(&sub);
    let client = github_client()?;
    let gist_id = match config.gist_id.clone() {
        Some(id) => Some(id),
        None => find_gist(&client, &token, &code).await?,
    };

    let blob = PlanBlob {
        v: WIRE_VERSION,
        rev: plan.created_at.timestamp(),
        synced_at: Utc::now().timestamp(),
        plan: PlanWire::from_plan(&plan),
        workouts: workouts.iter().map(WorkoutWire::from_workout).collect(),
    };
    let sealed = seal(&sub, &serde_json::to_vec(&blob)?)?;

    match gist_id.as_deref() {
        Some(id) => patch_gist(&client, &token, id, &sealed).await?,
        None => config.gist_id = Some(create_gist(&client, &token, &code, &sealed).await?),
    }
    config.last_pushed = Some(Utc::now().to_rfc3339());
    config.last_error = None;
    Ok(config)
}

/// Outcome of a pull, so the UI can say what actually happened rather than
/// guessing from a boolean.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullOutcome {
    pub installed: bool,
    pub plan_name: String,
    pub workouts: usize,
    /// Set when a *different* plan is already here and `force` was not passed.
    pub conflict: Option<String>,
}

/// Fetch, decrypt and install the synced plan.
///
/// `force` replaces a plan that is already here. Without it an existing,
/// different plan is reported as a conflict instead of being destroyed — the
/// phone may have its own plan the runner just made, and silently replacing it
/// is the worst thing a sync could do.
pub async fn pull(state: &AppState, force: bool) -> Result<(PlanSync, PullOutcome)> {
    let mut config = load(&state.data_dir)?;
    let token = stored_token().ok_or_else(|| {
        AppError::Other("Add your GitHub token in Settings → Plan sync to fetch a plan".into())
    })?;
    let sub = account_id().ok_or_else(|| {
        AppError::Other("Connect COROS on this device first — it is how it finds your plan".into())
    })?;
    let code = sync_code(&sub);
    let client = github_client()?;
    let gist_id = find_gist(&client, &token, &code)
        .await?
        .ok_or_else(|| {
            AppError::Other(
                "No synced plan found for this COROS account. Publish one from your computer."
                    .into(),
            )
        })?;
    config.gist_id = Some(gist_id.clone());
    save(&state.data_dir, &config)?;

    let sealed = fetch_blob(&client, &token, &gist_id).await?;
    let blob: PlanBlob = serde_json::from_slice(&open(&sub, &sealed)?)?;
    if blob.v > WIRE_VERSION {
        return Err(AppError::Other(
            "The synced plan was written by a newer version of Runalytics".into(),
        ));
    }
    if blob.workouts.is_empty() {
        return Err(AppError::Other("The synced plan has no workouts".into()));
    }

    let incoming = blob.plan.to_plan();
    if let Some((existing, _)) = state.store.current_plan()? {
        if existing.id != incoming.id && !force {
            config.last_error = None;
            return Ok((
                config,
                PullOutcome {
                    installed: false,
                    plan_name: String::new(),
                    workouts: 0,
                    conflict: Some(existing.id),
                },
            ));
        }
        // Same plan arriving again is a refresh, not a downgrade — but an older
        // *different* plan must never overwrite a newer one.
        if existing.id == incoming.id && blob.rev < config.installed_rev.unwrap_or(i64::MIN) {
            return Ok((
                config,
                PullOutcome {
                    installed: false,
                    plan_name: String::new(),
                    workouts: 0,
                    conflict: None,
                },
            ));
        }
    }

    // Keep this device's own completion answers for workouts it has already
    // matched to a run.
    let done_here: std::collections::HashSet<String> = state
        .store
        .current_plan()?
        .map(|(_, ws)| ws.into_iter().filter(|w| w.done).map(|w| w.id).collect())
        .unwrap_or_default();
    let workouts: Vec<PlanWorkout> = blob
        .workouts
        .iter()
        .map(|w| w.to_workout(done_here.contains(&w.id)))
        .collect();

    state.store.replace_plan(&incoming, &workouts)?;
    config.installed_rev = Some(blob.rev);
    config.last_pulled = Some(Utc::now().to_rfc3339());
    config.last_error = None;
    save(&state.data_dir, &config)?;

    Ok((
        config,
        PullOutcome {
            installed: true,
            plan_name: plan_summary(&incoming),
            workouts: workouts.len(),
            conflict: None,
        },
    ))
}

/// How the UI refers to a plan: what it is for and when it ends. Enough to
/// tell two plans apart in a confirmation, without assuming a name field that
/// the plan does not have.
fn plan_summary(plan: &TrainingPlan) -> String {
    format!(
        "{} on {}",
        plan.distance.label(),
        plan.target_date.format("%d %b %Y")
    )
}

/// Called after a sync completes on mobile: if this device has no plan but a
/// sync channel is configured, fetch the one the desktop published.
///
/// This is what makes the phone stop showing "create a plan" when the runner
/// already has one. Only mobile pulls — the desktop is the writer, and letting
/// it pull would race with its own edits.
pub async fn pull_if_missing(state: &AppState) {
    if !mobile() {
        return;
    }
    let config = load(&state.data_dir).unwrap_or_default();
    if !config.enabled || state.store.current_plan().map(|p| p.is_some()).unwrap_or(true) {
        return;
    }
    match pull(state, false).await {
        Ok((updated, outcome)) => {
            if outcome.installed {
                // The plan view is gated on having a plan, so say what landed.
                eprintln!(
                    "[plan-sync] installed {} workouts from {}",
                    outcome.workouts, outcome.plan_name
                );
            }
            let _ = save(&state.data_dir, &updated);
        }
        Err(error) => eprintln!("[plan-sync] automatic pull failed: {error}"),
    }
}

/// Whether the config pane can be shown at all: both a GitHub token and a
/// connected COROS account are required, and the UI says which one is missing.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSyncStatus {
    pub config: PlanSync,
    /// True when COROS has been connected, i.e. a shared identity exists.
    pub account_linked: bool,
    /// The channel code, so the two devices can be eyeballed as matching when
    /// something is wrong. Not a secret on its own — it is a hash of an id held
    /// only in each device's Keychain.
    pub code: Option<String>,
}

pub fn status(state: &AppState) -> Result<PlanSyncStatus> {
    let mut config = load(&state.data_dir)?;
    config.can_publish = !mobile();
    config.has_token = stored_token().is_some();
    let sub = account_id();
    Ok(PlanSyncStatus {
        account_linked: sub.is_some(),
        code: sub.as_deref().map(sync_code),
        config,
    })
}

/// Save the pane's settings. `token` follows the same convention as calendar
/// publishing: `None` leaves the stored token alone, `Some("")` removes it.
pub fn configure(state: &AppState, patch: PlanSyncPatch, token: Option<String>) -> Result<PlanSyncStatus> {
    let mut config = load(&state.data_dir)?;
    config.enabled = patch.enabled;
    config.can_publish = !mobile();
    // A token change invalidates a discovered gist: the new account may have a
    // different channel, and re-discovery is cheap.
    let token_changed = token.is_some();
    if let Some(token) = token {
        let entry = token_entry()?;
        if token.is_empty() {
            match entry.delete_credential() {
                Ok(()) | Err(keyring_core::Error::NoEntry) => {}
                Err(_) => {
                    return Err(AppError::Other("Could not clear the GitHub token".into()));
                }
            }
            config.has_token = false;
        } else {
            entry.set_password(token.trim()).map_err(|_| {
                AppError::Other("Could not save the GitHub token in the Keychain".into())
            })?;
            config.has_token = true;
        }
    } else {
        config.has_token = stored_token().is_some();
    }
    if token_changed {
        config.gist_id = None;
    }
    config.last_error = None;
    save(&state.data_dir, &config)?;
    status(state)
}

fn mobile() -> bool {
    cfg!(mobile)
}

// ---- base64url -------------------------------------------------------------
//
// Hand-rolled, matching the repo's existing approach in `providers::coros`.
// The gist payload and the JWT segment are the only things that need it and a
// dependency for two functions is not worth it.

fn base64url_encode(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(T[((n >> 6) & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(T[(n & 63) as usize] as char);
        }
    }
    out
}

fn base64url_decode(s: &str) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() / 4 * 3 + 3);
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    for c in s.bytes() {
        let Some(pos) = b64_index(c) else {
            // '=' padding and any whitespace terminate the data.
            break;
        };
        acc = (acc << 6) | u32::from(pos);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

/// Reverse of [`base64url_encode`]. Accepts the URL-safe alphabet the encoder
/// emits and, since JWT segments in the wild are written both ways, standard
/// base64 as well — which is why the lookup table is the standard one and the
/// two URL-safe characters are folded back into it.
fn b64_index(c: u8) -> Option<u8> {
    const STD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let c = match c {
        b'-' => b'+',
        b'_' => b'/',
        x => x,
    };
    STD.iter().position(|&x| x == c).map(|p| p as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn wire() -> PlanBlob {
        PlanBlob {
            v: WIRE_VERSION,
            rev: 1_700_000_000,
            synced_at: 1_700_000_100,
            plan: PlanWire {
                id: "plan-1".into(),
                distance: crate::types::RaceDistance::K21,
                kind: crate::types::PlanKind::Race,
                target_date: NaiveDate::from_ymd_opt(2026, 12, 6).unwrap(),
                status: crate::types::PlanStatus::Active,
                weekly_frequency: 4,
                longrun_day: 7,
                increase: crate::types::IncreaseMethod::Conservative,
                threshold_sec_km: 250.0,
                created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
                pushed_id: None,
                baseline_weekly_km: Some(32.5),
                quality_offset_sec_km: 0.0,
                quality_offset_from: None,
            },
            workouts: vec![WorkoutWire {
                id: "w-1".into(),
                plan_id: "plan-1".into(),
                date: NaiveDate::from_ymd_opt(2026, 10, 26).unwrap(),
                kind: crate::types::WorkoutKind::Tempo,
                title: "Tempo 6km".into(),
                distance_m: Some(6000.0),
                duration_min: None,
                pace_lo_s_km: Some(245),
                pace_hi_s_km: Some(252),
            }],
        }
    }

    #[test]
    fn sync_code_is_stable_and_account_specific() {
        let a = sync_code("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert_eq!(a.len(), 16);
        assert_eq!(a, sync_code("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        assert_ne!(a, sync_code("baaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
    }

    #[test]
    fn seal_round_trips_under_the_same_account() {
        let json = serde_json::to_vec(&wire()).unwrap();
        let sealed = seal("sub-abc", &json).unwrap();
        assert!(sealed.starts_with("v1."));
        // The plaintext must not appear in the wire form.
        assert!(!sealed.contains("Tempo 6km"));
        let opened = open("sub-abc", &sealed).unwrap();
        assert_eq!(opened, json);
    }

    #[test]
    fn seal_is_randomised_so_repeat_pushes_differ() {
        let json = serde_json::to_vec(&wire()).unwrap();
        assert_ne!(seal("sub-abc", &json).unwrap(), seal("sub-abc", &json).unwrap());
    }

    #[test]
    fn another_account_cannot_read_the_blob() {
        let sealed = seal("sub-abc", b"{\"secret\":true}").unwrap();
        let err = open("sub-xyz", &sealed).unwrap_err().to_string();
        assert!(err.contains("same COROS account"), "got: {err}");
    }

    #[test]
    fn tampering_with_the_blob_is_detected() {
        let sealed = seal("sub-abc", b"{\"secret\":true}").unwrap();
        let mut parts: Vec<String> = sealed.split('.').map(str::to_string).collect();
        let last = parts.len() - 1;
        let mut body = base64url_decode(&parts[last]).unwrap();
        let n = body.len();
        body[n - 1] ^= 0x01;
        parts[last] = base64url_encode(&body);
        assert!(open("sub-abc", &parts.join(".")).is_err());
    }

    #[test]
    fn unknown_format_version_is_rejected_not_guessed() {
        assert!(open("s", "v9.abc.def").unwrap_err().to_string().contains("cannot read"));
        assert!(open("s", "nonsense").unwrap_err().to_string().contains("format"));
        assert!(open("s", "v1.aa.bb.cc").unwrap_err().to_string().contains("malformed"));
    }

    #[test]
    fn jwt_sub_is_read_from_a_token_without_verification() {
        let header = base64url_encode(b"{\"alg\":\"RS256\"}");
        let payload = base64url_encode(b"{\"sub\":\"deadbeefdeadbeefdeadbeefdeadbeef\"}");
        assert_eq!(
            jwt_sub(&format!("{header}.{payload}.signature")).as_deref(),
            Some("deadbeefdeadbeefdeadbeefdeadbeef")
        );
        assert_eq!(jwt_sub("not-a-token"), None);
        assert_eq!(jwt_sub(&format!("{header}.{}", base64url_encode(b"{}"))), None);
    }

    #[test]
    fn base64url_round_trips_every_length() {
        for len in 0..=64usize {
            let data: Vec<u8> = (0..len).map(|i| (i * 37 + 11) as u8).collect();
            assert_eq!(base64url_decode(&base64url_encode(&data)).unwrap(), data, "len {len}");
        }
    }

    #[test]
    fn base64url_uses_no_padding_or_forbidden_chars() {
        let encoded = base64url_encode(&[0xff; 33]);
        assert!(!encoded.contains('='), "padding leaked into a URL segment");
        assert!(!encoded.contains('+') && !encoded.contains('/'));
    }

    #[test]
    fn blob_round_trips_through_json() {
        let json = serde_json::to_vec(&wire()).unwrap();
        let back: PlanBlob = serde_json::from_slice(&json).unwrap();
        assert_eq!(back.plan.id, "plan-1");
        assert_eq!(back.workouts[0].title, "Tempo 6km");
        assert_eq!(back.plan.baseline_weekly_km, Some(32.5));
    }

    #[test]
    fn blob_omitting_optional_fields_still_reads() {
        // A blob written by an older build must still load: the optionals are
        // #[serde(default)], not required. Keys are camelCase because that is
        // the wire shape — a snake_case fixture would test a format nobody
        // actually sends.
        let minimal = r#"{"v":1,"rev":1,"syncedAt":1,"plan":{"id":"p","distance":"21K",
            "targetDate":"2026-12-06","status":"active","weeklyFrequency":4,"longrunDay":7,
            "increase":"conservative","thresholdSecKm":250.0,"createdAt":"2026-10-01T00:00:00Z"},
            "workouts":[]}"#;
        let blob: PlanBlob = serde_json::from_str(minimal).unwrap();
        assert_eq!(blob.plan.quality_offset_sec_km, 0.0);
        assert_eq!(blob.plan.kind, crate::types::PlanKind::Race);
    }

    #[test]
    fn blob_keys_are_camel_case_end_to_end() {
        // The gist is a durable artefact: if the key style drifted between the
        // top level and the nested DTOs, an older app would fail to read a
        // newer blob in a way no compile could catch.
        let json = serde_json::to_string(&wire()).unwrap();
        for key in [
            "\"syncedAt\"",
            "\"targetDate\"",
            "\"weeklyFrequency\"",
            "\"paceLoSKm\"",
        ] {
            assert!(json.contains(key), "missing {key} in {json}");
        }
        assert!(!json.contains("target_date"), "snake_case leaked into the wire format");
    }

    #[test]
    fn marker_is_the_only_gist_matching_rule() {
        assert_eq!(format!("{MARKER}abc"), "runalytics-plan:abc");
        assert_eq!(BLOB_FILE, "plan.json");
    }
}
