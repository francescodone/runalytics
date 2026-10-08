//! Turning a week's volume target into dated, structured sessions.
//!
//! Placement is deliberately conservative: the engine writes the plan a coach
//! would recognise (quality midweek, long run on the weekend, easy days around
//! the hard ones) and then lets the athlete's constraints veto specific days.
//! A vetoed day degrades to rest or an easy run rather than moving a hard
//! session onto a day the athlete has ruled out — moving it would be the
//! athlete's decision to make, not the engine's.

use chrono::Datelike;
use runalytics_core::{
    AthleteSnapshot, BlockTarget, Date, DurationSecs, GoalKind, HrZone, Pace, Phase,
    PlanConstraints, PlannedSession, PlannedSessionId, SESSION_NAMESPACE, SessionKind,
    StructuredWorkout, Uuid, VolumeKm, WorkoutBlock,
};

use crate::pace::PaceModel;
use crate::volume::WeekVolume;

/// How many runs a week contains at a given weekly volume.
///
/// Frequency follows volume, not experience: an athlete running 25 km a week
/// has 3 slots whether they have trained for a decade or a season, and
/// spreading the same volume over more slots produces sessions too short to
/// train anything.
#[must_use]
pub fn weekly_frequency(volume: VolumeKm, phase: Phase) -> usize {
    let base = match volume.as_f64() {
        v if v < 18.0 => 3,
        v if v < 32.0 => 4,
        v if v < 50.0 => 5,
        v if v < 70.0 => 6,
        _ => 7,
    };
    // Race week is short on purpose; rebuild weeks protect recovering tissue.
    match phase {
        Phase::Race => (base - 2).max(2),
        Phase::Rebuild => (base - 1).max(3),
        _ => base,
    }
}

/// Quality sessions for a week, after phase and injury adjustments.
#[must_use]
pub fn weekly_quality_count(
    athlete: &AthleteSnapshot,
    phase: Phase,
    constraints: &PlanConstraints,
    is_deload: bool,
) -> usize {
    if let Some(requested) = constraints.quality_sessions {
        return usize::from(requested);
    }
    let base = usize::from(athlete.experience.quality_sessions());
    let scaled = (base as f64 * phase.quality_factor()).round() as usize;
    let capped = match phase {
        // One quality day is enough to hold a detrained athlete's sharpness;
        // two would outpace the tissue's adaptation rate.
        Phase::Base => scaled.min(1),
        Phase::Taper => scaled.min(1),
        Phase::Race | Phase::Rebuild => 0,
        _ => scaled,
    };
    let capped = if athlete.injury_constrained() {
        capped.min(1)
    } else {
        capped
    };
    // A deload keeps the *kind* of training but not the amount of it.
    if is_deload { capped.min(1) } else { capped }
}

/// The quality kind to prescribe for a given week position.
///
/// Alternates the two quality slots so an athlete with two quality days gets
/// one threshold-flavoured and one shorter/faster session, which is what
/// actually develops both ends of the curve.
#[must_use]
pub fn quality_kind(
    phase: Phase,
    slot: usize,
    goal: GoalKind,
    weeks_to_race: usize,
) -> SessionKind {
    if phase == Phase::Race {
        return SessionKind::Race;
    }
    // Late in a taper, sharpen without accumulating fatigue: short reps, not
    // long threshold work.
    if phase == Phase::Taper {
        return if weeks_to_race <= 1 {
            SessionKind::Intervals
        } else {
            SessionKind::Fartlek
        };
    }
    match (phase, slot % 2) {
        (Phase::Base, _) => SessionKind::Fartlek,
        (Phase::Hold, 0) => SessionKind::Tempo,
        (Phase::Hold, _) => SessionKind::Fartlek,
        (_, 0) => match goal {
            GoalKind::Marathon | GoalKind::HalfMarathon => SessionKind::ExtensiveIntervals,
            GoalKind::TenK | GoalKind::FiveK => SessionKind::CruiseIntervals,
            GoalKind::Recovery | GoalKind::Maintain => SessionKind::Tempo,
        },
        (_, _) => match goal {
            GoalKind::Marathon | GoalKind::HalfMarathon => SessionKind::Tempo,
            GoalKind::TenK | GoalKind::FiveK => SessionKind::Intervals,
            GoalKind::Recovery | GoalKind::Maintain => SessionKind::Fartlek,
        },
    }
}

