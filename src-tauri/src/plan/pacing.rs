//! Did the tempo and interval sessions actually land in their range?
//!
//! Every quality session carries a pace range — the reps of an interval set,
//! the blocks of a cruise tempo. Runna's plans worked the same way, and the
//! reason a range is worth having is not decoration: it is the feedback
//! channel. Run every rep a little down the range and the session was too
//! easy; run every rep a little up it and the session asked for more than was
//! there. Either way the plan is wrong by a small amount, and the fix is to
//! move the range slightly — not to rewrite the runner's threshold.
//!
//! One session off range is not evidence of anything. It is a hill, a group
//! run, too much coffee, or a bad night's sleep, and a coach who re-cut the
//! plan after one weird Tuesday would spend the whole block chasing noise. So
//! this module counts a *run* of sessions leaning the same way
//! ([`Thresholds::pace_streak`], three by default) before it offers anything,
//! and every offer still goes through the runner: nothing here is applied
//! automatically.
//!
//! Grading uses the pace held over the *reps*, never the session average. A
//! 6×400 m workout averages far slower than 4:20/km because half of it is jog
//! recoveries, and calling that "too slow" would be a lie — so the main set's
//! position in the run is taken from the plan's own structure (warm-up, main
//! set, cool-down) and only the work metres inside it are measured.

use chrono::{NaiveDate, Utc};

use crate::error::Result;
use crate::plan::session;
use crate::store::Store;
use crate::types::*;

/// How far one accepted adjustment moves the range, in seconds per km.
///
/// The band itself is 10 s wide (threshold ±5 s), so a 5 s step is half a
/// band: enough that the next session is measurably different, small enough
/// that a wrong read costs little. Deliberately not proportional to the miss —
/// the miss is measured through GPS noise, and a step sized to noise would
/// lurch.
const STEP_SEC_KM: f64 = 5.0;

/// The most the ranges may ever be moved from where the plan started, in
/// seconds per km. Four steps either way: past that the plan is no longer a
/// little adjusted, it is describing a different runner, and the honest answer
/// is a new threshold value — which the watch measures, and this app does not
/// invent.
const MAX_OFFSET_SEC_KM: f64 = 20.0;

/// How many completed quality sessions the report looks back over.
const LOOKBACK: usize = 8;

/// Window length (metres) the track is cut into when measuring the reps. Short
/// enough to separate a rep from the jog after it, long enough that GPS jitter
/// inside one rep does not dominate its pace.
const WINDOW_M: f64 = 200.0;

/// A session is only graded when the recorded run is close enough to the
/// planned one for the main set to be findable inside it. Outside this band
/// the runner ran something else, and guessing where the reps were would be
/// worse than not grading the day.
const DISTANCE_TOLERANCE: (f64, f64) = (0.6, 1.6);

/// Where the reps sat inside a completed session, and what the plan asked of
/// them. Derived from the plan's own session structure, so the measurement and
/// the prescription can never disagree about where the work was.
#[derive(Debug)]
struct MainSet {
    /// Metres run before the main set began (the warm-up).
    start_m: f64,
    /// Metres inside the main set, work plus recoveries.
    region_m: f64,
    /// Metres of actual work inside it — the part to measure.
    work_m: f64,
}

impl MainSet {
    /// Reconstruct the main set from the session's steps. The first step is the
    /// warm-up and the last is the cool-down; everything between is the work.
    /// Returns `None` for a session with no measurable main set (an easy run
    /// mislabelled, or a quality session too short to have been built properly
    /// and rendered as one continuous easy effort).
    fn from_steps(steps: &[session::WorkoutStep]) -> Option<MainSet> {
        if steps.len() < 3 {
            return None;
        }
        let label = |s: &session::WorkoutStep| s.label.to_lowercase();
        if !label(&steps[0]).contains("warm-up") || !label(&steps[steps.len() - 1]).contains("cool")
        {
            return None;
        }
        let main = &steps[1..steps.len() - 1];
        let work_m: f64 = main
            .iter()
            .map(|s| s.distance_m * s.repeats as f64)
            .sum::<f64>();
        let recoveries_m: f64 = main
            .iter()
            .map(|s| s.recovery_m * s.repeats.saturating_sub(1) as f64)
            .sum::<f64>();
        if work_m < 600.0 {
            return None;
        }
        Some(MainSet {
            start_m: steps[0].distance_m,
            region_m: work_m + recoveries_m,
            work_m,
        })
    }
}

