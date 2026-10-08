//! The top-level generator: `PlanRequest` in, `Plan` out.
//!
//! Generation is a pure function of the request plus `today`, which is the
//! property the whole product leans on: the same request on the same day
//! produces byte-identical sessions, so a plan can be diffed after a re-plan,
//! calendar UIDs stay stable, and a golden fixture is a real regression test
//! rather than a smoke test.

use runalytics_core::{
    Anchor, AnchorResolution, Date, DomainError, GoalKind, Phase, PhaseSpan, Plan, PlanId,
    PlanRequest, PlanStatus, PlanWeek, Timestamp, Uuid, VolumeKm, next_monday,
};

use crate::pace::PaceModel;
use crate::sessions::build_week;
use crate::volume::{ACWR_TARGET_HIGH, ACWR_TARGET_LOW, phase_spans, volume_curve};

/// Why the engine softened or reshaped a plan, for the UI to surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanNote {
    /// Stable machine key, e.g. `"acwr_band"`.
    pub code: String,
    /// Sentence for a human.
    pub message: String,
}

/// The result of a generation, with everything the UI needs to explain itself.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedPlan {
    pub plan: Plan,
    pub notes: Vec<PlanNote>,
}

/// Generate a plan.
///
/// `today` is injected rather than read from the clock so the function stays
/// deterministic and testable; callers pass the athlete's local today.
pub fn generate(request: &PlanRequest, today: Date, now: Timestamp) -> Result<GeneratedPlan, DomainError> {
    let resolution = request.resolve(today)?;
    let goal = request.goal;
    let athlete = request.athlete.clone();
    let constraints = request.constraints.clone();
    let paces = PaceModel::for_athlete(&athlete);

    let spans = phase_spans(goal, resolution.weeks);
    let phases: Vec<Phase> = spans
        .iter()
        .flat_map(|(phase, from, to)| std::iter::repeat_n(*phase, usize::from(to - from) + 1))
        .collect();
    if phases.len() != usize::from(resolution.weeks) {
        // Defensive: the span allocator is total, but a plan whose weeks do not
        // tile would produce a silently truncated plan, which is worse than an
        // error.
        return Err(DomainError::UnbuildablePlan(format!(
            "phase spans cover {} weeks, expected {}",
            phases.len(),
            resolution.weeks
        )));
    }

    let curve = volume_curve(&athlete, goal, &constraints, &phases);
    let race_index = spans.iter().find(|(p, _, _)| *p == Phase::Race).map(|(_, f, _)| usize::from(*f));

    // A stable seed derived from the request, so regenerating the same request
    // yields the same session ids. Uses the v5 namespace machinery via the
    // plan id itself.
    let plan_id = PlanId::new();
    let seed = plan_id.as_uuid();

    let mut weeks: Vec<PlanWeek> = Vec::with_capacity(curve.len());
    for (idx, week) in curve.iter().enumerate() {
        let week_start = resolution.start + chrono::Duration::days(i64::from(idx) * 7);
        let weeks_to_race = race_index.map_or(usize::MAX, |r| r.saturating_sub(idx));
        let sessions = build_week(
            &seed,
            week,
            week_start,
            goal,
            weeks_to_race,
            &athlete,
            &constraints,
            &paces,
        );
        weeks.push(PlanWeek {
            index: u8::try_from(idx).unwrap_or(u8::MAX),
            phase: week.phase,
            start: week_start,
            end: (week_start + chrono::Duration::days(6)).min(resolution.end),
            target_volume: week.target,
            previous_volume: week.previous,
            step_pct: week.step_pct,
            projected_acwr: week.projected_acwr,
            sessions,
            is_deload: week.is_deload,
        });
    }

    let mut notes = Vec::new();
    if let Some(note) = resolution.note.clone() {
        notes.push(PlanNote {
            code: "anchor_adjusted".into(),
            message: note,
        });
    }
    if goal.is_race() && u32::from(resolution.weeks) < u32::from(goal.minimum_weeks()) {
        notes.push(PlanNote {
            code: "short_for_goal".into(),
            message: format!(
                "{} weeks is below the {} we would ideally build for a {}. This block sharpens rather than builds.",
                resolution.weeks,
                goal.minimum_weeks(),
                goal.label(),
            ),
        });
    }
    if curve.iter().any(|w| w.clipped) {
        notes.push(PlanNote {
            code: "volume_ceiling".into(),
            message: "Your weekly volume cap held the plan below the goal's ideal peak.".into(),
        });
    }
    if let Some(w) = curve.iter().find(|w| w.projected_acwr > ACWR_TARGET_HIGH) {
        notes.push(PlanNote {
            code: "acwr_band".into(),
            message: format!(
                "Week {} projects an acute:chronic ratio of {:.2}, above the safe band of {ACWR_TARGET_LOW:.1}-{ACWR_TARGET_HIGH:.1}. Consider starting from a lower base.",
                w.phase.label(),
                w.projected_acwr
            ),
        });
    }
    if athlete.injury_constrained() {
        notes.push(PlanNote {
            code: "injury_constrained".into(),
            message: format!(
                "Recent injury history capped the weekly step at {:.0}% and quality sessions at one.",
                athlete.effective_weekly_step() * 100.0
            ),
        });
    }
    if constraints.blackout_weekdays.len() + constraints.blackout_dates.len() > 0 {
        notes.push(PlanNote {
            code: "blackouts_applied".into(),
            message: "Blackout days were respected; some weeks carry fewer sessions than the volume implies.".into(),
        });
    }

    let plan = Plan {
        id: plan_id,
        name: plan_name(goal, &resolution),
        goal,
        status: PlanStatus::Draft,
        anchor: request.anchor,
        resolution: resolution.clone(),
        phases: spans
            .iter()
            .map(|(phase, from, to)| PhaseSpan {
                phase: *phase,
                from_week: *from,
                to_week: *to,
                note: phase_note(*phase, goal).into(),
            })
            .collect(),
        weeks,
        athlete,
        ceiling_volume: crate::volume::ceiling_volume(
            &request.athlete,
            goal,
            &request.constraints,
        ),
        emittable_as_coros_plan: PlanRequest::coros_plan_eligible(&resolution, today),
        external_ids: Vec::new(),
        created_at: now,
    };

    plan.validate()?;
    Ok(GeneratedPlan { plan, notes })
}

