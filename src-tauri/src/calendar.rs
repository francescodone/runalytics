use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::PathBuf;

use chrono::{Datelike, Duration, Local, NaiveDate, Utc};
use serde::Serialize;

use crate::error::{AppError, Result};
use crate::metrics;
use crate::plan::session;
use crate::store::Store;
use crate::types::*;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEvent {
    pub id: String,
    pub workout_id: Option<String>,
    pub activity_id: Option<i64>,
    pub date: NaiveDate,
    /// The session's kind — `easy`, `tempo`, `intervals`, `longRun`, `race` —
    /// carried by planned *and* completed events: a finished run keeps the
    /// kind it was planned as, so its colour still says what was run. Only a
    /// run with no planned session behind it is `completed`.
    pub kind: String,
    pub title: String,
    pub distance_m: Option<f64>,
    pub duration_min: Option<f64>,
    pub notes: String,
    pub start: String,
    pub end: String,
    /// Planned sessions are all-day until they are matched to a recorded
    /// run, which replaces them with the activity's real time slot.
    pub all_day: bool,
    /// True when the runner has dragged this session off the day the plan
    /// generated it for. The grid marks these cells and the agenda marks these
    /// rows, because "you moved this" is the one fact the date alone cannot
    /// tell you.
    pub moved: bool,
    /// `planned` or `skipped`. A skipped session stays on the calendar with its
    /// kind — it was a decision, and hiding it would make the week's planned
    /// mileage disagree with what the runner can see.
    pub status: String,
    /// The day the plan generated this session into, for the marker's tooltip
    /// and the reset preview. `None` on an event with no plan behind it.
    pub planned_date: Option<NaiveDate>,
    /// A planned session whose day has passed without being run or skipped.
    /// Derived here rather than in each client so the calendar, the adherence
    /// ratio and the plan page all count a lapse the same way.
    pub missed: bool,
    /// Why the plan softened this session on its own, when it did. The chip
    /// tooltip, the agenda row and the session sheet all read this: an
    /// adjustment nobody explained reads as a bug, so the reason has to appear
    /// in the same frame as the softened session. `None` for a session the plan
    /// left alone, and for a banked run — the run happened, so there is nothing
    /// left to explain away.
    pub adjusted: Option<String>,
}

/// Emoji prefix for calendar titles, so the subscribed calendar reads
/// at a glance even in a cramped month cell. Every session — planned or
/// completed — is a run, so it always wears the same runner.
const RUN_EMOJI: &str = "🏃";

pub struct CalendarExport {
    pub path: PathBuf,
}

/// Signature line of every description.
const BRAND: &str = "Made with Runalytics - a virtual personal vibecoded coach 🏃";

/// Every session description — planned or completed — is: brand,
/// the event title, a `Plan:` lap list, then `Why:`.
pub fn workout_notes(workout: &PlanWorkout, plan: &TrainingPlan) -> String {
    let details = session::details_for_plan(workout, plan);
    let mut notes = format!("{BRAND}\n\n{RUN_EMOJI} {}", workout.title);
    // A title that already ends in its own distance would just repeat it.
    let total_m = workout.distance_m.unwrap_or_else(|| details.distance_m());
    if total_m > 0.0 && trim_total(&workout.title) == workout.title {
        let _ = write!(notes, " (total {:.1} km)", total_m / 1000.0);
    }
    if !details.steps.is_empty() {
        notes.push_str("\n\nPlan:");
        for step in &details.steps {
            let _ = write!(notes, "\n  • {}", lap_line(step));
        }
    }
    let _ = write!(notes, "\n\nWhy: {}", details.purpose);
    notes
}

/// Open-ended (easy) steps only have a floor; everything slower is fine.
///
/// The ceiling is stated as permission, not as an objective: a cap on an easy
/// day is the one number in a plan an athlete can be tempted to chase, and the
/// whole point of an easy day is that it feels easy.
fn no_faster_than(low: i64) -> String {
    format!(
        "no faster than {}/km, at conversational pace 🗣️ — a limit, not a target: \
         slower is fine, and often better",
        fmt_pace(low)
    )
}

/// Prescribed pace reads as `at`, recorded pace reads as `@` (see
/// [`ran_line`]). Two notations, two meanings: a plan note and a run log sit
/// side by side in the same exported event, and one glyph for both makes a
/// completed week unreadable.
fn lap_pace(step: &session::WorkoutStep) -> String {
    match (step.open_ended, step.pace_lo_s_km, step.pace_hi_s_km) {
        (true, Some(low), _) => format!(", {}", no_faster_than(low)),
        (false, Some(low), Some(high)) => format!(" at {}-{}/km", fmt_pace(low), fmt_pace(high)),
        _ => String::new(),
    }
}

/// One lap of the session, e.g. `3 × 1.6 km Tempo at 4:45-4:55/km + 400 m recovery`.
fn lap_line(step: &session::WorkoutStep) -> String {
    let (head, tail) = step.label.split_once(" - ").unwrap_or((&step.label, ""));
    let name = if head == "Long run" && !tail.is_empty() { tail } else { head };
    let mut part = String::new();
    if step.repeats > 1 {
        let _ = write!(part, "{} × ", step.repeats);
    }
    let _ = write!(part, "{} {}", fmt_distance(step.distance_m), name);
    part.push_str(&lap_pace(step));
    if step.recovery_m > 0.0 {
        let _ = write!(part, " + {} recovery", fmt_distance(step.recovery_m));
    }
    part
}

/// Strip a trailing ` <number> km` total, as older notes put on the name line.
fn trim_total(line: &str) -> &str {
    let Some(before) = line.strip_suffix(" km") else {
        return line;
    };
    match before.rsplit_once(' ') {
        Some((name, number)) if number.parse::<f64>().is_ok() => name,
        _ => line,
    }
}

/// The workout title from stored plan notes: the third line of the shared
/// pattern (or, for older notes, the first), minus emoji and any total.
fn plan_name(notes: &str) -> String {
    let line = stored_title_line(notes);
    let line = line.strip_prefix(RUN_EMOJI).unwrap_or(line).trim();
    let line = match line.rsplit_once(" (total ") {
        Some((title, _)) if line.ends_with(" km)") => title,
        _ => line,
    };
    trim_total(line).to_string()
}

fn stored_title_line(notes: &str) -> &str {
    let line = if notes.starts_with(BRAND) {
        notes.lines().nth(2)
    } else {
        notes.lines().next()
    };
    line.unwrap_or("Run").trim()
}

/// Stored plan notes normalised to the current layout; run stats and the
/// quoted plan are dropped.
fn completed_notes(stored: &str) -> String {
    // The stored title line keeps its `(total ...)`; older notes only have a name.
    let title = if stored.contains(" (total ") {
        stored_title_line(stored).to_string()
    } else {
        format!("{RUN_EMOJI} {}", plan_name(stored))
    };
    let mut notes = format!("{BRAND}\n\n{title}");
    // Notes from before the shared pattern have no body to carry over.
    let rest = if stored.starts_with(BRAND) {
        stored.splitn(4, '\n').nth(3).unwrap_or_default()
    } else {
        ""
    };
    let markers = ["—— Original plan ——", "—— Why ——", "Why:"];
    let cut = markers
        .iter()
        .filter_map(|marker| rest.find(marker))
        .min()
        .unwrap_or(rest.len());
    let plan = rest[..cut].trim();
    if !plan.is_empty() {
        let _ = write!(notes, "\n\n{plan}");
    }
    let why = rest
        .split_once("—— Why ——")
        .or_else(|| rest.split_once("Why:"))
        .map(|(_, why)| why.trim());
    if let Some(why) = why {
        let _ = write!(notes, "\n\nWhy: {why}");
    }
    notes
}

