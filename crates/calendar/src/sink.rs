//! Calendar sinks: places the generated ICS can be published to.
//!
//! Three sinks, deliberately independent — a failing Calendar.app sync must
//! not stop the file feed from updating:
//!
//! * [`FileSink`] — atomic `plan.ics` write for any calendar app that watches
//!   a folder, and the fallback that always works.
//! * [`WebcalSink`] — in-memory feed served over HTTP with `webcal:`
//!   subscription; the calendar app polls it, so no write access is needed.
//! * [`ApplescriptSink`] — drives Calendar.app directly through `osascript`,
//!   replacing events by the Runalytics uid marker. The most native option
//!   and macOS-only, which the whole workspace already is.
//!
//! Every sink keys on the session UID from [`crate::ics`] — Calendar.app
//! rewrites event uids on insert, so the marker also lives in the event
//! description, which is what the delete predicate matches.

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use icalendar::{Calendar, Component};
use runalytics_core::{Plan, Timestamp, Tz};

use crate::ics::{calendar_for_plan, ics_for_plan};

/// The substring that marks an event as ours, inside its description.
pub const UID_MARKER: &str = "runalytics://session/";

/// What went wrong publishing to a calendar.
#[derive(Debug, thiserror::Error)]
pub enum CalendarError {
    /// Filesystem trouble writing the feed, or `osascript` not runnable.
    #[error("calendar write failed: {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// `osascript` exited non-zero — usually a missing Automation
    /// permission (the TCC prompt the user declined).
    #[error("Calendar.app sync failed (exit {exit_code}): {stderr}")]
    Applescript { exit_code: i32, stderr: String },

    /// The generated ICS failed to re-parse, which means a bug in `ics`,
    /// not a user-facing condition. Surfaced loudly rather than swallowed.
    #[error("generated calendar did not round-trip: {0}")]
    Internal(String),
}

/// Result of one publish round across a fan-out registry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PublishReport {
    /// Names of the sinks that accepted the plan.
    pub ok: Vec<String>,
    /// `name: error` for each sink that failed.
    pub failed: Vec<String>,
}

impl PublishReport {
    /// True when every configured sink accepted the plan.
    #[must_use]
    pub fn all_ok(&self) -> bool {
        self.failed.is_empty()
    }
}

/// A destination for the plan calendar.
#[async_trait]
pub trait CalendarSink: Send + Sync {
    /// Human name for logs and the report.
    fn name(&self) -> &'static str;

    /// Regenerate and publish the plan's calendar.
    async fn publish(&self, plan: &Plan, tz: &Tz) -> Result<(), CalendarError>;

    /// Remove everything Runalytics published (empty calendar, not a
    /// deleted one — the subscription must survive a plan deletion).
    async fn clear(&self) -> Result<(), CalendarError>;
}

/// Writes the feed to a fixed path, atomically.
///
/// Temp-file + rename means a calendar app watching the folder never sees a
/// half-written file — it either gets the old feed or the new one.
pub struct FileSink {
    path: PathBuf,
}

impl FileSink {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The path this sink owns.
    #[must_use]
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    fn write_atomic(&self, ics: &str) -> Result<(), CalendarError> {
        let tmp = self.path.with_extension("ics.tmp");
        if let Err(e) = std::fs::write(&tmp, ics) {
            let _ = std::fs::remove_file(&tmp);
            return Err(CalendarError::Io {
                path: tmp.clone(),
                source: e,
            });
        }
        std::fs::rename(&tmp, &self.path).map_err(|source| CalendarError::Io {
            path: self.path.clone(),
            source,
        })
    }
}

#[async_trait]
impl CalendarSink for FileSink {
    fn name(&self) -> &'static str {
        "file"
    }

    async fn publish(&self, plan: &Plan, tz: &Tz) -> Result<(), CalendarError> {
        let ics = ics_for_plan(plan, tz, chrono::Utc::now());
        self.write_atomic(&ics)
    }

    async fn clear(&self) -> Result<(), CalendarError> {
        self.write_atomic(&Calendar::new().to_string())
    }
}