/// Pick the weekdays the week's sessions land on, Monday = 0.
///
/// Returns one slot per session in the week, ordered by weekday. The normal
/// shape puts quality midweek and the long run at the weekend. Race week fills
/// from the *end* of the week backwards, because the race has to be as late as
/// the athlete's constraints allow with the shakeout immediately before it.
///
/// `week_end` clamps the selection: the final week of a race-anchored plan is
/// truncated to race day, and scheduling a session after it would put training
/// after the start line.
#[must_use]
pub fn pick_weekdays(
    count: usize,
    phase: Phase,
    long_run_weekday: Option<u32>,
    constraints: &PlanConstraints,
    week_start: Date,
    week_end: Date,
) -> Vec<u32> {
    // Preferred shape: long run Sunday, quality Tuesday/Thursday, easy elsewhere.
    let preference: [u32; 7] = match phase {
        Phase::Race => [6, 5, 4, 3, 2, 1, 0],
        _ => [0, 2, 4, 6, 1, 3, 5],
    };
    let trainable = |day: u32| {
        let date = week_start + chrono::Duration::days(i64::from(day));
        date <= week_end && constraints.allows_training(date)
    };
    let mut chosen: Vec<u32> = Vec::with_capacity(count);

    if let Some(day) = long_run_weekday
        && phase != Phase::Race
        && trainable(day)
        && count > 0
    {
        chosen.push(day);
    }

    for day in preference {
        if chosen.len() >= count {
            break;
        }
        if chosen.contains(&day) || !trainable(day) {
            continue;
        }
        chosen.push(day);
    }

    // Fewer trainable days than slots: shrink the week rather than schedule on
    // a blackout day.
    chosen.sort_unstable();
    chosen
}

