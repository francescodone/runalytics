//! Weekly volume targets: the periodised curve and the safety rails around it.
//!
//! The curve is built in two passes on purpose. The first pass grows volume by
//! the athlete's permitted weekly step toward the goal's peak; the second
//! applies phase multipliers, deloads and hard ceilings. Separating them means
//! a ceiling that clips one week never silently changes the shape of the
//! progression — the clip is recorded and reported instead.

use runalytics_core::{AthleteSnapshot, GoalKind, Phase, PlanConstraints, VolumeKm};

/// EMA window lengths used for the projected load curves.
///
/// 42-day chronic and 7-day acute are the conventional intervals; keeping them
/// as constants here means the plan's projected ACWR and the scoring crate's
/// realised ACWR are computed the same way.
const CHRONIC_DAYS: f64 = 42.0;
const ACUTE_DAYS: f64 = 7.0;

/// The ACWR band the engine aims to keep the plan inside.
pub const ACWR_TARGET_LOW: f64 = 0.8;
pub const ACWR_TARGET_HIGH: f64 = 1.3;

/// The volume curve for one week, before sessions are placed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeekVolume {
    pub phase: Phase,
    pub target: VolumeKm,
    pub previous: VolumeKm,
    pub step_pct: f64,
    pub projected_acwr: f64,
    pub is_deload: bool,
    /// True when a ceiling (constraint, step, or long-run share) clipped the
    /// week below what the curve wanted. Surfaced in the UI as a warning.
    pub clipped: bool,
    /// True on week 0 when the acute:chronic guard lowered the starting volume
    /// below what the athlete's past peak would otherwise have justified.
    pub acwr_throttled: bool,
}

/// Assign a phase to every week index of the plan.
///
/// Race goals run Base -> Build -> Peak -> Taper -> Race. The taper is sized
/// from the goal's `taper_days` so a marathon clears fatigue for three weeks
/// and a 5K for one. Short plans shed phases from the *front* (Base, then
/// Peak, then Build) and never from the taper — the taper is what makes the
/// race day fresh, and cutting it would ship a plan that fails at its one
/// visible moment. Non-race goals skip the taper entirely: `Recovery`
/// rebuilds from a deload, `Maintain` simply holds.
#[must_use]
pub fn phase_spans(goal: GoalKind, weeks: u8) -> Vec<(Phase, u8, u8)> {
    let total = usize::from(weeks);
    if total == 0 {
        return Vec::new();
    }

    // Weeks allocated to each phase, in order. A zero count drops the phase.
    let alloc: Vec<(Phase, usize)> = match goal {
        GoalKind::Maintain => vec![(Phase::Hold, total)],
        GoalKind::Recovery => {
            if total <= 2 {
                vec![(Phase::Rebuild, total)]
            } else {
                let deload = total / 2;
                vec![(Phase::Taper, deload), (Phase::Rebuild, total - deload)]
            }
        }
        _ => {
            // The last week is always race week, and the taper is sized from
            // the goal but never allowed to eat the whole block: at least one
            // week of real training has to survive ahead of it.
            let race = 1;
            let max_taper = total.saturating_sub(2);
            let taper = ((goal.taper_days() / 7) as usize)
                .clamp(1, 3)
                .min(max_taper);
            let pre_taper = total - race - taper;

            let peak = usize::from(pre_taper >= 5);
            let rest = pre_taper - peak;
            let base = if rest >= 3 { rest / 2 } else { rest / 3 };
            let build = rest - base;

            let mut v = vec![
                (Phase::Base, base),
                (Phase::Build, build),
                (Phase::Peak, peak),
            ];
            if taper > 0 {
                v.push((Phase::Taper, taper));
            }
            v.push((Phase::Race, race));
            v
        }
    };

    let mut spans: Vec<(Phase, u8, u8)> = Vec::new();
    let mut cursor = 0usize;
    for (phase, count) in alloc {
        if count == 0 {
            continue;
        }
        let from = cursor;
        let to = cursor + count - 1;
        cursor = to + 1;
        // Merge with the previous span if a dropped phase left neighbours of
        // the same kind adjacent (can happen on very short plans).
        match spans.last_mut() {
            Some(last) if last.0 == phase => last.2 = to as u8,
            _ => spans.push((phase, from as u8, to as u8)),
        }
    }
    spans
}