/// Grade one completed quality session against the range it prescribed.
///
/// `None` when there is nothing honest to say: the session has no recorded
/// run, its track has not synced yet, the run is a different shape than
/// planned, or there are too few points to cut into windows. Absent sessions
/// are *skipped*, never guessed at — a missing track must not read as a
/// session that went badly.
fn review(
    store: &Store,
    plan: &TrainingPlan,
    workout: &PlanWorkout,
    activity_id: i64,
    tolerance: f64,
) -> Option<SessionReview> {
    let (band_lo, band_hi) = session::quality_band(plan, workout)?;
    let planned_m = workout.distance_m.filter(|d| *d > 0.0)?;
    let activity = store.activity_by_id(activity_id).ok()??;
    if !(DISTANCE_TOLERANCE.0..=DISTANCE_TOLERANCE.1).contains(&(activity.distance_m / planned_m)) {
        return None;
    }
    let details = session::details_for_plan(workout, plan);
    let main = MainSet::from_steps(&details.steps)?;

    // The plan says where the work *should* be; scale it onto the distance the
    // runner actually covered so a slightly short or long run still lines up.
    let scale = activity.distance_m / planned_m;
    let points = store.track_points(activity_id, 4000).ok()?;
    let pace = rep_pace(
        &points,
        main.start_m * scale,
        main.region_m * scale,
        main.work_m,
    )?;

    let execution = if pace < band_lo as f64 - tolerance {
        Execution::Ahead
    } else if pace > band_hi as f64 + tolerance {
        Execution::Behind
    } else {
        Execution::InRange
    };
    Some(SessionReview {
        workout_id: workout.id.clone(),
        date: workout.date,
        kind: workout.kind,
        title: workout.title.clone(),
        band_lo_s_km: band_lo,
        band_hi_s_km: band_hi,
        actual_s_km: Some(pace),
        measured_m: main.work_m,
        execution,
    })
}

/// Pace (s/km) held over the work metres of a main set, from GPS points.
///
/// The track inside the main-set region is cut into [`WINDOW_M`]-metre windows;
/// the fastest `work_m` metres of them are the reps, and their distance-weighted
/// mean is the answer. Taking the fastest work-length of windows is what makes
/// this independent of the band being graded against — it locates the effort by
/// its own shape rather than by looking for paces that match the target, which
/// would find them whether or not the runner ran them.
fn rep_pace(points: &[TrackPoint], start_m: f64, region_m: f64, work_m: f64) -> Option<f64> {
    if points.len() < 4 || region_m <= 0.0 {
        return None;
    }
    // Cumulative distance along the track, from consecutive point pairs.
    let mut cum: Vec<f64> = Vec::with_capacity(points.len());
    let mut total = 0.0f64;
    cum.push(0.0);
    for pair in points.windows(2) {
        total += metres_between(&pair[0], &pair[1]);
        cum.push(total);
    }
    if total <= 0.0 {
        return None;
    }
    let lo = start_m.min(total);
    let hi = (start_m + region_m).min(total);
    if hi - lo < WINDOW_M * 2.0 {
        return None;
    }
    // Pace of each window inside the region.
    let mut windows: Vec<(f64, f64)> = Vec::new(); // (pace s/km, metres)
    let mut cursor = lo;
    while cursor < hi {
        let end = (cursor + WINDOW_M).min(hi);
        let (secs, metres) = span_stats(points, &cum, cursor, end)?;
        if metres >= WINDOW_M * 0.5 && secs > 0.0 {
            windows.push((secs / metres * 1000.0, metres));
        }
        cursor = end;
    }
    if windows.is_empty() {
        return None;
    }
    windows.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut budget = work_m.min(region_m);
    let mut secs = 0.0;
    let mut measured = 0.0;
    for (pace, metres) in windows {
        if budget <= 0.0 {
            break;
        }
        let take = metres.min(budget);
        secs += take / 1000.0 * pace;
        measured += take;
        budget -= take;
    }
    if measured < work_m.min(region_m) * 0.5 || measured <= 0.0 {
        return None;
    }
    Some(secs / measured * 1000.0)
}

/// Elapsed seconds and metres covered between two cumulative distances,
/// interpolating the endpoints and summing whole segments in between.
fn span_stats(points: &[TrackPoint], cum: &[f64], from: f64, to: f64) -> Option<(f64, f64)> {
    let idx = cum.partition_point(|&c| c < from).saturating_sub(1);
    let mut secs = 0.0;
    let mut metres = 0.0;
    let mut cursor = from;
    let mut i = idx;
    while cursor < to && i + 1 < points.len() {
        let seg_lo = cum[i];
        let seg_hi = cum[i + 1];
        if seg_hi <= seg_lo {
            i += 1;
            continue;
        }
        let a = cursor.max(seg_lo);
        let b = to.min(seg_hi);
        if b > a {
            let dt = (points[i + 1].ts - points[i].ts).max(0) as f64;
            let frac = (b - a) / (seg_hi - seg_lo);
            let part = (b - a).min(seg_hi - seg_lo);
            secs += dt * frac;
            metres += part;
        }
        cursor = b;
        i += 1;
    }
    if metres > 0.0 {
        Some((secs, metres))
    } else {
        None
    }
}