fn fmt_distance(metres: f64) -> String {
    if metres < 1000.0 {
        format!("{metres:.0} m")
    } else {
        format!("{:.1} km", metres / 1000.0)
    }
}

fn is_run(activity: &Activity) -> bool {
    let sport = activity.sport.to_lowercase();
    sport.contains("run") || matches!(sport.as_str(), "jog" | "jogging")
}

/// Bring the current plan up to the naming and description conventions:
/// quality sessions get their names and completed runs get notes rebuilt in
/// the shared pattern. Idempotent, so it is safe on every start.
pub fn adopt_naming(store: &Store) -> Result<()> {
    let Some((plan, workouts)) = store.current_plan()? else {
        return Ok(());
    };
    let completions = store.calendar_completions()?;
    for workout in workouts {
        let mut workout = workout;
        if let Some(title) = session::title_for_plan(&workout, &plan) {
            workout.title = title;
            store.update_workout(&workout)?;
        }
        if let Some(entry) = completions.iter().find(|entry| entry.0 == workout.id) {
            let notes = workout_notes(&workout, &plan);
            if entry.2 != notes {
                store.set_completion_notes(&workout.id, &notes)?;
            }
        }
    }
    Ok(())
}

pub fn reconcile(store: &Store) -> Result<()> {
    let Some((plan, workouts)) = store.current_plan()? else {
        return Ok(());
    };
    let completions = store.calendar_completions()?;
    let used: HashSet<i64> = completions.iter().map(|entry| entry.1).collect();
    let matched: HashSet<&str> = completions.iter().map(|entry| entry.0.as_str()).collect();
    let activities = store.list_activities(-1)?;
    let mut runs_by_day: HashMap<NaiveDate, Vec<&Activity>> = HashMap::new();
    for activity in &activities {
        if is_run(activity)
            && !used.contains(&activity.id)
            && activity.start_time >= plan.created_at
            && activity.duration_s > 0.0
        {
            runs_by_day
                .entry(activity.start_time.with_timezone(&Local).date_naive())
                .or_default()
                .push(activity);
        }
    }
    let mut workouts_by_day: HashMap<NaiveDate, Vec<&PlanWorkout>> = HashMap::new();
    for workout in &workouts {
        if workout.kind != WorkoutKind::Rest && !matched.contains(workout.id.as_str()) {
            workouts_by_day
                .entry(workout.date)
                .or_default()
                .push(workout);
        }
    }
    for (day, pending) in workouts_by_day {
        if pending.len() != 1 {
            continue;
        }
        let Some(runs) = runs_by_day.get(&day) else {
            continue;
        };
        if runs.len() != 1 {
            continue;
        }
        let workout = pending[0];
        let run = runs[0];
        let ratio = workout
            .distance_m
            .filter(|distance| *distance > 0.0)
            .map(|distance| run.distance_m / distance);
        if ratio.is_some_and(|ratio| !(0.5..=1.5).contains(&ratio)) {
            continue;
        }
        store.complete_calendar_workout(&workout.id, run.id, &workout_notes(workout, &plan))?;
    }
    Ok(())
}

pub fn match_activity(store: &Store, workout_id: &str, activity_id: i64) -> Result<()> {
    let (plan, workouts) = store
        .current_plan()?
        .ok_or_else(|| AppError::Other("No active plan".into()))?;
    let workout = workouts
        .iter()
        .find(|workout| workout.id == workout_id && workout.kind != WorkoutKind::Rest)
        .ok_or_else(|| AppError::Other("Planned workout not found".into()))?;
    let activity = store
        .activity_by_id(activity_id)?
        .filter(is_run)
        .ok_or_else(|| AppError::Other("Running activity not found".into()))?;
    store.complete_calendar_workout(workout_id, activity.id, &workout_notes(workout, &plan))
}

/// Snapshot the runner's load ratio and injury risk for every run activity
/// that does not have one yet, evaluated as of the day it was banked. This
/// is the calendar's half of "update the injury risk after every new done
/// activity": the numbers are frozen when the run lands, so a description
/// written in October keeps saying what the risk was in October even after
/// November's health data arrives. Called from every calendar refresh, so
/// both matched sessions and standalone synced runs are covered.
pub fn snapshot_scores(store: &Store, th: &Thresholds) -> Result<()> {
    let have: HashSet<i64> = store
        .activity_scores()?
        .into_iter()
        .map(|(activity_id, _, _, _)| activity_id)
        .collect();
    for activity in store.list_activities(-1)?.into_iter().filter(is_run) {
        if have.contains(&activity.id) || activity.duration_s <= 0.0 {
            continue;
        }
        let day = activity.start_time.with_timezone(&Local).date_naive();
        // The same load ratio the training-status chart shows for that day.
        let load = store
            .training_load_on_or_before(day)?
            .and_then(|entry| entry.ratio);
        // The same headline the dashboard hero shows, evaluated as of that
        // day: the blended risk (point-in-time formula averaged with the
        // range-aware factor sum), with the recent-sessions window pinned to
        // this run's own start — so the run being scored counts in its own
        // week's pressure, and an easy jog and a threshold session on the
        // same day get different numbers.
        let session_ts = activity.start_time.timestamp();
        let risk = metrics::risk_headline_as_of(store, day, 30, th, Some(session_ts))?;
        // The run's own impact: the same headline minus itself. Every run
        // has one — that is the point of the model — so every description
        // gets a delta, and it stays the same number no matter when the
        // snapshot is taken or how the model later evolves.
        let risk_delta =
            metrics::session_risk_impact(store, day, 30, th, session_ts, activity.id)?;
        store.insert_activity_score(activity.id, load, risk, risk_delta)?;
    }
    Ok(())
}

/// The stat block appended to a banked run's description: the load and injury
/// risk the body was carrying, then what was actually run. The scores are
/// percentages — injury risk is already 0..=100, and the load ratio is the
/// COROS Intensity Trend percentage (1.35 reads as 135 %, the number the
/// training-status chart's band is named by). The risk delta is the run's own
/// impact — risk with the session minus risk without it — so it is never
/// missing: an easy jog moves the number a little, a threshold session a lot.
/// The load delta compares against the run banked before this one (snapshots
/// arrive oldest first), and a first-ever run simply has no earlier ratio to
/// change against.
///
/// The `Ran:` line closes the block because it answers a different question
/// than the two above it: those say what the load did to the runner, this one
/// says what the runner did that day. A matched session keeps the planned
/// distance in its title, so without this line the description would restate
/// the plan and never record the outcome.
fn score_lines(
    snapshots: &[(i64, Option<f64>, u32, i64)],
    index: Option<usize>,
    activity: &Activity,
) -> String {
    let mut lines = Vec::new();
    if let Some(index) = index {
        let (id, load, risk, risk_delta) = snapshots[index];
        debug_assert_eq!(id, activity.id);
        let previous = index.checked_sub(1).map(|i| snapshots[i]);
        if let Some(ratio) = load {
            let load_pct = ratio * 100.0;
            let mut load_line = format!("Load: {load_pct:.0}%");
            if let Some((_, Some(before), _, _)) = previous {
                let delta = load_pct - before * 100.0;
                if delta.abs() >= 0.5 {
                    let _ = write!(load_line, " ({delta:+.0}%)");
                }
            }
            lines.push(load_line);
        }
        lines.push(format!("Injury risk: {risk}% ({risk_delta:+}%)"));
    }
    // A run the provider never scored still has a distance and a pace, so the
    // block is not tied to the snapshots existing.
    if let Some(ran) = ran_line(activity) {
        lines.push(ran);
    }
    // The block always starts with its own blank line, whatever it ends up
    // holding — a run with no Load line must not have its risk line glued to
    // the paragraph above.
    if lines.is_empty() {
        String::new()
    } else {
        format!("\n\n{}", lines.join("\n"))
    }
}

