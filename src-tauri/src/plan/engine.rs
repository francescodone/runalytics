//! Rule-based race training plan generator.
//!
//! Model: weekly volume grows geometrically from the runner's recent baseline
//! to a peak (capped by race distance), with a 3-week build / 1-downweek
//! cycle, a fixed long-run day, one quality session (intervals or tempo),
//! easy runs filling the remaining frequency, and a 2-week taper.
//!
//! The marathon constants are calibrated against two completed builds aimed at
//! 2:50 (Runna, ~80 km peak, one track session a week): their biggest long
//! runs were 32.4 km and 35.0 km, and the long run held ~42-46 % of weekly
//! volume through the peak weeks. See `peak_long_run_km` and `long_run_share`.

use chrono::{Datelike, NaiveDate, Utc};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::types::*;

/// Longest long-run (km) worth doing before the taper, per race.
///
/// The marathon cap is 35 km, not the round 32 the model started from: both
/// reference builds ran a 32-35 km long run in the final fortnight (the 35 km
/// one at 4:22/km was the strongest session of either), and capping at 32 kept
/// the generated plan a session short of the one that made those races.
fn peak_long_run_km(d: RaceDistance) -> f64 {
    match d {
        RaceDistance::K5 => 14.0,
        RaceDistance::K10 => 18.0,
        RaceDistance::K21 => 26.0,
        RaceDistance::K42 => 35.0,
    }
}

/// Long run as a share of the week's target volume in a build week.
///
/// Calibrated against the two 2:50-goal marathon builds: the long run held
/// ~42-46 % of weekly volume (median ~43 %) right through the peak weeks, well
/// above the flat 38 % a shorter race needs — at an 80 km peak, 38 % gives a
/// 30 km long run where the builds actually ran 33-35 km. Shorter races keep
/// 38 %: their long run is a smaller slice of a smaller week.
fn long_run_share(d: RaceDistance) -> f64 {
    match d {
        RaceDistance::K42 => 0.44,
        RaceDistance::K21 => 0.40,
        _ => 0.38,
    }
}

/// Peak weekly volume (km) target for a well-prepared runner.
///
/// Scaled by weekly frequency: the base figures assume a 4-session week, and
/// a runner who turns up 3×/week cannot hold the same volume as one who runs
/// 7× — while the extra days give the higher-frequency runner room to build
/// more. ±7.5 % per session away from 4, so 3× targets 92.5 % of base and
/// 7× targets 122.5 %.
fn peak_weekly_km(d: RaceDistance, increase: IncreaseMethod, frequency: u32) -> f64 {
    let base = match d {
        RaceDistance::K5 => 32.0,
        RaceDistance::K10 => 42.0,
        RaceDistance::K21 => 58.0,
        RaceDistance::K42 => 80.0,
    };
    let freq_factor = (1.0 + (frequency.saturating_sub(4) as f64) * 0.075).clamp(0.85, 1.3);
    let scaled = base * freq_factor;
    match increase {
        IncreaseMethod::Conservative => scaled * 0.85,
        IncreaseMethod::Moderate => scaled,
        IncreaseMethod::Ambitious => scaled * 1.15,
    }
}

/// Shortest run a plan prescribes (km); shorter easy runs fold into the others.
const MIN_RUN_KM: f64 = 6.0;

/// Planned distances are whole km, rounded up (the epsilon keeps float noise from adding a km).
fn ceil_km(km: f64) -> f64 {
    (km - 1e-6).ceil()
}

/// Weeks spent tapering before race day.
///
/// Only a race build tapers. A taper exists to arrive at a start line fresh,
/// and both other kind end on an ordinary week: holding the sharpness a
/// maintenance block works for, or the fitness an improvement block built,
/// is the point of running on past the horizon, not a reason to shed it.
fn taper_weeks_for(kind: PlanKind, d: RaceDistance) -> i64 {
    if !kind.has_race() {
        return 0;
    }
    match d {
        RaceDistance::K5 | RaceDistance::K10 => 1,
        _ => 2,
    }
}

/// Main-set load (threshold-minutes) a quality session carries at `progress`
/// through the plan: set by the race and scaled by the runner's current
/// volume against the distance's typical peak, then ramped with fitness —
/// week 1 of a plan asks 85 % of the load, the final build week the full
/// amount. Flat load across the plan meant week 1's intervals were the same
/// session as week 10's; the reference builds' sessions grew harder too.
pub(super) fn quality_load_at(d: RaceDistance, baseline_km: f64, progress: f64) -> f64 {
    let base = match d {
        RaceDistance::K5 => 20.0,
        RaceDistance::K10 => 22.0,
        RaceDistance::K21 => 23.0,
        RaceDistance::K42 => 24.0,
    };
    // Status is judged against the standard 4-session peak, so quality
    // session hardness stays comparable across plans of different frequency.
    let status = (baseline_km / peak_weekly_km(d, IncreaseMethod::Moderate, 4)).clamp(0.0, 1.0);
    base * (0.7 + 0.3 * status) * (0.85 + 0.15 * progress.clamp(0.0, 1.0))
}

/// The load at the *end* of a plan — the full amount a developed runner holds.
/// Kept for callers that want the ceiling without a progress value.
#[cfg(test)]
pub(super) fn quality_load(d: RaceDistance, baseline_km: f64) -> f64 {
    quality_load_at(d, baseline_km, 1.0)
}

/// The first week that is neither a down, taper nor race week is week 0 of
/// the build; everything before it (there is nothing before it in this
/// schedule) counts as progress 0. The build ends with the last build week —
/// taper and race weeks hold fitness rather than add it.
fn build_start(_weeks: usize) -> usize {
    0
}

/// Weeks that actually add fitness in a plan of `weeks` weeks.
fn build_weeks_count(kind: PlanKind, d: RaceDistance, weeks: usize) -> usize {
    // Only a race build loses its last week to the race itself.
    let tail = taper_weeks_for(kind, d) as usize + if kind.has_race() { 1 } else { 0 };
    weeks.saturating_sub(tail).max(build_start(weeks)).max(1)
}