/// One week's worth of sessions.
///
/// The volume budget is allocated explicitly — long run first, then quality
/// days, then whatever is left split across the easy slots — so the emitted
/// sessions sum back to the week's target instead of drifting from it.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn build_week(
    plan_seed: &Uuid,
    week_index: u8,
    week: &WeekVolume,
    week_start: Date,
    week_end: Date,
    goal: GoalKind,
    weeks_to_race: usize,
    athlete: &AthleteSnapshot,
    constraints: &PlanConstraints,
    paces: &PaceModel,
) -> Vec<PlannedSession> {
    let slots = weekly_frequency(week.target, week.phase);
    let days = pick_weekdays(
        slots,
        week.phase,
        constraints.long_run_weekday,
        constraints,
        week_start,
        week_end,
    );
    if days.is_empty() {
        return Vec::new();
    }

    let quality_target = weekly_quality_count(athlete, week.phase, constraints, week.is_deload);

    // The long run is the week's anchor and is capped as a share of the week —
    // the single most common overuse trigger in runner data is a long run that
    // outgrows the week around it. It is placed before quality slots so those
    // can be chosen around it.
    // Race week puts the race on the last scheduled day; otherwise the athlete's
    // preferred long-run day wins, falling back to the last day of the week.
    let long_run_idx = if week.phase == Phase::Race {
        days.len() - 1
    } else {
        days.iter()
            .position(|d| Some(*d) == constraints.long_run_weekday)
            .unwrap_or(days.len() - 1)
    };
    let quality_slots =
        pick_quality_slots(&days, quality_target, long_run_idx, constraints, week_start);
    let long_run_volume = if week.phase == Phase::Race {
        goal.race_distance_km()
            .map_or_else(|| VolumeKm(week.target.as_f64() * 0.6), VolumeKm)
    } else {
        let share = athlete.experience.max_long_run_share().min(0.4);
        VolumeKm(week.target.as_f64() * share)
    }
    .rounded_to(0.5);

    let quality_volume = if quality_slots.is_empty() {
        VolumeKm::ZERO
    } else {
        VolumeKm(week.target.as_f64() * 0.18).rounded_to(0.5)
    };

    let easy_indices: Vec<usize> = (0..days.len())
        .filter(|i| *i != long_run_idx && !quality_slots.contains(i))
        .collect();
    let used = long_run_volume
        + VolumeKm(
            quality_volume.as_f64() * f64::from(u32::try_from(quality_slots.len()).unwrap_or(0)),
        );
    let easy_volume = if easy_indices.is_empty() {
        VolumeKm::ZERO
    } else {
        let rest = (week.target - used).as_f64();
        let per_slot = if rest <= 0.0 {
            // The anchors already consumed the week; keep the easy days as
            // genuine short runs rather than emitting zero-volume sessions.
            2.0
        } else {
            rest / f64::from(u32::try_from(easy_indices.len()).unwrap_or(1))
        };
        // An easy day must never be the longest session of the week. When the
        // leftover volume is large because the week is short on slots, the
        // answer is more easy days at a sane length, not one long easy day —
        // and the long run has already been sized against the week.
        VolumeKm(per_slot.min(long_run_volume.as_f64().max(2.0))).rounded_to(0.5)
    };

    let mut sessions = Vec::with_capacity(days.len());
    let mut quality_given = 0usize;
    for (idx, &weekday) in days.iter().enumerate() {
        let date = week_start + chrono::Duration::days(i64::from(weekday));
        let is_long = idx == long_run_idx;
        let is_quality = !is_long && quality_slots.contains(&idx);

        let slot_volume = if is_long {
            long_run_volume
        } else if is_quality {
            quality_volume
        } else {
            easy_volume
        };

        let kind = if week.phase == Phase::Race && is_long {
            SessionKind::Race
        } else if is_long {
            SessionKind::LongRun
        } else if is_quality {
            let kind = quality_kind(week.phase, quality_given, goal, weeks_to_race);
            quality_given += 1;
            kind
        } else if week.phase == Phase::Rebuild || week.is_deload {
            SessionKind::Recovery
        } else if week.phase == Phase::Base && idx == 0 {
            // One progression day per base week keeps neuromuscular sharpness
            // without adding real load while tissue is adapting.
            SessionKind::Progression
        } else {
            SessionKind::Easy
        };

        sessions.push(make_session(
            plan_seed,
            u32::from(week_index),
            date,
            kind,
            slot_volume,
            athlete,
            constraints,
            paces,
            goal,
            week.phase,
        ));
    }

    // Race week keeps only the shakeout and the race: the athlete has to arrive
    // fresh, and every extra easy day costs a little freshness.
    if week.phase == Phase::Race {
        sessions.retain(|s| matches!(s.kind, SessionKind::Race | SessionKind::Recovery));
    }
    sessions.sort_by_key(|s| s.date);
    sessions
}

/// Choose which slots host quality days, honouring no-quality days.
///
/// Slots are ranked by proximity to midweek — the conventional home for hard
/// sessions, and the furthest possible point from a weekend long run, so hard
/// days never stack against each other.
///
/// When no slot satisfies the athlete's quality constraints, the week degrades
/// to zero quality sessions rather than borrowing a day the athlete ruled out.
/// Silently scheduling a tempo run on a day someone has said they cannot do
/// intervals is the fastest way to lose their trust in the plan.
fn pick_quality_slots(
    days: &[u32],
    count: usize,
    long_run_idx: usize,
    constraints: &PlanConstraints,
    week_start: Date,
) -> Vec<usize> {
    if count == 0 {
        return Vec::new();
    }
    let mut ranked: Vec<(i32, usize)> = days
        .iter()
        .enumerate()
        .filter(|(i, d)| {
            let date = week_start + chrono::Duration::days(i64::from(**d));
            *i != long_run_idx && constraints.allows_quality(date)
        })
        .map(|(i, d)| (-(i32::try_from(*d).unwrap_or(0) - 2).abs(), i))
        .collect();
    ranked.sort_by_key(|(score, i)| (*score, *i));
    ranked.iter().take(count).map(|(_, i)| *i).collect()
}

