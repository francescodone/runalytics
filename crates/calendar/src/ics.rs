//! ICS generation for a training plan.
//!
//! One VEVENT per schedulable session, keyed by a deterministic UID derived
//! from the session id (`runalytics://session/{uuid}`). Determinism is the
//! whole point: every sink — file, webcal, Calendar.app — recognises an
//! updated session as *the same event* and edits it instead of duplicating,
//! and deleting a session from a plan removes exactly one event everywhere.
//!
//! Times are emitted with the athlete's timezone as a `TZID` parameter rather
//! than UTC. A 06:30 morning run must read 06:30 in the calendar after a trip,
//! which is what a floating local time with a named zone does; UTC would
//! shift the event and make an early start look like a midnight one.

use icalendar::{Calendar, Component, Property};
use runalytics_core::{Date, Plan, PlannedSession, TimeOfDay, Timestamp, Tz};

/// The stable calendar UID for one planned session.
///
/// Derived from the session id alone — not the date — so moving a session in
/// a re-plan updates the existing event rather than orphaning it.
#[must_use]
pub fn session_uid(session: &PlannedSession) -> String {
    format!("runalytics://session/{}", session.id)
}

/// The ICS timestamp form for a local date + time, `YYYYMMDDTHHMMSS`.
fn ical_local(date: Date, time: TimeOfDay) -> String {
    format!("{}T{}", date.format("%Y%m%d"), time.format("%H%M%S"))
}

/// The ICS timestamp form for an instant, UTC basic format.
#[must_use]
pub fn ical_utc(instant: Timestamp) -> String {
    instant.format("%Y%m%dT%H%M%SZ").to_string()
}

/// Build the full calendar for a plan.
///
/// Rest days are skipped — a calendar full of "Rest" entries trains the
/// athlete to ignore it, and the app already shows rest. Sinks compare
/// per-event (by UID), so re-generating an unchanged plan changes nothing
/// the athlete can see.
#[must_use]
pub fn calendar_for_plan(plan: &Plan, tz: &Tz, generated_at: Timestamp) -> Calendar {
    let mut calendar = Calendar::new();
    calendar.name("Runalytics");
    calendar.append_property(Property::new("X-WR-CALNAME", "Runalytics"));
    calendar.append_property(Property::new("X-WR-TIMEZONE", tz.to_string()));

    let tzid = tz.to_string();
    for session in plan.all_sessions() {
        if !session.kind.is_schedulable() {
            continue;
        }
        let start = ical_local(session.date, session.start);
        // DTEND from the prescribed duration; a zero-duration session would
        // be an invalid VEVENT, so it gets a one-minute floor.
        let end_secs = i64::from(session.target_duration.as_u32()).max(60);
        let end_time = session.start + chrono::Duration::seconds(end_secs);
        // A session that runs past midnight ends on the next day.
        let end_date = if end_time < session.start {
            session.date.succ_opt().unwrap_or(session.date)
        } else {
            session.date
        };

        let mut event = icalendar::Event::new();
        event.uid(&session_uid(session));
        event.summary(&session.title);
        event.description(&session_description(session));
        let mut dtstart = Property::new("DTSTART", start);
        dtstart.add_parameter("TZID", &tzid);
        event.append_property(dtstart);
        let mut dtend = Property::new("DTEND", ical_local(end_date, end_time));
        dtend.add_parameter("TZID", &tzid);
        event.append_property(dtend);
        event.append_property(Property::new("DTSTAMP", ical_utc(generated_at)));
        event.append_property(Property::new(
            "CATEGORIES",
            session.kind.as_str().to_uppercase(),
        ));
        // TRANSP:TRANSPARENT keeps the event from blocking the athlete's
        // real meetings in a shared calendar — a tempo run is not an
        // unbookable appointment.
        event.append_property(Property::new("TRANSP", "TRANSPARENT"));
        calendar.push(event.done());
    }
    calendar
}