/// The volume the athlete is building from.
///
/// `current_weekly_volume` is the honest base. An athlete with a much higher
/// recent peak is detrained, and the base is pulled toward that peak so the
/// plan rebuilds toward known capacity instead of starting from the detrained
/// value and never reaching the goal.
#[must_use]
pub fn starting_volume(athlete: &AthleteSnapshot, goal: GoalKind) -> VolumeKm {
    let current = athlete.current_weekly_volume.as_f64().max(10.0);
    let peak = athlete.peak_weekly_volume.as_f64();
    let base = if peak > current * 1.25 {
        // Detrained: start partway back toward the peak rather than at zero.
        current + (peak - current) * 0.35
    } else {
        current
    };
    // A recovery block starts below the current base by definition.
    if goal == GoalKind::Recovery {
        VolumeKm(base * 0.7)
    } else {
        VolumeKm(base)
    }
}

/// How far above recent volume week one of a rebuild may open.
///
/// A detrained athlete genuinely returns faster than 10 % a week — that is the
/// well-documented detraining effect, and pretending otherwise wastes their
/// residual fitness. But it is a *return* to previous capacity, not new load:
/// one week may not jump further than this past what they are running now.
pub const REBUILD_ALLOWANCE: f64 = 1.20;

/// The base volume after the rebuild guards.
///
/// Two independent guards, because they catch different mistakes:
///
/// * an **absolute** cap — week one may not open more than
///   [`REBUILD_ALLOWANCE`] above the athlete's recent volume. ACWR cannot catch
///   this: it is a lagging ratio, and a single heavy week barely moves it.
/// * an **ACWR** cap — the opening week must not project outside the safe band,
///   which catches shapes the absolute cap would miss.
///
/// When either binds, the guard reports it, so the UI can say "we throttled your
/// rebuild" rather than shipping a plan that spikes and staying silent.
#[must_use]
pub fn throttled_start(
    athlete: &AthleteSnapshot,
    goal: GoalKind,
    first_factor: f64,
) -> (VolumeKm, bool) {
    let wanted = starting_volume(athlete, goal);
    let recent = athlete.current_weekly_volume.as_f64();
    if recent <= 0.0 {
        return (wanted, false);
    }
    let absolute_cap = recent * REBUILD_ALLOWANCE;
    // Never throttle below what the athlete is already running: that would turn
    // a rebuild into an unplanned deload.
    let floor = VolumeKm(recent);
    let mut candidate = wanted;
    for _ in 0..32 {
        let week_one = candidate.as_f64() * first_factor;
        let mut series = vec![recent; 4];
        series.push(week_one);
        let within_band = projected_acwr(&series) <= ACWR_TARGET_HIGH;
        let within_cap = week_one <= absolute_cap;
        if (within_band && within_cap) || candidate <= floor {
            return (at_least(candidate, floor), candidate < wanted);
        }
        candidate = VolumeKm(candidate.as_f64() * 0.95);
    }
    (at_least(candidate, floor), candidate < wanted)
}

/// `VolumeKm` is only `PartialOrd`, so there is no `max` method.
fn at_least(value: VolumeKm, minimum: VolumeKm) -> VolumeKm {
    if value < minimum { minimum } else { value }
}

/// The volume the peak week is allowed to reach.
#[must_use]
pub fn ceiling_volume(
    athlete: &AthleteSnapshot,
    goal: GoalKind,
    constraints: &PlanConstraints,
) -> VolumeKm {
    let base = athlete
        .peak_weekly_volume
        .as_f64()
        .max(athlete.current_weekly_volume.as_f64());
    let mut ceiling = base * goal.peak_volume_multiplier();
    if let Some(max) = constraints.max_weekly_volume {
        ceiling = ceiling.min(max.as_f64());
    }
    VolumeKm(ceiling)
}

/// Where the curve wants to be at each week, before phase multipliers.
///
/// The ceiling is deliberately *not* applied here: clipping happens in the
/// emission pass, which is what records that a constraint bound the plan.
/// Clipping twice would hide that from the UI.
fn desired_volumes(
    athlete: &AthleteSnapshot,
    goal: GoalKind,
    start: VolumeKm,
    step: f64,
    phases: &[Phase],
) -> Vec<f64> {
    let mut desired: Vec<f64> = Vec::with_capacity(phases.len());
    let mut prev = start.as_f64();
    for phase in phases {
        prev = match phase {
            // Base consolidates rather than grows: tissue adapts before load.
            Phase::Base => prev * (1.0 + step * 0.4),
            Phase::Build | Phase::Peak => prev * (1.0 + step),
            Phase::Hold => prev,
            // Taper, race and rebuild weeks follow the curve; the phase factor
            // does the reducing.
            Phase::Taper | Phase::Race | Phase::Rebuild => prev,
        };
        desired.push(prev);
    }

    // A recovery block's shape is "down, then back toward the base" and is
    // expressed directly. Multiplying a deloaded start by the Taper and Rebuild
    // factors would reduce it twice and produce a block that never recovers,
    // which is the opposite of the goal.
    if goal == GoalKind::Recovery {
        let base = athlete.current_weekly_volume.as_f64().max(10.0);
        let floor = base * 0.6;
        let last = phases.len() - 1;
        for (idx, slot) in desired.iter_mut().enumerate() {
            let progress = if last == 0 {
                1.0
            } else {
                idx as f64 / last as f64
            };
            *slot = floor + (base * 0.95 - floor) * progress;
        }
    }

    desired
}