/// Assemble one session with its structured workout, title and intent.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn make_session(
    plan_seed: &Uuid,
    week_index: u32,
    date: Date,
    kind: SessionKind,
    volume: VolumeKm,
    athlete: &AthleteSnapshot,
    constraints: &PlanConstraints,
    paces: &PaceModel,
    goal: GoalKind,
    phase: Phase,
) -> PlannedSession {
    let volume = if kind.counts_toward_volume() {
        volume
    } else {
        VolumeKm::ZERO
    };
    let hard_pace = paces.hard_pace(kind, goal);
    let workout = build_workout(kind, volume, athlete, paces, goal);
    let duration = workout.total_duration();
    let target_pace = match kind {
        SessionKind::Easy | SessionKind::LongRun | SessionKind::Progression => Some(paces.easy),
        SessionKind::Recovery => Some(paces.recovery),
        SessionKind::Race => Some(paces.for_goal(goal)),
        _ => Some(hard_pace),
    };

    PlannedSession {
        // Deterministic id: regenerating the same plan produces the same
        // session ids, so calendar UIDs and provider workout ids stay stable
        // across a re-generation that does not change the day.
        id: PlannedSessionId::from_external(
            &SESSION_NAMESPACE,
            &format!("{plan_seed}:{week_index}:{}", date.format("%Y-%m-%d")),
        ),
        date,
        start: constraints.preferred_start,
        kind,
        title: title_for(kind, &workout, paces, goal),
        intent: intent_for(kind, phase, athlete),
        workout,
        target_volume: volume.rounded_to(0.1),
        target_duration: duration,
        target_pace,
        rpe_target: Some(rpe_for(kind, phase)),
        quality: kind.is_quality(),
        external_id: None,
    }
}

/// A standard interval session: warm-up, `reps` hard efforts separated by
/// floats, cool-down. Every quality session except the fartlek is this shape.
fn repeats(
    reps: u32,
    rep: DurationSecs,
    float: DurationSecs,
    pace: Pace,
    zone: HrZone,
    athlete: &AthleteSnapshot,
    float_target: BlockTarget,
) -> StructuredWorkout {
    let ceiling = athlete.hr_zone_ceiling(zone);
    let mut middle = Vec::new();
    for i in 0..reps {
        middle.push(
            WorkoutBlock::new(BlockTarget::Hard, rep)
                .with_pace(pace)
                .with_hr_ceiling(ceiling, zone),
        );
        if i + 1 < reps {
            middle.push(WorkoutBlock::new(float_target, float));
        }
    }
    StructuredWorkout::with_warmup_cooldown(
        DurationSecs::from_minutes(15),
        middle,
        DurationSecs::from_minutes(10),
    )
}

/// The prescribed blocks for a session kind.
/// Steady-state sessions: everything that is not a repeat of hard efforts.
fn aerobic_workout(
    kind: SessionKind,
    total: DurationSecs,
    volume: VolumeKm,
    athlete: &AthleteSnapshot,
    paces: &PaceModel,
    goal: GoalKind,
) -> StructuredWorkout {
    match kind {
        SessionKind::Rest | SessionKind::CrossTraining => {
            StructuredWorkout::continuous(BlockTarget::Other, total)
        }
        SessionKind::Easy => StructuredWorkout::continuous(BlockTarget::Easy, total)
            .with_hr_ceiling_for(athlete, HrZone::Z2),
        SessionKind::Recovery => StructuredWorkout::continuous(BlockTarget::Recovery, total)
            .with_hr_ceiling_for(athlete, HrZone::Z1),
        SessionKind::LongRun => {
            // Long runs finish with the last 20% at goal pace once there is
            // enough of them to matter — the "progression" long run that
            // rehearses running fast on tired legs.
            let steady = if volume.as_f64() >= 18.0 {
                DurationSecs::new((f64::from(total.as_u32()) * 0.2) as u32)
            } else {
                DurationSecs::ZERO
            };
            let easy_part = total - steady;
            let mut blocks = vec![
                WorkoutBlock::new(BlockTarget::Easy, easy_part)
                    .with_hr_ceiling(athlete.hr_zone_ceiling(HrZone::Z2), HrZone::Z2),
            ];
            if steady.as_u32() > 0 {
                blocks.push(
                    WorkoutBlock::new(BlockTarget::Steady, steady)
                        .with_pace(paces.hard_pace(SessionKind::ExtensiveIntervals, goal)),
                );
            }
            StructuredWorkout { blocks }
        }
        SessionKind::Progression => {
            let strides = 6;
            let stride = DurationSecs::from_minutes(1);
            let float = DurationSecs::from_minutes(2);
            let mut blocks = vec![
                WorkoutBlock::new(BlockTarget::Easy, total)
                    .with_hr_ceiling(athlete.hr_zone_ceiling(HrZone::Z2), HrZone::Z2),
            ];
            for _ in 0..strides {
                blocks
                    .push(WorkoutBlock::new(BlockTarget::Strides, stride).with_pace(paces.five_k));
                blocks.push(WorkoutBlock::new(BlockTarget::Easy, float));
            }
            StructuredWorkout { blocks }
        }
        SessionKind::Race => StructuredWorkout {
            blocks: vec![
                WorkoutBlock::new(BlockTarget::Warmup, DurationSecs::from_minutes(15)),
                WorkoutBlock::new(BlockTarget::Hard, total).with_pace(paces.for_goal(goal)),
                WorkoutBlock::new(BlockTarget::Cooldown, DurationSecs::from_minutes(10)),
            ],
        },
        _ => unreachable!("quality repeats are built elsewhere"),
    }
}

