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
    let start = starting_volume(athlete, goal);
    let ceiling = ceiling_volume(athlete, goal, constraints);
    let step = athlete.effective_weekly_step();

    // Where the curve wants to be at each week, before phase multipliers.
    let mut desired: Vec<f64> = Vec::with_capacity(phases.len());
    let mut prev = start.as_f64();
    for phase in phases {
        prev = match phase {
            // Base consolidates rather than grows: tissue adapts before load.
            Phase::Base => prev * (1.0 + step * 0.4),
            Phase::Build | Phase::Peak => (prev * (1.0 + step)).min(ceiling.as_f64()),
            Phase::Hold => prev,
            // Taper, race and rebuild weeks follow the curve; the phase factor
            // does the reducing.
            Phase::Taper | Phase::Race | Phase::Rebuild => prev,
        };
        desired.push(prev);
    }

    let mut out: Vec<WeekVolume> = Vec::with_capacity(phases.len());
    let mut chronic: Vec<f64> = Vec::with_capacity(phases.len());

    for (idx, phase) in phases.iter().enumerate() {
        // Every fourth build week is a deload, unless the plan is too short for
        // the pattern to mean anything.
        let is_deload = matches!(phase, Phase::Base | Phase::Build)
            && phases.len() >= 5
            && idx > 0
            && idx % 4 == 3;

        let previous = out.last().map_or(start, |w: &WeekVolume| w.target);
        let wanted = desired[idx] * phase.volume_factor();
        let clipped = wanted > ceiling.as_f64();

        let target = if is_deload {
            VolumeKm(wanted.min(ceiling.as_f64()) * 0.7).rounded_to(1.0)
        } else {
            let capped = wanted.min(ceiling.as_f64());
            // Clamp against the volume actually emitted last week, not the
            // model's intent, so the athlete never sees a step above the
            // permitted one. Floor so rounding cannot undo the clamp.
            let stepped = if previous.as_f64() > 0.0 {
                capped.min(previous.as_f64() * (1.0 + step))
            } else {
                capped
            };
            VolumeKm(stepped.floor().max(0.0))
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
    if chronic <= 0.0 {
        1.0
    } else {
        acute / chronic
    }
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
            vec![Phase::Base, Phase::Build, Phase::Peak, Phase::Taper, Phase::Race]
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
        let curve = volume_curve(
            &a,
            GoalKind::Marathon,
            &constraints(),
            &[Phase::Build; 8],
        );
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
        assert!(base > a.current_weekly_volume, "rebuild starts above the floor");
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
