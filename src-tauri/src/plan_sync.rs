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
//! Both devices may edit. An edit can publish only against its acknowledged
//! Gist snapshot; a changed Gist replaces conflicting local plan changes.
//! Device clocks do not determine authority. Gist PATCH has no documented
//! compare-and-swap guarantee, so a read-back detects intervening writes but
//! cannot prevent all simultaneous-write races.
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
const WIRE_VERSION: u32 = 4;
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
    /// Local mutation baseline for the acknowledged snapshot, not a global clock.
    pub installed_rev: Option<i64>,
    pub acknowledged_blob: Option<String>,
    pub published_plan_id: Option<String>,
    pub channel_code: Option<String>,
    pub remote_etag: Option<String>,
    pub retry_after: Option<i64>,
    pub last_checked: Option<String>,
    pub last_error: Option<String>,
    /// Plans this device has explicitly ended ("End plan"). An automatic pull
    /// must never reimport one of these — not even while the local store is
    /// momentarily empty between ending a plan and creating the next, which is
    /// exactly the window in which the abandoned plan still sitting in the
    /// channel would otherwise slip back in and overwrite the new one.
    ///
    /// A set rather than a single id: a runner who recreates the same plan
    /// several times ends several, and every one of them stays ended. The old
    /// single-id field let a later abort overwrite the marker for an earlier
    /// abandoned plan, which is how a plan the runner had already thrown away
    /// kept returning on the next sync.
    #[serde(default)]
    pub reset_plan_ids: Vec<String>,
    /// The single-slot marker written by apps before the set above existed.
    /// Read once on load and folded into `reset_plan_ids` by
    /// [`migrate_legacy_reset`]; never written again, so a config saved by a
    /// current app no longer carries it.
    #[serde(default, alias = "resetPlanId", skip_serializing)]
    legacy_reset_plan_id: Option<String>,
}

impl PlanSync {
    /// Fold the pre-set single `resetPlanId` into the set, so upgrading does
    /// not forget the plan the previous version had already blocked.
    fn migrate_legacy_reset(&mut self) {
        if let Some(id) = self.legacy_reset_plan_id.take() {
            if !self.reset_plan_ids.iter().any(|existing| *existing == id) {
                self.reset_plan_ids.push(id);
            }
        }
    }
}

fn load_device(state: &AppState) -> Result<PlanSync> {
    let mut config = load(&state.data_dir)?;
    if let Some(checkpoint) = state.store.plan_sync_checkpoint()? {
        let acknowledged: PlanSync = serde_json::from_str(&checkpoint)?;
        config.acknowledged_blob = acknowledged.acknowledged_blob;
        config.published_plan_id = acknowledged.published_plan_id;
        config.installed_rev = acknowledged.installed_rev;
        config.channel_code = acknowledged.channel_code;
        config.gist_id = acknowledged.gist_id;
    }
    Ok(config)
}

fn save_device(state: &AppState, config: &PlanSync) -> Result<()> {
    state.store.save_plan_sync_checkpoint(&serde_json::to_string(config)?)?;
    save(&state.data_dir, config)
}