/// Flat-earth distance between two nearby GPS samples — good enough over a few
/// seconds of a run, and the app has no need for a geodesic library here.
fn metres_between(a: &TrackPoint, b: &TrackPoint) -> f64 {
    const LAT_M: f64 = 111_320.0;
    let lon_m = LAT_M * a.lat.to_radians().cos();
    let dy = (b.lat - a.lat) * LAT_M;
    let dx = (b.lon - a.lon) * lon_m;
    (dx * dx + dy * dy).sqrt()
}

/// Is this workout a session whose pace range means something?
fn is_quality(kind: WorkoutKind) -> bool {
    matches!(kind, WorkoutKind::Intervals | WorkoutKind::Tempo)
}

/// Every completed quality session, oldest first, that can be graded.
///
/// Sessions dated before the plan's current offset took effect are left out:
/// they were run under the old ranges and were already read. Counting them
/// again after an adjustment is accepted would re-trigger the offer the runner
/// has just answered.
pub fn reviews(store: &Store, plan: &TrainingPlan, th: &Thresholds) -> Result<Vec<SessionReview>> {
    let completions = store.calendar_completions()?;
    let workouts = store.plan_workouts(&plan.id)?;
    let today = Utc::now().date_naive();
    let mut out = Vec::new();
    for workout in workouts
        .iter()
        .filter(|w| is_quality(w.kind) && w.done && w.date <= today)
        .filter(|w| match plan.quality_offset_from {
            Some(from) => w.date >= from,
            None => true,
        })
    {
        let Some(entry) = completions.iter().find(|c| c.0 == workout.id) else {
            continue;
        };
        if let Some(review) = review(store, plan, workout, entry.1, th.pace_tolerance_sec_km) {
            out.push(review);
        }
    }
    out.sort_by_key(|r| r.date);
    if out.len() > LOOKBACK {
        out.drain(0..out.len() - LOOKBACK);
    }
    Ok(out)
}

/// The trailing run of sessions that all leaned the same way, and which way.
///
/// Only the sessions at the *end* of the list count. A runner who went too
/// fast three times, then hit two in range, has already answered the question:
/// the ranges are right now, and offering to move them again would be arguing
/// with history.
pub fn streak(reviews: &[SessionReview]) -> (usize, Option<Execution>) {
    let Some(last) = reviews.last().filter(|r| r.execution.off_range()) else {
        return (0, None);
    };
    let direction = last.execution;
    let count = reviews
        .iter()
        .rev()
        .take_while(|r| r.execution == direction)
        .count();
    (count, Some(direction))
}

/// The size of the next adjustment, in the direction the evidence points.
///
/// Returns `None` when the plan is already at the limit of how far its ranges
/// may be moved — at that point the honest message is "your threshold pace is
/// wrong", not a fifth nudge.
fn next_offset(plan: &TrainingPlan, direction: Execution) -> Option<f64> {
    let target = plan.quality_offset_sec_km
        + if direction == Execution::Ahead {
            -STEP_SEC_KM
        } else {
            STEP_SEC_KM
        };
    if target.abs() > MAX_OFFSET_SEC_KM + 1e-9 {
        return None;
    }
    Some(target)
}

/// The first day an adjustment can apply to: the day after the most recent
/// graded session, so a session already run keeps the range it was run against
/// and only what comes next moves.
fn effective_from(reviews: &[SessionReview]) -> NaiveDate {
    let last = reviews.iter().map(|r| r.date).max();
    let today = Utc::now().date_naive();
    let candidate = last.map(|d| d + chrono::Duration::days(1)).unwrap_or(today);
    candidate.max(today)
}