/// Serves the current feed over HTTP for `webcal:` subscription.
///
/// The sink holds the bytes; an axum router built from the same handle
/// serves them. Calendar apps poll rather than push, so publishing is just
/// swapping the buffer — cheap and failure-free.
#[derive(Clone)]
pub struct WebcalSink {
    feed: Arc<RwLock<String>>,
}

impl WebcalSink {
    #[must_use]
    pub fn new() -> Self {
        Self {
            feed: Arc::new(RwLock::new(String::new())),
        }
    }

    /// The bytes currently being served (empty before the first publish).
    #[must_use]
    pub fn current_feed(&self) -> String {
        self.feed
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// The `webcal:` subscription URL for a feed served at `http://host:port/path`.
    ///
    /// Calendar apps accept `webcal://` in place of `http://` and subscribe
    /// (poll) instead of one-shot downloading the file.
    #[must_use]
    pub fn webcal_url(host: &str, port: u16, path: &str) -> String {
        format!("webcal://{host}:{port}/{path}")
    }

    /// An axum router serving the feed as `text/calendar`.
    pub fn router(self) -> axum::Router {
        use axum::{Json, Router, routing::get};
        Router::new()
            .route("/plan.ics", get(serve_feed))
            .route(
                "/health",
                get(|| async { Json(serde_json::json!({"ok": true})) }),
            )
            .with_state(self)
    }
}

impl Default for WebcalSink {
    fn default() -> Self {
        Self::new()
    }
}

async fn serve_feed(sink: axum::extract::State<WebcalSink>) -> axum::response::Response {
    use axum::response::IntoResponse;
    let body = sink.current_feed();
    axum::response::Response::builder()
        .header(
            axum::http::header::CONTENT_TYPE,
            "text/calendar; charset=utf-8",
        )
        .header(axum::http::header::CACHE_CONTROL, "no-cache")
        .body(axum::body::Body::from(body))
        .unwrap_or_else(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

#[async_trait]
impl CalendarSink for WebcalSink {
    fn name(&self) -> &'static str {
        "webcal"
    }

    async fn publish(&self, plan: &Plan, tz: &Tz) -> Result<(), CalendarError> {
        let ics = ics_for_plan(plan, tz, chrono::Utc::now());
        // A feed we cannot re-parse is a bug in ics.rs; refuse to serve it.
        Calendar::from_str(&ics).map_err(CalendarError::Internal)?;
        let mut guard = self
            .feed
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = ics;
        Ok(())
    }

    async fn clear(&self) -> Result<(), CalendarError> {
        let mut guard = self
            .feed
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = Calendar::new().to_string();
        Ok(())
    }
}

/// Publishes straight into Calendar.app via `osascript`.
///
/// Each publish deletes the calendar's Runalytics events (matched by the
/// uid marker in their description — Calendar.app assigns its own event
/// uids, so the ICS uid cannot be matched directly) and re-inserts the
/// current set. Delete-then-insert is heavier than a diff, but the plan is
/// tens of events and Calendar.app's AppleScript update surface is
/// unreliable for partial edits — correctness beats cleverness at this size.
pub struct ApplescriptSink {
    calendar_name: String,
    /// Overridable for tests; the real binary on macOS.
    osascript: PathBuf,
}

impl Default for ApplescriptSink {
    fn default() -> Self {
        Self::new("Runalytics")
    }
}

impl ApplescriptSink {
    #[must_use]
    pub fn new(calendar_name: impl Into<String>) -> Self {
        Self {
            calendar_name: calendar_name.into(),
            osascript: PathBuf::from("/usr/bin/osascript"),
        }
    }

    /// Point at a different interpreter (tests use a recording stub).
    #[must_use]
    pub fn with_interpreter(mut self, path: impl Into<PathBuf>) -> Self {
        self.osascript = path.into();
        self
    }

    /// Build the AppleScript that replaces the calendar's contents with the
    /// plan's events. Pure function — tests assert on its text, CI never
    /// needs Calendar.app or Automation permissions.
    pub fn script_for(&self, plan: &Plan, tz: &Tz, generated_at: Timestamp) -> String {
        let calendar = calendar_for_plan(plan, tz, generated_at);
        let name = escape_applescript(&self.calendar_name);
        let mut lines: Vec<String> = vec![
            // Locale-safe date builder: field assignment, never `date "..."`
            // string parsing, which only works on English macOS locales.
            // Day is floored to 1 first so e.g. Jan 31 -> Feb never overflows.
            "on runDate(y, m, d, h, min)".to_string(),
            "set dt to current date".to_string(),
            "set day of dt to 1".to_string(),
            "set year of dt to y".to_string(),
            "set month of dt to m".to_string(),
            "set day of dt to d".to_string(),
            "set hours of dt to h".to_string(),
            "set minutes of dt to min".to_string(),
            "set seconds of dt to 0".to_string(),
            "return dt".to_string(),
            "end runDate".to_string(),
            String::new(),
            "tell application \"Calendar\"".to_string(),
            format!("if not (exists calendar \"{name}\") then").to_string(),
            format!("make new calendar with properties {{name:\"{name}\", color:\"#C0442C\"}}"),
            "end if".to_string(),
            format!("tell calendar \"{name}\"").to_string(),
            format!("delete (every event whose description contains \"{UID_MARKER}\")"),
        ];

        for event in calendar.events() {
            let (Some(summary), Some(start)) = (
                event.property_value("SUMMARY"),
                event.property_value("DTSTART"),
            ) else {
                continue;
            };
            let description = event.property_value("DESCRIPTION").unwrap_or("");
            let end = event.property_value("DTEND").unwrap_or(start);
            lines.push(format!(
                "make new event with properties {{summary:\"{}\", description:\"{}\", start date:{}, end date:{}}}",
                escape_applescript(summary),
                escape_applescript(description),
                applescript_date(start),
                applescript_date(end),
            ));
        }

        lines.push("end tell".to_string());
        lines.push("end tell".to_string());
        lines.join("\n")
    }

    async fn run_script(&self, script: &str) -> Result<(), CalendarError> {
        let output = tokio::process::Command::new(&self.osascript)
            .arg("-e")
            .arg(script)
            .output()
            .await
            .map_err(|source| CalendarError::Io {
                path: self.osascript.clone(),
                source,
            })?;
        if output.status.success() {
            return Ok(());
        }
        Err(CalendarError::Applescript {
            exit_code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        })
    }
}

/// Escape a string for embedding in an AppleScript quoted literal.
fn escape_applescript(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Turn an ICS local timestamp (`20261012T063000`) into a `runDate` call.
///
/// The plan times are already the athlete's local wall times, and `runDate`
/// builds a local `date` object, so the event lands at the right wall time
/// in whatever timezone the Mac is currently in — which is what the athlete
/// expects for a training calendar.
fn applescript_date(ical_ts: &str) -> String {
    let ts = ical_ts.trim();
    let (d, t) = ts.split_once('T').unwrap_or((ts, "000000"));
    let num = |s: &str| -> i64 { s.parse().unwrap_or(0) };
    let (year, rest) = d.split_at(4.min(d.len()));
    let (month, day) = if rest.len() >= 4 {
        (&rest[0..2], &rest[2..4])
    } else {
        ("01", "01")
    };
    let hour = &t[0..2.min(t.len())];
    let minute = t.get(2..4).unwrap_or("00");
    format!(
        "(runDate {} {} {} {} {})",
        num(year),
        num(month),
        num(day),
        num(hour),
        num(minute)
    )
}

#[async_trait]
impl CalendarSink for ApplescriptSink {
    fn name(&self) -> &'static str {
        "calendar-app"
    }

    async fn publish(&self, plan: &Plan, tz: &Tz) -> Result<(), CalendarError> {
        let script = self.script_for(plan, tz, chrono::Utc::now());
        self.run_script(&script).await
    }

    async fn clear(&self) -> Result<(), CalendarError> {
        let name = escape_applescript(&self.calendar_name);
        let script = format!(
            "tell application \"Calendar\" to tell calendar \"{name}\" to delete (every event whose description contains \"{UID_MARKER}\")"
        );
        self.run_script(&script).await
    }
}

/// Fans one publish out to every sink; one sink failing does not stop the
/// others, and the report names each outcome for the UI to surface.
#[derive(Default)]
pub struct SinkRegistry {
    sinks: Vec<Box<dyn CalendarSink>>,
}

impl SinkRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self { sinks: Vec::new() }
    }