/// 0..=1 — how far through the build the start of week `w` sits. Taper and
/// race weeks read as fully progressed.
///
/// A maintenance block is fully progressed from week 1: it asks for the
/// session it is holding, not a softened version of it. Ramping a plan whose
/// stated job is to *not* build fitness would quietly open with sessions
/// below the runner's current level.
pub(super) fn fitness_progress(kind: PlanKind, d: RaceDistance, weeks: usize, w: usize) -> f64 {
    if kind == PlanKind::Maintenance {
        return 1.0;
    }
    let build = build_weeks_count(kind, d, weeks);
    if w >= build {
        return 1.0;
    }
    (w as f64 / build as f64).clamp(0.0, 1.0)
}

/// Total threshold sharpening a plan delivers, as a multiplier on the
/// threshold pace: ~0.4 % per build week (the reference builds' 16-20 week
/// marathons moved threshold by roughly this much between first session and
/// race), capped at 6 % so a 30-week plan doesn't promise a transformation.
///
/// A maintenance block promises none: its sessions hold the threshold the
/// runner arrived with, because the plan is not trying to move it.
fn plan_fitness_gain(kind: PlanKind, d: RaceDistance, weeks: usize) -> f64 {
    if kind == PlanKind::Maintenance {
        return 1.0;
    }
    (1.0 - 0.004 * build_weeks_count(kind, d, weeks) as f64).clamp(0.94, 1.0)
}

/// The runner's threshold pace (s/km) at the start of week `w`: it sharpens
/// linearly from the value the plan was created with to that value × the
/// plan's fitness gain by the last build week. Every pace band and session
/// load derives from this, so the plan's sessions get faster week by week
/// instead of repeating the same workout for months.
pub(super) fn threshold_at(
    kind: PlanKind,
    d: RaceDistance,
    threshold: f64,
    weeks: usize,
    w: usize,
) -> f64 {
    let gain = plan_fitness_gain(kind, d, weeks);
    threshold * (1.0 - (1.0 - gain) * fitness_progress(kind, d, weeks, w))
}

/// Easy pace from threshold pace (s/km): ~threshold + 45-60 s.
pub fn easy_pace_s_km(threshold: f64) -> (i64, i64) {
    (
        (threshold + 45.0).round() as i64,
        (threshold + 75.0).round() as i64,
    )
}

/// Race pace (s/km) for `distance_m` implied by threshold, via the same Riegel
/// model the UI predicts with: T = 3600 * (D / D_1h) ^ 1.06, where D_1h is the
/// distance covered at threshold in one hour. Dividing T by D reduces it to
/// `pace = threshold * (D / D_1h) ^ 0.06`, which is always *slower* than
/// threshold — you cannot hold threshold pace for a marathon. Anchoring the
/// bands here keeps the paces the plan prescribes consistent with the finish
/// time the plan preview promises.
pub fn race_pace_s_km(threshold: f64, distance_m: f64) -> f64 {
    let one_hour_km = 3600.0 / threshold;
    threshold * (distance_m / 1000.0 / one_hour_km).powf(0.06)
}

/// Marathon pace band, centred on the Riegel marathon pace (threshold + ~15 s
/// for a sub-3 runner). The model previously used `threshold - 15 .. + 5`, i.e.
/// *faster* than threshold — for a 2:50 runner that prescribed 3:43/km reps,
/// a ~2:37 marathon, ~20 s/km off the goal the plan was built for.
pub fn mp_s_km(threshold: f64) -> (i64, i64) {
    let mp = race_pace_s_km(threshold, RaceDistance::K42.distance_m());
    ((mp - 8.0).round() as i64, (mp + 8.0).round() as i64)
}

/// Tempo band: at to just under threshold (a true lactate-threshold effort).
/// The old `threshold - 25 .. - 5` sat 25 s *faster* than threshold, which is
/// VO2 territory, not tempo — tempo is by definition the pace held ~1 h, i.e.
/// threshold itself.
pub fn tempo_s_km(threshold: f64) -> (i64, i64) {
    (
        (threshold - 5.0).round() as i64,
        (threshold + 5.0).round() as i64,
    )
}

/// Short interval band centred on predicted 3 km pace.
pub fn interval_s_km(threshold: f64) -> (i64, i64) {
    let pace = race_pace_s_km(threshold, 3000.0);
    ((pace - 5.0).round() as i64, (pace + 5.0).round() as i64)
}

/// ISO weekday (1=Mon..7=Sun) of a date.
fn iso_weekday(d: NaiveDate) -> u32 {
    d.weekday().num_days_from_monday() + 1
}

fn add_days(d: NaiveDate, n: i64) -> NaiveDate {
    d + chrono::Duration::days(n)
}

/// Monday of the week containing `d`.
fn week_start(d: NaiveDate) -> NaiveDate {
    add_days(d, -(iso_weekday(d) as i64 - 1))
}