/// Build the offer for a run of out-of-range sessions, if one is warranted.
///
/// `None` when nothing is pending-worthy: the streak is too short, the sessions
/// lean different ways, an identical offer is already pending or was recently
/// dismissed, or the plan has nowhere left to move.
pub fn build(store: &Store, plan: &TrainingPlan, th: &Thresholds) -> Result<Option<PlanProposal>> {
    if matches!(plan.status, PlanStatus::Aborted | PlanStatus::Completed) {
        return Ok(None);
    }
    let reviews = reviews(store, plan, th)?;
    let (count, direction) = streak(&reviews);
    let Some(direction) = direction.filter(|_| count >= th.pace_streak as usize) else {
        return Ok(None);
    };
    let Some(offset) = next_offset(plan, direction) else {
        return Ok(None);
    };

    // One unanswered pacing offer at a time, and a dismissed one is not asked
    // again until the plan has actually moved or a week has passed.
    let signature = format!("pacing:{}:{}", direction_key(direction), plan.id);
    if store.has_pending_pacing(&plan.id)? {
        return Ok(None);
    }
    if let Some(dismissed) = store.last_dismissed(&signature)? {
        let newest = reviews.last().map(|r| r.date);
        let stale = Utc::now() - dismissed > chrono::Duration::days(7);
        // A dismissal means "leave my plan alone". It stops holding once the
        // runner has since run another quality session under the same ranges,
        // because that is new evidence rather than the same ask repeated.
        let ran_since = newest.map(|d| d >= dismissed.date_naive()).unwrap_or(false);
        if !stale && !ran_since {
            return Ok(None);
        }
    }

    let from = effective_from(&reviews);
    let band_shift = if direction == Execution::Ahead {
        "sharpen".to_string()
    } else {
        "soften".to_string()
    };
    let recent: Vec<&SessionReview> = reviews.iter().rev().take(count).collect();
    let summary = format!(
        "{} of the last {} {} sessions landed {} the range — {} every quality pace by {:.0} s/km from {}.",
        count,
        recent.len().max(count),
        session_kind_label(&recent),
        if direction == Execution::Ahead { "faster than" } else { "slower than" },
        band_shift,
        STEP_SEC_KM,
        from.format("%b %e"),
    );
    let mut reason = format!(
        "{} quality sessions in a row outside the range, all of them {}.",
        count,
        direction.label()
    );
    for r in recent.iter().rev() {
        if let Some(actual) = r.actual_s_km {
            reason.push_str(&format!(
                " {} — held {} min/km against {}-{}.",
                r.date.format("%b %e"),
                fmt_pace(actual.round() as i64),
                fmt_pace(r.band_lo_s_km),
                fmt_pace(r.band_hi_s_km)
            ));
        }
    }
    reason.push_str(&format!(
        " Easy days, long runs and race pace are untouched — only the tempo and interval ranges move."
    ));
    if plan.quality_offset_sec_km != 0.0 {
        reason.push_str(&format!(
            " This is on top of the {:.0} s/km already applied.",
            plan.quality_offset_sec_km
        ));
    }

    Ok(Some(PlanProposal {
        id: uuid::Uuid::new_v4().to_string(),
        body_part: String::new(),
        source: ProposalSource::Pacing,
        signature,
        summary,
        reason,
        edits: Vec::new(),
        plan_quality_offset_sec_km: Some(offset),
        plan_quality_offset_from: Some(from),
        status: ProposalStatus::Pending,
        created_at: Utc::now(),
        acted_at: None,
    }))
}

fn direction_key(direction: Execution) -> &'static str {
    match direction {
        Execution::Ahead => "ahead",
        Execution::Behind => "behind",
        Execution::InRange => "in",
    }
}

/// "tempo" / "interval" / "tempo and interval" — what the streak was made of.
fn session_kind_label(recent: &[&SessionReview]) -> &'static str {
    let any_intervals = recent.iter().any(|r| r.kind == WorkoutKind::Intervals);
    let any_tempo = recent.iter().any(|r| r.kind == WorkoutKind::Tempo);
    match (any_intervals, any_tempo) {
        (true, true) => "quality",
        (true, false) => "interval",
        (false, true) => "tempo",
        (false, false) => "quality",
    }
}

fn fmt_pace(seconds: i64) -> String {
    let sign = if seconds < 0 { "-" } else { "" };
    let s = seconds.abs();
    format!("{sign}{}:{:02}", s / 60, s % 60)
}