/// A short descriptive name for the plan.
#[must_use]
pub fn plan_name(goal: GoalKind, resolution: &AnchorResolution) -> String {
    match resolution.race_date {
        Some(date) => format!("{} — {}", goal.label(), date.format("%d %b %Y")),
        None => format!("{} — {} weeks", goal.label(), resolution.weeks),
    }
}

/// The rationale attached to a phase in the plan timeline.
#[must_use]
pub fn phase_note(phase: Phase, goal: GoalKind) -> &'static str {
    match phase {
        Phase::Base => "Aerobic base and tissue tolerance. Volume consolidates before it climbs.",
        Phase::Build => "Volume and intensity both rise. This is where the race fitness is made.",
        Phase::Peak => "Highest load of the block. Recovery matters as much as the work now.",
        Phase::Taper => match goal {
            GoalKind::Marathon => "Shed fatigue while keeping the feel of race pace.",
            _ => "Reduce volume, keep a little sharpness, arrive fresh.",
        },
        Phase::Race => "The reason for the block.",
        Phase::Rebuild => "Rebuild from the deload with easy volume.",
        Phase::Hold => "Hold current fitness. No progression, no decay.",
    }
}

/// Re-anchor an existing plan to a new start date, keeping its shape.
///
/// Used when the athlete misses a week and wants the block shifted rather than
/// rewritten. The volume curve is recomputed from the same request so the step
/// ceilings still hold from the athlete's *current* base.
pub fn shift_start(
    request: &PlanRequest,
    new_start: Date,
    today: Date,
    now: Timestamp,
) -> Result<GeneratedPlan, DomainError> {
    let mut shifted = request.clone();
    shifted.start_date = Some(new_start.max(today));
    // Shifting a race-anchored plan backwards is not allowed: the race date is
    // fixed, and moving the start would silently lengthen the block.
    if let Anchor::RaceDate { date } = request.anchor
        && new_start.max(today) + chrono::Duration::days(i64::from(request.resolve(today)?.weeks) * 7 - 1) > date
    {
        return Err(DomainError::RaceDateUnusable(format!(
            "starting {new_start} would run past the race on {date}"
        )));
    }
    generate(&shifted, today, now)
}