/// What the run actually was: real distance and real average pace, as opposed
/// to the plan the session was matched to. A run with no distance at all (an
/// indoor session, a provider that never reported one) has nothing to say.
fn ran_line(activity: &Activity) -> Option<String> {
    if activity.distance_m <= 0.0 {
        return None;
    }
    let distance = fmt_distance(activity.distance_m);
    Some(match average_pace_s_km(activity) {
        Some(pace) => format!("Ran: {distance} @ {}/km", fmt_pace(pace.round() as i64)),
        None => format!("Ran: {distance}"),
    })
}

/// The run's average pace in seconds per km: what the provider reported, or
/// the flat average over the recorded distance and duration when it didn't.
fn average_pace_s_km(activity: &Activity) -> Option<f64> {
    activity.avg_pace_s_km.or_else(|| {
        (activity.distance_m > 0.0 && activity.duration_s > 0.0)
            .then_some(activity.duration_s / (activity.distance_m / 1000.0))
    })
}

/// Reschedule a planned, not-yet-run session to any day from today to the end
/// of the plan. Plans meet real life: a session missed this week usually
/// belongs next week, not in the bin. Race day is fixed by the event,
/// completed runs are history, and the plan's own span bounds the move.
///
/// The generated day is *not* touched: `planned_date` stays where the plan put
/// it and `moved_at` records that the runner overrode that. Without the pair
/// there is nothing to reset back to, no way to mark a touched day, and no way
/// to tell that a week has drifted at all.
pub fn move_workout(
    store: &Store,
    workout_id: &str,
    to: NaiveDate,
    today: NaiveDate,
) -> Result<PlanWorkout> {
    let (plan, workouts) = store
        .current_plan()?
        .ok_or_else(|| AppError::Other("No active plan".into()))?;
    let workout = workouts
        .iter()
        .find(|workout| workout.id == workout_id)
        .ok_or_else(|| AppError::Other("Planned workout not found".into()))?;
    if workout.kind == WorkoutKind::Race {
        return Err(AppError::Other("Race day is fixed by the event".into()));
    }
    let completed = store
        .calendar_completions()?
        .iter()
        .any(|entry| entry.0 == workout.id);
    if workout.done || completed {
        return Err(AppError::Other("This session is already completed".into()));
    }
    if to < today {
        return Err(AppError::Other(
            "Sessions can't be moved into the past".into(),
        ));
    }
    if to > plan.target_date {
        return Err(AppError::Other(
            "Sessions can't be moved past the end of your plan".into(),
        ));
    }
    // Two long runs on consecutive days is the one arrangement the generator
    // would never produce and a drag can: the spacing it applies is a fixed
    // weekday, so the invariant holds by construction until something moves
    // (`ins §7.2` asks for the guard *before* anything can move one).
    if workout.kind == WorkoutKind::LongRun {
        let touching = workouts.iter().any(|other| {
            other.id != workout.id
                && other.kind == WorkoutKind::LongRun
                && (other.date - to).num_days().abs() == 1
        });
        if touching {
            return Err(AppError::Other(
                "That would put two long runs back to back — pick another day".into(),
            ));
        }
    }
    let moved = PlanWorkout {
        date: to,
        // Stamped only on a real move, so a session that never moved keeps a
        // clean `moved_at: None` rather than a timestamp from a no-op write.
        moved_at: if to != workout.date {
            Some(Utc::now())
        } else {
            workout.moved_at
        },
        ..workout.clone()
    };
    if to != workout.date {
        store.update_workout(&moved)?;
    }
    Ok(moved)
}

/// The sessions in one ISO week that have drifted off their generated days,
/// each with the day a reset would restore it to and whether it can be. Feeds
/// the confirm sheet, which must name the days it is about to move before it
/// moves them — and must not promise a session it is going to refuse.
pub fn week_deviations(
    store: &Store,
    week_start: NaiveDate,
    today: NaiveDate,
) -> Result<Vec<(PlanWorkout, NaiveDate, bool)>> {
    let completions = store.calendar_completions()?;
    let (_, workouts) = store
        .current_plan()?
        .ok_or_else(|| AppError::Other("No active plan".into()))?;
    let (from, to) = iso_week_bounds(week_start);
    Ok(workouts
        .iter()
        .filter(|w| w.date >= from && w.date <= to && w.is_moved())
        .map(|w| {
            let planned = w.planned_date;
            let restorable = can_restore(w, planned, today, &completions);
            (w.clone(), planned, restorable)
        })
        .collect())
}

/// Whether a reset may put one session back. Shared with [`week_deviations`] so
/// the preview can never promise a move the reset will skip.
///
/// The past case is the subtle one: restoring a session to the day the plan
/// generated is only sensible while that day is still ahead of the runner. A
/// Tuesday session dragged to Saturday has no Tuesday to go back to, and
/// pretending otherwise would file a pending run under a day that has already
/// happened — the one thing the move guards exist to prevent.
fn can_restore(
    workout: &PlanWorkout,
    planned: NaiveDate,
    today: NaiveDate,
    completions: &[(String, i64, String)],
) -> bool {
    workout.kind != WorkoutKind::Race
        && !workout.done
        && workout.status == WorkoutStatus::Planned
        && planned >= today
        && !completions.iter().any(|entry| entry.0 == workout.id)
}

/// Put one ISO week back onto the days the plan generated. Only sessions that
/// were *moved* return: race day is fixed by the event, a completed run is
/// history, a deliberately skipped session stays skipped on the day it was
/// skipped, and a session whose generated day has already passed has nowhere to
/// come back to (see [`can_restore`]).
///
/// Returns how many sessions were restored, so the caller can say so out loud
/// rather than leaving the calendar to shift silently.
pub fn reset_week(store: &Store, week_start: NaiveDate, today: NaiveDate) -> Result<usize> {
    let completions = store.calendar_completions()?;
    let pending: Vec<PlanWorkout> = {
        let (_, workouts) = store
            .current_plan()?
            .ok_or_else(|| AppError::Other("No active plan".into()))?;
        let (from, to) = iso_week_bounds(week_start);
        workouts
            .iter()
            .filter(|w| {
                w.date >= from && w.date <= to && w.is_moved() && can_restore(w, w.planned_date, today, &completions)
            })
            .cloned()
            .collect()
    };
    let count = pending.len();
    for workout in pending {
        store.update_workout(&PlanWorkout {
            date: workout.planned_date,
            moved_at: None,
            ..workout
        })?;
    }
    Ok(count)
}