/// Grade a single session, for the surface where the runner opens it.
///
/// The plan page shows every session at once; the activity sheet shows one,
/// and the runner reading it wants the verdict on *that* run rather than a
/// table they have to find their own day in.
///
/// Same rule as [`reviews`], and for the same reason: a session run before an
/// adjustment took effect is not graded. Its band has moved since, so a verdict
/// now would tell the runner they missed a target that did not exist on the day
/// they ran it.
pub fn review_for_workout(
    store: &Store,
    plan: &TrainingPlan,
    workout_id: &str,
    th: &Thresholds,
) -> Result<Option<SessionReview>> {
    let workout = store
        .plan_workouts(&plan.id)?
        .into_iter()
        .find(|w| w.id == workout_id);
    let Some(workout) = workout.filter(|w| is_quality(w.kind) && w.done) else {
        return Ok(None);
    };
    if plan.quality_offset_from.is_some_and(|from| workout.date < from) {
        return Ok(None);
    }
    let entry = store
        .calendar_completions()?
        .into_iter()
        .find(|c| c.0 == workout.id);
    let Some(entry) = entry else {
        return Ok(None);
    };
    Ok(review(
        store,
        plan,
        &workout,
        entry.1,
        th.pace_tolerance_sec_km,
    ))
}

/// Everything the Plan page needs to show where recent sessions landed, whether
/// or not an offer exists yet.
pub fn report(store: &Store, plan: &TrainingPlan, th: &Thresholds) -> Result<PacingReport> {
    let reviews = reviews(store, plan, th)?;
    let (streak, direction) = streak(&reviews);
    Ok(PacingReport {
        needed: th.pace_streak,
        triggered: direction.is_some() && streak >= th.pace_streak as usize,
        streak,
        direction,
        reviews,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::engine;
    use crate::store::Store;

    fn store() -> Store {
        let path = std::env::temp_dir().join(format!("rk-paced-{}.sqlite", uuid::Uuid::new_v4()));
        Store::open(&path).expect("open store")
    }

    /// A plan whose quality sessions are dated on `offsets` days before today,
    /// generated for real so the session structure is the engine's own.
    fn plan_with_quality(s: &Store, offsets: &[i64]) -> (TrainingPlan, Vec<PlanWorkout>) {
        let today = Utc::now().date_naive();
        let req = PlanRequest {
            distance: RaceDistance::K10,
            kind: PlanKind::Race,
            target_date: today + chrono::Duration::days(70),
            threshold_sec_km: 285.0,
            weekly_frequency: 4,
            longrun_day: 7,
            increase: IncreaseMethod::Moderate,
            current_weekly_km: Some(40.0),
        };
        let (plan, workouts) = engine::generate(&req, 40.0).unwrap();
        s.insert_plan(&plan).unwrap();
        s.insert_workouts(&workouts).unwrap();
        let quality: Vec<PlanWorkout> = workouts
            .iter()
            .filter(|w| is_quality(w.kind))
            .cloned()
            .collect();
        assert!(
            quality.len() >= offsets.len(),
            "the plan needs at least {} quality sessions",
            offsets.len()
        );
        // Re-date the quality sessions into the past so they can be completed.
        let mut taken: Vec<PlanWorkout> = Vec::new();
        for (i, off) in offsets.iter().enumerate() {
            let mut w = quality[i].clone();
            w.date = today - chrono::Duration::days(*off);
            s.update_workout(&w).unwrap();
            taken.push(w);
        }
        (plan, taken)
    }

    /// A synthetic track: `total_m` metres at `pace_s_km`, with the metres in
    /// `[start, start+length)` run at `fast_s_km` instead.
    fn synthetic_track(
        s: &Store,
        record_id: &str,
        day: NaiveDate,
        total_m: f64,
        pace_s_km: f64,
        fast: Option<(f64, f64, f64)>,
    ) -> i64 {
        let start_ts = day.and_hms_opt(8, 0, 0).unwrap().and_utc().timestamp();
        let step_m = 20.0;
        let mut points = Vec::new();
        let mut dist = 0.0;
        // Seconds accumulate as a float and only the absolute timestamp is
        // rounded: rounding each step's duration instead would bias every
        // synthetic track fast (20 m at 6:45/km is 8.1 s, and eight 8 s steps
        // silently become an 8.0 s/km track).
        let mut ts_f = start_ts as f64;
        while dist < total_m {
            let next = (dist + step_m).min(total_m);
            let inside = fast.map_or(false, |(f_start, f_len, _)| {
                dist >= f_start && dist < f_start + f_len
            });
            let pace = if inside { fast.unwrap().2 } else { pace_s_km };
            ts_f += step_m / 1000.0 * pace;
            // Walk north at ~0.00018° per 20 m so cumulative distance is real.
            points.push(crate::fit::TrackPoint {
                ts: ts_f.round() as i64,
                lat: 48.13 + dist / 111_320.0,
                lon: 11.58,
                hr: None,
            });
            dist = next;
        }
        let rec = crate::fit::DecodedActivity {
            record_id: record_id.into(),
            sport: "run".into(),
            provider_sport_code: Some(100),
            start_ts,
            duration_s: ts_f - start_ts as f64,
            distance_m: total_m,
            avg_hr: None,
            max_hr: None,
            elevation_gain_m: None,
            points,
        };
        s.upsert_activity(ProviderKind::Coros, &rec)
            .expect("insert")
            .expect("new activity")
    }

    /// Complete `workout` with a run whose reps held `delta` seconds per km
    /// off **its own** band: negative = that far faster than the fast end,
    /// positive = that far slower than the slow end, 0 = mid-range. Grading
    /// compares each session to the range it was actually given, and bands
    /// ramp week to week, so an absolute pace would grade one session's
    /// target against another's range.
    fn complete(s: &Store, plan: &TrainingPlan, workout: &PlanWorkout, delta: f64) {
        let (lo, hi) = session::quality_band(plan, workout).expect("quality band");
        let rep_pace_s_km = if delta < 0.0 {
            lo as f64 + delta
        } else if delta > 0.0 {
            hi as f64 + delta
        } else {
            (lo + hi) as f64 / 2.0
        };
        let details = session::details_for_plan(workout, plan);
        let main = MainSet::from_steps(&details.steps).expect("main set");
        let total = workout.distance_m.unwrap();
        let id = synthetic_track(
            s,
            &format!("{}-run", workout.id),
            workout.date,
            total,
            400.0, // everything outside the reps jogs along at 6:40/km
            Some((main.start_m, main.work_m, rep_pace_s_km)),
        );
        s.complete_calendar_workout(
            &workout.id,
            id,
            &crate::calendar::workout_notes(workout, plan),
        )
        .unwrap();
    }

    #[test]
    fn a_session_inside_its_range_is_not_evidence() {
        let s = store();
        let (plan, quality) = plan_with_quality(&s, &[21]);
        let w = &quality[0];
        complete(&s, &plan, w, 0.0);
        let reviews = reviews(&s, &plan, &Thresholds::default()).unwrap();
        assert_eq!(reviews.len(), 1, "the session should be gradeable");
        assert_eq!(reviews[0].execution, Execution::InRange);
        assert_eq!(streak(&reviews), (0, None));
    }

    /// The whole point of measuring the reps: a session run perfectly, with
    /// jog recoveries dragging the overall average far slower than the range,
    /// must still read as in range. Grading the session average here would
    /// report `Behind` for a flawless workout.
    #[test]
    fn jog_recoveries_do_not_make_a_good_session_look_slow() {
        let s = store();
        let (plan, quality) = plan_with_quality(&s, &[21]);
        let w = &quality[0];
        let details = session::details_for_plan(w, &plan);
        let main = MainSet::from_steps(&details.steps).unwrap();
        let total = w.distance_m.unwrap();
        let (lo, hi) = session::quality_band(&plan, w).unwrap();
        let mid = (lo + hi) as f64 / 2.0;
        let id = synthetic_track(
            &s,
            "recoveries",
            w.date,
            total,
            430.0, // 7:10/km jogs
            Some((main.start_m, main.work_m, mid)),
        );
        let activity = s.activity_by_id(id).unwrap().unwrap();
        let overall = activity.duration_s / (activity.distance_m / 1000.0);
        assert!(
            overall > hi as f64,
            "the session average ({overall:.0}) should be slower than the range, \
             which is exactly why it must not be graded"
        );
        s.complete_calendar_workout(&w.id, id, "notes").unwrap();
        let reviews = reviews(&s, &plan, &Thresholds::default()).unwrap();
        assert_eq!(reviews[0].execution, Execution::InRange);
    }

    #[test]
    fn two_off_sessions_are_fine_and_three_offer_a_change() {
        let s = store();
        let (plan, quality) = plan_with_quality(&s, &[21, 14, 7]);
        let th = Thresholds::default();
        // Every rep block a good 15 s/km down its own range = clearly too fast.
        // Complete two first: a bad day is not a trend, and neither is two.
        for w in quality.iter().take(2) {
            complete(&s, &plan, w, -15.0);
        }
        let reviews = reviews(&s, &plan, &th).unwrap();
        assert_eq!(reviews.len(), 2);
        assert!(reviews.iter().all(|r| r.execution == Execution::Ahead));
        assert_eq!(streak(&reviews), (2, Some(Execution::Ahead)));
        assert!(build(&s, &plan, &th).unwrap().is_none(), "not before three");

        // The default streak is three, so the third is what triggers it.
        complete(&s, &plan, &quality[2], -15.0);
        let offer = build(&s, &plan, &th)
            .unwrap()
            .expect("three sessions down the range should offer a change");
        assert_eq!(offer.source, ProposalSource::Pacing);
        assert!(
            offer.edits.is_empty(),
            "a pacing offer moves ranges, not days"
        );
        // Running down the range means the plan was too easy → sharpen it.
        assert_eq!(offer.plan_quality_offset_sec_km, Some(-5.0));
        assert!(offer.summary.contains("sharpen"));
    }

    #[test]
    fn running_slow_offers_the_opposite_change() {
        let s = store();
        let (plan, quality) = plan_with_quality(&s, &[21, 14, 7]);
        for w in &quality {
            complete(&s, &plan, w, 15.0);
        }
        let th = Thresholds::default();
        let offer = build(&s, &plan, &th)
            .unwrap()
            .expect("three sessions above the range should offer a change");
        assert_eq!(offer.plan_quality_offset_sec_km, Some(5.0));
        assert!(offer.summary.contains("soften"));
    }

    /// A run that leans one way and then another is not a trend in either.
    #[test]
    fn mixed_directions_never_trigger() {
        let s = store();
        let (plan, quality) = plan_with_quality(&s, &[21, 14, 7]);
        complete(&s, &plan, &quality[0], -15.0);
        complete(&s, &plan, &quality[1], -15.0);
        complete(&s, &plan, &quality[2], 15.0);
        let th = Thresholds::default();
        let reviews = reviews(&s, &plan, &th).unwrap();
        assert_eq!(streak(&reviews), (1, Some(Execution::Behind)));
        assert!(build(&s, &plan, &th).unwrap().is_none());
    }

    /// The tolerance is the difference between "missed by a second of GPS
    /// noise" and "missed by something you could feel".
    #[test]
    fn a_narrow_miss_is_not_out_of_range() {
        let s = store();
        let (plan, quality) = plan_with_quality(&s, &[21, 14, 7]);
        for w in &quality {
            // 2 s/km faster than the fast end — inside the 4 s tolerance.
            complete(&s, &plan, w, -2.0);
        }
        let th = Thresholds::default();
        let reviews = reviews(&s, &plan, &th).unwrap();
        assert!(
            reviews.iter().all(|r| r.execution == Execution::InRange),
            "a 2 s/km miss must not count"
        );
        assert!(build(&s, &plan, &th).unwrap().is_none());
    }

    /// Once an adjustment is accepted, the sessions that justified it are
    /// history: they must not keep re-triggering the offer the runner answered.
    #[test]
    fn accepting_an_offer_resets_the_streak() {
        let s = store();
        let (plan, quality) = plan_with_quality(&s, &[21, 14, 7]);
        for w in &quality {
            complete(&s, &plan, w, -15.0);
        }
        let th = Thresholds::default();
        assert!(build(&s, &plan, &th).unwrap().is_some());
        let adjusted = TrainingPlan {
            quality_offset_sec_km: -5.0,
            quality_offset_from: Some(Utc::now().date_naive()),
            ..plan.clone()
        };
        assert!(
            reviews(&s, &adjusted, &th).unwrap().is_empty(),
            "sessions run under the old ranges are already read"
        );
        assert!(build(&s, &adjusted, &th).unwrap().is_none());
    }

    /// Past the limit, another nudge would be dishonest — the threshold itself
    /// is the number that is wrong, and that is the watch's to measure.
    #[test]
    fn the_offset_is_capped() {
        let s = store();
        let (plan, _) = plan_with_quality(&s, &[21]);
        let maxed = TrainingPlan {
            quality_offset_sec_km: -20.0,
            ..plan.clone()
        };
        assert_eq!(next_offset(&maxed, Execution::Ahead), None);
        let one_short = TrainingPlan {
            quality_offset_sec_km: -15.0,
            ..plan
        };
        assert_eq!(next_offset(&one_short, Execution::Ahead), Some(-20.0));
    }

    /// A session with no recorded run, or one whose track has not synced, is
    /// skipped rather than guessed at.
    #[test]
    fn a_session_without_a_track_is_skipped_not_judged() {
        let s = store();
        let (plan, quality) = plan_with_quality(&s, &[21, 14, 7]);
        // Mark them done with no activity at all.
        for w in &quality {
            s.complete_calendar_workout(&w.id, -1, "notes").ok();
        }
        let th = Thresholds::default();
        assert!(reviews(&s, &plan, &th).unwrap().is_empty());
        assert!(build(&s, &plan, &th).unwrap().is_none());
    }

    #[test]
    fn an_offset_moves_the_prescribed_range() {
        let s = store();
        let (plan, quality) = plan_with_quality(&s, &[21]);
        let w = &quality[0];
        let base = session::quality_band(&plan, w).unwrap();
        let shifted = TrainingPlan {
            quality_offset_sec_km: -5.0,
            quality_offset_from: Some(w.date),
            ..plan.clone()
        };
        let moved = session::quality_band(&shifted, w).unwrap();
        assert_eq!((moved.0, moved.1), (base.0 - 5, base.1 - 5));
        // A session dated before the adjustment keeps the range it was run to.
        let later = TrainingPlan {
            quality_offset_from: Some(w.date + chrono::Duration::days(1)),
            ..shifted
        };
        assert_eq!(session::quality_band(&later, w).unwrap(), base);
    }

    /// Easy days and long runs must never move — a tempo session being too
    /// easy says nothing about what an easy run should feel like.
    #[test]
    fn only_quality_sessions_are_graded() {
        let s = store();
        let today = Utc::now().date_naive();
        let (plan, _) = plan_with_quality(&s, &[21]);
        // `plan_with_quality` returns only the sessions it re-dated, so the
        // easy day has to come from the store.
        let all = s.plan_workouts(&plan.id).unwrap();
        let mut easy = all
            .into_iter()
            .find(|w| w.kind == WorkoutKind::Easy)
            .expect("a four-run week has an easy day");
        easy.date = today - chrono::Duration::days(14);
        s.update_workout(&easy).unwrap();
        let id = synthetic_track(&s, "easy-run", easy.date, 10_000.0, 200.0, None);
        s.complete_calendar_workout(&easy.id, id, "notes").unwrap();
        let graded = reviews(&s, &plan, &Thresholds::default()).unwrap();
        assert!(
            graded.iter().all(|r| is_quality(r.kind)),
            "an easy run flown far under its range is not a pacing signal"
        );
        assert!(!graded.iter().any(|r| r.workout_id == easy.id));
    }

    #[test]
    fn the_report_shows_the_recent_sessions() {
        let s = store();
        let (plan, quality) = plan_with_quality(&s, &[14, 7]);
        for w in &quality {
            complete(&s, &plan, w, -15.0);
        }
        let report = report(&s, &plan, &Thresholds::default()).unwrap();
        assert_eq!(report.reviews.len(), 2);
        assert_eq!(report.streak, 2);
        assert_eq!(report.direction, Some(Execution::Ahead));
        assert_eq!(report.needed, 3);
        assert!(!report.triggered, "two is not yet three");
    }

    /// The activity sheet asks about one session, so the single-session lookup
    /// has to agree with the report — and stay quiet for the sessions the
    /// report is quiet about, or the sheet would invent a verdict.
    #[test]
    fn a_single_session_can_be_graded_on_its_own() {
        let s = store();
        let (plan, quality) = plan_with_quality(&s, &[14, 7]);
        for w in &quality {
            complete(&s, &plan, w, -15.0);
        }
        let th = Thresholds::default();
        let graded = reviews(&s, &plan, &th).unwrap();
        for review in &graded {
            let alone = review_for_workout(&s, &plan, &review.workout_id, &th)
                .unwrap()
                .expect("the report graded it, so the sheet can too");
            assert_eq!((alone.band_lo_s_km, alone.band_hi_s_km), (review.band_lo_s_km, review.band_hi_s_km));
            assert_eq!(alone.execution, review.execution);
        }
        // An easy day has no verdict to show, and an unknown id is not an error.
        let all = s.plan_workouts(&plan.id).unwrap();
        let easy = all.into_iter().find(|w| w.kind == WorkoutKind::Easy).unwrap();
        assert!(review_for_workout(&s, &plan, &easy.id, &th).unwrap().is_none());
        assert!(review_for_workout(&s, &plan, "no-such-workout", &th).unwrap().is_none());
    }

    /// After an adjustment is accepted, sessions run under the old ranges stop
    /// being evidence. The sheet must apply the same cut: telling a runner they
    /// missed a target that did not exist on the day they ran it is worse than
    /// saying nothing.
    #[test]
    fn a_single_session_before_the_offset_is_not_graded() {
        let s = store();
        let (plan, quality) = plan_with_quality(&s, &[14]);
        let w = &quality[0];
        complete(&s, &plan, w, -15.0);
        let th = Thresholds::default();
        assert!(review_for_workout(&s, &plan, &w.id, &th).unwrap().is_some());
        let adjusted = TrainingPlan {
            quality_offset_sec_km: -5.0,
            quality_offset_from: Some(w.date + chrono::Duration::days(1)),
            ..plan.clone()
        };
        assert!(review_for_workout(&s, &adjusted, &w.id, &th).unwrap().is_none());
    }
}
