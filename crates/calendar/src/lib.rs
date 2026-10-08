//! Calendar publishing for Runalytics.
//!
//! Turns a [`runalytics_core::Plan`] into an ICS calendar ([`ics`]) and
//! delivers it to the athlete's calendar app through one or more sinks
//! ([`sink`]):
//!
//! * [`sink::FileSink`] — atomic `plan.ics` on disk (works with anything
//!   that watches a file).
//! * [`sink::WebcalSink`] — localhost `webcal:` feed served by axum; the
//!   calendar app polls it, no filesystem or Automation permission needed.
//! * [`sink::ApplescriptSink`] — Calendar.app via `osascript`, macOS-native.
//!
//! [`SinkRegistry`] fans a publish out to all configured sinks; each sink
//! fails or succeeds independently and the [`sink::PublishReport`] names the
//! outcomes for the UI.
//!
//! Identity model: every event carries a deterministic UID derived from the
//! planned session id (`runalytics://session/{uuid}`), mirrored into the
//! event description because Calendar.app assigns its own event uids. That
//! is what lets a re-plan *update* events instead of duplicating them, and
//! lets `clear()` remove exactly the events we own.

pub mod ics;
pub mod sink;

pub use ics::{calendar_for_plan, ics_for_plan, session_uid};
pub use sink::{
    ApplescriptSink, CalendarError, CalendarSink, FileSink, PublishReport, SinkRegistry, WebcalSink,
};