/// Mark a planned session as deliberately skipped, or undo that. Skipping is
/// the only way a session leaves the schedule *without* becoming a lapse, which
/// is why the adherence ratio and the missed-workout detector both read it.
pub fn set_skipped(store: &Store, workout_id: &str, skipped: bool) -> Result<PlanWorkout> {
    let (_, workouts) = store
        .current_plan()?
        .ok_or_else(|| AppError::Other("No active plan".into()))?;
    let workout = workouts
        .iter()
        .find(|workout| workout.id == workout_id)
        .ok_or_else(|| AppError::Other("Planned workout not found".into()))?;
    if workout.kind == WorkoutKind::Race {
        return Err(AppError::Other("Race day is fixed by the event".into()));
    }
    if workout.done || store.calendar_completions()?.iter().any(|e| e.0 == workout.id) {
        return Err(AppError::Other("This session is already completed".into()));
    }
    let updated = PlanWorkout {
        status: if skipped { WorkoutStatus::Skipped } else { WorkoutStatus::Planned },
        ..workout.clone()
    };
    store.update_workout(&updated)?;
    Ok(updated)
}

/// The Monday and Sunday bounding the ISO week containing `day`, so a week's
/// worth of sessions can be selected without teaching SQLite ISO-week
/// bucketing (which does not survive a year boundary — see `WEEK_START_SQL`).
fn iso_week_bounds(day: NaiveDate) -> (NaiveDate, NaiveDate) {
    let offset = (day.weekday().num_days_from_monday()) as i64;
    let monday = day - Duration::days(offset);
    (monday, monday + Duration::days(6))
}

/// The `kind` a calendar event carries, spelled the way the frontend's chip
/// classes expect.
fn event_kind(kind: WorkoutKind) -> String {
    if kind == WorkoutKind::LongRun {
        "longRun".into()
    } else {
        format!("{:?}", kind).to_lowercase()
    }
}

pub fn events(store: &Store) -> Result<Vec<CalendarEvent>> {
    let completions = store.calendar_completions()?;
    let current = store.current_plan()?;
    let adjustments: HashMap<String, String> = store.workout_adjustments()?.into_iter().collect();
    let snapshots = store.activity_scores()?;
    let snapshot_index: HashMap<i64, usize> = snapshots
        .iter()
        .enumerate()
        .map(|(i, (activity_id, _, _, _))| (*activity_id, i))
        .collect();
    let planned: HashMap<&str, (String, String)> = current
        .iter()
        .flat_map(|(_, workouts)| workouts.iter())
        .map(|workout| {
            (
                workout.id.as_str(),
                (workout.title.clone(), event_kind(workout.kind)),
            )
        })
        .collect();
    let mut output = Vec::new();
    for activity in store.list_activities(-1)?.into_iter().filter(is_run) {
        let completion = completions.iter().find(|entry| entry.1 == activity.id);
        // Matched runs keep the name and the kind they had in the plan (the
        // plan's own title while it exists, else the name in the stored
        // notes); standalone runs get a plain title and no kind to inherit.
        let meta = completion.and_then(|entry| planned.get(entry.0.as_str()));
        let base_title = match (completion, meta) {
            (Some(_), Some((title, _))) => title.clone(),
            (Some(entry), None) => plan_name(&entry.2),
            (None, _) => format!("Run {:.2} km", activity.distance_m / 1000.0),
        };
        let kind = meta
            .map(|(_, kind)| kind.clone())
            .unwrap_or_else(|| "completed".into());
        let notes = match completion {
            Some(entry) => completed_notes(&entry.2),
            None => format!("{BRAND}\n\n{RUN_EMOJI} {base_title}"),
        };
        // A banked run carries what the body was carrying when it landed.
        let notes = format!(
            "{notes}{}",
            score_lines(&snapshots, snapshot_index.get(&activity.id).copied(), &activity)
        );
        output.push(CalendarEvent {
            id: completion
                .map(|entry| entry.0.clone())
                .unwrap_or_else(|| format!("activity-{}", activity.id)),
            workout_id: completion.map(|entry| entry.0.clone()),
            activity_id: Some(activity.id),
            date: activity.start_time.with_timezone(&Local).date_naive(),
            kind,
            title: format!("{RUN_EMOJI} {base_title}"),
            distance_m: Some(activity.distance_m),
            duration_min: Some(activity.duration_s / 60.0),
            notes,
            start: activity.start_time.format("%Y%m%dT%H%M%SZ").to_string(),
            end: (activity.start_time
                + Duration::seconds(activity.duration_s.ceil().max(1.0) as i64))
            .format("%Y%m%dT%H%M%SZ")
            .to_string(),
            all_day: false,
            // A banked run is not a deviation: it happened, wherever it
            // happened, so there is nothing to mark or restore.
            moved: false,
            status: "planned".into(),
            planned_date: None,
            missed: false,
            // A softened session that was then run keeps its reason: the title
            // it inherited still says "(was: …)", and a half-said change is
            // worse than none.
            adjusted: completion.and_then(|entry| adjustments.get(entry.0.as_str()).cloned()),
        });
    }
    if let Some((plan, workouts)) = current {
        for workout in workouts {
            if workout.kind == WorkoutKind::Rest
                || completions.iter().any(|entry| entry.0 == workout.id)
            {
                continue;
            }
            let details = session::details_for_plan(&workout, &plan);
            let duration = details
                .estimated_min
                .or(workout.duration_min)
                .unwrap_or(45.0);
            output.push(CalendarEvent {
                id: workout.id.clone(),
                workout_id: Some(workout.id.clone()),
                activity_id: None,
                date: workout.date,
                kind: event_kind(workout.kind),
                title: format!("{RUN_EMOJI} {}", workout.title),
                distance_m: workout.distance_m,
                duration_min: Some(duration),
                notes: workout_notes(&workout, &plan),
                // All-day until matched: the plan says *what* to run that
                // day, not *when*. Matching a run swaps in the real slot.
                start: workout.date.format("%Y%m%d").to_string(),
                end: (workout.date + Duration::days(1))
                    .format("%Y%m%d")
                    .to_string(),
                all_day: true,
                moved: workout.is_moved(),
                status: workout_status_name(workout.status).to_string(),
                planned_date: Some(workout.planned_date),
                missed: workout.is_missed(Utc::now().date_naive()),
                adjusted: adjustments.get(workout.id.as_str()).cloned(),
            });
        }
    }
    output.sort_by_key(|event| (event.date, event.id.clone()));
    Ok(output)
}

impl CalendarExport {
    pub fn new(data_dir: &PathBuf) -> Self {
        Self {
            path: data_dir.join("runalytics.ics"),
        }
    }

    pub fn regenerate(&self, store: &Store) -> Result<PathBuf> {
        let out = render(store, &events(store)?)?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = self.path.with_extension("ics.tmp");
        std::fs::write(&temporary, out)?;
        std::fs::rename(temporary, &self.path)?;
        Ok(self.path.clone())
    }
}