    pub fn register(&mut self, sink: Box<dyn CalendarSink>) {
        self.sinks.push(sink);
    }

    #[must_use]
    pub fn sink_names(&self) -> Vec<String> {
        self.sinks.iter().map(|s| s.name().to_string()).collect()
    }

    /// Publish to all sinks, collecting per-sink outcomes.
    pub async fn publish(&self, plan: &Plan, tz: &Tz) -> PublishReport {
        let mut report = PublishReport::default();
        for sink in &self.sinks {
            match sink.publish(plan, tz).await {
                Ok(()) => report.ok.push(sink.name().to_string()),
                Err(e) => {
                    tracing::warn!(sink = sink.name(), error = %e, "calendar sink failed");
                    report.failed.push(format!("{}: {e}", sink.name()));
                }
            }
        }
        report
    }

    /// Clear every sink, same no-fan-stop semantics as publish.
    pub async fn clear(&self) -> PublishReport {
        let mut report = PublishReport::default();
        for sink in &self.sinks {
            match sink.clear().await {
                Ok(()) => report.ok.push(sink.name().to_string()),
                Err(e) => report.failed.push(format!("{}: {e}", sink.name())),
            }
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ics::test_support::{plan_with, session, stamp, tz};
    use runalytics_core::SessionKind;

    #[tokio::test]
    async fn file_sink_writes_atomically_and_clears() {
        let dir = tempfile::tempdir().expect("tmp");
        let path = dir.path().join("plan.ics");
        let sink = FileSink::new(&path);
        let plan = plan_with(vec![session(SessionKind::Tempo)]);

        sink.publish(&plan, &tz()).await.expect("publish");
        let ics = std::fs::read_to_string(&path).expect("read");
        assert!(ics.contains("BEGIN:VEVENT"));
        // The temp file must be gone — rename completed, no litter.
        assert!(!path.with_extension("ics.tmp").exists());

        sink.clear().await.expect("clear");
        let cleared = std::fs::read_to_string(&path).expect("read");
        assert!(!cleared.contains("BEGIN:VEVENT"));
    }

    #[tokio::test]
    async fn file_sink_reports_io_error_with_path() {
        let sink = FileSink::new("/nonexistent-dir-xyz/plan.ics");
        let err = sink
            .publish(&plan_with(vec![session(SessionKind::Tempo)]), &tz())
            .await
            .expect_err("must fail");
        assert!(matches!(err, CalendarError::Io { .. }));
        assert!(err.to_string().contains("nonexistent-dir-xyz"));
    }

    #[tokio::test]
    async fn webcal_sink_serves_the_current_feed() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let sink = WebcalSink::new();
        assert_eq!(sink.current_feed(), String::new());

        sink.publish(&plan_with(vec![session(SessionKind::Tempo)]), &tz())
            .await
            .expect("publish");
        assert!(sink.current_feed().contains("BEGIN:VEVENT"));

        let response = sink
            .router()
            .oneshot(
                Request::builder()
                    .uri("/plan.ics")
                    .body(Body::empty())
                    .expect("req"),
            )
            .await
            .expect("serve");
        assert_eq!(response.status(), StatusCode::OK);
        let ct = response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .expect("ct")
            .to_str()
            .expect("ct str");
        assert!(ct.starts_with("text/calendar"));
        let body = axum::body::to_bytes(response.into_body(), 1 << 20)
            .await
            .expect("body");
        assert!(String::from_utf8_lossy(&body).contains("BEGIN:VEVENT"));
    }

    #[test]
    fn webcal_url_shape() {
        assert_eq!(
            WebcalSink::webcal_url("127.0.0.1", 8731, "plan.ics"),
            "webcal://127.0.0.1:8731/plan.ics"
        );
    }

    #[test]
    fn applescript_script_creates_calendar_replaces_by_marker_and_sets_dates() {
        let sink = ApplescriptSink::new("Runalytics");
        let script = sink.script_for(
            &plan_with(vec![session(SessionKind::Tempo)]),
            &tz(),
            stamp(),
        );
        // Calendar is created if missing, old events dropped by marker.
        assert!(script.contains("exists calendar \"Runalytics\""));
        assert!(
            script.contains(
                "delete (every event whose description contains \"runalytics://session/\")"
            )
        );
        // Locale-safe field-wise date construction, never string dates.
        assert!(script.contains("(runDate 2026 10 12 6 30)"));
        assert!(script.contains("set day of dt to 1"));
        assert!(script.contains("make new event with properties"));
        // The plan's own title survives into the script.
        assert!(script.contains("Tempo 40'"));
    }

    #[test]
    fn applescript_escaping_neutralises_quotes() {
        assert_eq!(escape_applescript("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape_applescript("back\\slash"), "back\\\\slash");
    }

    #[test]
    fn ics_timestamp_to_applescript_date() {
        assert_eq!(
            applescript_date("20261012T063000"),
            "(runDate 2026 10 12 6 30)"
        );
    }

    #[tokio::test]
    async fn applescript_sink_reports_nonzero_exit() {
        // A stub interpreter that always fails like a denied TCC prompt.
        let dir = tempfile::tempdir().expect("tmp");
        let stub = dir.path().join("osascript-stub.sh");
        std::fs::write(
            &stub,
            "#!/bin/sh\necho 'Not authorized to send Apple events to Calendar.' >&2\nexit 1\n",
        )
        .expect("stub");
        std::fs::set_permissions(&stub, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .expect("chmod");

        let sink = ApplescriptSink::new("Runalytics").with_interpreter(&stub);
        let err = sink
            .publish(&plan_with(vec![session(SessionKind::Tempo)]), &tz())
            .await
            .expect_err("must fail");
        match err {
            CalendarError::Applescript { stderr, .. } => {
                assert!(stderr.contains("Not authorized"));
            }
            other => panic!("expected Applescript error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn registry_fans_out_and_one_failure_does_not_stop_the_rest() {
        let dir = tempfile::tempdir().expect("tmp");
        let mut registry = SinkRegistry::new();
        registry.register(Box::new(FileSink::new(dir.path().join("plan.ics"))));
        registry.register(Box::new(FileSink::new("/nonexistent-dir-xyz/plan.ics")));
        registry.register(Box::new(WebcalSink::new()));

        let report = registry
            .publish(&plan_with(vec![session(SessionKind::Tempo)]), &tz())
            .await;
        assert_eq!(report.ok, vec!["file", "webcal"]);
        assert_eq!(report.failed.len(), 1);
        assert!(report.failed[0].starts_with("file:"));
        assert!(!report.all_ok());

        // The good file sink really wrote and the webcal sink really
        // updated — independence, not ordering.
        assert!(dir.path().join("plan.ics").exists());
    }
}