pub(crate) fn invalidate_channel(state: &AppState) -> Result<()> {
    let mut config = load_device(state)?;
    config.gist_id = None;
    config.acknowledged_blob = None;
    config.published_plan_id = None;
    config.installed_rev = None;
    config.remote_etag = None;
    config.channel_code = None;
    config.retry_after = None;
    save_device(state, &config)
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
    #[serde(default)]
    training_method: crate::types::TrainingMethod,
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
    /// The runner's label for the plan. Optional on the wire: a blob
    /// published before the field existed has no name, and the honest
    /// reading of one is "nobody named it".
    #[serde(default)]
    name: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkoutWire {
    #[serde(default)]
    customization: Option<crate::plan::difficulty::Customization>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    original_notes: Option<String>,
}

impl PlanWire {
    fn from_plan(p: &TrainingPlan) -> Self {
        PlanWire {
            id: p.id.clone(),
            distance: p.distance,
            training_method: p.training_method,
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
            name: p.name.clone(),
        }
    }

    fn to_plan(&self) -> TrainingPlan {
        TrainingPlan {
            id: self.id.clone(),
            distance: self.distance,
            training_method: self.training_method,
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
            name: self.name.clone(),
        }
    }
}

impl WorkoutWire {
    fn from_workout(w: &PlanWorkout, completion: Option<CompletionWire>) -> Self {
        WorkoutWire {
            customization: w.customization.clone(),
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
            customization: self.customization.clone(),
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
        Ok(bytes) => {
            let mut config: PlanSync = serde_json::from_slice(&bytes)?;
            config.migrate_legacy_reset();
            Ok(config)
        }
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
    if !config.reset_plan_ids.iter().any(|id| id == plan_id) {
        config.reset_plan_ids.push(plan_id.to_owned());
        // Bound the marker. The plans a runner ended longest ago are the least
        // likely to be resurrected, so if someone cycles through very many
        // plans the oldest ended ones drop off the front rather than growing
        // the config file without limit.
        const MAX_REMEMBERED: usize = 50;
        if config.reset_plan_ids.len() > MAX_REMEMBERED {
            config.reset_plan_ids.remove(0);
        }
    }
    save(directory, &config)
}

fn is_reset_plan(config: &PlanSync, plan_id: &str, force: bool) -> bool {
    !force && config.reset_plan_ids.iter().any(|id| id == plan_id)
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

fn check_github_status(response: &reqwest::Response) -> Result<()> {
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
        || (response.status() == reqwest::StatusCode::FORBIDDEN
            && (response.headers().contains_key("retry-after")
                || response.headers().get("x-ratelimit-remaining").is_some_and(|value| value == "0"))) {
        let now = Utc::now().timestamp();
        let retry_at = response.headers().get("retry-after").and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<i64>().ok().map(|seconds| now + seconds)
                .or_else(|| DateTime::parse_from_rfc2822(value).ok().map(|date| date.timestamp())))
            .or_else(|| response.headers().get("x-ratelimit-reset").and_then(|value| value.to_str().ok()?.parse::<i64>().ok()))
            .unwrap_or(now + 60).max(now + 1);
        return Err(AppError::PlanSyncRateLimited { retry_at });
    }
    Ok(())
}

/// The gist we are looking for, among the ones this token can see. Matching on
/// the description rather than the name means neither device stores a URL.
async fn find_gist(client: &reqwest::Client, token: &str, code: &str, api: &str) -> Result<Option<String>> {
    let want = format!("{MARKER}{code}");
    let mut found = None;
    for page in 1..=3u32 {
        // Plain /gists is "the authenticated user's gists" when sent with a
        // token — there is no /user/gists route, and GitHub answers 404.
        let url = format!("{api}/gists?per_page=100&page={page}");
        let response = github_headers(client.get(&url), token)
            .send()
            .await
            .map_err(|_| AppError::Other("Could not reach GitHub".into()))?;
        check_github_status(&response)?;
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
            return Err(AppError::Other("GitHub returned an unreadable gist list".into()));
        };
        for gist in list {
            if gist.get("description").and_then(|d| d.as_str()) == Some(want.as_str()) {
                if let Some(id) = gist.get("id").and_then(|i| i.as_str()) {
                    if found.is_some() {
                        return Err(AppError::Other("Multiple plan sync gists match this account. Keep one channel before syncing.".into()));
                    }
                    found = Some(id.to_string());
                }
            }
        }
        // Fewer than a full page means there is no next page.
        if list.len() < 100 {
            return Ok(found);
        }
    }
    Err(AppError::Other("Could not finish discovering the plan sync channel".into()))
}

async fn create_gist(
    client: &reqwest::Client,
    token: &str,
    code: &str,
    body: &str,
    api: &str,
) -> Result<String> {
    let mut files = serde_json::Map::new();
    files.insert(BLOB_FILE.into(), serde_json::json!({ "content": body }));
    let response = github_headers(
        client.post(format!("{api}/gists")).json(&serde_json::json!({
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
    check_github_status(&response)?;
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
    api: &str,
) -> Result<()> {
    let mut files = serde_json::Map::new();
    files.insert(BLOB_FILE.into(), serde_json::json!({ "content": body }));
    let response = github_headers(
        client
            .patch(format!("{api}/gists/{gist_id}"))
            .json(&serde_json::json!({ "files": files })),
        token,
    )
    .send()
    .await
    .map_err(|_| AppError::Other("Could not update the plan gist on GitHub".into()))?;
    check_github_status(&response)?;
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
async fn fetch_blob(client: &reqwest::Client, token: &str, gist_id: &str, api: &str) -> Result<String> {
    fetch_remote(client, token, gist_id, api, None).await?.0
        .ok_or_else(|| AppError::Other("GitHub returned no plan snapshot".into()))
}

async fn fetch_remote(client: &reqwest::Client, token: &str, gist_id: &str, api: &str, etag: Option<&str>) -> Result<(Option<String>, Option<String>)> {
    let url = format!("{api}/gists/{gist_id}");
    let request = github_headers(client.get(&url), token);
    let request = if let Some(etag) = etag { request.header("If-None-Match", etag) } else { request };
    let response = request
        .send()
        .await
        .map_err(|_| AppError::Other("Could not reach GitHub".into()))?;
    check_github_status(&response)?;
    let remote_etag = response.headers().get("etag").and_then(|value| value.to_str().ok()).map(str::to_owned);
    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        if etag.is_none() { return Err(AppError::Other("GitHub returned no plan snapshot".into())); }
        return Ok((None, remote_etag.or_else(|| etag.map(str::to_owned))));
    }
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
        return Ok((Some(body), remote_etag));
    }
    file["content"]
        .as_str()
        .map(|content| (Some(content.to_string()), remote_etag))
        .ok_or_else(|| AppError::Other("The synced plan gist has no plan file".into()))
}

// ---- publish / pull --------------------------------------------------------

fn blob_fingerprint(sealed: &str) -> String {
    use aws_lc_rs::digest::{digest, SHA256};
    base64url_encode(digest(&SHA256, sealed.as_bytes()).as_ref())
}

fn remote_changed(config: &PlanSync, sealed: &str) -> bool {
    config.acknowledged_blob.as_deref() != Some(blob_fingerprint(sealed).as_str())
}

fn decode_snapshot(sub: &str, sealed: &str) -> Result<Option<PlanBlob>> {
    let value: serde_json::Value = serde_json::from_slice(&open(sub, sealed)?)?;
    let version = value["v"].as_u64().ok_or_else(|| AppError::Other("The synced plan has no format version".into()))?;
    if version > u64::from(WIRE_VERSION) {
        return Err(AppError::Other("The synced plan was written by a newer version of Runalytics".into()));
    }
    if version >= 3 && value["state"] == "empty" {
        return Ok(None);
    }
    let blob: PlanBlob = serde_json::from_value(value)?;
    if blob.workouts.is_empty() || blob.workouts.iter().any(|workout| workout.plan_id != blob.plan.id) {
        return Err(AppError::Other("The synced plan has invalid workouts".into()));
    }
    Ok(Some(blob))
}

fn local_changed(state: &AppState, config: &PlanSync) -> Result<bool> {
    let current = state.store.current_plan()?;
    Ok(current.as_ref().map(|(plan, _)| plan.id.as_str()) != config.published_plan_id.as_deref()
        || state.store.current_plan_rev()? != config.installed_rev)
}

pub fn pending_changes(state: &AppState) -> Result<bool> {
    let config = load_device(state)?;
    Ok(config.acknowledged_blob.is_some() && local_changed(state, &config)?)
}

/// `None` when the channel is
/// not configured (nothing was attempted, so nothing to show), `Some` when a
/// push actually ran. The staged toast in the commands layer uses this to
/// decide whether to draw a gist line at all.
pub async fn try_push_if_enabled(state: &AppState) -> Option<Result<PlanSync>> {
    let config = match load_device(state) { Ok(config) => config, Err(error) => return Some(Err(error)) };
    if !config.enabled {
        return None;
    }
    let result = push(state, false).await;
    if let Err(error) = &result {
        eprintln!("[plan-sync] automatic push failed: {error}");
        let mut retry = load_device(state).unwrap_or(config);
        retry.last_error = Some(error.to_string());
        let _ = save(&state.data_dir, &retry);
    }
    Some(result)
}

/// Seal and upload the current plan, persisting the resulting config. `explicit`
/// is true when the runner pressed the button, which is the only time a missing
/// prerequisite is worth an error.
pub async fn push(state: &AppState, explicit: bool) -> Result<PlanSync> {
    let result = push_inner(state, explicit).await;
    match result {
        Ok(config) => {
            save_device(state, &config)?;
            Ok(config)
        }
        Err(error) => {
            let mut config = load_device(state)?;
            config.last_error = Some(error.to_string());
            if let AppError::PlanSyncRateLimited { retry_at } = &error { config.retry_after = Some(*retry_at); }
            save(&state.data_dir, &config)?;
            Err(error)
        }
    }
}

async fn push_inner(state: &AppState, explicit: bool) -> Result<PlanSync> {
    let mut config = load_device(state)?;
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
    sync_channel(state, config, &token, &sub, "https://api.github.com").await
}

async fn sync_channel(state: &AppState, mut config: PlanSync, token: &str, sub: &str, api: &str) -> Result<PlanSync> {
    if let Some(retry_at) = config.retry_after.filter(|retry_at| *retry_at > Utc::now().timestamp()) {
        return Err(AppError::PlanSyncRateLimited { retry_at });
    }

    // Pair each done session with the run that completed it, named by the
    // provider's record id so the other device can resolve the same activity.
    let records = state.store.calendar_completion_records()?;
    let completed_notes = state.store.calendar_completions()?;
    let completion_of = |workout: &PlanWorkout| -> Option<CompletionWire> {
        records
            .iter()
            .find(|(workout_id, _, _)| workout_id == &workout.id)
            .map(|(_, provider, record_id)| CompletionWire {
                provider: provider.clone(),
                record_id: record_id.clone(),
                original_notes: completed_notes.iter()
                    .find(|(workout_id, _, _)| workout_id == &workout.id)
                    .map(|(_, _, notes)| notes.clone())
                    .filter(|notes| !notes.trim().is_empty()),
            })
    };

    let code = sync_code(sub);
    if config.channel_code.as_ref().is_some_and(|previous| previous != &code) {
        config.gist_id = None;
        config.acknowledged_blob = None;
        config.published_plan_id = None;
        config.installed_rev = None;
        config.remote_etag = None;
    }
    config.channel_code = Some(code.clone());
    let client = github_client()?;
    let gist_id = match config.gist_id.clone() {
        Some(id) => Some(id),
        None => find_gist(&client, token, &code, api).await?,
    };

    if let Some(id) = gist_id.as_deref() {
        config.gist_id = Some(id.to_owned());
        let etag = config.acknowledged_blob.as_ref().and(config.remote_etag.as_deref());
        let (remote, etag) = fetch_remote(&client, token, id, api, etag).await?;
        config.remote_etag = etag;
        config.last_checked = Some(Utc::now().to_rfc3339());
        if let Some(remote) = remote {
            decode_snapshot(sub, &remote)?;
            if remote_changed(&config, &remote) {
            let (config, outcome) = install_snapshot(state, config, sub, &remote, true, false).await?;
            if outcome.conflict.is_some() {
                return Err(AppError::Other("The legacy Gist contains an ended plan. Import it explicitly to restore it.".into()));
            }
            return Ok(config);
            }
        }
        if !local_changed(state, &config)? {
            config.last_error = None;
            return Ok(config);
        }
    }

    let current = state.store.current_plan()?;
    if current.is_none() && gist_id.is_none() {
        return Ok(config);
    }

    let payload = if let Some((plan, workouts)) = current.as_ref() {
        serde_json::to_vec(&PlanBlob {
        v: WIRE_VERSION,
        rev: state
            .store
            .current_plan_rev()?
            .unwrap_or_else(|| plan.created_at.timestamp()),
        synced_at: Utc::now().timestamp(),
        plan: PlanWire::from_plan(plan),
        workouts: workouts
            .iter()
            .map(|w| WorkoutWire::from_workout(w, completion_of(w)))
            .collect(),
        calendar: calendar_target(&state.data_dir),
        })?
    } else {
        serde_json::to_vec(&serde_json::json!({"v": WIRE_VERSION, "state": "empty"}))?
    };
    let sealed = seal(sub, &payload)?;

    match gist_id.as_deref() {
        Some(id) => patch_gist(&client, token, id, &sealed, api).await?,
        None => config.gist_id = Some(create_gist(&client, token, &code, &sealed, api).await?),
    }
    config.gist_id = config.gist_id.or(gist_id);
    save(&state.data_dir, &config)?;
    let accepted = fetch_blob(&client, token, config.gist_id.as_deref().unwrap(), api).await?;
    if accepted != sealed {
        let (config, _) = install_snapshot(state, config, sub, &accepted, true, false).await?;
        return Ok(config);
    }
    config.acknowledged_blob = Some(blob_fingerprint(&sealed));
    config.published_plan_id = current.as_ref().map(|(plan, _)| plan.id.clone());
    config.installed_rev = state.store.current_plan_rev()?;
    config.last_pushed = Some(Utc::now().to_rfc3339());
    config.last_error = None;
    config.retry_after = None;
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
    let mut config = load_device(state)?;
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
        None => find_gist(&client, &token, &code, "https://api.github.com").await?.ok_or_else(|| {
            AppError::Other(
                "No synced plan found for this COROS account. Publish one from your computer."
                    .into(),
            )
        })?,
    };
    config.gist_id = Some(gist_id.clone());
    save(&state.data_dir, &config)?;

    let sealed = fetch_blob(&client, &token, &gist_id, "https://api.github.com").await?;
    install_snapshot(state, config, &sub, &sealed, false, force).await
}

async fn install_snapshot(
    state: &AppState,
    mut config: PlanSync,
    sub: &str,
    sealed: &str,
    authoritative: bool,
    force: bool,
) -> Result<(PlanSync, PullOutcome)> {
    let snapshot = decode_snapshot(sub, sealed)?;
    config.last_checked = Some(Utc::now().to_rfc3339());
    let Some(blob) = snapshot else {
        let installed = state.store.current_plan()?.is_some();
        config.acknowledged_blob = Some(blob_fingerprint(sealed));
        config.published_plan_id = None;
        config.installed_rev = None;
        state.store.clear_synced_plan_data(Some(&serde_json::to_string(&config)?))?;
        config.last_pulled = Some(Utc::now().to_rfc3339());
        config.last_error = None;
        save_device(state, &config)?;
        return Ok((config, PullOutcome { installed, plan_name: "No active plan".into(), workouts: 0, conflict: None }));
    };
    if authoritative && !remote_changed(&config, sealed) {
        save(&state.data_dir, &config)?;
        return Ok((config, PullOutcome { installed: false, plan_name: String::new(), workouts: 0, conflict: None }));
    }

    let incoming = blob.plan.to_plan();
    if is_reset_plan(&config, &incoming.id, force) && !(authoritative && blob.v >= 3) {
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
        if existing.id != incoming.id && !force && !authoritative {
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
        if existing.id == incoming.id && blob.rev <= local_rev && !authoritative && !force {
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

    let workouts: Vec<PlanWorkout> = blob
        .workouts
        .iter()
        .map(WorkoutWire::to_workout)
        .collect();

    config.acknowledged_blob = Some(blob_fingerprint(sealed));
    config.published_plan_id = Some(incoming.id.clone());
    config.installed_rev = Some(blob.rev);
    state.store.replace_synced_plan(&incoming, &workouts, blob.rev, Some(&serde_json::to_string(&config)?))?;

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
        let notes = completion.original_notes.as_deref()
            .filter(|notes| !notes.trim().is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| crate::calendar::workout_notes(workout, &incoming));
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

    // A forced import is the runner deliberately choosing this plan again, so
    // it stops being an ended plan and resumes syncing normally from here.
    if force {
        config
            .reset_plan_ids
            .retain(|id| id != incoming.id.as_str());
    }
    config.installed_rev = state.store.current_plan_rev()?;
    config.acknowledged_blob = Some(blob_fingerprint(sealed));
    config.published_plan_id = Some(incoming.id.clone());
    config.last_pulled = Some(Utc::now().to_rfc3339());
    config.last_error = None;
    save_device(state, &config)?;

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

/// How the UI refers to a plan: the runner's own name when they gave one,
/// otherwise what it is for and when it ends. Enough to tell two plans apart
/// in a confirmation, and it follows the same rule every other naming
/// surface uses so the sync channel never contradicts the plan view.
fn plan_summary(plan: &TrainingPlan) -> String {
    plan.display_name()
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

fn pill_status(
    local: Option<&TrainingPlan>,
    blob_plan_id: &str,
    changed: bool,
    dirty: bool,
) -> PillStatus {
    if changed {
        return match local {
            None => PillStatus::ImportAvailable,
            Some(plan) if plan.id != blob_plan_id => PillStatus::OtherPlan,
            _ => PillStatus::UpdateAvailable,
        };
    }
    if dirty {
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
    let config = load_device(state)?;
    if !config.enabled {
        return Ok(None);
    }
    let (Some(token), Some(sub)) = (stored_token(), account_id()) else {
        return Ok(None);
    };
    let client = github_client()?;
    let gist_id = match config.gist_id.clone() {
        Some(id) => id,
            None => match find_gist(&client, &token, &sync_code(&sub), "https://api.github.com").await? {
            Some(id) => id,
            None => return Ok(None),
        },
    };
    let sealed = fetch_blob(&client, &token, &gist_id, "https://api.github.com").await?;
    let Some(blob) = decode_snapshot(&sub, &sealed)? else { return Ok(None); };
    // Same refusals as `pull`, reported as "nothing to say" rather than an
    // error: a pill cannot act on an unreadable blob any better than on none.
    if blob.v > WIRE_VERSION || blob.workouts.is_empty() {
        return Ok(None);
    }
    let current = state.store.current_plan()?;
    Ok(Some(RemotePlan {
        status: pill_status(current.as_ref().map(|(plan, _)| plan), &blob.plan.id, remote_changed(&config, &sealed), local_changed(state, &config)?),
        plan_name: plan_summary(&blob.plan.to_plan()),
        workouts: blob.workouts.len(),
        synced_at: blob.synced_at,
    }))
}

pub async fn refresh(state: &AppState) -> Result<bool> {
    let before = load_device(state)?;
    if !before.enabled || stored_token().is_none() || account_id().is_none() {
        return Ok(false);
    }
    match push(state, false).await {
        Ok(after) => Ok(before.last_pulled != after.last_pulled),
        Err(error) => {
            let mut config = load(&state.data_dir)?;
            config.last_checked = Some(Utc::now().to_rfc3339());
            config.last_error = Some(error.to_string());
            save(&state.data_dir, &config)?;
            Err(error)
        }
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
    let mut config = load_device(state)?;
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
    let mut config = load_device(state)?;
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
        config.acknowledged_blob = None;
        config.published_plan_id = None;
        config.installed_rev = None;
        config.remote_etag = None;
        config.channel_code = None;
        config.retry_after = None;
    }
    config.last_error = None;
    save_device(state, &config)?;
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
    fn shared_credential_change_invalidates_the_persisted_channel_checkpoint() {
        let state = device();
        let config = PlanSync {
            enabled: true,
            gist_id: Some("previous-account-gist".into()),
            acknowledged_blob: Some("previous-snapshot".into()),
            remote_etag: Some("previous-etag".into()),
            installed_rev: Some(42),
            ..Default::default()
        };
        save_device(&state, &config).unwrap();
        invalidate_channel(&state).unwrap();
        let changed = load_device(&state).unwrap();
        assert!(changed.enabled);
        assert!(changed.gist_id.is_none());
        assert!(changed.acknowledged_blob.is_none());
        assert!(changed.remote_etag.is_none());
        assert!(changed.installed_rev.is_none());
        std::fs::remove_dir_all(state.data_dir).unwrap();
    }

    fn device() -> AppState {
        let data_dir = std::env::temp_dir().join(format!("plan-sync-device-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&data_dir).unwrap();
        AppState {
            store: crate::store::Store::open(&data_dir.join("store.sqlite")).unwrap(),
            settings: std::sync::Mutex::new(Default::default()),
            providers: Vec::new(),
            data_dir,
            syncing: std::sync::atomic::AtomicBool::new(false),
            sync_lock: tokio::sync::Mutex::new(()),
            calendar_lock: tokio::sync::Mutex::new(()),
        }
    }

    async fn fake_gist(
        remote: String,
        status: u16,
    ) -> (String, std::sync::Arc<std::sync::Mutex<String>>, std::sync::Arc<std::sync::Mutex<Vec<String>>>, tokio::task::JoinHandle<()>) {
        fake_gist_with_race(remote, status, None).await
    }

    async fn fake_gist_with_race(
        remote: String,
        status: u16,
        concurrent: Option<String>,
    ) -> (String, std::sync::Arc<std::sync::Mutex<String>>, std::sync::Arc<std::sync::Mutex<Vec<String>>>, tokio::task::JoinHandle<()>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let api = format!("http://{}", listener.local_addr().unwrap());
        let snapshot = std::sync::Arc::new(std::sync::Mutex::new(remote));
        let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let content = snapshot.clone();
        let captured = requests.clone();
        let server = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let header_end = loop {
                    let mut chunk = [0; 4096];
                    let count = socket.read(&mut chunk).await.unwrap();
                    if count == 0 { return; }
                    request.extend_from_slice(&chunk[..count]);
                    if let Some(end) = request.windows(4).position(|part| part == b"\r\n\r\n") { break end + 4; }
                };
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let line = headers.lines().next().unwrap().to_string();
                let conditional = headers.lines().find_map(|header| {
                    let (name, value) = header.split_once(':')?;
                    name.eq_ignore_ascii_case("if-none-match").then(|| value.trim().to_owned())
                });
                let length = headers.lines().find_map(|header| header.to_ascii_lowercase().strip_prefix("content-length:").and_then(|value| value.trim().parse::<usize>().ok())).unwrap_or(0);
                while request.len() < header_end + length {
                    let mut chunk = [0; 4096];
                    let count = socket.read(&mut chunk).await.unwrap();
                    if count == 0 { return; }
                    request.extend_from_slice(&chunk[..count]);
                }
                captured.lock().unwrap().push(line.clone());
                if line.starts_with("PATCH ") {
                    let payload: serde_json::Value = serde_json::from_slice(&request[header_end..header_end + length]).unwrap();
                    *content.lock().unwrap() = concurrent.clone().unwrap_or_else(|| payload["files"][BLOB_FILE]["content"].as_str().unwrap().to_string());
                }
                let etag = format!("\"{}\"", blob_fingerprint(&content.lock().unwrap()));
                if status == 200 && conditional.as_deref() == Some(&etag) {
                    socket.write_all(format!("HTTP/1.1 304 Not Modified\r\nETag: {etag}\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
                    continue;
                }
                let body = serde_json::json!({"files": {BLOB_FILE: {"content": content.lock().unwrap().clone()}}}).to_string();
                let reply = format!("HTTP/1.1 {status} Response\r\nETag: {etag}\r\nRetry-After: 90\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                socket.write_all(reply.as_bytes()).await.unwrap();
            }
        });
        (api, snapshot, requests, server)
    }

    #[tokio::test]
    async fn completed_prescription_survives_sync_restart_and_legacy_import() {
        let state = device();
        let mut blob = wire();
        let original = "Made with Runalytics - a virtual personal vibecoded coach 🏃\n\n🏃 Slow Burn (total 7.0 km)\n\nPlan:\n  • 2.0 km Warm-up\n  • 1.0 km Tempo at 4:13-4:23/km\n  • 1.0 km Tempo at 3:58-4:08/km\n  • 2.0 km Tempo at 3:48-3:58/km\n  • 1.0 km Cool-down\n\nWhy: Progressive blocks.";
        blob.workouts[0].completion = Some(CompletionWire {
            provider: "coros".into(),
            record_id: "completed-run".into(),
            original_notes: Some(original.into()),
        });
        blob.workouts[0].done = true;
        state.store.conn.lock().unwrap().execute(
            "INSERT INTO activities(provider,provider_record_id,sport,start_ts,duration_s,distance_m) VALUES('coros','completed-run','running',1793032200,1820,7020)",
            [],
        ).unwrap();
        let sealed = seal("synthetic-account", &serde_json::to_vec(&blob).unwrap()).unwrap();
        let (api, content, _, server) = fake_gist(sealed, 200).await;
        let config = PlanSync { enabled: true, gist_id: Some("channel".into()), ..Default::default() };
        let installed = sync_channel(&state, config, "synthetic-token", "synthetic-account", &api).await.unwrap();
        crate::calendar::adopt_naming(&state.store).unwrap();
        assert_eq!(state.store.calendar_completions().unwrap()[0].2, original);
        let previous_rev = state.store.current_plan_rev().unwrap();
        let corrected = original.replace("Progressive blocks.", "Progressive blocks, controlled not raced.");
        state.store.set_completion_notes("w-1", &corrected).unwrap();
        assert!(state.store.current_plan_rev().unwrap() > previous_rev);
        let corrected_rev = state.store.current_plan_rev().unwrap();
        state.store.set_completion_notes("w-1", &corrected).unwrap();
        assert_eq!(state.store.current_plan_rev().unwrap(), corrected_rev);
        let published = sync_channel(&state, installed, "synthetic-token", "synthetic-account", &api).await.unwrap();
        let accepted = decode_snapshot("synthetic-account", &content.lock().unwrap()).unwrap().unwrap();
        assert_eq!(accepted.workouts[0].completion.as_ref().unwrap().original_notes.as_deref(), Some(corrected.as_str()));
        blob.rev += 100;
        blob.workouts[0].completion.as_mut().unwrap().original_notes = None;
        *content.lock().unwrap() = seal("synthetic-account", &serde_json::to_vec(&blob).unwrap()).unwrap();
        sync_channel(&state, published, "synthetic-token", "synthetic-account", &api).await.unwrap();
        let reopened = crate::store::Store::open(&state.data_dir.join("store.sqlite")).unwrap();
        for _ in 0..3 {
            crate::calendar::adopt_naming(&reopened).unwrap();
            assert_eq!(reopened.calendar_completions().unwrap()[0].2, corrected);
            let matched = crate::commands::matched_sessions_for_store(&reopened).unwrap();
            assert_eq!(matched[0].purpose, "Progressive blocks, controlled not raced.");
            assert!(matched[0].notes.contains("1.0 km Tempo at 4:13-4:23/km"));
            assert!(!matched[0].notes.contains("Controlled threshold"));
        }
        let legacy: CompletionWire = serde_json::from_value(serde_json::json!({"provider":"coros","recordId":"completed-run"})).unwrap();
        assert!(legacy.original_notes.is_none());
        server.abort();
        drop(reopened);
        std::fs::remove_dir_all(state.data_dir).unwrap();
    }

    #[tokio::test]
    async fn stale_mobile_installs_desktop_replacement_without_patching_even_with_clock_skew() {
        let mobile = device();
        let desktop = device();
        let mut old = wire();
        old.rev = i64::MAX / 2;
        mobile.store.replace_plan(&old.plan.to_plan(), &old.workouts.iter().map(WorkoutWire::to_workout).collect::<Vec<_>>(), old.rev).unwrap();
        let mut latest = wire();
        latest.plan.id = "desktop-plan-b".into();
        latest.plan.name = Some("Desktop replacement".into());
        latest.workouts[0].plan_id = latest.plan.id.clone();
        let remote = seal("synthetic-account", &serde_json::to_vec(&latest).unwrap()).unwrap();
        let (api, _, requests, server) = fake_gist(remote.clone(), 200).await;
        let config = PlanSync { enabled: true, gist_id: Some("channel".into()), ..Default::default() };
        let installed = sync_channel(&mobile, config, "synthetic-token", "synthetic-account", &api).await.unwrap();
        assert_eq!(mobile.store.current_plan().unwrap().unwrap().0.id, "desktop-plan-b");
        assert_eq!(installed.acknowledged_blob, Some(blob_fingerprint(&remote)));
        let unchanged = sync_channel(&mobile, installed, "synthetic-token", "synthetic-account", &api).await.unwrap();
        assert!(!local_changed(&mobile, &unchanged).unwrap());
        assert!(requests.lock().unwrap().iter().all(|request| request.starts_with("GET ")));
        assert!(desktop.store.current_plan().unwrap().is_none());
        server.abort();
        std::fs::remove_dir_all(mobile.data_dir).unwrap();
        std::fs::remove_dir_all(desktop.data_dir).unwrap();
    }

    #[tokio::test]
    async fn unchanged_base_publishes_an_edit_and_tombstone_propagates_to_another_device() {
        let desktop = device();
        let mobile = device();
        let blob = wire();
        let sealed = seal("synthetic-account", &serde_json::to_vec(&blob).unwrap()).unwrap();
        let (api, content, requests, server) = fake_gist(sealed.clone(), 200).await;
        let config = PlanSync { enabled: true, gist_id: Some("channel".into()), ..Default::default() };
        let installed = sync_channel(&desktop, config.clone(), "synthetic-token", "synthetic-account", &api).await.unwrap();
        sync_channel(&mobile, config, "synthetic-token", "synthetic-account", &api).await.unwrap();
        let mut workout = desktop.store.current_plan().unwrap().unwrap().1.remove(0);
        workout.title = "Desktop edited session".into();
        desktop.store.update_workout(&workout).unwrap();
        let published = sync_channel(&desktop, installed, "synthetic-token", "synthetic-account", &api).await.unwrap();
        let accepted = decode_snapshot("synthetic-account", &content.lock().unwrap()).unwrap().unwrap();
        assert_eq!(accepted.workouts[0].title, "Desktop edited session");
        assert_eq!(requests.lock().unwrap().iter().filter(|request| request.starts_with("PATCH ")).count(), 1);
        desktop.store.clear_plan_data().unwrap();
        sync_channel(&desktop, published, "synthetic-token", "synthetic-account", &api).await.unwrap();
        let stale = load(&mobile.data_dir).unwrap();
        sync_channel(&mobile, stale, "synthetic-token", "synthetic-account", &api).await.unwrap();
        assert!(mobile.store.current_plan().unwrap().is_none());
        assert!(decode_snapshot("synthetic-account", &content.lock().unwrap()).unwrap().is_none());
        server.abort();
        std::fs::remove_dir_all(desktop.data_dir).unwrap();
        std::fs::remove_dir_all(mobile.data_dir).unwrap();
    }

    #[tokio::test]
    async fn failed_or_unreadable_remote_never_authorizes_a_write() {
        for (status, content) in [(503, "unavailable"), (200, "malformed")] {
            let state = device();
            let blob = wire();
            state.store.replace_plan(&blob.plan.to_plan(), &blob.workouts.iter().map(WorkoutWire::to_workout).collect::<Vec<_>>(), blob.rev).unwrap();
            let (api, _, requests, server) = fake_gist(content.into(), status).await;
            let config = PlanSync { enabled: true, gist_id: Some("channel".into()), ..Default::default() };
            assert!(sync_channel(&state, config, "synthetic-token", "synthetic-account", &api).await.is_err());
            assert_eq!(requests.lock().unwrap().len(), 1);
            assert!(requests.lock().unwrap()[0].starts_with("GET "));
            assert_eq!(state.store.current_plan().unwrap().unwrap().0.id, blob.plan.id);
            server.abort();
            std::fs::remove_dir_all(state.data_dir).unwrap();
        }
    }

    #[tokio::test]
    async fn same_plan_remote_change_overrides_equal_revisions_and_pending_local_edits() {
        let state = device();
        let blob = wire();
        let base = seal("synthetic-account", &serde_json::to_vec(&blob).unwrap()).unwrap();
        let (api, remote, requests, server) = fake_gist(base, 200).await;
        let config = PlanSync { enabled: true, gist_id: Some("channel".into()), ..Default::default() };
        let config = sync_channel(&state, config, "synthetic-token", "synthetic-account", &api).await.unwrap();
        let mut workout = state.store.current_plan().unwrap().unwrap().1.remove(0);
        workout.title = "Unpublished local edit".into();
        state.store.update_workout(&workout).unwrap();
        let mut changed = wire();
        changed.workouts[0].title = "Authoritative same-revision remote edit".into();
        *remote.lock().unwrap() = seal("synthetic-account", &serde_json::to_vec(&changed).unwrap()).unwrap();
        sync_channel(&state, config, "synthetic-token", "synthetic-account", &api).await.unwrap();
        assert_eq!(state.store.current_plan().unwrap().unwrap().1[0].title, changed.workouts[0].title);
        assert!(requests.lock().unwrap().iter().all(|request| request.starts_with("GET ")));
        server.abort();
        std::fs::remove_dir_all(state.data_dir).unwrap();
    }

    #[tokio::test]
    async fn conditional_cache_hit_does_not_reinstall_or_publish_and_pending_edits_survive_restart() {
        let mut state = device();
        let blob = wire();
        let remote = seal("synthetic-account", &serde_json::to_vec(&blob).unwrap()).unwrap();
        let (api, _, requests, server) = fake_gist(remote, 200).await;
        let config = PlanSync { enabled: true, gist_id: Some("channel".into()), ..Default::default() };
        let config = sync_channel(&state, config, "synthetic-token", "synthetic-account", &api).await.unwrap();
        let unchanged = sync_channel(&state, config, "synthetic-token", "synthetic-account", &api).await.unwrap();
        assert!(unchanged.remote_etag.is_some());
        assert_eq!(requests.lock().unwrap().len(), 2);
        assert!(fetch_remote(&github_client().unwrap(), "synthetic-token", "channel", &api, unchanged.remote_etag.as_deref()).await.unwrap().0.is_none());
        let mut workout = state.store.current_plan().unwrap().unwrap().1.remove(0);
        workout.title = "Offline edit retained after restart".into();
        state.store.update_workout(&workout).unwrap();
        save_device(&state, &unchanged).unwrap();
        state.store = crate::store::Store::open(&state.data_dir.join("store.sqlite")).unwrap();
        let loaded = load_device(&state).unwrap();
        assert!(local_changed(&state, &loaded).unwrap());
        sync_channel(&state, loaded, "synthetic-token", "synthetic-account", &api).await.unwrap();
        assert_eq!(requests.lock().unwrap().iter().filter(|request| request.starts_with("PATCH ")).count(), 1);
        server.abort();
        std::fs::remove_dir_all(state.data_dir).unwrap();
    }

    #[tokio::test]
    async fn rate_limits_and_unsupported_snapshots_block_publication() {
        let state = device();
        let (api, _, requests, server) = fake_gist("unused".into(), 429).await;
        let config = PlanSync { enabled: true, gist_id: Some("channel".into()), ..Default::default() };
        let result = sync_channel(&state, config, "synthetic-token", "synthetic-account", &api).await;
        assert!(matches!(result, Err(AppError::PlanSyncRateLimited { retry_at }) if retry_at >= Utc::now().timestamp() + 85));
        assert_eq!(requests.lock().unwrap().len(), 1);
        let future = seal("synthetic-account", br#"{"v":999,"state":"empty"}"#).unwrap();
        assert!(decode_snapshot("synthetic-account", &future).is_err());
        server.abort();
        std::fs::remove_dir_all(state.data_dir).unwrap();
    }

    #[tokio::test]
    async fn readback_adopts_a_concurrent_gist_write_instead_of_retrying_the_local_snapshot() {
        let state = device();
        let base = seal("synthetic-account", &serde_json::to_vec(&wire()).unwrap()).unwrap();
        let mut concurrent = wire();
        concurrent.plan.name = Some("Concurrent Gist winner".into());
        let concurrent = seal("synthetic-account", &serde_json::to_vec(&concurrent).unwrap()).unwrap();
        let (api, _, requests, server) = fake_gist_with_race(base, 200, Some(concurrent)).await;
        let config = PlanSync { enabled: true, gist_id: Some("channel".into()), ..Default::default() };
        let config = sync_channel(&state, config, "synthetic-token", "synthetic-account", &api).await.unwrap();
        let mut workout = state.store.current_plan().unwrap().unwrap().1.remove(0);
        workout.title = "Losing local edit".into();
        state.store.update_workout(&workout).unwrap();
        sync_channel(&state, config, "synthetic-token", "synthetic-account", &api).await.unwrap();
        assert_eq!(state.store.current_plan().unwrap().unwrap().0.name.as_deref(), Some("Concurrent Gist winner"));
        assert_eq!(requests.lock().unwrap().iter().filter(|request| request.starts_with("PATCH ")).count(), 1);
        server.abort();
        std::fs::remove_dir_all(state.data_dir).unwrap();
    }

    #[test]
    fn changed_gist_wins_over_stale_mobile_and_untracked_legacy_cache() {
        let mut config = super::PlanSync::default();
        assert!(super::remote_changed(&config, "desktop-plan-b"));
        config.acknowledged_blob = Some(super::blob_fingerprint("mobile-plan-a"));
        assert!(super::remote_changed(&config, "desktop-plan-b"));
    }

    #[test]
    fn difficulty_feedback_and_frozen_steps_survive_wire_replace_and_restart() {
        let state = device();
        let mut blob = wire();
        let mut workout = blob.workouts[0].to_workout();
        let plan = blob.plan.to_plan();
        let details = crate::plan::session::details_for_plan(&workout, &plan);
        workout.customization = Some(crate::plan::difficulty::Customization {
            prescription: Some(crate::plan::difficulty::Prescription { version: 1, baseline: details.clone(), current: details.clone() }),
            rating: Some(4), submitted_at: Some(1794300000), dismissed: true,
        });
        blob.workouts[0] = WorkoutWire::from_workout(&workout, None);
        let encoded = serde_json::to_vec(&blob).unwrap();
        let decoded: PlanBlob = serde_json::from_slice(&encoded).unwrap();
        state.store.replace_synced_plan(&plan, &[decoded.workouts[0].to_workout()], decoded.rev, None).unwrap();
        let reopened = crate::store::Store::open(&state.data_dir.join("store.sqlite")).unwrap();
        let mut stored = reopened.plan_workouts(&plan.id).unwrap().remove(0);
        assert_eq!(stored.customization.as_ref().unwrap().rating, Some(4));
        assert!(stored.customization.as_ref().unwrap().dismissed);
        stored.done = true;
        assert_eq!(serde_json::to_value(crate::plan::session::details_for_plan(&stored, &plan)).unwrap(), serde_json::to_value(details).unwrap());
        std::fs::remove_dir_all(state.data_dir).unwrap();
    }

    #[test]
    fn unchanged_gist_allows_a_local_edit_against_its_acknowledged_base() {
        let config = super::PlanSync {
            acknowledged_blob: Some(super::blob_fingerprint("shared-base")),
            ..Default::default()
        };
        assert!(!super::remote_changed(&config, "shared-base"));
    }

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

    #[test]
    fn every_ended_plan_stays_ended_not_just_the_last_one() {
        // The bug this guards: a runner recreates the same plan several times,
        // ending each. A single-slot marker let the newest abort overwrite the
        // marker for an earlier abandoned plan, so that earlier plan — still in
        // the channel — slipped back in on the next sync while the store was
        // empty. Every ended plan must stay remembered.
        let directory = std::env::temp_dir().join(format!("plan-reset-many-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        super::remember_reset(&directory, "first-plan").unwrap();
        super::remember_reset(&directory, "second-plan").unwrap();
        let config = super::load(&directory).unwrap();
        assert!(super::is_reset_plan(&config, "first-plan", false));
        assert!(super::is_reset_plan(&config, "second-plan", false));
        // Remembering the same plan twice does not duplicate it.
        super::remember_reset(&directory, "second-plan").unwrap();
        let config = super::load(&directory).unwrap();
        assert_eq!(config.reset_plan_ids.len(), 2);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn a_legacy_single_reset_marker_survives_the_upgrade() {
        // A config written by the previous version carries the old
        // single-slot "resetPlanId". Loading must fold it into the set —
        // otherwise upgrading silently un-blocks the plan that version had
        // already ended, and the first sync after the upgrade resurrects it.
        let directory = std::env::temp_dir().join(format!("plan-reset-legacy-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("plan-sync.json"),
            br#"{"enabled":true,"resetPlanId":"aborted-before-upgrade"}"#,
        )
        .unwrap();
        let config = super::load(&directory).unwrap();
        assert!(super::is_reset_plan(&config, "aborted-before-upgrade", false));
        // Saving again must not keep writing the legacy field back out.
        // (Match the whole key, since the set's own "resetPlanIds" contains
        // the legacy name as a prefix.)
        super::save(&directory, &config).unwrap();
        let on_disk = std::fs::read_to_string(directory.join("plan-sync.json")).unwrap();
        assert!(!on_disk.contains("\"resetPlanId\":"));
        let config = super::load(&directory).unwrap();
        assert!(super::is_reset_plan(&config, "aborted-before-upgrade", false));
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
                training_method: crate::types::TrainingMethod::Norwegian,
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
                name: None,
            },
            workouts: vec![WorkoutWire {
                customization: None,
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
            training_method: crate::types::TrainingMethod::Norwegian,
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
            name: None,
        }
    }

    #[test]
    fn pill_reports_up_to_date_only_when_the_acknowledged_gist_matches() {
        let plan = shipped_plan();
        assert_eq!(
            pill_status(Some(&plan), "plan-1", false, false),
            PillStatus::UpToDate
        );
        assert_eq!(
            pill_status(Some(&plan), "plan-1", false, true),
            PillStatus::Unpublished
        );
    }

    #[test]
    fn changed_gist_takes_precedence_over_pending_local_edits() {
        let plan = shipped_plan();
        assert_eq!(
            pill_status(Some(&plan), "plan-1", true, true),
            PillStatus::UpdateAvailable
        );
        assert_eq!(
            pill_status(Some(&plan), "plan-2", true, true),
            PillStatus::OtherPlan
        );
    }

    #[test]
    fn pill_distinguishes_a_foreign_plan_from_a_waiting_import() {
        let plan = shipped_plan();
        assert_eq!(
            pill_status(None, "plan-1", true, false),
            PillStatus::ImportAvailable
        );
        assert_eq!(
            pill_status(Some(&plan), "plan-2", true, false),
            PillStatus::OtherPlan
        );
        assert_eq!(
            pill_status(Some(&plan), "plan-2", false, true),
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
    fn training_method_survives_wire_store_replace_and_reopen() {
        let mut plan = shipped_plan();
        plan.training_method = crate::types::TrainingMethod::RunnaLike;
        let mut json = serde_json::to_value(PlanWire::from_plan(&plan)).unwrap();
        let decoded: PlanWire = serde_json::from_value(json.clone()).unwrap();
        let restored = decoded.to_plan();
        assert_eq!(restored.training_method, plan.training_method);
        let directory = std::env::temp_dir().join(format!("plan-method-{}", uuid::Uuid::new_v4()));
        let database = directory.join("method.sqlite");
        let store = crate::store::Store::open(&database).unwrap();
        store.insert_plan(&restored).unwrap();
        assert_eq!(store.current_plan().unwrap().unwrap().0.training_method, plan.training_method);
        assert_eq!(store.all_plans().unwrap()[0].training_method, plan.training_method);
        store.replace_plan(&restored, &[], 123).unwrap();
        drop(store);
        let reopened = crate::store::Store::open(&database).unwrap();
        assert_eq!(reopened.current_plan().unwrap().unwrap().0.training_method, plan.training_method);
        drop(reopened);
        json.as_object_mut().unwrap().remove("trainingMethod");
        let legacy: PlanWire = serde_json::from_value(json).unwrap();
        assert_eq!(legacy.to_plan().training_method, crate::types::TrainingMethod::Norwegian);
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