fn render(store: &Store, events: &[CalendarEvent]) -> Result<String> {
    let mut output = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Runalytics//EN\r\nCALSCALE:GREGORIAN\r\nMETHOD:PUBLISH\r\nX-WR-CALNAME:Runalytics Training\r\nREFRESH-INTERVAL;VALUE=DURATION:PT1H\r\nX-PUBLISHED-TTL:PT1H\r\n");
    for event in events {
        let content = serde_json::to_string(event)?;
        let (sequence, modified) = store.calendar_revision(&event.id, &content)?;
        output.push_str("BEGIN:VEVENT\r\n");
        let dates = if event.all_day {
            [
                format!("DTSTART;VALUE=DATE:{}", event.start),
                format!("DTEND;VALUE=DATE:{}", event.end),
            ]
        } else {
            [
                format!("DTSTART:{}", event.start),
                format!("DTEND:{}", event.end),
            ]
        };
        let lines = [
            format!("UID:{}@runalytics", event.id),
            format!("DTSTAMP:{modified}"),
            format!("LAST-MODIFIED:{modified}"),
            format!("SEQUENCE:{sequence}"),
            dates[0].clone(),
            dates[1].clone(),
            format!("SUMMARY:{}", escape(&event.title)),
            format!("DESCRIPTION:{}", escape(&event.notes)),
            "STATUS:CONFIRMED".into(),
            "TRANSP:TRANSPARENT".into(),
        ];
        for line in lines {
            fold_line(&mut output, &line);
        }
        output.push_str("END:VEVENT\r\n");
    }
    output.push_str("END:VCALENDAR\r\n");
    Ok(output)
}

fn fold_line(output: &mut String, line: &str) {
    let mut bytes = 0;
    for character in line.chars() {
        if bytes + character.len_utf8() > 75 {
            output.push_str("\r\n ");
            bytes = 1;
        }
        output.push(character);
        bytes += character.len_utf8();
    }
    output.push_str("\r\n");
}