/// Build the weekly volume curve.
///
/// `phases` is one phase per week index, as produced by expanding
/// [`phase_spans`]. Volumes are rounded to whole kilometres because those are
/// the numbers a human reads on a watch — but the rounding is applied *after*
/// the step and ceiling clamps and always rounds down on a growing week, so
/// rounding can never push the emitted week over the permitted step.
#[must_use]
pub fn volume_curve(
    athlete: &AthleteSnapshot,
    goal: GoalKind,
    constraints: &PlanConstraints,
    phases: &[Phase],
) -> Vec<WeekVolume> {
    if phases.is_empty() {
        return Vec::new();
    }
    let ceiling = ceiling_volume(athlete, goal, constraints);
    let step = athlete.effective_weekly_step();

    // The first week's growth factor, needed to evaluate the acute:chronic guard
    // before the curve exists.
    let first_factor = match phases[0] {
        Phase::Base => 1.0 + step * 0.4,
        Phase::Build | Phase::Peak => 1.0 + step,
        _ => 1.0,
    };
    let (start, acwr_throttled) = if goal == GoalKind::Recovery {
        // A recovery block deliberately starts below the base, so the guard
        // cannot bind and must not raise it.
        (starting_volume(athlete, goal), false)
    } else {
        throttled_start(athlete, goal, first_factor)
    };

    let desired = desired_volumes(athlete, goal, start, step, phases);

    // A taper has to keep falling. A flat 65 % for three weeks leaves the
    // athlete fresh but not sharp, and the final week before a race should be
    // the lightest training week of the block. So the taper factor is graded by
    // how far the week sits from the race rather than taken from `Phase` alone.
    let race_index = phases.iter().position(|p| *p == Phase::Race);
    let taper_factor = |idx: usize, phase: Phase| -> f64 {
        if phase != Phase::Taper {
            return phase.volume_factor();
        }
        // Weeks remaining before race week; 1 means "the week of the race is next".
        let weeks_out = race_index.map_or(1, |r| r.saturating_sub(idx).saturating_sub(1));
        match weeks_out {
            0 => 0.50,
            1 => 0.65,
            _ => 0.80,
        }
    };

    // The race week's target is not a training target: the race distance is
    // fixed, so the week is the race plus a shakeout. Deriving it from the
    // volume curve would produce a week whose single session is longer than the
    // week it belongs to.
    let race_week_target = goal.race_distance_km().map(|km| km + 6.0);

    let mut out: Vec<WeekVolume> = Vec::with_capacity(phases.len());
    // The athlete's trailing four weeks anchor the chronic load. Without them a
    // detrained athlete prescribed 48 km in week 1 would project an ACWR near 1
    // — the plan would look safe because the projection had no memory of the 20
    // km they were actually running last month.
    let mut chronic: Vec<f64> = vec![athlete.current_weekly_volume.as_f64(); 4];

    for (idx, phase) in phases.iter().enumerate() {
        // Every fourth build week is a deload, unless the plan is too short for
        // the pattern to mean anything.
        let is_deload = matches!(phase, Phase::Base | Phase::Build)
            && phases.len() >= 5
            && idx > 0
            && idx % 4 == 3;

        let previous = out.last().map_or(start, |w: &WeekVolume| w.target);
        let is_recovery = goal == GoalKind::Recovery;
        let factor = if is_recovery {
            1.0
        } else {
            taper_factor(idx, *phase)
        };
        let wanted = match (*phase, race_week_target) {
            (Phase::Race, Some(km)) if !is_recovery => km,
            _ => desired[idx] * factor,
        };
        let clipped = wanted > ceiling.as_f64();

        let target = if is_deload {
            VolumeKm(wanted.min(ceiling.as_f64()) * 0.7)
        } else {
            // Clamp against the volume actually emitted last week, not the
            // model's intent, so the athlete never sees a step above the
            // permitted one. The race week is exempt: a marathon is 42.2 km
            // whatever the athlete ran the week before.
            let stepped = if previous.as_f64() > 0.0 && *phase != Phase::Race {
                wanted
                    .min(ceiling.as_f64())
                    .min(previous.as_f64() * (1.0 + step))
            } else {
                wanted.min(ceiling.as_f64())
            };
            // A taper or race week legitimately steps *down* hard, so only the
            // upward step is clamped.
            VolumeKm(stepped.max(previous.as_f64() * 0.5))
        };
        // Round down on a growing week so rounding cannot undo the step clamp.
        let target = if target.as_f64() >= previous.as_f64() {
            VolumeKm(target.as_f64().floor().max(0.0))
        } else {
            target.rounded_to(1.0)
        };

        let step_pct = if previous.as_f64() > 0.0 {
            target.as_f64() / previous.as_f64() - 1.0
        } else {
            0.0
        };

        chronic.push(target.as_f64());
        let projected_acwr = projected_acwr(&chronic);

        out.push(WeekVolume {
            phase: *phase,
            target,
            previous,
            step_pct,
            projected_acwr,
            is_deload,
            clipped,
            acwr_throttled: acwr_throttled && idx == 0,
        });
    }
    out
}