fn build_workout(
    kind: SessionKind,
    volume: VolumeKm,
    athlete: &AthleteSnapshot,
    paces: &PaceModel,
    goal: GoalKind,
) -> StructuredWorkout {
    // Time on feet implied by the prescribed distance at the session's own pace.
    let reference = match kind {
        SessionKind::Recovery => paces.recovery,
        SessionKind::Race => paces.for_goal(goal),
        _ => paces.easy,
    };
    let total = DurationSecs::new((volume.as_f64() * reference.as_secs_per_km()) as u32);

    match kind {
        SessionKind::Tempo => {
            let steady = DurationSecs::new(
                (f64::from(total.as_u32()) * 0.55)
                    .min(f64::from(DurationSecs::from_minutes(45).as_u32())) as u32,
            );
            StructuredWorkout::with_warmup_cooldown(
                DurationSecs::from_minutes(15),
                vec![
                    WorkoutBlock::new(BlockTarget::Steady, steady)
                        .with_pace(paces.tempo)
                        .with_hr_ceiling(athlete.hr_zone_ceiling(HrZone::Z3), HrZone::Z3),
                ],
                DurationSecs::from_minutes(10),
            )
        }
        SessionKind::CruiseIntervals => repeats(
            4,
            DurationSecs::from_minutes(8),
            DurationSecs::from_minutes(2),
            paces.threshold,
            HrZone::Z4,
            athlete,
            BlockTarget::Recovery,
        ),
        SessionKind::Intervals => repeats(
            6,
            DurationSecs::from_minutes(3),
            DurationSecs::from_minutes(2),
            paces.interval,
            HrZone::Z5,
            athlete,
            BlockTarget::Recovery,
        ),
        SessionKind::ExtensiveIntervals => repeats(
            3,
            DurationSecs::from_minutes(12),
            DurationSecs::from_minutes(3),
            paces.hard_pace(SessionKind::ExtensiveIntervals, goal),
            HrZone::Z3,
            athlete,
            BlockTarget::Easy,
        ),
        SessionKind::Fartlek => {
            // Unstructured by design: reps vary so the athlete learns to run by
            // feel rather than by the watch.
            let pattern = [3u32, 2, 4, 2, 3];
            let mut middle = Vec::new();
            for (i, mins) in pattern.iter().enumerate() {
                middle.push(
                    WorkoutBlock::new(BlockTarget::Hard, DurationSecs::from_minutes(*mins))
                        .with_pace(paces.half),
                );
                if i + 1 < pattern.len() {
                    middle.push(WorkoutBlock::new(
                        BlockTarget::Recovery,
                        DurationSecs::from_minutes(2),
                    ));
                }
            }
            StructuredWorkout::with_warmup_cooldown(
                DurationSecs::from_minutes(15),
                middle,
                DurationSecs::from_minutes(10),
            )
        }
        _ => aerobic_workout(kind, total, volume, athlete, paces, goal),
    }
}