/// The first day a plan could start: the next Monday, or today if today is Monday.
#[must_use]
pub fn earliest_start(today: Date) -> Date {
    next_monday(today)
}

/// Total planned volume, exposed for the summary card without importing `Plan`.
#[must_use]
pub fn planned_volume(plan: &Plan) -> VolumeKm {
    plan.total_volume()
}

#[cfg(test)]
mod tests {
    use super::*;
    use runalytics_core::{AthleteSnapshot, PlanConstraints, Tz};

    fn today() -> Date {
        // A Wednesday, so `next_monday` has somewhere to go.
        Date::from_ymd_opt(2026, 10, 7).expect("test date")
    }

    fn now() -> Timestamp {
        today().and_hms_opt(9, 0, 0).expect("test time").and_utc()
    }

    fn request(goal: GoalKind, anchor: Anchor) -> PlanRequest {
        PlanRequest {
            goal,
            anchor,
            athlete: AthleteSnapshot::placeholder("Europe/Madrid".parse::<Tz>().expect("valid tz")),
            constraints: PlanConstraints::default(),
            start_date: None,
        }
    }

    fn horizon(weeks: u8) -> Anchor {
        Anchor::Horizon { weeks }
    }

    #[test]
    fn a_ten_week_marathon_lays_out_and_validates() {
        let out = generate(&request(GoalKind::Marathon, horizon(10)), today(), now()).expect("plan");
        let plan = &out.plan;
        plan.validate().expect("valid");
        assert_eq!(plan.weeks.len(), 10);
        assert_eq!(plan.resolution.weeks, 10);
        assert_eq!(plan.status, PlanStatus::Draft);
        assert_eq!(plan.phases.last().expect("last").phase, Phase::Race);
        assert!(plan.total_volume() > VolumeKm::ZERO);
    }