/// Projected acute:chronic ratio after the weeks supplied.
///
/// Weekly volume is treated as a daily load divided by 7, which is the same
/// normalisation the scoring crate applies to realised activity load, so the
/// plan's projection and the dashboard's measurement are directly comparable.
#[must_use]
pub fn projected_acwr(weekly_volumes: &[f64]) -> f64 {
    if weekly_volumes.is_empty() {
        return 1.0;
    }
    let daily: Vec<f64> = weekly_volumes
        .iter()
        .flat_map(|v| std::iter::repeat_n(*v / 7.0, 7))
        .collect();
    let acute = ema(&daily, ACUTE_DAYS);
    let chronic = ema(&daily, CHRONIC_DAYS);
    if chronic <= 0.0 { 1.0 } else { acute / chronic }
}

/// Exponential moving average over a window in days.
#[must_use]
pub fn ema(values: &[f64], window_days: f64) -> f64 {
    if values.is_empty() || window_days <= 0.0 {
        return 0.0;
    }
    let alpha = 2.0 / (window_days + 1.0);
    let mut acc = values[0];
    for v in &values[1..] {
        acc += alpha * (v - acc);
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;
    use runalytics_core::Tz;

    fn athlete() -> AthleteSnapshot {
        AthleteSnapshot::placeholder("Europe/Madrid".parse::<Tz>().expect("valid tz"))
    }

    fn constraints() -> PlanConstraints {
        PlanConstraints::default()
    }

    #[test]
    fn a_marathon_block_lays_out_base_build_peak_taper_race() {
        let spans = phase_spans(GoalKind::Marathon, 10);
        let phases: Vec<Phase> = spans.iter().map(|s| s.0).collect();
        assert_eq!(
            phases,
            vec![
                Phase::Base,
                Phase::Build,
                Phase::Peak,
                Phase::Taper,
                Phase::Race
            ]
        );
        // Marathon tapers for 21 days: three weeks including nothing else.
        let taper = spans.iter().find(|s| s.0 == Phase::Taper).expect("taper");
        assert_eq!(taper.2 - taper.1, 2, "three taper weeks");
        let race = spans.iter().find(|s| s.0 == Phase::Race).expect("race");
        assert_eq!(race.1, 9, "race lands on the final week");
    }

    #[test]
    fn a_5k_tapers_for_one_week_only() {
        let spans = phase_spans(GoalKind::FiveK, 6);
        let taper = spans.iter().find(|s| s.0 == Phase::Taper).expect("taper");
        assert_eq!(taper.2, taper.1, "single taper week");
    }

    #[test]
    fn maintain_holds_and_recovery_rebuilds() {
        assert_eq!(
            phase_spans(GoalKind::Maintain, 6),
            vec![(Phase::Hold, 0, 5)]
        );
        let recovery: Vec<Phase> = phase_spans(GoalKind::Recovery, 6)
            .iter()
            .map(|s| s.0)
            .collect();
        assert_eq!(recovery, vec![Phase::Taper, Phase::Rebuild]);
    }

    #[test]
    fn short_plans_stay_coherent() {
        for weeks in 1..=10 {
            for goal in [
                GoalKind::Marathon,
                GoalKind::HalfMarathon,
                GoalKind::TenK,
                GoalKind::FiveK,
                GoalKind::Recovery,
                GoalKind::Maintain,
            ] {
                let spans = phase_spans(goal, weeks);
                assert!(!spans.is_empty(), "{goal} at {weeks} weeks");
                assert_eq!(spans.first().expect("first").1, 0);
                assert_eq!(spans.last().expect("last").2, weeks - 1);
                // Spans must tile the plan with no gaps and no overlaps.
                for pair in spans.windows(2) {
                    assert_eq!(pair[0].2 + 1, pair[1].1, "{goal} at {weeks} weeks: gap");
                    assert!(pair[0].0 != pair[1].0, "adjacent spans share a phase");
                }
            }
        }
    }

    #[test]
    fn volume_rises_by_the_permitted_step_and_peaks_under_the_ceiling() {
        let a = athlete();
        let phases = vec![Phase::Build; 8];
        let curve = volume_curve(&a, GoalKind::Marathon, &constraints(), &phases);
        let step = a.effective_weekly_step();
        let ceiling = ceiling_volume(&a, GoalKind::Marathon, &constraints());
        for w in &curve {
            assert!(
                w.step_pct <= step + 0.01,
                "week stepped {} > permitted {step}",
                w.step_pct
            );
            assert!(w.target <= ceiling + VolumeKm(1.0));
        }
        assert!(curve.last().expect("last").target > curve[0].target);
    }

    #[test]
    fn a_constraint_ceiling_clips_and_reports_it() {
        let a = athlete();
        let mut c = constraints();
        c.max_weekly_volume = Some(VolumeKm(34.0));
        let curve = volume_curve(&a, GoalKind::Marathon, &c, &[Phase::Build; 8]);
        assert!(
            curve.iter().all(|w| w.target <= VolumeKm(35.0)),
            "constraint must bind: {:?}",
            curve.iter().map(|w| w.target.as_f64()).collect::<Vec<_>>()
        );
        assert!(
            curve.iter().any(|w| w.clipped),
            "the clip has to be visible to the UI"
        );
    }

    #[test]
    fn deload_weeks_dip_without_breaking_the_curve() {
        let a = athlete();
        let curve = volume_curve(&a, GoalKind::Marathon, &constraints(), &[Phase::Build; 8]);
        let deloads: Vec<usize> = curve
            .iter()
            .enumerate()
            .filter(|(_, w)| w.is_deload)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(deloads, vec![3, 7]);
        let w3 = &curve[3];
        assert!(w3.step_pct < -0.15, "deload week 3 should drop sharply");
        assert!(curve[4].step_pct > 0.0, "the curve resumes after a deload");
        // A later deload can land on a week the ceiling already flattened, so
        // all it guarantees is that a deload never adds volume.
        let w7 = &curve[7];
        assert!(w7.target <= w7.previous, "a deload never adds volume");
    }

    #[test]
    fn recovery_starts_below_the_current_base() {
        let a = athlete();
        let base = starting_volume(&a, GoalKind::Recovery);
        assert!(base < a.current_weekly_volume);
        assert!(starting_volume(&a, GoalKind::Marathon) >= a.current_weekly_volume);
    }

    #[test]
    fn a_detrained_athlete_starts_partway_back_toward_their_peak() {
        let mut a = athlete();
        a.current_weekly_volume = VolumeKm(20.0);
        a.peak_weekly_volume = VolumeKm(60.0);
        a.consistency = 0.33;
        let base = starting_volume(&a, GoalKind::HalfMarathon);
        assert!(
            base > a.current_weekly_volume,
            "rebuild starts above the floor"
        );
        assert!(base < a.peak_weekly_volume, "but not at the peak");
    }

    #[test]
    fn projected_acwr_stays_in_the_safe_band_for_a_steady_block() {
        let acwr = projected_acwr(&[40.0; 10]);
        assert!(
            (acwr - 1.0).abs() < 0.05,
            "flat load projects ~1.0, got {acwr}"
        );
    }

    #[test]
    fn projected_acwr_rises_with_a_ramp_and_falls_with_a_taper() {
        let ramp = projected_acwr(&[30.0, 33.0, 36.0, 40.0, 44.0, 48.0]);
        let flat = projected_acwr(&[40.0; 6]);
        assert!(ramp > flat, "ramping load raises ACWR");
        let tapered = projected_acwr(&[50.0, 50.0, 50.0, 35.0, 25.0]);
        assert!(tapered < flat, "cutting volume lowers ACWR");
    }

    #[test]
    fn ema_of_a_constant_series_is_that_constant() {
        assert!((ema(&[5.0; 50], 7.0) - 5.0).abs() < 1e-9);
        assert_eq!(ema(&[], 7.0), 0.0);
    }
}