/// The event body: intent first, then the prescribed numbers, then the
/// structured blocks rendered as a readable list.
fn session_description(session: &PlannedSession) -> String {
    let mut lines = Vec::new();
    if !session.intent.is_empty() {
        lines.push(session.intent.clone());
    }
    let mut facts = Vec::new();
    if session.target_volume.as_f64() > 0.0 {
        facts.push(format!("{}", session.target_volume));
    }
    if session.target_duration.as_u32() > 0 {
        facts.push(format!("{}", session.target_duration));
    }
    if let Some(pace) = session.target_pace {
        facts.push(format!("target {pace}"));
    }
    if !facts.is_empty() {
        lines.push(facts.join(" · "));
    }
    for block in &session.workout.blocks {
        lines.push(format!(
            "{} {}{}",
            block.target.as_str(),
            block.duration,
            block.pace.map(|p| format!(" @ {p}")).unwrap_or_default()
        ));
    }
    lines.push(format!("uid {}", session_uid(session)));
    lines.join("\n")
}

/// Render a plan's calendar as an ICS string.
#[must_use]
pub fn ics_for_plan(plan: &Plan, tz: &Tz, generated_at: Timestamp) -> String {
    calendar_for_plan(plan, tz, generated_at).to_string()
}

/// Shared fixtures for this crate's tests (also used by `sink`).
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use chrono::TimeZone;
    use runalytics_core::{
        Anchor, AnchorResolution, AthleteSnapshot, DurationSecs, GoalKind, Pace, Phase, PlanId,
        PlanStatus, PlanWeek, PlannedSessionId, SessionKind, StructuredWorkout, VolumeKm,
    };

    pub(crate) fn tz() -> Tz {
        "Europe/Berlin".parse().expect("tz")
    }

    pub(crate) fn date(y: i32, m: u32, d: u32) -> Date {
        Date::from_ymd_opt(y, m, d).expect("test date")
    }

    pub(crate) fn stamp() -> Timestamp {
        chrono::Utc
            .with_ymd_and_hms(2026, 10, 1, 12, 0, 0)
            .single()
            .expect("stamp")
    }

    /// A tempo session on the standard test Monday, 06:30, 50 minutes.
    pub(crate) fn session(kind: SessionKind) -> PlannedSession {
        session_on(kind, date(2026, 10, 12))
    }

    pub(crate) fn session_on(kind: SessionKind, day: Date) -> PlannedSession {
        PlannedSession {
            id: PlannedSessionId::new(),
            date: day,
            start: TimeOfDay::from_hms_opt(6, 30, 0).expect("t"),
            kind,
            title: "Tempo 40'".into(),
            intent: "Comfortably hard, then settle.".into(),
            workout: StructuredWorkout::default(),
            target_volume: VolumeKm(10.0),
            target_duration: DurationSecs(3000),
            target_pace: Some(Pace::new(255.0)),
            rpe_target: Some(7),
            quality: kind.is_quality(),
            external_id: None,
        }
    }

    pub(crate) fn plan_with(sessions: Vec<PlannedSession>) -> Plan {
        let start = date(2026, 10, 12);
        let week = PlanWeek {
            index: 0,
            phase: Phase::Base,
            start,
            end: start + chrono::Duration::days(6),
            target_volume: VolumeKm(30.0),
            previous_volume: VolumeKm(28.0),
            step_pct: 0.07,
            projected_acwr: 1.05,
            sessions,
            is_deload: false,
        };
        Plan {
            id: PlanId::new(),
            name: "Berlin build".into(),
            goal: GoalKind::Marathon,
            status: PlanStatus::Draft,
            anchor: Anchor::Horizon { weeks: 1 },
            resolution: AnchorResolution {
                start,
                end: start + chrono::Duration::days(6),
                weeks: 1,
                race_date: None,
                adjusted: false,
                note: None,
            },
            phases: Vec::new(),
            weeks: vec![week],
            athlete: AthleteSnapshot::placeholder(tz()),
            ceiling_volume: VolumeKm(55.0),
            emittable_as_coros_plan: false,
            external_ids: Vec::new(),
            created_at: stamp(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{date, plan_with, session_on, stamp, tz};
    use super::*;
    use runalytics_core::{
        BlockTarget, DurationSecs, Pace, SessionKind, StructuredWorkout, WorkoutBlock,
    };
    use std::str::FromStr;

    #[test]
    fn uid_is_stable_and_session_scoped() {
        let s = session_on(SessionKind::Tempo, date(2026, 10, 12));
        let uid = session_uid(&s);
        assert!(uid.starts_with("runalytics://session/"));
        assert_eq!(uid, session_uid(&s));
        // Moving the session to another day keeps the uid — update, not dupe.
        let mut moved = s.clone();
        moved.date = date(2026, 10, 15);
        assert_eq!(uid, session_uid(&moved));
    }

    #[test]
    fn rest_days_never_reach_the_calendar() {
        let d = date(2026, 10, 12);
        let plan = plan_with(vec![
            session_on(SessionKind::Rest, d),
            session_on(SessionKind::Tempo, d),
        ]);
        let ics = ics_for_plan(&plan, &tz(), stamp());
        assert_eq!(ics.matches("BEGIN:VEVENT").count(), 1);
        assert!(ics.contains("Tempo 40'"));
    }

    #[test]
    fn times_carry_the_athlete_zone_not_utc() {
        let d = date(2026, 10, 12);
        let plan = plan_with(vec![session_on(SessionKind::Tempo, d)]);
        let ics = ics_for_plan(&plan, &tz(), stamp());
        assert!(
            ics.contains("DTSTART;TZID=Europe/Berlin:20261012T063000"),
            "the morning run must read 06:30 in the athlete's zone"
        );
        // 50 minutes later.
        assert!(ics.contains("DTEND;TZID=Europe/Berlin:20261012T072000"));
    }

    #[test]
    fn a_session_past_midnight_ends_the_next_day() {
        let d = date(2026, 10, 12);
        let mut s = session_on(SessionKind::LongRun, d);
        s.start = TimeOfDay::from_hms_opt(23, 30, 0).expect("t");
        s.target_duration = DurationSecs(5400); // 90 min -> 01:00 next day
        let plan = plan_with(vec![s]);
        let ics = ics_for_plan(&plan, &tz(), stamp());
        assert!(ics.contains("DTEND;TZID=Europe/Berlin:20261013T010000"));
    }

    #[test]
    fn output_is_a_parseable_vcalendar() {
        let d = date(2026, 10, 12);
        let plan = plan_with(vec![session_on(SessionKind::Tempo, d)]);
        let ics = ics_for_plan(&plan, &tz(), stamp());
        assert!(ics.starts_with("BEGIN:VCALENDAR"));
        assert!(ics.trim_end().ends_with("END:VCALENDAR"));
        // Round-trip: the crate's own parser must accept what we emit.
        let parsed = Calendar::from_str(&ics).expect("parses");
        let event = parsed.events().next().expect("one event");
        assert_eq!(
            event.get_uid().expect("uid"),
            format!(
                "runalytics://session/{}",
                plan.all_sessions().next().expect("s").id
            )
        );
        assert_eq!(event.property_value("TRANSP"), Some("TRANSPARENT"));
    }

    #[test]
    fn description_carries_intent_and_blocks() {
        let d = date(2026, 10, 12);
        let mut s = session_on(SessionKind::Intervals, d);
        s.workout = StructuredWorkout {
            blocks: vec![
                WorkoutBlock::new(BlockTarget::Hard, DurationSecs(480)).with_pace(Pace::new(225.0)),
            ],
        };
        let desc = session_description(&s);
        assert!(desc.contains("Comfortably hard"));
        assert!(desc.contains("hard 8:00 @ 3:45/km"));
        assert!(desc.contains("runalytics://session/"));
    }
}