/// Generate a full plan. `baseline_km` is the runner's current average weekly
/// volume (km); the first week starts there (or at 70 % of it when the plan
/// has many weeks, to leave room to build).
pub fn generate(req: &PlanRequest, baseline_km: f64) -> Result<(TrainingPlan, Vec<PlanWorkout>)> {
    let today = Utc::now().date_naive();
    if req.target_date <= today {
        return Err(AppError::Other(if req.kind.has_race() {
            "race date must be in the future".into()
        } else {
            "the plan's end date must be in the future".into()
        }));
    }
    let weeks = ((req.target_date - today).num_days() as f64 / 7.0).ceil() as usize;
    if weeks < 2 {
        return Err(AppError::Other(
            "need at least 2 weeks before the plan ends".into(),
        ));
    }
    // COROS accepts 4..=16 whole weeks for a plan, and a push truncates
    // whatever is longer. Truncating a race build costs a race week the runner
    // can plan around; truncating a maintenance block silently deletes ordinary
    // training weeks and reports success, so the horizon is capped here where
    // the runner can still be told.
    if !req.kind.has_race() && weeks > 16 {
        return Err(AppError::Other(
            "a plan without a race date runs at most 16 weeks — that is the longest block COROS will hold".into(),
        ));
    }
    if !(3..=7).contains(&req.weekly_frequency) {
        return Err(AppError::Other("weekly frequency must be 3..=7".into()));
    }
    if !(1..=7).contains(&req.longrun_day) {
        return Err(AppError::Other("long-run day must be 1..=7".into()));
    }

    // An explicit mileage from the form wins over the synced-activities baseline.
    let requested = req.current_weekly_km.unwrap_or(baseline_km);
    // The distance's typical peak is a target for a developing runner, not a
    // ceiling for one already training past it: clamping such a runner down
    // would open the plan with a volume cut and invert the ramp.
    //
    // A maintenance block is the exception: its whole purpose is to hold the
    // volume the runner arrived with, so aiming at the distance's peak here
    // would open a plan labelled "hold" with a volume jump instead.
    let ramping = req.kind != PlanKind::Maintenance;
    let peak = if ramping {
        peak_weekly_km(req.distance, req.increase, req.weekly_frequency).max(requested * 1.10)
    } else {
        requested.max(10.0)
    };
    let baseline = requested.clamp(10.0, peak);

    let taper_weeks = taper_weeks_for(req.kind, req.distance);
    // The geometric ramp only multiplies on *build* weeks — down, taper and
    // race weeks hold or cut volume without applying the ratio. Sizing the
    // ratio over every week undershot the peak: a 16-week marathon plan
    // multiplied ~11 times and topped out near 68 km instead of the 80 km both
    // reference builds actually reached (with a 32-35 km long run). Count the
    // weeks that really multiply and ramp to peak across exactly those.
    let build_weeks = (0..weeks)
        .filter(|&w| {
            let is_race = req.kind.has_race() && w == weeks - 1;
            let weeks_to_race = (weeks - 1 - w) as i64;
            let tapering = weeks_to_race < taper_weeks && !is_race;
            let down = !is_race && !tapering && (w + 1) % 4 == 0;
            !is_race && !tapering && !down
        })
        .count();
    // First build week sits at baseline; each later build week applies the
    // ratio once, so the peak lands after `build_weeks - 1` multiplications.
    let grow_steps = build_weeks.saturating_sub(1).max(1);
    // Geometric ramp baseline -> peak over the build weeks, respecting cap.
    let cap = req.increase.weekly_factor();
    let mut ratio = (peak / baseline).powf(1.0 / grow_steps as f64).min(cap);
    if weeks > 3 && ramping {
        ratio = ratio.max(1.02);
    }
    // A maintenance block runs the ratio at exactly 1: the +2 % floor keeps a
    // developing runner moving, and there is nothing to develop here.
    if !ramping {
        ratio = 1.0;
    }

    let plan_id = Uuid::new_v4().to_string();
    let plan = TrainingPlan {
        id: plan_id.clone(),
        distance: req.distance,
        kind: req.kind,
        target_date: req.target_date,
        status: PlanStatus::Draft,
        weekly_frequency: req.weekly_frequency,
        longrun_day: req.longrun_day,
        increase: req.increase,
        threshold_sec_km: req.threshold_sec_km,
        created_at: Utc::now(),
        pushed_id: None,
        baseline_weekly_km: Some(baseline),
        // A new plan has never had its ranges moved. The offset only appears
        // once the runner accepts a pacing offer, and then only from the day
        // that offer takes effect.
        quality_offset_sec_km: 0.0,
        quality_offset_from: None,
    };

    let first_monday = week_start(add_days(req.target_date, -7 * weeks as i64 + 1));
    let mut workouts = Vec::new();
    let mut week_volume = baseline;

    for w in 0..weeks {
        let monday = add_days(first_monday, 7 * w as i64);
        let is_race_week = req.kind.has_race() && w == weeks - 1;
        let weeks_to_race = (weeks - 1 - w) as i64;
        let tapering = weeks_to_race < taper_weeks && !is_race_week;
        // Down week every 4th build week.
        let down = !is_race_week && !tapering && (w + 1) % 4 == 0;

        let target = if is_race_week {
            week_volume * 0.55
        } else if tapering {
            week_volume
                * if taper_weeks == 2 && weeks_to_race == 1 {
                    0.85
                } else {
                    0.7
                }
        } else if down {
            week_volume * 0.7
        } else {
            week_volume
        };
        if !is_race_week && !tapering && !down && ramping {
            // The ramp floors at +2 %/wk so a developing runner always builds,
            // but that floor must not push an already-peaked runner past the
            // peak the plan was sized to: clamp so the ceiling holds.
            week_volume = (week_volume * ratio).min(peak);
        }

        let target_km = ceil_km(target);
        let lr_off = req.longrun_day as usize - 1;
        let before_race = |off: usize| add_days(monday, off as i64) < req.target_date;

        // Long run distance: ramps with weekly volume, capped.
        let long_cap = peak_long_run_km(req.distance);
        let mut long_km = if !before_race(lr_off) {
            0.0
        } else if is_race_week {
            match req.distance {
                RaceDistance::K42 => 10.0,
                RaceDistance::K21 => 8.0,
                _ => 6.0,
            }
        } else if tapering {
            ceil_km((target * 0.35).min(long_cap * 0.6)).max(MIN_RUN_KM)
        } else {
            ceil_km((target * long_run_share(req.distance)).min(long_cap)).max(MIN_RUN_KM)
        };

        // One session per day: the long-run day is fixed, the remaining slots
        // are evenly spaced and collisions nudged forward.
        let gap = |off: usize| {
            let gap = (off as i64 - lr_off as i64).abs();
            gap.min(7 - gap)
        };
        let mut used = [false; 7];
        used[lr_off] = true;
        let mut slots: Vec<usize> = Vec::new();
        for i in 0..req.weekly_frequency as usize - 1 {
            let mut off =
                ((i as f64 + 0.5) * (7.0 / req.weekly_frequency as f64)).floor() as usize % 7;
            while used[off] {
                off = (off + 1) % 7;
            }
            used[off] = true;
            slots.push(off);
        }
        slots.sort();
        let mut quality_off = slots.iter().copied().max_by_key(|&off| gap(off));
        if let Some(off) = quality_off.filter(|&off| gap(off) < 2) {
            if let Some(free) = (0..7)
                .filter(|&free| !used[free] && gap(free) >= 2)
                .max_by_key(|&free| gap(free))
            {
                slots.retain(|&slot| slot != off);
                slots.push(free);
                quality_off = Some(free);
            }
        }

        // Fitness ramps week by week: the threshold sharpens and the quality
        // load grows with it, so week 1's intervals are not week 10's.
        let thr_w = threshold_at(req.kind, req.distance, req.threshold_sec_km, weeks, w);
        let load = quality_load_at(
            req.distance,
            baseline,
            fitness_progress(req.kind, req.distance, weeks, w),
        );

        // Every quality session carries the plan's fixed load; a week too
        // small to hold it alongside the long run runs easy instead.
        let pattern = super::session::quality_pattern(req.kind, req.distance, w, weeks);
        let mut quality_km = 0.0;
        let quality_off = quality_off
            .filter(|&off| !is_race_week && !down && before_race(off))
            .filter(|_| {
                let session_m = super::session::quality_session_m(pattern, thr_w, load);
                quality_km = ceil_km(session_m / 1000.0).max(MIN_RUN_KM);
                quality_km <= target_km - long_km
            });
        if quality_off.is_none() {
            quality_km = 0.0;
        }

        // Easy runs share what is left in whole km. Any that would fall under
        // the minimum are dropped (closest to the long run first) and their
        // mileage moves to the runs that remain.
        let mut easy_offs: Vec<usize> = slots
            .iter()
            .copied()
            .filter(|&off| Some(off) != quality_off && before_race(off))
            .collect();
        easy_offs.sort_by_key(|&off| std::cmp::Reverse(gap(off)));
        let mut rest = (target_km - long_km - quality_km).max(0.0);
        while rest < MIN_RUN_KM * easy_offs.len() as f64 {
            easy_offs.pop();
        }
        if easy_offs.is_empty() {
            if quality_off.is_some() {
                let extra = if long_km > 0.0 {
                    (rest / 2.0).ceil()
                } else {
                    rest
                };
                quality_km += extra;
                rest -= extra;
            }
            if long_km > 0.0 {
                long_km += rest;
            }
            rest = 0.0;
        }
        let easy_base = (rest / easy_offs.len().max(1) as f64).floor();
        let easy_extra = rest - easy_base * easy_offs.len() as f64;

        let mut sessions: Vec<(usize, WorkoutKind, f64)> = easy_offs
            .iter()
            .enumerate()
            .map(|(i, &off)| {
                let km = easy_base + if (i as f64) < easy_extra { 1.0 } else { 0.0 };
                (off, WorkoutKind::Easy, km)
            })
            .collect();
        if long_km > 0.0 {
            sessions.push((lr_off, WorkoutKind::LongRun, long_km));
        }
        if let Some(off) = quality_off {
            sessions.push((off, pattern.kind(), quality_km));
        }
        sessions.sort_by_key(|session| session.0);

        let easy_pace = easy_pace_s_km(thr_w);
        for (off, kind, km) in sessions {
            let pace = match kind {
                WorkoutKind::LongRun
                    if req.distance == RaceDistance::K42
                        && !tapering
                        && !is_race_week
                        && w >= weeks / 2 =>
                {
                    mp_s_km(thr_w)
                }
                WorkoutKind::Intervals | WorkoutKind::Tempo => pattern.pace(thr_w),
                _ => easy_pace,
            };
            let mut workout = PlanWorkout {
                id: Uuid::new_v4().to_string(),
                plan_id: plan_id.clone(),
                date: add_days(monday, off as i64),
                kind,
                title: String::new(),
                distance_m: Some(km * 1000.0),
                duration_min: None,
                pace_lo_s_km: Some(pace.0),
                pace_hi_s_km: Some(pace.1),
                done: false,
            };
            workout.title = match kind {
                WorkoutKind::LongRun => format!("Long run {km:.0} km"),
                WorkoutKind::Easy => format!("Easy run {km:.0} km"),
                _ => {
                    let details = super::session::details_for_plan(&workout, &plan);
                    let main = &details.steps[1];
                    if kind == WorkoutKind::Intervals {
                        workout.pace_lo_s_km = main.pace_lo_s_km;
                        workout.pace_hi_s_km = main.pace_hi_s_km;
                        format!("{} x {:.0} m intervals", main.repeats, main.distance_m)
                    } else if main.repeats > 1 {
                        format!(
                            "{} x {:.1} km tempo",
                            main.repeats,
                            main.distance_m / 1000.0
                        )
                    } else if details.steps.len() > 3 {
                        let work: f64 = details.steps[1..details.steps.len() - 1]
                            .iter()
                            .map(|step| step.distance_m)
                            .sum();
                        format!("Progressive tempo {:.1} km", work / 1000.0)
                    } else {
                        format!("Continuous tempo {:.1} km", main.distance_m / 1000.0)
                    }
                }
            };
            workouts.push(workout);
        }
    }

    // Shakeout two days out: the easy run already there, or an extra minimum-length one.
    // Race-build only — with no start line to arrive at, the last two days are
    // ordinary training days and an unexplained short run just looks lost.
    if req.kind.has_race() {
        let shake = add_days(req.target_date, -2);
        match workouts.iter_mut().find(|workout| workout.date == shake) {
            Some(workout) if workout.kind == WorkoutKind::Easy => {
                workout.title = format!(
                    "Shakeout {:.0} km",
                    workout.distance_m.unwrap_or(0.0) / 1000.0
                );
            }
            Some(_) => {}
            None => {
                let pace = easy_pace_s_km(req.threshold_sec_km);
                workouts.push(PlanWorkout {
                    id: Uuid::new_v4().to_string(),
                    plan_id: plan_id.clone(),
                    date: shake,
                    kind: WorkoutKind::Easy,
                    title: format!("Shakeout {MIN_RUN_KM:.0} km"),
                    distance_m: Some(MIN_RUN_KM * 1000.0),
                    duration_min: None,
                    pace_lo_s_km: Some(pace.0),
                    pace_hi_s_km: Some(pace.1),
                    done: false,
                });
            }
        }
        let race_pace = race_pace_s_km(req.threshold_sec_km, req.distance.distance_m());
        workouts.push(PlanWorkout {
            id: Uuid::new_v4().to_string(),
            plan_id: plan_id.clone(),
            date: req.target_date,
            kind: WorkoutKind::Race,
            title: format!("Race day — {}", req.distance.label()),
            distance_m: Some(req.distance.distance_m()),
            duration_min: None,
            pace_lo_s_km: Some((race_pace - 6.0).round() as i64),
            pace_hi_s_km: Some((race_pace + 6.0).round() as i64),
            done: false,
        });
    }
    // An improvement block finishes with a time trial over its target distance:
    // the week's proof of what the block bought, and the session that decides
    // whether the next block starts higher. Maintenance ends on its last
    // ordinary week — there is nothing to test.
    if req.kind == PlanKind::Improvement {
        let trial_pace = race_pace_s_km(req.threshold_sec_km, req.distance.distance_m());
        workouts.push(PlanWorkout {
            id: Uuid::new_v4().to_string(),
            plan_id: plan_id.clone(),
            date: req.target_date,
            kind: WorkoutKind::Race,
            title: format!("Time trial — {}", req.distance.label()),
            distance_m: Some(req.distance.distance_m()),
            duration_min: None,
            pace_lo_s_km: Some((trial_pace - 6.0).round() as i64),
            pace_hi_s_km: Some((trial_pace + 6.0).round() as i64),
            done: false,
        });
    }

    for workout in &mut workouts {
        workout.duration_min = super::session::details_for_plan(workout, &plan).estimated_min;
    }
    workouts.sort_by_key(|x| x.date);
    Ok((plan, workouts))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(distance: RaceDistance, weeks_out: i64) -> PlanRequest {
        PlanRequest {
            distance,
            kind: PlanKind::Race,
            target_date: NaiveDate::from_ymd_opt(2026, 10, 4).unwrap()
                + chrono::Duration::days(weeks_out),
            threshold_sec_km: 285.0, // 4:45/km
            weekly_frequency: 4,
            longrun_day: 6,
            increase: IncreaseMethod::Moderate,
            current_weekly_km: None,
        }
    }

    #[test]
    fn marathon_plan_has_long_runs_and_race() {
        // Anchored on "now": this asserts a week count, and `req()`'s fixed
        // base date drifts into the past, quietly shortening the plan.
        let r = req_from_now(RaceDistance::K42, 16);
        let (plan, workouts) = generate(&r, 40.0).unwrap();
        assert_eq!(plan.distance, RaceDistance::K42);
        let long_runs: Vec<_> = workouts
            .iter()
            .filter(|w| w.kind == WorkoutKind::LongRun)
            .collect();
        assert!(
            long_runs.len() >= 10,
            "expected many long runs, got {}",
            long_runs.len()
        );
        // Long run never exceeds the cap and grows overall.
        for w in &long_runs {
            assert!(w.distance_m.unwrap() <= 35_500.0);
        }
        assert!(
            long_runs.first().unwrap().distance_m.unwrap()
                < long_runs
                    .iter()
                    .map(|w| w.distance_m.unwrap())
                    .fold(f64::NEG_INFINITY, f64::max)
        );
        let race = workouts
            .iter()
            .find(|w| w.kind == WorkoutKind::Race)
            .unwrap();
        assert_eq!(race.date, r.target_date);
        // Every week has the requested frequency (except race week).
        assert!(
            workouts
                .iter()
                .filter(|w| w.kind != WorkoutKind::Race)
                .count()
                >= 4 * 15
        );
    }

    #[test]
    fn weekly_volume_respects_increase_cap() {
        let r = req_from_now(RaceDistance::K10, 8);
        let (_, workouts) = generate(&r, 20.0).unwrap();
        // Bucket by the same Monday-anchored week the engine builds with, so a
        // plan week and a measured week are the same thing (iso_week() splits
        // differently at year boundaries and mis-merged sessions here).
        let weeks = week_volumes(&workouts);
        // The engine's schedule for 8 weeks: index 3 is a down week, the last
        // is race week; both are excluded so only build-to-build growth is
        // measured. Classifying by position is exact, unlike a dip heuristic.
        // The week before race week is dropped too — the shakeout sits two
        // days before the race, which lands in race week's bucket or this
        // one's depending on the weekday the plan was generated.
        let last = weeks.len() - 1;
        let build: Vec<f64> = weeks
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != last && *i + 1 != last && (i + 1) % 4 != 0)
            .map(|(_, w)| w.1)
            .collect();
        assert!(build.len() >= 4, "expected several build weeks");
        for pair in build.windows(2) {
            // Weeks are planned in whole km rounded up, so allow one km of rounding.
            assert!(
                pair[1] <= pair[0] * 1.08 + 1.0 + 1e-6,
                "build-week jump {:.2} -> {:.2} exceeds the +8% cap",
                pair[0],
                pair[1]
            );
        }
    }

    /// Every run is whole km, at least 6 km (bar the race), one per day.
    #[test]
    fn runs_are_whole_km_at_least_six_and_one_per_day() {
        for distance in [
            RaceDistance::K5,
            RaceDistance::K10,
            RaceDistance::K21,
            RaceDistance::K42,
        ] {
            for frequency in 3..=7 {
                for longrun_day in 1..=7 {
                    for baseline in [10.0, 25.0, 60.0] {
                        let r = PlanRequest {
                            weekly_frequency: frequency,
                            longrun_day,
                            current_weekly_km: Some(baseline),
                            ..req_from_now(distance, 12)
                        };
                        let (_, workouts) = generate(&r, baseline).unwrap();
                        let mut dates = std::collections::BTreeSet::new();
                        for w in &workouts {
                            let ctx = format!(
                                "{distance:?} f{frequency} lr{longrun_day} {baseline} km: {} on {}",
                                w.title, w.date
                            );
                            assert!(dates.insert(w.date), "double session: {ctx}");
                            if w.kind == WorkoutKind::Race {
                                continue;
                            }
                            let km = w.distance_m.unwrap() / 1000.0;
                            assert!(km >= MIN_RUN_KM, "short run: {ctx}");
                            assert!((km - km.round()).abs() < 1e-9, "not whole km: {ctx}");
                        }
                    }
                }
            }
        }
    }

    /// Pins the magnitude: a runner already at the marathon's 80 km peak gets the full 24 threshold-minutes.
    #[test]
    fn quality_load_scales_with_race_and_volume() {
        assert!((quality_load(RaceDistance::K42, 80.0) - 24.0).abs() < 1e-9);
        assert!((quality_load(RaceDistance::K42, 10.0) - 24.0 * (0.7 + 0.3 * 0.125)).abs() < 1e-9);
        assert!(quality_load(RaceDistance::K5, 32.0) < quality_load(RaceDistance::K42, 80.0));
    }

    /// The fitness ramp must stay inside its promise: threshold sharpens
    /// monotonically from the value the plan was created with, by ~0.4 % per
    /// build week and never more than 6 % total.
    #[test]
    fn threshold_ramps_monotonically_within_its_cap() {
        let thr = 228.0; // ~3:48/km, the 2:50-marathon threshold
        let weeks = 16;
        let first = threshold_at(PlanKind::Race, RaceDistance::K42, thr, weeks, 0);
        let mid = threshold_at(PlanKind::Race, RaceDistance::K42, thr, weeks, 6);
        let last = threshold_at(PlanKind::Race, RaceDistance::K42, thr, weeks, 13);
        assert!(first > mid && mid > last, "threshold must sharpen weekly");
        assert!(last >= thr * 0.94, "ramp exceeded its 6 % cap");
        // 13 build weeks x 0.4 % — the ramp must actually travel.
        assert!(
            last < thr * 0.955,
            "ramp too small to feel: {last:.0} vs {thr:.0}"
        );
    }

    /// The point of the ramp: a plan's quality sessions must get faster and
    /// harder as it progresses — week 1's intervals cannot be week 10's.
    /// The reference builds' 400 m repeats ran 3:35 in month 1 and 2:50-3:05
    /// by month 3 of the same build.
    #[test]
    fn quality_sessions_progress_across_the_plan() {
        let r = req_from_now(RaceDistance::K42, 16);
        let (_, workouts) = generate(&r, 50.0).unwrap();
        // Compare like with like: the pattern rotates, so take the first and
        // last session of one kind (tempo appears in both halves of a
        // marathon build) and check the week's threshold actually sharpened.
        let tempos: Vec<_> = workouts
            .iter()
            .filter(|w| w.kind == WorkoutKind::Tempo)
            .collect();
        assert!(tempos.len() >= 3, "expected several tempo sessions");
        let (first, last) = (tempos.first().unwrap(), tempos.last().unwrap());
        assert!(
            last.pace_lo_s_km.unwrap() < first.pace_lo_s_km.unwrap(),
            "last tempo {} ({} s/km) must be faster than first tempo {} ({} s/km)",
            last.date,
            last.pace_lo_s_km.unwrap(),
            first.date,
            first.pace_lo_s_km.unwrap()
        );
        // The stored bands must match the week's threshold the session was
        // sized at — the same value details_for_plan reconstructs with.
        // And harder: the load climbs from 85 % of the plan's ceiling in week
        // 1 to the full amount by the final build week.
        let early = quality_load_at(
            RaceDistance::K42,
            50.0,
            fitness_progress(PlanKind::Race, RaceDistance::K42, 16, 0),
        );
        let late = quality_load_at(
            RaceDistance::K42,
            50.0,
            fitness_progress(PlanKind::Race, RaceDistance::K42, 16, 12),
        );
        assert!(
            late > early * 1.1,
            "load barely moved: {early:.1} -> {late:.1}"
        );
    }

    #[test]
    fn past_race_date_rejected() {
        let mut r = req(RaceDistance::K5, 1);
        r.target_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        assert!(generate(&r, 20.0).is_err());
    }

    /// Marathon pace must be *slower* than threshold — you cannot hold
    /// threshold for 42 km. The old band (`threshold - 15 .. + 5`) inverted
    /// this: for a 2:50 runner (threshold ~3:48) it prescribed 3:43/km reps, a
    /// ~2:37 pace ~20 s/km faster than the goal the plan was built for. The
    /// bands now derive from the same Riegel model the UI predicts with, so a
    /// 2:50-goal threshold yields a marathon-pace band around the true 4:02.
    #[test]
    fn marathon_pace_is_slower_than_threshold() {
        // Threshold that the Riegel model maps to a 2:50 marathon (~3:48/km).
        let thr = 228.0;
        let (lo, hi) = mp_s_km(thr);
        let goal_mp = 10_200.0 / 42.195; // 2:50 over 42.195 km = 4:02/km
        assert!(
            lo as f64 > thr,
            "marathon pace {lo} must be slower than threshold {thr}"
        );
        assert!(
            (lo as f64..=hi as f64).contains(&goal_mp),
            "MP band {lo}..{hi} misses the true 2:50 pace {goal_mp:.0}"
        );
        // Race pace is monotonic in distance: faster than threshold for a 5K,
        // slower for a marathon.
        assert!(race_pace_s_km(thr, RaceDistance::K5.distance_m()) < thr);
        assert!(race_pace_s_km(thr, RaceDistance::K42.distance_m()) > thr);
    }

    /// The marathon ramp must actually *reach* the peak the plan is sized to.
    /// Both reference builds (Runna, aimed at 2:50) peaked at 76-80 km/wk with
    /// a 32-35 km long run; the old ratio, sized over every week but applied
    /// only on build weeks, topped a 50 km runner out near 68 km / 30 km — a
    /// full tier below the plan that made those races. A 16-week plan from a
    /// 50 km baseline now has to land on that peak.
    #[test]
    fn marathon_plan_reaches_its_measured_peak() {
        let r = req_from_now(RaceDistance::K42, 16);
        let (_, workouts) = generate(&r, 50.0).unwrap();
        let weeks = week_volumes(&workouts);
        let peak_week = weeks.iter().map(|(_, v)| *v).fold(0.0f64, f64::max);
        let peak_long = workouts
            .iter()
            .filter(|w| w.kind == WorkoutKind::LongRun)
            .map(|w| w.distance_m.unwrap() / 1000.0)
            .fold(0.0f64, f64::max);
        assert!(
            peak_week > 76.0,
            "peak week {peak_week:.1} km falls short of the 76-80 km both builds reached"
        );
        assert!(
            peak_long >= 32.0,
            "peak long run {peak_long:.1} km is below the 32-35 km the builds ran"
        );
        assert!(
            peak_long <= 35.0,
            "peak long run {peak_long:.1} km over cap"
        );
    }

    /// Through the build weeks the marathon long run holds ~44 % of weekly
    /// volume (the builds measured 42-46 %, median ~43 %). A flat 38 % — the
    /// shorter-race value — left an 80 km week with only a 30 km long run.
    #[test]
    fn marathon_long_run_holds_its_share_of_peak_weeks() {
        let r = req_from_now(RaceDistance::K42, 16);
        let (_, workouts) = generate(&r, 50.0).unwrap();
        // Look at the three heaviest build weeks: the long-run share only
        // matters where the volume is, and taper/down weeks are excluded by
        // taking the top weeks (all deep in the build).
        let mut weeks = week_volumes(&workouts);
        weeks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        for (mon, vol) in weeks.iter().take(3) {
            let long: f64 = workouts
                .iter()
                .filter(|w| {
                    w.kind == WorkoutKind::LongRun
                        && (w.date
                            - chrono::Duration::days(
                                w.date.weekday().num_days_from_monday() as i64,
                            ))
                        .format("%Y-%m-%d")
                        .to_string()
                        == *mon
                })
                .map(|w| w.distance_m.unwrap() / 1000.0)
                .sum();
            let share = long / vol;
            assert!(
                (0.40..=0.46).contains(&share),
                "long-run share {share:.2} in the {vol:.0} km week {mon} is outside the 0.40-0.46 band"
            );
        }
    }

    /// Anchored on "now" so the plan always spans exactly `weeks_out` weeks.
    /// `req()`'s fixed base date drifts as time passes, which quietly changes
    /// how many weeks a plan has and with it any assertion about week counts.
    fn req_from_now(distance: RaceDistance, weeks_out: i64) -> PlanRequest {
        PlanRequest {
            distance,
            kind: PlanKind::Race,
            target_date: Utc::now().date_naive() + chrono::Duration::days(weeks_out * 7),
            threshold_sec_km: 285.0,
            weekly_frequency: 4,
            longrun_day: 6,
            increase: IncreaseMethod::Moderate,
            current_weekly_km: None,
        }
    }

    fn week_volumes(workouts: &[PlanWorkout]) -> Vec<(String, f64)> {
        use chrono::Datelike;
        let mut per_week: std::collections::BTreeMap<String, f64> = Default::default();
        for w in workouts {
            if w.kind == WorkoutKind::Race {
                continue;
            }
            let mon =
                w.date - chrono::Duration::days(w.date.weekday().num_days_from_monday() as i64);
            *per_week
                .entry(mon.format("%Y-%m-%d").to_string())
                .or_default() += w.distance_m.unwrap_or(0.0) / 1000.0;
        }
        per_week.into_iter().collect()
    }

    /// A build week must actually *deliver* the volume the ramp asks for.
    /// The growth-cap test above only compares week-to-week ratios, so it kept
    /// passing while every easy run divided the leftover volume by the total
    /// session count (long run and quality session included) and each week
    /// landed ~20 % short of its target.
    #[test]
    fn build_week_delivers_its_target_volume() {
        let baseline = 20.0;
        let r = req_from_now(RaceDistance::K10, 8);
        let (plan, workouts) = generate(&r, baseline).unwrap();
        let weeks = week_volumes(&workouts);
        let first = weeks.first().expect("plan has weeks").1;
        assert!(
            (first - baseline).abs() / baseline < 0.05,
            "first week should sit at the runner's {baseline} km baseline, got {first:.1} km"
        );
        // The recorded baseline is what the form showed, not a later ramp value.
        assert_eq!(plan.baseline_weekly_km, Some(baseline));
        // No week may be empty and every session keeps a distance.
        assert!(weeks.iter().all(|(_, v)| *v > 0.0));
        assert!(workouts.iter().all(|w| w.distance_m.is_some()));
    }

    /// A runner already training above the distance's typical peak must not
    /// be clamped down to it — that would open the plan with a volume cut.
    #[test]
    fn high_mileage_runner_is_not_clamped_to_the_distance_peak() {
        let r = PlanRequest {
            current_weekly_km: Some(65.0),
            ..req_from_now(RaceDistance::K5, 8)
        };
        let (plan, workouts) = generate(&r, 65.0).unwrap();
        let weeks = week_volumes(&workouts);
        let first = weeks.first().unwrap().1;
        assert!(
            first > 55.0,
            "a 65 km/wk runner must not start below their volume, got {first:.1}"
        );
        assert!(plan.baseline_weekly_km.unwrap() > 60.0);
    }

    // ---- plan kinds --------------------------------------------------------
    //
    // A plan's kind decides its shape, so each kind is pinned by what it must
    // NOT contain as much as by what it does: a maintenance block that ramps
    // volume or ends on a start line is the same bug wearing a different label.

    fn kind_req(kind: PlanKind, distance: RaceDistance, weeks_out: i64) -> PlanRequest {
        PlanRequest {
            kind,
            current_weekly_km: Some(30.0),
            ..req_from_now(distance, weeks_out)
        }
    }

    /// The point of a maintenance block: the volume holds. Every non-down week
    /// sits at the runner's own mileage rather than climbing toward the
    /// distance's typical peak, which is what a race build does.
    #[test]
    fn maintenance_plan_holds_volume_instead_of_building() {
        let baseline = 30.0;
        let (_, workouts) = generate(
            &kind_req(PlanKind::Maintenance, RaceDistance::K21, 8),
            baseline,
        )
        .unwrap();
        let weeks = week_volumes(&workouts);
        assert!(
            weeks.len() >= 6,
            "expected most of the 8 weeks, got {}",
            weeks.len()
        );
        for (monday, vol) in &weeks {
            let holding = (vol - baseline).abs() / baseline < 0.12;
            let down = (vol - baseline * 0.7).abs() / baseline < 0.12;
            assert!(
                holding || down,
                "maintenance week {monday} ran {vol:.1} km — neither the {baseline} km \
                 holding volume nor a down week of it"
            );
        }
        // And it must not quietly become a build: the heaviest week is not
        // meaningfully heavier than the first.
        let heaviest = weeks.iter().map(|(_, v)| *v).fold(0.0f64, f64::max);
        assert!(
            heaviest <= baseline * 1.15,
            "a maintenance block grew to {heaviest:.1} km from a {baseline} km baseline"
        );
    }

    /// A maintenance block has no start line, so it gets no race day, no
    /// shakeout and no taper — and an improvement block gets a time trial
    /// instead of a race.
    #[test]
    fn only_a_race_build_ends_on_a_race_day() {
        let (_, race) = generate(&kind_req(PlanKind::Race, RaceDistance::K10, 8), 30.0).unwrap();
        assert!(race.iter().any(|w| w.kind == WorkoutKind::Race));

        let (_, improve) =
            generate(&kind_req(PlanKind::Improvement, RaceDistance::K10, 8), 30.0).unwrap();
        let trial = improve
            .iter()
            .find(|w| w.kind == WorkoutKind::Race)
            .expect("an improvement block should close with a time trial");
        assert!(
            trial.title.starts_with("Time trial"),
            "expected a time trial, got {}",
            trial.title
        );
        assert!(
            !improve.iter().any(|w| w.title.starts_with("Shakeout")),
            "nothing is being raced, so nothing needs shaking out"
        );

        let (_, hold) =
            generate(&kind_req(PlanKind::Maintenance, RaceDistance::K10, 8), 30.0).unwrap();
        assert!(
            !hold.iter().any(|w| w.kind == WorkoutKind::Race),
            "a maintenance block must not schedule a race or a trial"
        );
        assert!(
            !hold.iter().any(|w| w.title.starts_with("Shakeout")),
            "with no start line, the last two days are ordinary training days"
        );
    }

    /// A maintenance block asks for the session it is holding from week 1: its
    /// threshold does not sharpen and its quality load does not grow, because
    /// the plan's stated job is to not build fitness. Ramping it would open
    /// with sessions below the runner's current level.
    #[test]
    fn maintenance_plan_asks_the_same_of_every_week() {
        let thr = 285.0;
        let weeks = 12;
        let first = threshold_at(PlanKind::Maintenance, RaceDistance::K21, thr, weeks, 0);
        let last = threshold_at(PlanKind::Maintenance, RaceDistance::K21, thr, weeks, 11);
        assert!(
            (first - last).abs() < 1e-9 && (first - thr).abs() < 1e-9,
            "maintenance threshold moved: {first} -> {last} (created at {thr})"
        );
        // The same plan sharpening as a race build is the control: the flat
        // line above is the kind, not a broken ramp.
        let race_last = threshold_at(PlanKind::Race, RaceDistance::K21, thr, weeks, 9);
        assert!(race_last < thr, "the race ramp must still travel");

        let early = fitness_progress(PlanKind::Maintenance, RaceDistance::K21, weeks, 0);
        assert!(
            (early - 1.0).abs() < 1e-9,
            "week 1 of a holding block must ask full value, got {early}"
        );
    }

    /// COROS holds 4..=16 whole weeks and silently drops courses past the end
    /// of that window. A race build's truncation is visible (the race week is
    /// missing); a holding block's is invisible, so the horizon is refused here.
    #[test]
    fn a_plan_without_a_race_date_is_capped_at_the_provider_window() {
        assert!(generate(
            &kind_req(PlanKind::Maintenance, RaceDistance::K21, 20),
            30.0
        )
        .is_err());
        assert!(generate(
            &kind_req(PlanKind::Improvement, RaceDistance::K21, 20),
            30.0
        )
        .is_err());
        // A race build may run longer than 16 weeks — the race date is fixed
        // by the event, not by the watch.
        assert!(generate(&kind_req(PlanKind::Race, RaceDistance::K42, 20), 30.0).is_ok());
    }

    /// The target distance still shapes a holding block: a 42K maintenance
    /// plan keeps a marathon-length long run and marathon-pace work in it, and
    /// a 5K one does neither. Only the *volume* stops building.
    #[test]
    fn maintenance_target_distance_still_shapes_the_sessions() {
        let build = |d: RaceDistance| {
            let (_, workouts) = generate(
                &PlanRequest {
                    current_weekly_km: Some(60.0),
                    ..kind_req(PlanKind::Maintenance, d, 10)
                },
                60.0,
            )
            .unwrap();
            workouts
        };
        let longest_long = |ws: &[PlanWorkout]| {
            ws.iter()
                .filter(|w| w.kind == WorkoutKind::LongRun)
                .map(|w| w.distance_m.unwrap() / 1000.0)
                .fold(0.0f64, f64::max)
        };
        let short = build(RaceDistance::K5);
        let long = build(RaceDistance::K42);
        assert!(
            longest_long(&long) > longest_long(&short) * 1.5,
            "a 42K holding block (long run {:.0} km) should look nothing like a 5K \
             one ({:.0} km)",
            longest_long(&long),
            longest_long(&short)
        );
        // The 42K block's long runs carry a marathon-pace band; the 5K block's
        // are easy throughout. That is the distance showing up in the sessions
        // themselves, not only in their length.
        let mp_bands = |ws: &[PlanWorkout]| {
            ws.iter()
                .filter(|w| w.kind == WorkoutKind::LongRun)
                .filter(|w| w.pace_lo_s_km.unwrap() > 285 && w.pace_hi_s_km.unwrap() < 330)
                .count()
        };
        assert!(
            mp_bands(&long) > 0 && mp_bands(&short) == 0,
            "marathon pace appeared in the wrong block: 42K {} bands, 5K {} bands",
            mp_bands(&long),
            mp_bands(&short)
        );
    }
}
