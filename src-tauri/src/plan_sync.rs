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
//! Both directions, last-edit-wins. Either device may edit the plan — move a
//! session, skip it, mark it done against a run — and the edit is stamped with
//! a revision (the wall-clock second of the change, kept monotonic per device).
//! Every device pushes what it holds and pulls anything newer, so the gist is
//! the single point of truth the two devices converge on. Two devices edited
//! between syncs is resolved by revision: the later edit stands, which for one
//! runner's own two devices is the honest rule — the alternative is a merge
//! the runner would only ever resolve by picking one side anyway.
//!
//! Completion travels too, but not as a bare `done` flag: a completion is a
//! pair (planned session, activity), and the activity is identified by the
//! *provider's* record id rather than this device's row id. Both devices sync
//! the same watch independently and land on the same record ids, so a session
//! marked done on the phone resolves to the same run on the desktop.

use std::time::Duration;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};
use crate::state::AppState;
use crate::types::{PlanWorkout, TrainingPlan};

/// Version of the on-disk/on-gist wire format. Bump when the DTO shape changes
/// in a way older apps cannot read.
const WIRE_VERSION: u32 = 2;
/// Domain separation for both the sync code and the encryption key, so neither
/// can collide with any other hash or KDF this app might do with the same `sub`.
const PROTOCOL: &[u8] = b"runalytics.plan.sync.v1";
/// Gist file holding the sealed blob.
const BLOB_FILE: &str = "plan.json";
/// Gist description prefix that marks a gist as ours.
const MARKER: &str = "runalytics-plan:";
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
    pub reset_plan_id: Option<String>,
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
    /// The publisher's calendar subscription target, so the other device can
    /// start publishing to the same gist without being told the URLs twice.
    /// Only carried when the target is a gist — the plan channel authenticates
    /// with the same GitHub token, so a gist target is the only one the other
    /// device can adopt from this information alone.
    #[serde(default)]
    calendar: Option<CalendarWire>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CalendarWire {
    pub(crate) upload_url: String,
    pub(crate) subscription_url: String,
    pub(crate) username: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlanWire {
    id: String,
    distance: crate::types::RaceDistance,
    #[serde(default = "crate::types::PlanKind::default_race")]
    kind: crate::types::PlanKind,
    #[serde(default)]
    start_date: Option<NaiveDate>,
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
    #[serde(default)]
    threshold_state: Option<crate::plan::threshold::AthleteState>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkoutWire {
    id: String,
    plan_id: String,
    date: NaiveDate,
    /// The day the plan generated this session into. Optional because a blob
    /// published before this field existed has no such record, and the honest
    /// reading of one is "it was never moved" — so an absent value means the
    /// date *is* the generated date, which `to_workout` applies.
    #[serde(default)]
    planned_date: Option<NaiveDate>,
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
    /// A skip is a decision the runner made, not something this device can
    /// derive from its own syncs, so it belongs on the wire.
    #[serde(default)]
    status: crate::types::WorkoutStatus,
    /// Whether the session was marked done. Since the plan channel became
    /// two-way, a completion is a plan edit like any other: the phone marks a
    /// session done against a run and the desktop has to see it. A blob from
    /// the one-way era has no field here, and `false` is the honest reading —
    /// the importer's own matcher still grades its own synced runs.
    #[serde(default)]
    done: bool,
    /// Which run completed it, named by the provider's record id so the other
    /// device can resolve it against its own copy of the same watch data.
    /// `None` when done without a linked activity (a manual tick), which the
    /// importer still honours as done.
    #[serde(default)]
    completion: Option<CompletionWire>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct CompletionWire {
    /// The provider that owns the activity, e.g. `coros`.
    provider: String,
    /// That provider's id for the run — identical on every device that synced
    /// the same watch, which is what makes the pairing portable.
    record_id: String,
}

impl PlanWire {
    fn from_plan(p: &TrainingPlan) -> Self {
        PlanWire {
            id: p.id.clone(),
            distance: p.distance,
            kind: p.kind,
            start_date: p.start_date,
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
            threshold_state: p.threshold_state.clone(),
        }
    }

    fn to_plan(&self) -> TrainingPlan {
        TrainingPlan {
            id: self.id.clone(),
            distance: self.distance,
            kind: self.kind,
            start_date: self.start_date,
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
            threshold_state: self.threshold_state.clone(),
        }
    }
}

impl WorkoutWire {
    fn from_workout(w: &PlanWorkout, completion: Option<CompletionWire>) -> Self {
        WorkoutWire {
            id: w.id.clone(),
            plan_id: w.plan_id.clone(),
            date: w.date,
            planned_date: Some(w.planned_date),
            kind: w.kind,
            title: w.title.clone(),
            distance_m: w.distance_m,
            duration_min: w.duration_min,
            pace_lo_s_km: w.pace_lo_s_km,
            pace_hi_s_km: w.pace_hi_s_km,
            status: w.status,
            done: w.done,
            completion,
        }
    }

    /// `moved_at` is deliberately not on the wire: it records *when this
    /// device* saw the move, which a pull cannot know. The moved marker reads
    /// the date pair instead, so it survives the trip regardless.
    ///
    /// `done` here is the *wire's* answer; the pull ORs it with this device's
    /// own completions so a session matched locally is never un-ticked by an
    /// import, and the completion pairing is applied separately once the
    /// referenced activity is known locally.
    fn to_workout(&self) -> PlanWorkout {
        PlanWorkout {
            id: self.id.clone(),
            plan_id: self.plan_id.clone(),
            date: self.date,
            planned_date: self.planned_date.unwrap_or(self.date),
            kind: self.kind,
            title: self.title.clone(),
            distance_m: self.distance_m,
            duration_min: self.duration_min,
            pace_lo_s_km: self.pace_lo_s_km,
            pace_hi_s_km: self.pace_hi_s_km,
            done: self.done,
            status: self.status,
            moved_at: None,
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

pub fn remember_reset(directory: &std::path::Path, plan_id: &str) -> Result<()> {
    let mut config = load(directory)?;
    config.reset_plan_id = Some(plan_id.to_owned());
    save(directory, &config)
}

fn is_reset_plan(config: &PlanSync, plan_id: &str, force: bool) -> bool {
    !force && config.reset_plan_id.as_deref() == Some(plan_id)
}

/// The shared GitHub credential — the same one calendar gist publishing uses.
fn token_entry() -> Result<keyring_core::Entry> {
    crate::secure_store::entry(crate::secure_store::GITHUB_SERVICE, crate::secure_store::GITHUB_TOKEN_USER)
        .map_err(|_| AppError::Other("GitHub Keychain entry unavailable".into()))
}

fn legacy_token_entry() -> Option<keyring_core::Entry> {
    crate::secure_store::entry(
        crate::secure_store::LEGACY_PLAN_SERVICE,
        crate::secure_store::LEGACY_PLAN_TOKEN_USER,
    )
    .ok()
}

/// The token to authenticate with, preferring the shared entry. An install
/// that only ever saved it for plan sync is moved across on first read, so the
/// merge costs the runner nothing — and moving rather than copying keeps
/// clearing unambiguous, otherwise a deleted token would come back.
pub(crate) fn stored_token() -> Option<String> {
    let shared = token_entry().ok()?.get_password().ok();
    if shared.is_some() {
        return shared;
    }
    let legacy = legacy_token_entry()?.get_password().ok()?;
    if legacy.is_empty() {
        return None;
    }
    // If the copy fails the legacy read still works, so nothing is lost.
    if token_entry().ok()?.set_password(&legacy).is_ok() {
        let _ = legacy_token_entry()?.delete_credential();
    }
    Some(legacy)
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
    // Both devices write: the phone's edits (a skip, a move, a session marked
    // done against the morning run) have to reach the gist or the desktop
    // never sees them. Which copy wins is decided by revision at pull time,
    // not by which platform pressed the button.
    config.can_publish = true;
    if !config.enabled {
        if explicit {
            return Err(AppError::Other("Plan sync is switched off".into()));
        }
        return Ok(config);
    }
    let token = stored_token().ok_or_else(|| {
        AppError::Other(
            "Add your GitHub token in Settings → Calendar subscription to publish a plan".into(),
        )
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

    // Pair each done session with the run that completed it, named by the
    // provider's record id so the other device can resolve the same activity.
    let records = state.store.calendar_completion_records()?;
    let completion_of = |workout: &PlanWorkout| -> Option<CompletionWire> {
        records
            .iter()
            .find(|(workout_id, _, _)| workout_id == &workout.id)
            .map(|(_, provider, record_id)| CompletionWire {
                provider: provider.clone(),
                record_id: record_id.clone(),
            })
    };

    let code = sync_code(&sub);
    let client = github_client()?;
    let gist_id = match config.gist_id.clone() {
        Some(id) => Some(id),
        None => find_gist(&client, &token, &code).await?,
    };

    let blob = PlanBlob {
        v: WIRE_VERSION,
        // The plan's own last-edit revision, not its creation time: two
        // devices decide which copy is newer by comparing when they were last
        // edited, and creation time cannot express an edit at all.
        rev: state
            .store
            .current_plan_rev()?
            .unwrap_or_else(|| plan.created_at.timestamp()),
        synced_at: Utc::now().timestamp(),
        plan: PlanWire::from_plan(&plan),
        workouts: workouts
            .iter()
            .map(|w| WorkoutWire::from_workout(w, completion_of(w)))
            .collect(),
        calendar: calendar_target(&state.data_dir),
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

/// This device's calendar publishing target, when it is a gist. Carried on
/// the plan blob so a second device can adopt the same subscription without
/// being configured by hand; a WebDAV target is deliberately not carried,
/// because the plan channel's GitHub token cannot write to it.
fn calendar_target(directory: &std::path::Path) -> Option<CalendarWire> {
    let publishing = crate::calendar_publish::load(directory).ok()?;
    if !publishing.enabled || publishing.upload_url.is_empty() {
        return None;
    }
    let upload = reqwest::Url::parse(&publishing.upload_url).ok()?;
    let segments: Vec<_> = upload.path_segments()?.collect();
    matches!(segments.as_slice(), [.., "gists", id] if !id.is_empty()).then(|| CalendarWire {
        upload_url: publishing.upload_url,
        subscription_url: publishing.subscription_url,
        username: publishing.username,
    })
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
        AppError::Other(
            "Add your GitHub token in Settings → Calendar subscription to fetch a plan".into(),
        )
    })?;
    let sub = account_id().ok_or_else(|| {
        AppError::Other("Connect COROS on this device first — it is how it finds your plan".into())
    })?;
    let code = sync_code(&sub);
    let client = github_client()?;
    let gist_id = match config.gist_id.clone() {
        Some(id) => id,
        None => find_gist(&client, &token, &code).await?.ok_or_else(|| {
            AppError::Other(
                "No synced plan found for this COROS account. Publish one from your computer."
                    .into(),
            )
        })?,
    };
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
    if is_reset_plan(&config, &incoming.id, force) {
        return Ok((config, PullOutcome {
            installed: false,
            plan_name: String::new(),
            workouts: 0,
            conflict: Some(incoming.id),
        }));
    }
    let local_rev = state
        .store
        .current_plan_rev()?
        .unwrap_or(i64::MIN);
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
        // Same plan arriving again is only worth installing when it is a
        // strictly newer revision — the last-edit stamp, not the creation
        // time, is what says so. An equal or older copy is the channel
        // holding what we already have (or published ourselves), and
        // re-installing it would only churn.
        if existing.id == incoming.id && blob.rev <= local_rev {
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

    // A session counts as done if either copy says so: the wire's answer, or
    // this device's own completion the matcher produced from its own sync.
    // Un-ticking a session the runner has already run would be worse than any
    // conflict the revision rule could produce.
    let done_here: std::collections::HashSet<String> = state
        .store
        .current_plan()?
        .map(|(_, ws)| ws.into_iter().filter(|w| w.done).map(|w| w.id).collect())
        .unwrap_or_default();
    let workouts: Vec<PlanWorkout> = blob
        .workouts
        .iter()
        .map(|w| {
            let mut workout = w.to_workout();
            workout.done = workout.done || done_here.contains(&w.id);
            workout
        })
        .collect();

    state.store.replace_plan(&incoming, &workouts, blob.rev)?;

    // Re-attach the completions that travelled with the plan. Each names its
    // run by the provider's record id, which both devices hold after syncing
    // the same watch; a run the phone has not synced here yet is skipped and
    // picked up by a later pull once it has (the completion stays on the
    // wire until both ends agree). Workouts already completed locally are
    // left exactly as they are — re-inserting would fight the UNIQUE
    // constraint and re-stamping would churn the revision for nothing.
    let already: std::collections::HashSet<String> = state
        .store
        .calendar_completions()?
        .into_iter()
        .map(|(workout_id, _, _)| workout_id)
        .collect();
    for wire in &blob.workouts {
        let Some(completion) = wire.completion.as_ref() else {
            continue;
        };
        if already.contains(&wire.id) {
            continue;
        }
        let Ok(Some(activity_id)) = state
            .store
            .activity_id_by_record(&completion.provider, &completion.record_id)
        else {
            continue;
        };
        let Ok(workouts) = state.store.plan_workouts(&incoming.id) else {
            continue;
        };
        let Some(workout) = workouts.iter().find(|w| w.id == wire.id) else {
            continue;
        };
        let notes = crate::calendar::workout_notes(workout, &incoming);
        if let Err(error) = state
            .store
            .complete_calendar_workout(&wire.id, activity_id, &notes)
        {
            eprintln!("[plan-sync] could not attach completion for {}: {error}", wire.id);
        }
    }

    // A device that never configured calendar publishing can still take part:
    // if the publisher's subscription target is a gist — writable with the
    // same GitHub token this pull already authenticated with — adopt it, so
    // this device's syncs keep the same subscription fresh without being
    // typed into twice.
    if let Some(calendar) = blob.calendar.as_ref() {
        crate::calendar_publish::adopt_gist_target(state, calendar).await;
    }

    if force && config.reset_plan_id.as_deref() == Some(incoming.id.as_str()) {
        config.reset_plan_id = None;
    }
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

/// What the sync channel holds, as seen from this device. The read-only
/// counterpart of [`pull`]: a status pill has to be honest without installing
/// anything, and `pull` is the only path that mutates the plan.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePlan {
    pub status: PillStatus,
    /// The published plan described the way a confirmation would name it.
    pub plan_name: String,
    pub workouts: usize,
    /// Seconds since epoch when the blob was published, for "2 hours ago".
    pub synced_at: i64,
}

/// How the published plan relates to *this* device, phrased so each surface
/// can say it in its own voice. Both ends read the same verdicts: whichever
/// device holds the newer revision publishes, and whichever holds the older
/// one imports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PillStatus {
    /// The channel holds exactly what this device has.
    UpToDate,
    /// This device holds the newer copy, so what is here has not been
    /// published yet (or the last push failed).
    Unpublished,
    /// The channel holds a newer revision of this same plan to import.
    UpdateAvailable,
    /// The channel holds a newer *different* plan than the one here;
    /// importing it replaces the local plan, which needs a confirmation.
    OtherPlan,
    /// Nothing is installed here and the channel has a plan waiting.
    ImportAvailable,
}

/// The relation between the published blob and this device's plan, computed
/// from the same revision rule [`pull`] enforces — the pill must never
/// promise an import that the monotonic guard would then refuse. One rule
/// for both devices now that both can write: the higher revision is the
/// current copy, and the other end's action is to take it.
fn pill_status(
    local: Option<(&TrainingPlan, i64)>,
    blob_plan_id: &str,
    blob_rev: i64,
) -> PillStatus {
    let Some((plan, local_rev)) = local else {
        return PillStatus::ImportAvailable;
    };
    if blob_rev > local_rev {
        // The channel is ahead: same plan means an update to take, a
        // different plan means the other device moved on to something else.
        return if plan.id == blob_plan_id {
            PillStatus::UpdateAvailable
        } else {
            PillStatus::OtherPlan
        };
    }
    if plan.id != blob_plan_id || blob_rev < local_rev {
        // Either a different (older) plan is all the channel has, or ours is
        // simply not published yet. Both read as "publish this one".
        return PillStatus::Unpublished;
    }
    PillStatus::UpToDate
}

/// Fetch, decrypt and *report* the published plan without installing it.
///
/// Returns `None` when there is nothing to report — sync off, a prerequisite
/// missing, or no gist yet — so the pill stays hidden instead of announcing a
/// channel that does not exist. A real failure (GitHub unreachable) is an
/// error the caller hides quietly: a status pill is never worth an alert.
pub async fn check_remote(state: &AppState) -> Result<Option<RemotePlan>> {
    let config = load(&state.data_dir)?;
    if !config.enabled {
        return Ok(None);
    }
    let (Some(token), Some(sub)) = (stored_token(), account_id()) else {
        return Ok(None);
    };
    let client = github_client()?;
    let gist_id = match config.gist_id.clone() {
        Some(id) => id,
        None => match find_gist(&client, &token, &sync_code(&sub)).await? {
            Some(id) => id,
            None => return Ok(None),
        },
    };
    let sealed = fetch_blob(&client, &token, &gist_id).await?;
    let blob: PlanBlob = serde_json::from_slice(&open(&sub, &sealed)?)?;
    // Same refusals as `pull`, reported as "nothing to say" rather than an
    // error: a pill cannot act on an unreadable blob any better than on none.
    if blob.v > WIRE_VERSION || blob.workouts.is_empty() {
        return Ok(None);
    }
    let current = state.store.current_plan()?;
    let local_rev = state
        .store
        .current_plan_rev()?
        .unwrap_or_else(|| current.as_ref().map_or(0, |(plan, _)| plan.created_at.timestamp()));
    let local = current.as_ref().map(|(plan, _)| (plan, local_rev));
    Ok(Some(RemotePlan {
        status: pill_status(local, &blob.plan.id, blob.rev),
        plan_name: plan_summary(&blob.plan.to_plan()),
        workouts: blob.workouts.len(),
        synced_at: blob.synced_at,
    }))
}

/// Called on every device after a sync completes: take the channel's copy if
/// it is newer than what is here. Both directions now — the phone's edits
/// reach the desktop this way, and the desktop's reach the phone — and the
/// guards inside [`pull`] decide: an equal or older revision is a no-op, a
/// *different* newer plan is reported as a conflict rather than installed, so
/// the automatic path can never replace a plan the runner made locally.
pub async fn pull_if_newer(state: &AppState) {
    let config = load(&state.data_dir).unwrap_or_default();
    if !config.enabled {
        return;
    }
    match pull(state, false).await {
        Ok((_, outcome)) => {
            if outcome.installed {
                // The plan view is gated on having a plan, so say what landed.
                eprintln!(
                    "[plan-sync] installed {} workouts from {}",
                    outcome.workouts, outcome.plan_name
                );
            }
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
    // Both devices publish and both pull; the field stays on the shape
    // because the UI still reads it, but the platform no longer gates it.
    config.can_publish = true;
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
    config.can_publish = true;
    // A token change invalidates a discovered gist: the new account may have a
    // different channel, and re-discovery is cheap.
    let token_changed = token.is_some();
    if token_changed {
        // Whatever was stored under the old per-feature service is now stale.
        if let Some(entry) = legacy_token_entry() {
            let _ = entry.delete_credential();
        }
    }
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
    #[test]
    fn reset_plan_is_not_automatically_reimported() {
        let directory = std::env::temp_dir().join(format!("plan-reset-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        super::remember_reset(&directory, "old-plan").unwrap();
        let config = super::load(&directory).unwrap();
        assert!(super::is_reset_plan(&config, "old-plan", false));
        assert!(!super::is_reset_plan(&config, "new-plan", false));
        assert!(!super::is_reset_plan(&config, "old-plan", true));
        std::fs::remove_dir_all(directory).unwrap();
    }
    use super::*;
    use chrono::TimeZone;

    fn wire() -> PlanBlob {
        PlanBlob {
            v: WIRE_VERSION,
            rev: 1_700_000_000,
            synced_at: 1_700_000_100,
            plan: PlanWire {
                start_date: None,
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
                threshold_state: None,
            },
            workouts: vec![WorkoutWire {
                id: "w-1".into(),
                plan_id: "plan-1".into(),
                date: NaiveDate::from_ymd_opt(2026, 10, 26).unwrap(),
                // Left as `None` on purpose: this blob stands for one written
                // before `planned_date` existed, so the round-trip below also
                // proves an old device's plan still lands on a sane day.
                planned_date: None,
                kind: crate::types::WorkoutKind::Tempo,
                title: "Tempo 6km".into(),
                distance_m: Some(6000.0),
                duration_min: None,
                pace_lo_s_km: Some(245),
                pace_hi_s_km: Some(252),
                status: crate::types::WorkoutStatus::Planned,
                done: false,
                completion: None,
            }],
            calendar: None,
        }
    }

    fn shipped_plan() -> TrainingPlan {
        TrainingPlan {
            start_date: None,
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
            baseline_weekly_km: None,
            quality_offset_sec_km: 0.0,
            quality_offset_from: None,
            threshold_state: None,
        }
    }

    #[test]
    fn pill_reports_up_to_date_only_when_the_channel_matches_our_revision() {
        let plan = shipped_plan();
        // Same plan, same revision: the channel holds exactly what is here.
        assert_eq!(
            pill_status(Some((&plan, 1_700_000_000)), "plan-1", 1_700_000_000),
            PillStatus::UpToDate
        );
        // Ours is newer than the channel's: what is here is not published
        // yet — including when the channel holds an *older* revision of the
        // same plan, which the newer device must not be told to import.
        assert_eq!(
            pill_status(Some((&plan, 1_700_000_000)), "plan-1", 1_699_000_000),
            PillStatus::Unpublished
        );
    }

    #[test]
    fn pill_only_offers_an_import_for_a_strictly_newer_revision() {
        let plan = shipped_plan();
        // A newer revision of the same plan is the update the pill is for —
        // on either device, since either may hold the older copy.
        assert_eq!(
            pill_status(Some((&plan, 1_700_000_000)), "plan-1", 1_700_000_001),
            PillStatus::UpdateAvailable
        );
        // An older remote is never an update; the honest reading is that ours
        // is the copy that still needs publishing.
        assert_eq!(
            pill_status(Some((&plan, 1_700_000_000)), "plan-1", 1_699_999_999),
            PillStatus::Unpublished
        );
    }

    #[test]
    fn pill_distinguishes_a_foreign_plan_from_a_waiting_import() {
        let plan = shipped_plan();
        // Nothing installed here: the channel has something to take.
        assert_eq!(
            pill_status(None, "plan-1", 1_700_000_000),
            PillStatus::ImportAvailable
        );
        // A different plan on the channel, newer than ours: importing it
        // replaces the local plan, which needs a confirmation.
        assert_eq!(
            pill_status(Some((&plan, 1_700_000_000)), "plan-2", 1_700_000_001),
            PillStatus::OtherPlan
        );
        // A different plan that is *older* than ours: ours is the current
        // copy, so the verdict is to publish it over the stale one.
        assert_eq!(
            pill_status(Some((&plan, 1_700_000_000)), "plan-2", 1_699_999_999),
            PillStatus::Unpublished
        );
    }

    #[test]
    fn sync_code_is_stable_and_account_specific() {        let a = sync_code("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
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
    fn start_date_survives_wire_and_store_round_trips() {
        let mut plan = shipped_plan();
        plan.start_date = Some(NaiveDate::from_ymd_opt(2026, 11, 1).unwrap());
        let json = serde_json::to_vec(&PlanWire::from_plan(&plan)).unwrap();
        let decoded: PlanWire = serde_json::from_slice(&json).unwrap();
        let restored = decoded.to_plan();
        assert_eq!(restored.start_date, plan.start_date);
        let directory = std::env::temp_dir().join(format!("plan-start-{}", uuid::Uuid::new_v4()));
        let store = crate::store::Store::open(&directory.join("start-date.sqlite")).unwrap();
        store.insert_plan(&restored).unwrap();
        assert_eq!(store.current_plan().unwrap().unwrap().0.start_date, plan.start_date);
        store.replace_plan(&restored, &[], 123).unwrap();
        drop(store);
        let reopened = crate::store::Store::open(&directory.join("start-date.sqlite")).unwrap();
        assert_eq!(reopened.current_plan().unwrap().unwrap().0.start_date, plan.start_date);
        drop(reopened);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn threshold_policy_survives_wire_and_store_round_trips() {
        let mut plan = shipped_plan();
        plan.threshold_state = Some(crate::plan::threshold::athlete_state(&Default::default(), Utc::now().date_naive(), 250.0));
        let json = serde_json::to_vec(&PlanWire::from_plan(&plan)).unwrap();
        let decoded: PlanWire = serde_json::from_slice(&json).unwrap();
        let restored = decoded.to_plan();
        assert_eq!(restored.threshold_state.as_ref().unwrap().quality_minutes_cap, 15.0);
        let store = crate::store::Store::open(&std::env::temp_dir().join(format!("threshold-policy-{}.sqlite", uuid::Uuid::new_v4()))).unwrap();
        store.insert_plan(&restored).unwrap();
        let read = store.current_plan().unwrap().unwrap().0;
        assert_eq!(serde_json::to_value(&read.threshold_state).unwrap(), serde_json::to_value(&restored.threshold_state).unwrap());
        store.replace_plan(&read, &[], 123).unwrap();
        assert!(store.current_plan().unwrap().unwrap().0.threshold_state.is_some());
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
        assert_eq!(blob.plan.start_date, None);
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