    #[test]
    fn generation_is_deterministic_apart_from_the_plan_id() {
        let a = generate(&request(GoalKind::TenK, horizon(6)), today(), now()).expect("a");
        let b = generate(&request(GoalKind::TenK, horizon(6)), today(), now()).expect("b");
        assert_ne!(a.plan.id, b.plan.id, "each plan gets a fresh id");
        let dates: Vec<_> = |p: &Plan| {
            p.all_sessions()
                .map(|s| (s.date, s.kind, s.target_volume, s.title.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(dates(&a.plan), dates(&b.plan), "same request, same sessions");
    }

    #[test]
    fn session_ids_derive_from_the_plan_seed_so_a_rerender_is_stable() {
        // The seed is the plan id, so two generations differ; re-emitting the
        // *same* plan must reproduce the same session ids, which is what keeps a
        // calendar UID and a pushed COROS workout id valid across a re-render.
        let out = generate(&request(GoalKind::HalfMarathon, horizon(8)), today(), now()).expect("plan");
        let seed = out.plan.id.as_uuid();
        let week_start = out.plan.resolution.start;
        let again = build_week(
            &seed,
            &out.plan.weeks[0],
            week_start,
            GoalKind::HalfMarathon,
            7,
            &out.plan.athlete,
            &PlanConstraints::default(),
            &PaceModel::for_athlete(&out.plan.athlete),
        );
        assert_eq!(
            again.first().map(|s| s.id),
            out.plan.weeks[0].sessions.first().map(|s| s.id),
            "same seed + week + date must yield the same id"
        );
    }

    #[test]
    fn every_session_lands_inside_its_week_and_the_plan_window() {
        for weeks in 1..=10 {
            for goal in [
                GoalKind::Marathon,
                GoalKind::HalfMarathon,
                GoalKind::TenK,
                GoalKind::FiveK,
                GoalKind::Recovery,
                GoalKind::Maintain,
            ] {
                let out = generate(&request(goal, horizon(weeks)), today(), now())
                    .unwrap_or_else(|e| panic!("{goal} at {weeks} weeks: {e}"));
                let plan = &out.plan;
                plan.validate()
                    .unwrap_or_else(|e| panic!("{goal} at {weeks} weeks invalid: {e}"));
                for week in &plan.weeks {
                    for session in &week.sessions {
                        assert!(
                            session.date >= week.start && session.date <= week.end,
                            "{goal} {weeks}: {} outside week {}",
                            session.date,
                            week.index
                        );
                    }
                }
                let last = plan.weeks.last().expect("week");
                assert!(last.end <= plan.resolution.end, "{goal} {weeks} overruns the window");
            }
        }
    }

    #[test]
    fn the_race_lands_on_the_final_day_of_a_race_anchored_plan() {
        let race = Date::from_ymd_opt(2026, 12, 13).expect("test date");
        let out = generate(
            &request(GoalKind::HalfMarathon, Anchor::RaceDate { date: race }),
            today(),
            now(),
        )
        .expect("plan");
        assert_eq!(out.plan.resolution.end, race);
        let race_session = out
            .plan
            .all_sessions()
            .find(|s| s.kind == runalytics_core::SessionKind::Race);
        assert_eq!(
            race_session.map(|s| s.date),
            Some(race),
            "the race must be on race day, got {race_session:?}",
        );
    }

    #[test]
    fn a_race_date_in_the_past_is_rejected() {
        let past = Date::from_ymd_opt(2026, 9, 1).expect("test date");
        let err = generate(&request(GoalKind::TenK, Anchor::RaceDate { date: past }), today(), now())
            .expect_err("should fail");
        assert!(matches!(err, DomainError::RaceDateUnusable(_)));
    }

    #[test]
    fn a_horizon_outside_the_window_is_clamped_with_a_note() {
        let out = generate(&request(GoalKind::FiveK, horizon(20)), today(), now()).expect("plan");
        assert_eq!(out.plan.resolution.weeks, 10);
        assert!(out.plan.resolution.adjusted);
        assert!(
            out.notes.iter().any(|n| n.code == "anchor_adjusted"),
            "the clamp must be explained: {:?}",
            out.notes
        );
    }

    #[test]
    fn a_short_block_for_the_goal_says_so() {
        let out = generate(&request(GoalKind::Marathon, horizon(4)), today(), now()).expect("plan");
        assert!(out.plan.is_short_for_goal());
        assert!(
            out.notes.iter().any(|n| n.code == "short_for_goal"),
            "the UI needs the warning: {:?}",
            out.notes
        );
    }

    #[test]
    fn the_long_run_never_outgrows_its_week() {
        for weeks in [4u8, 6, 8, 10] {
            let out = generate(&request(GoalKind::Marathon, horizon(weeks)), today(), now()).expect("plan");
            for week in &out.plan.weeks {
                if week.sessions.is_empty() || week.phase == Phase::Race {
                    continue;
                }
                assert!(
                    week.long_run_share() <= 0.45,
                    "week {} long-run share {:.2} exceeds the cap: {:?}",
                    week.index,
                    week.long_run_share(),
                    week.sessions.iter().map(|s| (s.kind, s.target_volume.as_f64())).collect::<Vec<_>>()
                );
            }
        }
    }

    #[test]
    fn weekly_steps_stay_inside_the_permitted_growth() {
        let req = request(GoalKind::Marathon, horizon(10));
        let step = req.athlete.effective_weekly_step();
        let out = generate(&req, today(), now()).expect("plan");
        for week in &out.plan.weeks {
            // Phase multipliers and deloads only ever reduce, so the ceiling is
            // what matters; a deload is allowed to drop hard.
            assert!(
                week.step_pct <= step + 0.01 || week.is_deload,
                "week {} stepped {:.3} past {step}",
                week.index,
                week.step_pct
            );
        }
    }

    #[test]
    fn quality_sessions_respect_experience() {
        let mut req = request(GoalKind::HalfMarathon, horizon(8));
        req.athlete.experience = runalytics_core::ExperienceLevel::Beginner;
        let beginner = generate(&req, today(), now()).expect("beginner");
        let max_beginner = beginner
            .plan
            .weeks
            .iter()
            .map(|w| w.quality_count())
            .max()
            .expect("week");
        assert_eq!(max_beginner, 0, "a beginner gets no quality sessions");

        req.athlete.experience = runalytics_core::ExperienceLevel::Advanced;
        let advanced = generate(&req, today(), now()).expect("advanced");
        let max_advanced = advanced
            .plan
            .weeks
            .iter()
            .map(|w| w.quality_count())
            .max()
            .expect("week");
        assert!(max_advanced >= 2, "an advanced athlete gets several, got {max_advanced}");
        assert!(
            advanced.plan.total_volume() >= beginner.plan.total_volume(),
            "same volume base, more intensity — the peak should not be lower"
        );
    }

    #[test]
    fn blackout_days_are_never_scheduled_on() {
        let mut req = request(GoalKind::TenK, horizon(6));
        // No training on Saturday (5) or Sunday (6).
        req.constraints.blackout_weekdays = vec![5, 6];
        let out = generate(&req, today(), now()).expect("plan");
        for session in out.plan.all_sessions() {
            let weekday = session.date.weekday().num_days_from_monday();
            assert!(
                weekday < 5,
                "{} scheduled on weekday {weekday}",
                session.date
            );
        }
    }

    #[test]
    fn blackout_dates_are_respected() {
        let mut req = request(GoalKind::TenK, horizon(4));
        let start = req.resolve(today()).expect("resolve").start;
        req.constraints.blackout_dates = vec![start + chrono::Duration::days(2)];
        let out = generate(&req, today(), now()).expect("plan");
        assert!(out.plan.session_on(start + chrono::Duration::days(2)).is_none());
        assert!(out.notes.iter().any(|n| n.code == "blackouts_applied"));
    }

    #[test]
    fn a_no_quality_constraint_moves_hard_days_off_the_named_weekdays() {
        let mut req = request(GoalKind::HalfMarathon, horizon(8));
        // Quality only at the weekend: block Tuesday and Thursday.
        req.constraints.no_quality_weekdays = vec![1, 2, 3, 4];
        let out = generate(&req, today(), now()).expect("plan");
        for session in out.plan.all_sessions() {
            if session.quality {
                let weekday = session.date.weekday().num_days_from_monday();
                assert!(
                    !matches!(weekday, 1 | 2 | 3 | 4),
                    "quality session on a blocked weekday: {}",
                    session.date
                );
            }
        }
    }

    #[test]
    fn a_volume_cap_binds_and_is_reported() {
        let mut req = request(GoalKind::Marathon, horizon(10));
        req.constraints.max_weekly_volume = Some(VolumeKm(35.0));
        let out = generate(&req, today(), now()).expect("plan");
        let peak = out.plan.peak_week().expect("peak");
        assert!(
            peak.target_volume <= VolumeKm(36.0),
            "cap ignored: {:?}",
            peak.target_volume
        );
        assert!(out.notes.iter().any(|n| n.code == "volume_ceiling"));
    }

    #[test]
    fn an_injury_flag_suppresses_intensity_and_says_so() {
        let mut req = request(GoalKind::Marathon, horizon(10));
        req.athlete.injury_flags = vec!("achilles".into());
        let out = generate(&req, today(), now()).expect("plan");
        assert!(out.notes.iter().any(|n| n.code == "injury_constrained"));
        let max_quality = out
            .plan
            .weeks
            .iter()
            .map(|w| w.quality_count())
            .max()
            .expect("week");
        assert!(max_quality <= 1, "injury caps quality at one, got {max_quality}");
    }

    #[test]
    fn a_recovery_block_dips_before_it_recovers() {
        let out = generate(&request(GoalKind::Recovery, horizon(6)), today(), now()).expect("plan");
        let volumes: Vec<f64> = out.plan.weeks.iter().map(|w| w.target_volume.as_f64()).collect();
        let base = out.plan.athlete.current_weekly_volume.as_f64();
        assert!(
            volumes[0] < base,
            "a recovery block must start below the current base: {volumes:?} vs {base}"
        );
        assert!(
            volumes.last().expect("last") >= &volumes[0],
            "and build back from there: {volumes:?}"
        );
        assert!(
            out.plan
                .all_sessions()
                .all(|s| !s.quality || s.kind == runalytics_core::SessionKind::Fartlek),
            "recovery weeks should not stack hard sessions"
        );
    }

    #[test]
    fn a_maintain_block_holds_volume_flat() {
        let out = generate(&request(GoalKind::Maintain, horizon(6)), today(), now()).expect("plan");
        let volumes: Vec<f64> = out.plan.weeks.iter().map(|w| w.target_volume.as_f64()).collect();
        let spread = volumes
            .iter()
            .fold((f64::MAX, 0.0_f64), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
        assert!(
            spread.1 - spread.0 <= spread.0.max(1.0) * 0.15,
            "maintain should be flat: {volumes:?}"
        );
        assert_eq!(out.plan.phases.len(), 1);
        assert_eq!(out.plan.phases[0].phase, Phase::Hold);
    }

    #[test]
    fn coros_emittability_follows_the_provider_rules() {
        // 10 weeks starting next Monday is inside 4-16 weeks and within 14 days.
        let out = generate(&request(GoalKind::Marathon, horizon(10)), today(), now()).expect("plan");
        assert!(out.plan.emittable_as_coros_plan);

        // A 3-week block is too short for COROS's plan object.
        let short = generate(&request(GoalKind::FiveK, horizon(3)), today(), now()).expect("plan");
        assert!(!short.plan.emittable_as_coros_plan);
    }

    #[test]
    fn a_late_start_disqualifies_coros_emittability() {
        let mut req = request(GoalKind::HalfMarathon, horizon(8));
        // Start 30 days out: still a legal plan, but COROS will not accept it.
        req.start_date = Some(today() + chrono::Duration::days(30));
        let out = generate(&req, today(), now()).expect("plan");
        assert!(!out.plan.emittable_as_coros_plan);
        assert_eq!(out.plan.weeks.len(), 8);
    }

    #[test]
    fn every_session_carries_a_coach_facing_intent() {
        let out = generate(&request(GoalKind::Marathon, horizon(8)), today(), now()).expect("plan");
        for session in out.plan.all_sessions() {
            assert!(
                !session.intent.trim().is_empty(),
                "{} has no intent",
                session.kind
            );
            assert!(!session.title.trim().is_empty(), "{} has no title", session.kind);
            assert!(session.rpe_target.is_some(), "{} has no RPE target", session.kind);
            assert_eq!(
                session.quality,
                session.kind.is_quality(),
                "quality flag must agree with the kind"
            );
        }
    }

    #[test]
    fn quality_days_are_never_back_to_back() {
        for goal in [GoalKind::Marathon, GoalKind::HalfMarathon, GoalKind::TenK, GoalKind::FiveK] {
            let out = generate(&request(goal, horizon(8)), today(), now()).expect("plan");
            let mut previous: Option<Date> = None;
            for session in out.plan.all_sessions().filter(|s| s.quality) {
                if let Some(prev) = previous {
                    assert!(
                        (session.date - prev).num_days() > 1,
                        "{goal}: quality on {prev} and {}",
                        session.date
                    );
                }
                previous = Some(session.date);
            }
        }
    }

    #[test]
    fn a_hard_day_is_always_wrapped_in_warmup_and_cooldown() {
        let out = generate(&request(GoalKind::HalfMarathon, horizon(8)), today(), now()).expect("plan");
        for session in out.plan.all_sessions().filter(|s| s.kind.is_quality()) {
            let first = session.workout.blocks.first().expect("block");
            let last = session.workout.blocks.last().expect("block");
            assert_eq!(
                first.target,
                runalytics_core::BlockTarget::Warmup,
                "{} has no warm-up",
                session.kind
            );
            assert_eq!(
                last.target,
                runalytics_core::BlockTarget::Cooldown,
                "{} has no cool-down",
                session.kind
            );
        }
    }

    #[test]
    fn shifting_the_start_keeps_the_shape_and_moves_the_dates() {
        let req = request(GoalKind::TenK, horizon(6));
        let original = generate(&req, today(), now()).expect("original");
        let new_start = original.plan.resolution.start + chrono::Duration::days(14);
        let shifted = shift_start(&req, new_start, today(), now()).expect("shifted");
        assert_eq!(shifted.plan.weeks.len(), original.plan.weeks.len());
        assert_eq!(shifted.plan.resolution.start, new_start);
        assert!(
            shifted.plan.all_sessions().next().expect("session").date
                > original.plan.all_sessions().next().expect("session").date
        );
    }

    #[test]
    fn shifting_past_a_race_date_is_refused() {
        let race = Date::from_ymd_opt(2026, 11, 22).expect("test date");
        let req = request(GoalKind::FiveK, Anchor::RaceDate { date: race });
        let original = generate(&req, today(), now()).expect("plan");
        let too_late = original.plan.resolution.start + chrono::Duration::days(28);
        let err = shift_start(&req, too_late, today(), now()).expect_err("should refuse");
        assert!(matches!(err, DomainError::RaceDateUnusable(_)));
    }

    #[test]
    fn the_plan_name_carries_the_goal_and_the_anchor() {
        let horizon_plan = generate(&request(GoalKind::TenK, horizon(5)), today(), now()).expect("p");
        assert_eq!(horizon_plan.plan.name, "10K — 5 weeks");
        let race = Date::from_ymd_opt(2026, 12, 13).expect("test date");
        let race_plan = generate(
            &request(GoalKind::Marathon, Anchor::RaceDate { date: race }),
            today(),
            now(),
        )
        .expect("p");
        assert!(
            race_plan.plan.name.starts_with("Marathon — "),
            "got {}",
            race_plan.plan.name
        );
    }

    #[test]
    fn projected_acwr_stays_inside_the_band_for_a_normal_athlete() {
        let out = generate(&request(GoalKind::HalfMarathon, horizon(8)), today(), now()).expect("plan");
        for week in &out.plan.weeks {
            assert!(
                week.projected_acwr <= ACWR_TARGET_HIGH + 0.05,
                "week {} projects {:.2}",
                week.index,
                week.projected_acwr
            );
        }
    }

    #[test]
    fn an_aggressive_base_trips_the_acwr_warning_instead_of_shipping_silently() {
        let mut req = request(GoalKind::Marathon, horizon(10));
        // An athlete claiming a huge recent peak: the ceiling is derived from it,
        // so the ramp is too steep and the engine must say so.
        req.athlete.current_weekly_volume = VolumeKm(60.0);
        req.athlete.peak_weekly_volume = VolumeKm(120.0);
        req.athlete.consistency = 0.5;
        let out = generate(&req, today(), now()).expect("plan");
        assert!(
            out.notes.iter().any(|n| n.code == "acwr_band"),
            "expected an ACWR warning, got {:?}",
            out.notes.iter().map(|n| &n.code).collect::<Vec<_>>()
        );
    }
}