trait HrCeiling {
    fn with_hr_ceiling_for(self, athlete: &AthleteSnapshot, zone: HrZone) -> Self;
}
impl HrCeiling for StructuredWorkout {
    fn with_hr_ceiling_for(self, athlete: &AthleteSnapshot, zone: HrZone) -> Self {
        let ceiling = athlete.hr_zone_ceiling(zone);
        Self {
            blocks: self
                .blocks
                .into_iter()
                .map(|b| b.with_hr_ceiling(ceiling, zone))
                .collect(),
        }
    }
}

fn title_for(
    kind: SessionKind,
    workout: &StructuredWorkout,
    paces: &PaceModel,
    goal: GoalKind,
) -> String {
    let reps = workout
        .blocks
        .iter()
        .filter(|b| b.target.is_intensity())
        .count();
    let intensity = workout.intensity_duration().as_minutes();
    let hard = paces.hard_pace(kind, goal);
    match kind {
        SessionKind::Race => format!("Race — {}", goal.label()),
        SessionKind::LongRun if intensity > 0 => {
            format!("Long run + {intensity}' @ {}", hard.format())
        }
        SessionKind::Tempo => format!("Tempo {intensity}'"),
        SessionKind::CruiseIntervals => format!("{reps} x 8' @ {}", hard.format()),
        SessionKind::Intervals => format!("{reps} x 3' @ {}", hard.format()),
        SessionKind::ExtensiveIntervals => format!("{reps} x 12' @ {}", hard.format()),
        SessionKind::Fartlek => "Fartlek 3-2-4-2-3".into(),
        SessionKind::Progression => "Easy + 6 x 1' strides".into(),
        other => other.label().into(),
    }
}

fn intent_for(kind: SessionKind, phase: Phase, athlete: &AthleteSnapshot) -> String {
    let injured = athlete.injury_constrained();
    match kind {
        SessionKind::Easy if injured => "Aerobic volume at a conversational effort. Cap the heart rate — the point is tissue recovery, not fitness.".into(),
        SessionKind::Easy => "Aerobic volume at a conversational effort. If the last kilometres feel hard, slow down.".into(),
        SessionKind::Recovery => "Flush the previous session. Shorter and slower than it looks.".into(),
        SessionKind::LongRun => "The week's anchor. Endurance, fat oxidation and durability — the last kilometres are the training.".into(),
        SessionKind::Progression => "Neuromuscular sharpness without load. Strides should feel controlled, not all-out.".into(),
        SessionKind::Tempo => "Comfortably hard. Builds the ability to hold a comfortably hard effort — the single most transferable session for road racing.".into(),
        SessionKind::CruiseIntervals => "Threshold capacity. Reps should get faster, not slower; stop the session if they do.".into(),
        SessionKind::Intervals => "VO2max and economy. Finish each rep controlled.".into(),
        SessionKind::ExtensiveIntervals => "Race-pace rehearsal on tired legs. Practises the pace, the fueling and the self-talk.".into(),
        SessionKind::Fartlek => "Speed by feel. Run the hard efforts by effort, not by the watch.".into(),
        SessionKind::Race => "Everything up to today was the preparation. Warm up, start slower than you want to, and go to work.".into(),
        SessionKind::CrossTraining => "Aerobic volume without impact. Keep the heart rate in the same band as an easy run.".into(),
        SessionKind::Rest => match phase {
            Phase::Rebuild => "Full rest. The adaptation happens here.".into(),
            _ => "Full rest.".into(),
        },
    }
}

fn rpe_for(kind: SessionKind, phase: Phase) -> u8 {
    let base = match kind {
        SessionKind::Rest => 1,
        SessionKind::Recovery => 2,
        SessionKind::Easy | SessionKind::CrossTraining => 3,
        SessionKind::Progression | SessionKind::LongRun => 4,
        SessionKind::ExtensiveIntervals | SessionKind::Tempo => 6,
        SessionKind::CruiseIntervals | SessionKind::Fartlek => 7,
        SessionKind::Intervals => 8,
        SessionKind::Race => 10,
    };
    // A deload week asks for the same shapes at a lower effort.
    if phase == Phase::Taper && kind.is_quality() {
        base.max(5)
    } else {
        base
    }
}

/// Monday of the week containing `date`.
#[must_use]
pub fn week_start_of(date: Date) -> Date {
    let weekday = date.weekday().num_days_from_monday();
    date - chrono::Duration::days(i64::from(weekday))
}
