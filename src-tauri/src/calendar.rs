use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::PathBuf;

use chrono::{Duration, Local, NaiveDate};
use serde::Serialize;

use crate::error::{AppError, Result};
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
}

/// Emoji prefix for calendar titles, so the subscribed calendar reads
/// at a glance even in a cramped month cell. Every session — planned or
/// completed — is a run, so it always wears the same runner.
const RUN_EMOJI: &str = "🏃";

pub struct CalendarExport {
    pub path: PathBuf,
}

pub fn workout_notes(workout: &PlanWorkout, plan: &TrainingPlan) -> String {
    let details = session::details_for_plan(workout, plan);
    let mut notes = format!("{}\n{}\n{}", workout.title, workout.date, details.purpose);
    if let Some(distance) = workout.distance_m {
        let _ = write!(notes, "\nTotal: {:.2} km", distance / 1000.0);
    }
    if let Some(minutes) = details.estimated_min {
        let _ = write!(notes, "\nEstimated duration: {:.0} min", minutes);
    }
    for (index, step) in details.steps.iter().enumerate() {
        let _ = write!(notes, "\n\n{}. {}: ", index + 1, step.label);
        if step.repeats > 1 {
            let _ = write!(notes, "{} x ", step.repeats);
        }
        let _ = write!(
            notes,
            "{:.0} m{}",
            step.distance_m,
            pace_band(step.pace_lo_s_km, step.pace_hi_s_km)
        );
        if step.recovery_m > 0.0 {
            let _ = write!(notes, "\nBetween repetitions: {:.0} m easy jog{} ({} recoveries, none after the last rep)", step.recovery_m, pace_band(step.recovery_pace_lo_s_km, step.recovery_pace_hi_s_km), step.repeats - 1);
        }
    }
    notes
}

fn pace_band(low: Option<i64>, high: Option<i64>) -> String {
    match (low, high) {
        (Some(low), Some(high)) => format!(" at {}-{} min/km", fmt_pace(low), fmt_pace(high)),
        _ => String::new(),
    }
}

fn is_run(activity: &Activity) -> bool {
    let sport = activity.sport.to_lowercase();
    sport.contains("run") || matches!(sport.as_str(), "jog" | "jogging")
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

pub fn events(store: &Store) -> Result<Vec<CalendarEvent>> {
    let completions = store.calendar_completions()?;
    let mut output = Vec::new();
    for activity in store.list_activities(-1)?.into_iter().filter(is_run) {
        let completion = completions.iter().find(|entry| entry.1 == activity.id);
        let start_local = activity.start_time.with_timezone(&Local);
        let end_local =
            start_local + Duration::seconds(activity.duration_s.ceil().max(1.0) as i64);
        // Matched runs keep the name they had in the plan (the first line
        // of the stored plan notes); standalone runs get a plain title.
        let base_title = completion
            .map(|entry| {
                entry
                    .2
                    .lines()
                    .next()
                    .unwrap_or("Run")
                    .trim()
                    .to_string()
            })
            .unwrap_or_else(|| format!("Run {:.2} km", activity.distance_m / 1000.0));
        let mut notes = format!(
            "{base_title}\nDistance: {:.2} km\nDuration: {:.1} min\nStarted: {}\nEnded: {}",
            activity.distance_m / 1000.0,
            activity.duration_s / 60.0,
            start_local.format("%H:%M"),
            end_local.format("%H:%M")
        );
        if let Some(pace) = activity.avg_pace_s_km {
            let _ = write!(
                notes,
                "\nAverage pace: {} min/km",
                fmt_pace(pace.round() as i64)
            );
        }
        if let Some(hr) = activity.avg_hr {
            let _ = write!(notes, "\nAverage heart rate: {hr} bpm");
        }
        if let Some((_, _, original)) = completion {
            let _ = write!(notes, "\n\nOriginal plan\n{original}");
        }
        output.push(CalendarEvent {
            id: completion
                .map(|entry| entry.0.clone())
                .unwrap_or_else(|| format!("activity-{}", activity.id)),
            workout_id: completion.map(|entry| entry.0.clone()),
            activity_id: Some(activity.id),
            date: activity.start_time.with_timezone(&Local).date_naive(),
            kind: "completed".into(),
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
        });
    }
    if let Some((plan, workouts)) = store.current_plan()? {
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
                kind: if workout.kind == WorkoutKind::LongRun {
                    "longRun".into()
                } else {
                    format!("{:?}", workout.kind).to_lowercase()
                },
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
    use chrono::{TimeZone, Utc};

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
        assert!(done.notes.contains("Original plan"));
        assert!(done.notes.contains(&workout.title));
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
        assert!(retained[0].notes.contains("Original plan"));
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