fn escape(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

fn fmt_pace(seconds: i64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, TimeZone, Utc};

    fn fixture() -> (Store, TrainingPlan, Vec<PlanWorkout>) {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let request = PlanRequest {
            distance: RaceDistance::K10,
            kind: PlanKind::Race,
            target_date: Utc::now().date_naive() + Duration::days(56),
            threshold_sec_km: 285.0,
            weekly_frequency: 4,
            longrun_day: 7,
            increase: IncreaseMethod::Moderate,
            current_weekly_km: Some(30.0),
        };
        let (plan, workouts) = crate::plan::engine::generate(&request, 30.0).unwrap();
        store.insert_plan(&plan).unwrap();
        store.insert_workouts(&workouts).unwrap();
        (store, plan, workouts)
    }

    fn insert_run(store: &Store, workout: &PlanWorkout, sport: &str, record: &str) -> i64 {
        let start = Local
            .from_local_datetime(&workout.date.and_hms_opt(23, 45, 0).unwrap())
            .single()
            .unwrap()
            .timestamp();
        let conn = store.conn.lock().unwrap();
        conn.execute("INSERT INTO activities (provider, provider_record_id, sport, start_ts, duration_s, distance_m) VALUES ('coros', ?1, ?2, ?3, 3600, ?4)", rusqlite::params![record, sport, start, workout.distance_m]).unwrap();
        conn.last_insert_rowid()
    }

    /// A run shaped for `score_lines`, which takes the activity rather than
    /// querying the store — the score block is pure formatting. Duration is
    /// left at zero so a case without a reported pace really has no pace.
    fn scored_run(id: i64, distance_m: f64, pace_s_km: Option<f64>) -> Activity {
        Activity {
            id,
            provider: ProviderKind::Coros,
            provider_record_id: id.to_string(),
            sport: "running".into(),
            start_time: Utc::now(),
            duration_s: 0.0,
            distance_m,
            avg_hr: None,
            max_hr: None,
            avg_pace_s_km: pace_s_km,
            elevation_gain_m: None,
            has_track: false,
        }
    }

    #[test]
    fn completed_run_replaces_plan_and_keeps_original_after_plan_deletion() {
        let (store, plan, workouts) = fixture();
        let workout = &workouts[0];
        let original = render(&store, &events(&store).unwrap()).unwrap();
        let activity_id = insert_run(&store, workout, "running", "one");
        reconcile(&store).unwrap();
        reconcile(&store).unwrap();
        let updated = events(&store).unwrap();
        assert_eq!(updated.len(), workouts.len());
        let done = updated.iter().find(|event| event.id == workout.id).unwrap();
        assert_eq!(done.activity_id, Some(activity_id));
        assert!(done.notes.contains("Why:"));
        assert!(done.notes.starts_with(BRAND));
        assert!(
            store
                .plan_workouts(&plan.id)
                .unwrap()
                .iter()
                .find(|entry| entry.id == workout.id)
                .unwrap()
                .done
        );
        let ics = render(&store, &updated).unwrap();
        // Planned sessions are all-day (VALUE=DATE, emoji title); the
        // matched completed run keeps its real timed slot.
        assert!(original
            .lines()
            .all(|line| !line.starts_with("DTSTART:") && !line.contains("Completed run")));
        assert!(original.contains("DTSTART;VALUE=DATE:"));
        assert!(updated
            .iter()
            .filter(|event| event.activity_id.is_none())
            .all(|event| event.all_day
                && event.start.len() == 8
                && event.end.len() == 8
                && event.title.starts_with("🏃 ")));
        let done = updated.iter().find(|event| event.id == workout.id).unwrap();
        assert!(!done.all_day);
        // Completed runs keep the original plan name, never "Completed run".
        assert!(done.title.starts_with("🏃 "));
        assert!(done.title.contains(&workout.title));
        assert!(!done.title.contains("Completed run"));
        assert!(ics.contains(&format!("SUMMARY:🏃 {}", escape(&workout.title))));
        assert!(ics.contains("SEQUENCE:0"));
        assert!(ics.contains("SEQUENCE:1"));
        assert_eq!(ics, render(&store, &updated).unwrap());
        assert_eq!(
            ics.matches(&format!("UID:{}@runalytics", workout.id))
                .count(),
            1
        );
        store.delete_plan(&plan.id).unwrap();
        let retained = events(&store).unwrap();
        assert_eq!(retained.len(), 1);
        assert!(retained[0].notes.contains("Why:"));
    }

    #[test]
    fn planned_sessions_move_forward_within_the_plan_and_stay_pending() {
        let (store, plan, workouts) = fixture();
        let workout = workouts
            .iter()
            .find(|w| {
                w.kind != WorkoutKind::Race && w.date.weekday().num_days_from_monday() < 6
            })
            .unwrap();
        let today = workout.date;
        let next_day = workout.date + Duration::days(1);
        let moved = move_workout(&store, &workout.id, next_day, today).unwrap();
        assert_eq!(moved.date, next_day);
        let stored = store.plan_workouts(&plan.id).unwrap();
        assert_eq!(
            stored.iter().find(|w| w.id == workout.id).unwrap().date,
            next_day
        );
        assert!(events(&store)
            .unwrap()
            .iter()
            .any(|e| e.workout_id.as_deref() == Some(&workout.id) && e.date == next_day));

        // A week that slipped away is the reason to drag a session at all, so
        // the move crosses week boundaries freely — but never backwards in
        // time, and never past the race the plan was built for.
        let next_week = today + Duration::days(7);
        assert_eq!(
            move_workout(&store, &workout.id, next_week, today)
                .unwrap()
                .date,
            next_week
        );
        assert!(move_workout(&store, &workout.id, today - Duration::days(1), today).is_err());
        assert!(
            move_workout(&store, &workout.id, plan.target_date + Duration::days(1), today)
                .is_err()
        );

        // A completed session is history, and race day belongs to the event.
        let activity = insert_run(&store, &moved, "running", "moved");
        match_activity(&store, &workout.id, activity).unwrap();
        assert!(move_workout(&store, &workout.id, today, today).is_err());
        let race = workouts
            .iter()
            .find(|w| w.kind == WorkoutKind::Race)
            .unwrap();
        assert!(
            move_workout(&store, &race.id, race.date - Duration::days(1), race.date).is_err()
        );
    }

    /// A moved session keeps the day it was generated for, carries a timestamp,
    /// and is flagged as moved on the calendar — the three things that make a
    /// reschedule reversible instead of a silent rewrite of the plan.
    #[test]
    fn a_move_keeps_the_generated_day_and_marks_the_session_moved() {
        let (store, plan, workouts) = fixture();
        let workout = workouts
            .iter()
            .find(|w| w.kind != WorkoutKind::Race)
            .unwrap()
            .clone();
        let generated = workout.date;
        let today = workout.date;
        let to = workout.date + Duration::days(3);

        let moved = move_workout(&store, &workout.id, to, today).unwrap();
        assert_eq!(moved.date, to);
        assert_eq!(
            moved.planned_date, generated,
            "the generated day must survive a reschedule"
        );
        assert!(moved.moved_at.is_some(), "a real move stamps a time");

        let stored = store.plan_workouts(&plan.id).unwrap();
        let stored = stored.iter().find(|w| w.id == workout.id).unwrap();
        assert!(stored.is_moved());
        assert!(!stored.is_missed(today));
        let event = events(&store)
            .unwrap()
            .into_iter()
            .find(|e| e.workout_id.as_deref() == Some(workout.id.as_str()))
            .unwrap();
        assert!(event.moved);
        assert_eq!(event.planned_date, Some(generated));

        // Moving a session to the day it is already on is not a move: it must
        // not manufacture a timestamp, and it must not clear the marker either.
        let again = move_workout(&store, &workout.id, to, today).unwrap();
        assert_eq!(again.moved_at, stored.moved_at);
    }

    /// The back-to-back long-run guard (F-22): the generator keeps one long
    /// run a week by construction, but F-03 lets a runner drag one anywhere,
    /// and two long runs on consecutive days is the arrangement no plan
    /// should hold. Only long runs guard each other — an easy run may move
    /// next to a long run freely.
    #[test]
    fn a_long_run_cannot_be_moved_next_to_another_long_run() {
        let (store, plan, workouts) = fixture();
        let longs: Vec<&PlanWorkout> = workouts
            .iter()
            .filter(|w| w.kind == WorkoutKind::LongRun)
            .collect();
        assert!(longs.len() >= 2, "fixture should span several long runs");
        let (first, second) = (longs[0], longs[1]);
        let today = first.date;
        let taken =
            |day: NaiveDate, except: &str| workouts.iter().any(|w| w.date == day && w.id != except);

        // The days either side of the *next* long run are the refusal.
        for offset in [-1, 1] {
            let adjacent = second.date + Duration::days(offset);
            let err = move_workout(&store, &first.id, adjacent, today).unwrap_err();
            assert!(
                err.to_string().contains("back to back"),
                "moving to {adjacent} (offset {offset}) should refuse, got: {err}"
            );
        }

        // Two days clear of it, on an unoccupied day, is fine.
        let clear = (-3..=3)
            .map(|d| second.date + Duration::days(d))
            .find(|&day| {
                day >= today
                    && day <= plan.target_date
                    && (day - second.date).num_days().abs() >= 2
                    && !taken(day, &first.id)
            })
            .expect("a day clear of the next long run");
        let moved = move_workout(&store, &first.id, clear, today).unwrap();
        assert_eq!(moved.date, clear);

        // An easy run may sit beside a long run — the guard stacks long
        // efforts, it does not quarantine them.
        let easy = workouts
            .iter()
            .find(|w| w.kind == WorkoutKind::Easy)
            .unwrap();
        let beside = [-1, 1]
            .iter()
            .map(|&d| second.date + Duration::days(d))
            .find(|&day| day >= today && !taken(day, &easy.id))
            .expect("a free day beside the next long run");
        let moved = move_workout(&store, &easy.id, beside, today).unwrap();
        assert_eq!(moved.date, beside);
    }

    /// Skipping is how a session leaves the schedule without becoming a lapse,
    /// so it has to be a status the plan keeps — not a deletion, and not a
    /// completion.
    #[test]
    fn skipping_a_session_is_a_reversible_status_not_a_deletion() {
        let (store, plan, workouts) = fixture();
        let workout = workouts
            .iter()
            .find(|w| w.kind != WorkoutKind::Race)
            .unwrap()
            .clone();
        let today = workout.date;

        let skipped = set_skipped(&store, &workout.id, true).unwrap();
        assert_eq!(skipped.status, WorkoutStatus::Skipped);
        assert!(!skipped.done, "skipping is not running it");
        let stored = store.plan_workouts(&plan.id).unwrap();
        assert!(stored.iter().any(|w| w.id == workout.id));
        let event = events(&store)
            .unwrap()
            .into_iter()
            .find(|e| e.workout_id.as_deref() == Some(workout.id.as_str()))
            .unwrap();
        assert_eq!(event.status, "skipped");
        // A skipped session is a decision, so it is never a lapse — which is
        // what stops a deliberate rest from dragging adherence down.
        assert!(!skipped.is_missed(today + Duration::days(1)));

        set_skipped(&store, &workout.id, false).unwrap();
        let stored = store.plan_workouts(&plan.id).unwrap();
        assert_eq!(
            stored.iter().find(|w| w.id == workout.id).unwrap().status,
            WorkoutStatus::Planned
        );

        // Race day belongs to the event, and a finished run is history.
        let race = workouts.iter().find(|w| w.kind == WorkoutKind::Race).unwrap();
        assert!(set_skipped(&store, &race.id, true).is_err());
        let activity = insert_run(&store, &workout, "running", "skipped-run");
        match_activity(&store, &workout.id, activity).unwrap();
        assert!(set_skipped(&store, &workout.id, true).is_err());
    }

    /// A reset returns a drifted week to the plan's own layout — and refuses the
    /// sessions where "put it back" is not a meaningful instruction.
    #[test]
    fn reset_week_restores_drifted_sessions_and_refuses_the_rest() {
        let (store, plan, workouts) = fixture();
        // Two sessions sharing one ISO week, both early enough in it that a
        // day's slip stays inside the same week — otherwise the test would be
        // asserting about two different weeks. The plan's opening week can be
        // partial, so the candidates are grouped by week and the first week
        // with two of them wins.
        let monday_of = |d: NaiveDate| {
            d - Duration::days(d.weekday().num_days_from_monday() as i64)
        };
        let early = |w: &&PlanWorkout| {
            w.kind != WorkoutKind::Race && w.date.weekday().num_days_from_monday() <= 3
        };
        let monday = workouts
            .iter()
            .filter(early)
            .map(|w| monday_of(w.date))
            .find(|week| {
                workouts.iter().filter(early).filter(|w| monday_of(w.date) == *week).count() >= 2
            })
            .expect("a generated plan has a full training week");
        let pair: Vec<PlanWorkout> = workouts
            .iter()
            .filter(early)
            .filter(|w| monday_of(w.date) == monday)
            .take(2)
            .cloned()
            .collect();
        let (first, second) = (&pair[0], &pair[1]);

        // `today` is the earlier session's day, so both generated days are
        // still ahead of the runner and restoring is legal.
        let today = first.date.min(second.date);
        for workout in [&first, &second] {
            move_workout(&store, &workout.id, workout.date + Duration::days(1), today).unwrap();
        }
        let deviations = week_deviations(&store, monday, today).unwrap();
        assert_eq!(deviations.len(), 2);
        assert!(
            deviations.iter().all(|(_, _, ok)| *ok),
            "both drifted sessions are unfinished, so both are restorable"
        );

        // A completed session is history: it is listed, but not offered.
        let activity = insert_run(&store, &first, "running", "reset-run");
        match_activity(&store, &first.id, activity).unwrap();
        let deviations = week_deviations(&store, monday, today).unwrap();
        let listed = |id: &str| deviations.iter().find(|(w, _, _)| w.id == id);
        assert!(listed(&first.id).is_some(), "the drift is still visible");
        assert!(!listed(&first.id).unwrap().2, "but not restorable");
        assert!(listed(&second.id).unwrap().2);

        let restored = reset_week(&store, monday, today).unwrap();
        assert_eq!(restored, 1, "only the unfinished session came back");
        let stored = store.plan_workouts(&plan.id).unwrap();
        let back = stored.iter().find(|w| w.id == second.id).unwrap();
        assert!(!back.is_moved(), "a restored session is no longer a deviation");
        assert!(back.moved_at.is_none(), "the audit trail is cleared too");
        let ran = stored.iter().find(|w| w.id == first.id).unwrap();
        assert!(ran.done, "resetting a week must not undo a run");
        assert!(ran.is_moved(), "and it stays where the runner ran it");

        // Restoring into a day that has already passed is refused: the plan's
        // Tuesday is gone, so the session stays where the runner put it. The
        // drift is written straight to the store because the move guard itself
        // forbids dragging a session into the past — this state can only arise
        // from time passing while a session sits moved.
        let later_today = plan.target_date;
        let drifted = store
            .plan_workouts(&plan.id)
            .unwrap()
            .into_iter()
            .find(|w| w.id == second.id)
            .unwrap()
            .clone();
        store
            .update_workout(&PlanWorkout {
                date: drifted.planned_date + Duration::days(1),
                moved_at: Some(Utc::now()),
                ..drifted
            })
            .unwrap();
        assert_eq!(
            reset_week(&store, monday, later_today).unwrap(),
            0,
            "nothing may be restored into the past"
        );
        let stored = store.plan_workouts(&plan.id).unwrap();
        assert!(stored.iter().find(|w| w.id == second.id).unwrap().is_moved());
    }

    #[test]
    fn notes_are_brand_then_title_then_plan_then_why() {
        let (store, plan, workouts) = fixture();
        let workout = workouts
            .iter()
            .find(|workout| workout.kind == WorkoutKind::Intervals || workout.kind == WorkoutKind::Tempo)
            .unwrap();
        let notes = workout_notes(workout, &plan);
        let total = format!(" (total {:.1} km)", workout.distance_m.unwrap() / 1000.0);
        assert_eq!(
            notes.lines().nth(2),
            Some(format!("{RUN_EMOJI} {}{total}", workout.title).as_str())
        );
        assert!(notes.lines().nth(3) == Some(""));
        assert!(notes.lines().nth(4) == Some("Plan:"));
        assert!(!notes.contains("——"));
        let brand = notes.find(BRAND).unwrap();
        let plan_list = notes.find("\n\nPlan:\n  • ").unwrap();
        let why = notes.find("\n\nWhy: ").unwrap();
        assert_eq!(brand, 0);
        assert!(brand < plan_list && plan_list < why);
        insert_run(&store, workout, "run", "done");
        reconcile(&store).unwrap();
        let done = events(&store)
            .unwrap()
            .into_iter()
            .find(|event| event.id == workout.id)
            .unwrap();
        assert_eq!(done.notes.matches(BRAND).count(), 1);
        assert!(done.notes.contains("\n\nWhy: "));
        assert!(done.notes.contains("\n\nPlan:\n  • "));
        assert!(!done.notes.contains("——"));
    }

    #[test]
    fn quality_sessions_are_named_and_easy_steps_have_no_upper_bound() {
        let (store, plan, workouts) = fixture();
        let workout = workouts
            .iter()
            .find(|workout| matches!(workout.kind, WorkoutKind::Intervals | WorkoutKind::Tempo))
            .unwrap();
        assert!(workout.title.contains(session::TITLE_SEP), "{}", workout.title);
        let notes = workout_notes(workout, &plan);
        let warm = notes
            .lines()
            .find(|line| line.starts_with("  • ") && line.contains("Warm-up"))
            .unwrap();
        assert!(warm.contains(", no faster than "), "{warm}");
        assert!(warm.contains("at conversational pace 🗣️"), "{warm}");
        // The ceiling must read as permission, never as a target to hit.
        assert!(warm.contains("a limit, not a target"), "{warm}");

        // A plan stored with the old plain titles is renamed in place.
        let mut old = workout.clone();
        old.title = "6 x 400 m intervals".into();
        store.update_workout(&old).unwrap();
        adopt_naming(&store).unwrap();
        let (_, stored) = store.current_plan().unwrap().unwrap();
        let renamed = stored.iter().find(|w| w.id == workout.id).unwrap();
        assert!(renamed.title.contains(session::TITLE_SEP), "{}", renamed.title);
        adopt_naming(&store).unwrap();
        let (_, again) = store.current_plan().unwrap().unwrap();
        assert_eq!(again.iter().find(|w| w.id == workout.id).unwrap().title, renamed.title);
    }

    #[test]
    fn a_finished_run_keeps_the_kind_it_was_planned_as() {        let (store, _, workouts) = fixture();
        let long_run = workouts
            .iter()
            .find(|workout| workout.kind == WorkoutKind::LongRun)
            .unwrap();
        let activity_id = insert_run(&store, long_run, "run", "long");
        match_activity(&store, &long_run.id, activity_id).unwrap();
        let done = events(&store)
            .unwrap()
            .into_iter()
            .find(|event| event.id == long_run.id)
            .unwrap();
        // The chip's colour comes from the kind, so a finished long run must
        // not arrive as the generic tone every completed run used to wear.
        assert_eq!(done.kind, "longRun");
        let tempo = workouts
            .iter()
            .find(|workout| workout.kind == WorkoutKind::Tempo)
            .unwrap();
        let tempo_run = insert_run(&store, tempo, "run", "tempo");
        match_activity(&store, &tempo.id, tempo_run).unwrap();
        let done = events(&store)
            .unwrap()
            .into_iter()
            .find(|event| event.id == tempo.id)
            .unwrap();
        assert_eq!(done.kind, "tempo");
        // A run with no planned session behind it has no kind to inherit.
        insert_run(&store, &workouts[0], "run", "solo");
        let solo = events(&store)
            .unwrap()
            .into_iter()
            .find(|event| event.workout_id.is_none())
            .unwrap();
        assert_eq!(solo.kind, "completed");
    }

    #[test]
    fn score_lines_show_each_value_and_its_change_since_the_previous_run() {
        // (activity_id, load, risk, own impact) — oldest run first.
        let snapshots = vec![
            (1, Some(1.20), 30, 2),
            (2, Some(1.35), 35, 5),
            (3, None, 35, -1),
        ];
        // The first banked run has no earlier ratio to change against, but
        // its own impact is always there.
        assert_eq!(
            score_lines(&snapshots, Some(0), &scored_run(1, 10_000.0, Some(300.0))),
            "\n\nLoad: 120%\nInjury risk: 30% (+2%)\nRan: 10.0 km @ 5:00/km"
        );
        assert_eq!(
            score_lines(&snapshots, Some(1), &scored_run(2, 12_500.0, Some(291.4))),
            "\n\nLoad: 135% (+15%)\nInjury risk: 35% (+5%)\nRan: 12.5 km @ 4:51/km"
        );
        // A run that diluted a heavy week comes out negative, and a run the
        // provider never scored for load simply has no Load line.
        assert_eq!(
            score_lines(&snapshots, Some(2), &scored_run(3, 6_400.0, None)),
            "\n\nInjury risk: 35% (-1%)\nRan: 6.4 km"
        );
    }

    #[test]
    fn ran_line_falls_back_to_duration_over_distance() {
        // A provider that reports no pace still gets a real average: an hour
        // over 10 km is 6:00/km, not a missing number.
        let mut run = scored_run(1, 10_000.0, None);
        run.duration_s = 3600.0;
        assert_eq!(ran_line(&run).as_deref(), Some("Ran: 10.0 km @ 6:00/km"));
        // With neither distance nor pace there is nothing honest to print.
        let mut blank = scored_run(2, 0.0, None);
        blank.duration_s = 3600.0;
        assert_eq!(ran_line(&blank), None);
    }

    #[test]
    fn a_done_run_records_what_was_run_even_without_scores() {
        let (store, _, workouts) = fixture();
        let workout = workouts
            .iter()
            .find(|w| w.kind != WorkoutKind::Rest)
            .unwrap()
            .clone();
        let activity_id = insert_run(&store, &workout, "run", "unscored");
        match_activity(&store, &workout.id, activity_id).unwrap();
        // No snapshot_scores call: the description must still say what ran.
        let done = events(&store)
            .unwrap()
            .into_iter()
            .find(|event| event.id == workout.id)
            .unwrap();
        // insert_run banks an hour over the planned distance, so the average
        // pace is that division and nothing else.
        let km = workout.distance_m.unwrap() / 1000.0;
        let pace = (3600.0 / km).round() as i64;
        assert!(
            done.notes
                .ends_with(&format!("Ran: {km:.1} km @ {}:{:02}/km", pace / 60, pace % 60)),
            "{}",
            done.notes
        );
        assert!(!done.notes.contains("Injury risk"), "{}", done.notes);
    }

    #[test]
    fn scores_snapshot_once_per_run_and_appear_in_its_description() {
        let (store, _, workouts) = fixture();
        let days: Vec<PlanWorkout> = workouts
            .iter()
            .filter(|w| w.kind != WorkoutKind::Rest)
            .take(2)
            .cloned()
            .collect();
        assert_ne!(days[0].date, days[1].date);
        for (workout, record, ratio) in [
            (&days[0], "load-1", 1.2),
            (&days[1], "load-2", 1.35),
        ] {
            store
                .upsert_training_load(
                    ProviderKind::Coros,
                    &TrainingLoad {
                        date: workout.date,
                        short_term: None,
                        long_term: None,
                        ratio: Some(ratio),
                        comment: None,
                    },
                )
                .unwrap();
            insert_run(&store, workout, "run", record);
        }
        let th = Thresholds::default();
        snapshot_scores(&store, &th).unwrap();
        // Idempotent: the snapshot is what the run landed with, so a later
        // refresh must not overwrite it even when the load row has moved on.
        store
            .upsert_training_load(
                ProviderKind::Coros,
                &TrainingLoad {
                    date: days[0].date,
                    short_term: None,
                    long_term: None,
                    ratio: Some(2.0),
                    comment: None,
                },
            )
            .unwrap();
        snapshot_scores(&store, &th).unwrap();
        let stored = store.activity_scores().unwrap();
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[0].1, Some(1.2));

        let entries = events(&store).unwrap();
        // The runs are unmatched, so each day also holds its planned session:
        // only the activity-backed event carries the score lines.
        let run_notes = |day: NaiveDate| {
            entries
                .iter()
                .find(|event| event.date == day && event.activity_id.is_some())
                .unwrap()
                .notes
                .clone()
        };
        // insert_run banks an hour over the planned distance, which is the
        // average the Ran line has to report back.
        let ran_line = |workout: &PlanWorkout| {
            let km = workout.distance_m.unwrap() / 1000.0;
            let pace = (3600.0 / km).round() as i64;
            format!("Ran: {km:.1} km @ {}:{:02}/km", pace / 60, pace % 60)
        };
        let first = run_notes(days[0].date);
        let second = run_notes(days[1].date);
        // The first banked run has no earlier ratio to change against, but
        // its own risk impact is always there. Both values are percentages:
        // the 1.35 ratio reads as the chart's 135 %. The risk value itself
        // depends on the fixture's volumes, so only its line shape is pinned
        // here.
        let lines: Vec<&str> = first.lines().rev().take(3).collect();
        assert_eq!(lines[0], ran_line(&days[0]), "{first}");
        assert!(
            lines[1].starts_with("Injury risk: ") && lines[1].ends_with("%)")
                && lines[1].contains("% ("),
            "{first}"
        );
        assert_eq!(lines[2], "Load: 120%", "{first}");
        let lines: Vec<&str> = second.lines().rev().take(3).collect();
        assert_eq!(lines[0], ran_line(&days[1]), "{second}");
        assert!(lines[1].starts_with("Injury risk: "), "{second}");
        assert_eq!(lines[2], "Load: 135% (+15%)", "{second}");
    }

    #[test]
    fn ambiguous_runs_and_other_sports_are_not_auto_matched() {
        let (store, _, workouts) = fixture();
        let workout = &workouts[0];
        insert_run(&store, workout, "ride", "ride");
        reconcile(&store).unwrap();
        assert!(store.calendar_completions().unwrap().is_empty());
        let activity_id = insert_run(&store, workout, "run", "first");
        insert_run(&store, workout, "run", "second");
        reconcile(&store).unwrap();
        assert!(store.calendar_completions().unwrap().is_empty());
        match_activity(&store, &workout.id, activity_id).unwrap();
        assert_eq!(store.calendar_completions().unwrap().len(), 1);
        assert!(match_activity(&store, &workouts[1].id, activity_id).is_err());
    }

    #[test]
    fn utf8_folding_escaping_and_midnight_end_are_valid() {
        let (store, _, workouts) = fixture();
        let mut entries = events(&store).unwrap();
        entries[0].notes = "é,;\\\r\n".repeat(40);
        let ics = render(&store, &entries).unwrap();
        assert!(ics.split("\r\n").all(|line| line.len() <= 75));
        assert!(ics.replace("\r\n ", "").contains("é\\,\\;\\\\\\n"));
        insert_run(&store, &workouts[0], "run", "late");
        let entries = events(&store).unwrap();
        let activity = entries
            .iter()
            .find(|event| event.activity_id.is_some())
            .unwrap();
        let start =
            chrono::NaiveDateTime::parse_from_str(&activity.start, "%Y%m%dT%H%M%SZ").unwrap();
        let end = chrono::NaiveDateTime::parse_from_str(&activity.end, "%Y%m%dT%H%M%SZ").unwrap();
        assert_eq!(end - start, Duration::hours(1));
    }
}
