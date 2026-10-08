//! Session quality: did the athlete execute the session, and did it do the job?
//!
//! Two questions get conflated in most training apps and must be kept apart
//! here, because they have different answers and different consequences:
//!
//! * **Adherence** — did what happened match what was prescribed? A 12 km long
//!   run done as an easy 12 km jog when the plan said 12 km with the last 4 km
//!   at marathon pace is a *fully executed distance* and a *missed session*.
//!   Only structure catches that, so structure is scored explicitly rather
//!   than inferred from totals.
//! * **Execution quality** — was the work actually good? This is where heart
//!   rate, decoupling, and the athlete's own RPE and feedback enter. A session
//!   can be perfectly adhered to and still be a bad session.
//!
//! The blend weights adherence more heavily on structured sessions and the
//! physiological signals more heavily on long runs, which is where the two
//! genuinely diverge. Every component is reported separately so the UI can say
//! *what* to fix — "you ran the distance but skipped the tempo" is actionable,
//! "you scored 61" is not.
//!
//! When data is missing, the component is dropped and the weights renormalise,
//! exactly as in [`crate::readiness`]. A session with no heart rate is not a
//! session with perfect heart rate.

use runalytics_core::{
    Activity, DurationSecs, HrZone, Pace, PlannedSession, SessionKind, StructuredWorkout, VolumeKm,
};

use crate::load::{intensity_factor, training_stress_score};

/// The athlete's subjective report for a completed session.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SessionFeedback {
    /// Perceived effort, 1-10.
    pub rpe: Option<u8>,
    /// How the session felt overall, 1-5. `None` when not reported.
    pub feeling: Option<u8>,
    /// Free-text note, kept for the coach view but never scored.
    pub note: Option<String>,
}

/// Relative band inside which a pace is counted as held.
///
/// Five percent of a 4:43/km tempo is about 14 s/km — the band inside which a
/// runner genuinely cannot tell they are off, and the band inside which being
/// off has no training effect worth naming.
pub const PACE_TOLERANCE: f64 = 0.05;

/// Relative band inside which the prescribed intensity *share* counts as held.
///
/// Tighter than the volume tolerance, because the shape of an interval or tempo
/// session is not a detail — it is the session. An athlete who runs the full
/// duration and distance but does half the prescribed hard work has done a
/// different workout, and a 25 % band would have called that a success.
pub const STRUCTURE_TOLERANCE: f64 = 0.15;

/// Relative band inside which the prescribed *in-zone share* counts as held.
///
/// Slightly looser than [`STRUCTURE_TOLERANCE`]: where the hard work happened is
/// judged on heart rate, and heart rate is the noisier of the two signals.
pub const ZONE_SHARE_TOLERANCE: f64 = 0.20;

/// How far outside a zone's edges a heart rate may sit and still count as being
/// in the zone, as a fraction of max heart rate.
///
/// Two percent is roughly four beats at a max of 180 — the noise floor of wrist
/// optical heart rate, and less than the drift caused by a hill, a red light, or
/// a hot afternoon. Zone boundaries are model artefacts, not walls: an athlete
/// who held 81% of max against an 80% ceiling did the session asked of them, and
/// marking them down for it would teach them to under-train the zone.
pub const ZONE_TOLERANCE: f64 = 0.02;

/// Every component that fed the quality score, each `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct QualityComponents {
    pub duration: Option<f64>,
    pub distance: Option<f64>,
    /// Did the prescribed structure survive? The most important component for
    /// interval and tempo sessions.
    pub structure: Option<f64>,
    /// Was the target pace held for the main set?
    pub pace: Option<f64>,
    /// Was the heart-rate work done in the right zone?
    pub time_in_zone: Option<f64>,
    /// Planned RPE versus reported RPE.
    pub effort: Option<f64>,
    /// The athlete's own feeling, 1-5 mapped to `0..=1`.
    pub subjective: Option<f64>,
    /// Multiplicative penalty for cardiac drift on long runs, `0.0..=1.0`.
    pub decoupling_penalty: f64,
}

/// A scored session.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionQuality {
    /// `0..=100`.
    pub score: f64,
    pub components: QualityComponents,
    /// `0.0..=1.0` — share of the weight set actually observed.
    pub confidence: f64,
    /// Planned TSS versus actual TSS.
    pub planned_tss: f64,
    pub actual_tss: f64,
    /// `actual / planned`, the single number most dashboards actually want.
    pub adherence: f64,
    /// Whether the session was meant to be hard and was actually hard.
    pub planned_quality: bool,
    pub executed_quality: bool,
}

/// Score how close an actual value is to a target.
///
/// `tolerance` is a *relative* band — `0.05` means five percent either way —
/// because the deviation is normalised by the target below. Callers comparing
/// seconds-per-kilometre must pass a fraction, not a number of seconds.
///
/// Ten percent is a reasonable default for session volumes: roughly the distance
/// a runner can misjudge on a watch, or the gap between a 40-minute and a
/// 44-minute session caused by a red light. Demanding exactness would score
/// honest execution as failure and train athletes to chase their watch instead
/// of the session.
#[must_use]
pub fn proximity(actual: f64, target: f64, tolerance: f64) -> f64 {
    if target <= 0.0 {
        return 1.0;
    }
    if actual <= 0.0 {
        return 0.0;
    }
    let deviation = (actual - target).abs() / target;
    if deviation <= tolerance {
        return 1.0;
    }
    // Linear decay to zero at three times the tolerance, so a session that is
    // 30% off scores nothing rather than a misleading partial credit.
    (1.0 - (deviation - tolerance) / (tolerance * 2.0)).clamp(0.0, 1.0)
}

/// Score how well the prescribed structure survived.
///
/// Compares the *shape* of what was planned against what was done, using the
/// intensity share: the fraction of total time spent in hard targets. A session
/// planned at 40 % intensity and executed at 5 % is a skipped session no matter
/// what the distance says, and this catches it.
///
/// Sessions with no meaningful prescribed intensity (rest, easy, recovery) score
/// on duration alone and return `None` so they do not dilute the blend.
#[must_use]
pub fn structure_adherence(
    planned: &StructuredWorkout,
    actual: &Activity,
    max_hr: runalytics_core::HeartRate,
) -> Option<f64> {
    let planned_total = planned.total_duration().as_u32();
    if planned_total == 0 {
        return None;
    }
    let planned_hard = planned.intensity_duration().as_u32();
    if planned_hard == 0 {
        return None;
    }
    let planned_share = f64::from(planned_hard) / f64::from(planned_total);

    // Estimate the actual intensity share from heart rate: time above the Z3
    // ceiling counts as hard. Without heart rate there is no honest way to
    // judge structure, so we abstain rather than guess.
    let actual_hard = hard_time_share(actual, max_hr)?;

    // Compare shares, not absolute times: an athlete who ran 10% longer but kept
    // the same proportions executed the session correctly.
    Some(proximity(actual_hard, planned_share, STRUCTURE_TOLERANCE))
}

/// Total lap duration whose average heart rate is at or above `floor_bpm`.
///
/// Returns `None` when there is no lap data, so callers can distinguish
/// "measured as zero" from "not measured".
fn lap_time_at_or_above(activity: &Activity, floor_bpm: f64) -> Option<u32> {
    if activity.laps.is_empty() {
        return None;
    }
    Some(
        activity
            .laps
            .iter()
            .filter(|l| {
                l.avg_hr
                    .is_some_and(|hr| f64::from(hr.as_u16()) >= floor_bpm)
            })
            .map(|l| l.duration.as_u32())
            .sum(),
    )
}

/// Fraction of an activity spent above the Z3 heart-rate ceiling.
///
/// `max_hr` must be the *athlete's* maximum heart rate, not the session's peak.
/// The distinction is the whole integrity of the measure: if the ceiling is
/// derived from the activity's own highest reading, the bar lowers by exactly as
/// much as the athlete eased off, and a session run entirely at 120 bpm is
/// scored as though every minute of it was hard. An athlete's max HR is a fixed
/// property of the athlete, so the same effort always scores the same.
///
/// Returns `None` when no lap carried heart rate. Uses lap data when present and
/// the summary average otherwise, which is a coarse fallback — a single average
/// cannot distinguish an even tempo run from a jog with one sprint.
#[must_use]
pub fn hard_time_share(activity: &Activity, max_hr: runalytics_core::HeartRate) -> Option<f64> {
    if !max_hr.is_recorded() {
        return None;
    }
    // Z3 ceiling in the five-zone model used across the domain.
    let ceiling = f64::from(max_hr.as_u16()) * HrZone::Z3.ceiling_pct();
    let total: u32 = activity.laps.iter().map(|l| l.duration.as_u32()).sum();
    if total > 0 {
        let hard: u32 = activity
            .laps
            .iter()
            .filter(|l| l.avg_hr.is_some_and(|hr| f64::from(hr.as_u16()) >= ceiling))
            .map(|l| l.duration.as_u32())
            .sum();
        return Some(f64::from(hard) / f64::from(total));
    }
    let avg = activity.summary.avg_hr?;
    Some(if f64::from(avg.as_u16()) >= ceiling {
        1.0
    } else {
        0.0
    })
}

/// The pace actually held during the *hard* laps of an activity.
///
/// Duration-weighted over the laps at or above the Z3 heart-rate ceiling, which
/// is the closest thing to "the main set" that lap data gives us.
///
/// This exists because a structured session's prescribed pace is a pace for its
/// main set, and a whole-session average is not that pace: a 55-minute interval
/// session with a 15-minute warm-up and a 10-minute cool-down averages nowhere
/// near the rep pace, so comparing the two would score a perfectly executed
/// session as a catastrophic failure of pace. Returns `None` when there is no lap
/// data or no lap was hard, which is the honest answer for an easy run.
#[must_use]
pub fn intensity_lap_pace(activity: &Activity, max_hr: runalytics_core::HeartRate) -> Option<Pace> {
    if activity.laps.is_empty() || !max_hr.is_recorded() {
        return None;
    }
    let ceiling = f64::from(max_hr.as_u16()) * HrZone::Z3.ceiling_pct();
    let mut seconds = 0.0;
    let mut km = 0.0;
    for lap in &activity.laps {
        if !lap
            .avg_hr
            .is_some_and(|hr| f64::from(hr.as_u16()) >= ceiling)
        {
            continue;
        }
        let duration = f64::from(lap.duration.as_u32());
        // Distance from the lap when recorded, otherwise inferred from its pace.
        let recorded = lap.distance.as_f64();
        let distance = if recorded > 0.0 {
            recorded
        } else {
            lap.avg_pace
                .map_or(0.0, |pace| duration / pace.as_secs_per_km())
        };
        if distance > 0.0 {
            seconds += duration;
            km += distance;
        }
    }
    if km <= 0.0 || seconds <= 0.0 {
        return None;
    }
    Some(Pace::new(seconds / km))
}

/// Score pace adherence for the main set.
///
/// When lap data identifies hard laps, their pace is compared against the
/// prescribed target — the target describes the main set, so the main set is what
/// must be measured. Without lap data the whole-session average is all there is,
/// and the score is honest about being coarser by reporting it anyway; the
/// session's `confidence` is lowered elsewhere by the missing structure signal.
///
/// Deliberately not a best-lap comparison: the plan prescribes the session, not a
/// highlight reel, and an athlete who spiked one kilometre to make a number look
/// right did not do the workout.
#[must_use]
pub fn pace_adherence(
    planned: &PlannedSession,
    actual: &Activity,
    max_hr: runalytics_core::HeartRate,
) -> Option<f64> {
    let target = planned.target_pace?;
    let held = intensity_lap_pace(actual, max_hr).or(actual.summary.avg_pace)?;
    Some(proximity(
        held.as_secs_per_km(),
        target.as_secs_per_km(),
        PACE_TOLERANCE,
    ))
}

/// Score whether enough of the session was done at the prescribed intensity.
///
/// Compares the *share of the session* the plan asked for at or above the
/// prescribed zone's floor against the share that actually landed there. This is
/// deliberately not a comparison of average heart rate against the zone: an
/// interval session's average is a meaningless statistic, because the warm-up and
/// the recovery floats pull it towards the middle whatever the reps did. An
/// athlete can run every rep in the wrong zone and still average their way into
/// the right one.
///
/// The measure is **one-sided**: exceeding the zone is not penalised here. On a
/// short interval the heart rate legitimately runs above the zone ceiling, and an
/// athlete who did the reps at the right pace but finished them at 92 % of max
/// did the session that was asked of them. Punishing that would teach athletes to
/// under-run their intervals to protect a number. Working too hard *is* visible —
/// in reported RPE, in cardiac decoupling, and in the load the session adds — so
/// nothing is lost by not double-counting it here. What this component catches,
/// and the only thing it should catch, is not doing enough work at the intended
/// intensity.
#[must_use]
pub fn zone_adherence(
    planned: &PlannedSession,
    actual: &Activity,
    max_hr: runalytics_core::HeartRate,
) -> Option<f64> {
    let target_zone = planned
        .workout
        .blocks
        .iter()
        .filter(|b| b.target.is_intensity())
        .find_map(|b| b.zone)?;
    if !max_hr.is_recorded() {
        return None;
    }
    let max = f64::from(max_hr.as_u16());
    // The floor widened by the heart-rate noise floor, so an athlete a few beats
    // short of the boundary is not marked down for a watch artefact.
    let floor = target_zone.floor_pct() * max - ZONE_TOLERANCE * max;

    // Same denominator as `structure_adherence`: both shares are expressed
    // against the prescribed session length, so they stay comparable.
    let planned_total = planned.workout.total_duration().as_u32();
    let planned_hard = planned.workout.intensity_duration().as_u32();
    if planned_total == 0 || planned_hard == 0 {
        return None;
    }
    let planned_share = f64::from(planned_hard) / f64::from(planned_total);

    let total: u32 = actual.laps.iter().map(|l| l.duration.as_u32()).sum();
    let actual_share = if total > 0 {
        let at_intensity = lap_time_at_or_above(actual, floor)?;
        f64::from(at_intensity) / f64::from(total)
    } else {
        // No lap data: the session average is all there is, and it cannot
        // distinguish an even zone run from a jog with one sprint. Confidence in
        // the score is lowered elsewhere by the missing structure signal.
        let avg = actual.summary.avg_hr?;
        if f64::from(avg.as_u16()) >= floor {
            planned_share
        } else {
            0.0
        }
    };

    // One-sided: only the shortfall counts, and it is measured relative to what
    // was asked for. Zero at three times the tolerance, so a session that
    // delivered a third of the prescribed intensity time scores nothing.
    let shortfall = (planned_share - actual_share).max(0.0) / planned_share;
    Some(
        (1.0 - (shortfall - ZONE_SHARE_TOLERANCE).max(0.0) / (ZONE_SHARE_TOLERANCE * 2.0))
            .clamp(0.0, 1.0),
    )
}

/// Score planned RPE against reported RPE.
///
/// A session that felt much harder than prescribed is informative in both
/// directions: either the athlete is fitter than the plan assumed (good) or
/// they are fatigued (bad), and the readiness score disambiguates. Scored on
/// absolute deviation because RPE is an ordinal scale — a one-point difference
/// means the same thing at 4 as at 8.
#[must_use]
pub fn effort_adherence(planned_rpe: Option<u8>, reported: Option<u8>) -> Option<f64> {
    let planned = f64::from(planned_rpe?);
    let actual = f64::from(reported?);
    let deviation = (actual - planned).abs();
    // One point of RPE is noise; three or more is a different session.
    Some((1.0 - (deviation - 1.0).max(0.0) / 2.0).clamp(0.0, 1.0))
}

/// Map the athlete's 1-5 feeling to `0..=1`.
#[must_use]
pub fn subjective_score(feeling: Option<u8>) -> Option<f64> {
    let f = f64::from(feeling?.clamp(1, 5));
    Some((f - 1.0) / 4.0)
}

/// Cardiac decoupling on a long run: how much heart rate drifted upward
/// relative to pace over the second half.
///
/// This is the single most useful quality signal on a long run and the reason
/// a "successful" long run can still be a bad one. An athlete who holds pace
/// while heart rate climbs 10 % across the second half is holding pace with
/// effort rather than fitness, and the aerobic system is not doing the work the
/// session was prescribed to do. Above ~5 % drift the session stops counting as
/// aerobic development.
///
/// Returns `None` unless the activity has enough lap data to split in half.
#[must_use]
pub fn decoupling(activity: &Activity) -> Option<f64> {
    if activity.laps.len() < 4 {
        return None;
    }
    let mid = activity.laps.len() / 2;
    let half_hr = |laps: &[runalytics_core::ActivityLap]| -> Option<f64> {
        let total: u32 = laps.iter().map(|l| l.duration.as_u32()).sum();
        if total == 0 {
            return None;
        }
        let weighted: f64 = laps
            .iter()
            .filter_map(|l| {
                l.avg_hr
                    .map(|hr| f64::from(hr.as_u16()) * f64::from(l.duration.as_u32()))
            })
            .sum();
        Some(weighted / f64::from(total))
    };
    let first = half_hr(&activity.laps[..mid])?;
    let second = half_hr(&activity.laps[mid..])?;
    if first <= 0.0 {
        return None;
    }
    Some((second - first) / first)
}

/// The penalty multiplier applied for decoupling, `0.0..=1.0`.
///
/// Only applied to long runs, where the aerobic purpose is the point. Up to 5 %
/// drift is normal and free; beyond that the score is cut, saturating at 15 %.
#[must_use]
pub fn decoupling_penalty(activity: &Activity, kind: SessionKind) -> f64 {
    if kind != SessionKind::LongRun {
        return 1.0;
    }
    match decoupling(activity) {
        None => 1.0,
        Some(drift) if drift <= 0.05 => 1.0,
        Some(drift) => (1.0 - (drift - 0.05) * 6.0).clamp(0.5, 1.0),
    }
}

/// Weights per component. They need not sum to 1 — the blend renormalises over
/// whatever is present.
const W_DURATION: f64 = 0.15;
const W_DISTANCE: f64 = 0.15;
const W_STRUCTURE: f64 = 0.25;
const W_PACE: f64 = 0.15;
const W_ZONE: f64 = 0.10;
const W_EFFORT: f64 = 0.10;
const W_SUBJECTIVE: f64 = 0.10;

impl QualityComponents {
    /// Present components with their weights.
    #[must_use]
    pub fn weighted(&self) -> Vec<(&'static str, f64, f64)> {
        let mut parts = Vec::new();
        if let Some(v) = self.duration {
            parts.push(("duration", W_DURATION, v));
        }
        if let Some(v) = self.distance {
            parts.push(("distance", W_DISTANCE, v));
        }
        if let Some(v) = self.structure {
            parts.push(("structure", W_STRUCTURE, v));
        }
        if let Some(v) = self.pace {
            parts.push(("pace", W_PACE, v));
        }
        if let Some(v) = self.time_in_zone {
            parts.push(("time_in_zone", W_ZONE, v));
        }
        if let Some(v) = self.effort {
            parts.push(("effort", W_EFFORT, v));
        }
        if let Some(v) = self.subjective {
            parts.push(("subjective", W_SUBJECTIVE, v));
        }
        parts
    }

    #[must_use]
    pub fn coverage(&self) -> f64 {
        self.weighted().iter().map(|(_, w, _)| w).sum()
    }
}

/// Combine components into `0..=100`, applying the decoupling penalty.
#[must_use]
pub fn quality_score(components: &QualityComponents) -> f64 {
    let parts = components.weighted();
    if parts.is_empty() {
        return 50.0;
    }
    let weight_sum: f64 = parts.iter().map(|(_, w, _)| w).sum();
    let weighted: f64 = parts.iter().map(|(_, w, v)| w * v).sum();
    (weighted / weight_sum * 100.0 * components.decoupling_penalty).clamp(0.0, 100.0)
}

/// Score a completed session against what was planned.
///
/// `threshold` is the athlete's current threshold pace, used for both the
/// planned and actual TSS so the two are directly comparable.
#[must_use]
pub fn score_session(
    planned: &PlannedSession,
    actual: &Activity,
    feedback: &SessionFeedback,
    athlete: &runalytics_core::AthleteSnapshot,
    threshold: Pace,
) -> SessionQuality {
    let components = QualityComponents {
        duration: (actual.summary.duration.as_u32() > 0).then(|| {
            proximity(
                f64::from(actual.summary.duration.as_u32()),
                f64::from(planned.target_duration.as_u32()),
                0.10,
            )
        }),
        distance: (actual.summary.distance.as_f64() > 0.0).then(|| {
            proximity(
                actual.summary.distance.as_f64(),
                planned.target_volume.as_f64(),
                0.10,
            )
        }),
        structure: structure_adherence(&planned.workout, actual, athlete.max_hr),
        pace: pace_adherence(planned, actual, athlete.max_hr),
        time_in_zone: zone_adherence(planned, actual, athlete.max_hr),
        effort: effort_adherence(planned.rpe_target, feedback.rpe),
        subjective: subjective_score(feedback.feeling),
        decoupling_penalty: decoupling_penalty(actual, planned.kind),
    };

    let planned_tss = planned_tss(planned, threshold);
    let actual_tss = training_stress_score(actual, athlete, threshold);
    let adherence = if planned_tss > 0.0 {
        (actual_tss / planned_tss).clamp(0.0, 2.0)
    } else {
        1.0
    };

    // "Executed quality" is decided from the objective signals only. The
    // athlete's own feeling must not be able to certify a hard session, or the
    // metric becomes self-report and stops meaning anything.
    let executed_quality = executed_quality(planned, actual, athlete, threshold);

    SessionQuality {
        score: quality_score(&components),
        components,
        confidence: components.coverage(),
        planned_tss,
        actual_tss,
        adherence,
        planned_quality: planned.quality,
        executed_quality,
    }
}

/// The TSS the plan was asking for.
///
/// Uses the prescribed duration at the prescribed pace where one exists, and
/// falls back to the session's intensity label otherwise. This is an estimate
/// of intent, not a measurement, and is labelled as such.
#[must_use]
pub fn planned_tss(planned: &PlannedSession, threshold: Pace) -> f64 {
    let hours = f64::from(planned.target_duration.as_u32()) / 3600.0;
    let intensity = match planned.target_pace {
        Some(pace) => intensity_from_pace_for(pace, threshold),
        None => planned_kind_factor(planned.kind),
    };
    // A rest day asks for nothing; scoring it against zero would divide.
    if planned.kind == SessionKind::Rest || hours <= 0.0 {
        return 0.0;
    }
    100.0 * intensity * intensity * hours
}

fn intensity_from_pace_for(pace: Pace, threshold: Pace) -> f64 {
    if pace.as_secs_per_km() <= 0.0 {
        return 0.0;
    }
    (threshold.as_secs_per_km() / pace.as_secs_per_km()).clamp(0.0, 1.2)
}

fn planned_kind_factor(kind: SessionKind) -> f64 {
    match kind {
        SessionKind::Recovery => 0.55,
        SessionKind::Easy => 0.7,
        SessionKind::LongRun => 0.75,
        SessionKind::Tempo | SessionKind::Progression => 0.88,
        SessionKind::CruiseIntervals | SessionKind::ExtensiveIntervals => 0.95,
        SessionKind::Intervals | SessionKind::Fartlek => 1.05,
        SessionKind::Race => 1.0,
        SessionKind::Rest | SessionKind::CrossTraining => 0.0,
    }
}

/// The smallest share of a session that must be at intensity before its hard-lap
/// pace is allowed to describe the session.
///
/// Without this, one 400 m surge inside an easy hour would produce a hard-lap
/// pace at threshold and certify the whole run as quality work — the precise
/// failure mode this whole module exists to prevent, reintroduced through the
/// back door. A session with less than a sixth of its time at intensity was not
/// an intensity session, whatever one lap says.
pub const MIN_INTENSITY_SHARE_FOR_PEAK: f64 = 0.15;

/// The intensity factor actually reached during the hard part of a session.
///
/// The whole-session average is the wrong statistic for a structured session:
/// warm-up and recovery floats pull it down, so an interval session in which
/// every rep was run at threshold reads as an aerobic run. That mislabels exactly
/// the sessions an athlete most wants credit for, and it makes the "did you do
/// quality work" flag useless on interval days.
///
/// Falls back to the whole-session factor when there is no lap data, or when too
/// little of the session was at intensity for the hard-lap pace to describe it.
#[must_use]
pub fn peak_intensity_factor(
    actual: &Activity,
    athlete: &runalytics_core::AthleteSnapshot,
    threshold: Pace,
) -> f64 {
    let whole = intensity_factor(actual, athlete, threshold);
    let Some(pace) = intensity_lap_pace(actual, athlete.max_hr) else {
        return whole;
    };
    let Some(share) = hard_time_share(actual, athlete.max_hr) else {
        return whole;
    };
    if share < MIN_INTENSITY_SHARE_FOR_PEAK {
        return whole;
    }
    // The hard-lap pace is the best available estimate of what was actually
    // asked of the athlete. Take the higher of the two, consistent with
    // `intensity_factor`.
    intensity_from_pace_for(pace, threshold).max(whole)
}

/// Whether the session actually stressed the athlete.
///
/// Judged on measured intensity, not on what the plan called it. An "easy run"
/// the athlete turned into a tempo effort counts as quality, and that mismatch is
/// exactly the behaviour a coach needs to see.
#[must_use]
pub fn executed_quality(
    planned: &PlannedSession,
    actual: &Activity,
    athlete: &runalytics_core::AthleteSnapshot,
    threshold: Pace,
) -> bool {
    if planned.kind == SessionKind::Rest {
        return false;
    }
    let if_ = peak_intensity_factor(actual, athlete, threshold);
    // 0.88 is the conventional boundary between aerobic and threshold work.
    if_ >= 0.88
}

/// The load actually recorded for a session, for the store's `load` column.
#[must_use]
pub fn executed_load(
    actual: &Activity,
    athlete: &runalytics_core::AthleteSnapshot,
    threshold: Pace,
) -> f64 {
    training_stress_score(actual, athlete, threshold)
}

/// Duration actually recorded, for the store's `duration_s` column.
#[must_use]
pub fn executed_duration(actual: &Activity) -> DurationSecs {
    actual.summary.duration
}

/// Volume actually recorded.
#[must_use]
pub fn executed_volume(actual: &Activity) -> VolumeKm {
    actual.summary.distance
}

/// A short verdict for the UI, derived from the weakest component.
///
/// The most useful sentence about a session is the one that says what went
/// wrong, so this names the lowest-scoring component rather than restating the
/// number.
#[must_use]
pub fn verdict(components: &QualityComponents) -> &'static str {
    let parts = components.weighted();
    let weakest = parts
        .iter()
        .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
    let Some((code, _, value)) = weakest else {
        return "no data recorded";
    };
    if *value >= 0.9 {
        return "executed as prescribed";
    }
    match *code {
        "duration" if *value < 0.6 => "much shorter than planned",
        "distance" if *value < 0.6 => "much less distance than planned",
        "structure" if *value < 0.6 => "the main set was not done",
        "pace" if *value < 0.6 => "target pace was not held",
        "time_in_zone" if *value < 0.6 => "heart rate missed the intended zone",
        "effort" if *value < 0.6 => "it felt very different from the plan",
        "subjective" if *value < 0.6 => "it did not feel good",
        _ => "close to plan, with gaps",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runalytics_core::{
        ActivityLap, ActivitySummary, AthleteSnapshot, BlockTarget, HeartRate, Intensity,
        Timestamp, Tz, WorkoutBlock,
    };

    fn athlete() -> AthleteSnapshot {
        AthleteSnapshot::placeholder("Europe/Madrid".parse::<Tz>().expect("valid tz"))
    }

    fn threshold() -> Pace {
        Pace::new(270.0)
    }

    fn date() -> runalytics_core::Date {
        runalytics_core::Date::from_ymd_opt(2026, 10, 5).expect("date")
    }

    fn lap(minutes: u32, hr: u16, km: f64) -> ActivityLap {
        ActivityLap {
            index: 1,
            start: Timestamp::default(),
            duration: DurationSecs::from_minutes(minutes),
            distance: VolumeKm(km),
            avg_pace: None,
            avg_hr: Some(HeartRate::new(hr)),
            max_hr: None,
            elevation_gain: None,
            cadence: None,
        }
    }

    fn activity(minutes: u32, km: f64, avg_pace: f64, avg_hr: u16, max_hr: u16) -> Activity {
        Activity {
            id: runalytics_core::ActivityId::new(),
            account: runalytics_core::ProviderAccountId::new(),
            provider_activity_id: "a".into(),
            name: "run".into(),
            started_at: Timestamp::default(),
            local_date: date(),
            summary: ActivitySummary {
                distance: VolumeKm(km),
                duration: DurationSecs::from_minutes(minutes),
                avg_pace: (avg_pace > 0.0).then(|| Pace::new(avg_pace)),
                avg_hr: (avg_hr > 0).then(|| HeartRate::new(avg_hr)),
                max_hr: (max_hr > 0).then(|| HeartRate::new(max_hr)),
                elevation_gain: None,
                avg_cadence: None,
                training_load: None,
            },
            laps: vec![],
            intensity: Intensity::Aerobic,
            matched_session: None,
            fetched_at: Timestamp::default(),
        }
    }

    fn tempo_session() -> PlannedSession {
        PlannedSession {
            id: runalytics_core::PlannedSessionId::new(),
            date: date(),
            start: runalytics_core::TimeOfDay::from_hms_opt(7, 0, 0).expect("time"),
            kind: SessionKind::Tempo,
            title: "Tempo".into(),
            intent: "Steady".into(),
            workout: StructuredWorkout {
                blocks: vec![
                    WorkoutBlock::new(BlockTarget::Warmup, DurationSecs::from_minutes(15)),
                    WorkoutBlock::new(BlockTarget::Steady, DurationSecs::from_minutes(30))
                        .with_pace(Pace::new(283.5))
                        .with_hr_ceiling(HeartRate::new(160), HrZone::Z3),
                    WorkoutBlock::new(BlockTarget::Cooldown, DurationSecs::from_minutes(10)),
                ],
            },
            target_volume: VolumeKm(12.0),
            target_duration: DurationSecs::from_minutes(55),
            target_pace: Some(Pace::new(283.5)),
            rpe_target: Some(7),
            quality: true,
            external_id: None,
        }
    }

    /// A long run: no prescribed pace and no intensity block, so pace, structure
    /// and zone all abstain and the only thing that can move the score is the
    /// decoupling penalty. That isolation is what makes it useful for testing.
    fn long_run_session() -> PlannedSession {
        PlannedSession {
            kind: SessionKind::LongRun,
            title: "Long run".into(),
            workout: StructuredWorkout::continuous(
                BlockTarget::Easy,
                DurationSecs::from_minutes(105),
            ),
            target_volume: VolumeKm(18.0),
            target_duration: DurationSecs::from_minutes(105),
            target_pace: None,
            rpe_target: Some(5),
            quality: false,
            ..tempo_session()
        }
    }

    /// A long run run evenly, or run with the heart rate climbing through the
    /// second half at the same pace — cardiac drift, the signature of a long run
    /// that got away from the athlete.
    fn long_run(first_hr: u16, second_hr: u16) -> Activity {
        let mut a = activity(
            105,
            18.0,
            350.0,
            u16::midpoint(first_hr, second_hr),
            second_hr,
        );
        a.laps = vec![
            lap(18, first_hr, 3.0),
            lap(17, first_hr, 3.0),
            lap(18, first_hr, 3.0),
            lap(17, second_hr, 3.0),
            lap(18, second_hr, 3.0),
            lap(17, second_hr, 3.0),
        ];
        a
    }

    #[test]
    fn decoupling_lowers_a_long_run_score_and_nothing_else() {
        let planned = long_run_session();
        let even = long_run(130, 130);
        let drifted = long_run(130, 152);
        let ath = athlete();

        let clean = score_session(
            &planned,
            &even,
            &SessionFeedback::default(),
            &ath,
            threshold(),
        );
        let faded = score_session(
            &planned,
            &drifted,
            &SessionFeedback::default(),
            &ath,
            threshold(),
        );

        // Same duration, same distance, same pace — so every component other than
        // decoupling must be identical, and any difference is the drift.
        assert_eq!(clean.components.duration, faded.components.duration);
        assert_eq!(clean.components.distance, faded.components.distance);
        assert_eq!(clean.components.decoupling_penalty, 1.0);
        assert!(
            faded.components.decoupling_penalty < 1.0,
            "drift of {} should cost something",
            decoupling(&drifted).expect("drift")
        );
        assert!(
            faded.score < clean.score,
            "a drifted long run must score lower: {} vs {}",
            faded.score,
            clean.score
        );
    }

    #[test]
    fn an_even_paced_long_run_is_not_penalised() {
        let planned = long_run_session();
        let even = long_run(130, 130);
        let q = score_session(
            &planned,
            &even,
            &SessionFeedback::default(),
            &athlete(),
            threshold(),
        );
        assert_eq!(q.components.decoupling_penalty, 1.0);
        assert!(q.score > 95.0, "an honest long run scored {}", q.score);
    }

    #[test]
    fn proximity_is_generous_inside_the_tolerance() {
        assert_eq!(proximity(10.0, 10.0, 0.1), 1.0);
        assert_eq!(proximity(10.9, 10.0, 0.1), 1.0);
        assert_eq!(proximity(9.1, 10.0, 0.1), 1.0);
    }

    #[test]
    fn proximity_decays_to_zero_at_three_times_the_tolerance() {
        assert!(proximity(13.0, 10.0, 0.1) < 0.01);
        assert!(proximity(7.0, 10.0, 0.1) < 0.01);
        assert!(proximity(11.5, 10.0, 0.1) > 0.4);
    }

    #[test]
    fn proximity_handles_missing_targets() {
        assert_eq!(
            proximity(5.0, 0.0, 0.1),
            1.0,
            "no target -> nothing to miss"
        );
        assert_eq!(proximity(0.0, 10.0, 0.1), 0.0, "nothing done -> no credit");
    }

    #[test]
    fn a_skipped_main_set_fails_structure_even_at_full_distance() {
        let planned = tempo_session();
        // 55 min, 12 km, but all of it easy — the tempo block never happened.
        let mut jog = activity(55, 12.0, 330.0, 120, 140);
        jog.laps = vec![lap(55, 120, 12.0)];
        let score = structure_adherence(&planned.workout, &jog, athlete().max_hr).expect("scored");
        assert!(
            score < 0.3,
            "jogged tempo should fail structure, got {score}"
        );
    }

    /// The prescribed tempo session executed properly: 15' easy, 30' at the
    /// prescribed 4:43/km tempo, 10' easy.
    ///
    /// The main set is 6.35 km because that is what 30 minutes at 283.5 s/km
    /// actually covers. A fixture whose lap distances disagree with its own
    /// prescribed pace will happily pass while describing a session that nobody
    /// would ever plan — and the moment a component measures pace from laps, that
    /// inconsistency becomes a wrong score.
    fn executed_tempo_laps() -> Vec<ActivityLap> {
        vec![lap(15, 120, 3.4), lap(30, 165, 6.35), lap(10, 125, 2.25)]
    }

    #[test]
    fn a_properly_executed_tempo_passes_structure() {
        let planned = tempo_session();
        let mut done = activity(55, 12.0, 275.0, 155, 170);
        // 15' easy, 30' at Z4, 10' easy -> 55% hard, planned 55%.
        done.laps = executed_tempo_laps();
        let score = structure_adherence(&planned.workout, &done, athlete().max_hr).expect("scored");
        assert!(score > 0.9, "got {score}");
    }

    #[test]
    fn structure_abstains_without_heart_rate() {
        let planned = tempo_session();
        let no_hr = activity(55, 12.0, 300.0, 0, 0);
        assert!(structure_adherence(&planned.workout, &no_hr, athlete().max_hr).is_none());
    }

    #[test]
    fn structure_abstains_on_non_intensity_sessions() {
        let easy = PlannedSession {
            kind: SessionKind::Easy,
            workout: StructuredWorkout::continuous(
                BlockTarget::Easy,
                DurationSecs::from_minutes(40),
            ),
            ..tempo_session()
        };
        let done = activity(40, 8.0, 330.0, 130, 150);
        assert!(structure_adherence(&easy.workout, &done, athlete().max_hr).is_none());
    }

    #[test]
    fn structure_abstains_on_an_empty_plan() {
        let empty = StructuredWorkout { blocks: vec![] };
        let done = activity(40, 8.0, 330.0, 130, 150);
        assert!(structure_adherence(&empty, &done, athlete().max_hr).is_none());
    }

    #[test]
    fn pace_adherence_needs_both_paces() {
        let planned = tempo_session();
        let max_hr = athlete().max_hr;
        let done = activity(55, 12.0, 284.0, 155, 170);
        assert!(pace_adherence(&planned, &done, max_hr).expect("scored") > 0.95);
        let no_pace = activity(55, 12.0, 0.0, 155, 170);
        assert!(pace_adherence(&planned, &no_pace, max_hr).is_none());
        let unplanned = PlannedSession {
            target_pace: None,
            ..tempo_session()
        };
        assert!(pace_adherence(&unplanned, &done, max_hr).is_none());
    }

    #[test]
    fn running_way_off_pace_scores_nothing() {
        let planned = tempo_session();
        let slow = activity(55, 12.0, 400.0, 155, 170);
        assert!(pace_adherence(&planned, &slow, athlete().max_hr).expect("scored") < 0.1);
    }

    /// The regression this guards: a structured session prescribes a pace for its
    /// *main set*, and a whole-session average is not that pace. Judging the two
    /// against each other would fail every correctly executed interval session on
    /// the plan's own arithmetic.
    #[test]
    fn pace_is_judged_on_the_main_set_not_the_whole_session_average() {
        let planned = tempo_session();
        let max_hr = athlete().max_hr;
        // Whole session at 5:22/km, but the tempo block itself at the prescribed
        // 4:43/km. The session was done as asked.
        let mut honest = activity(55, 12.0, 322.0, 155, 170);
        honest.laps = executed_tempo_laps();
        let score = pace_adherence(&planned, &honest, max_hr).expect("scored");
        assert!(score > 0.95, "main set was at target, got {score}");
        // The whole-session average alone would have failed it.
        assert!(proximity(322.0, 283.5, PACE_TOLERANCE) < 0.3);
    }

    #[test]
    fn zone_adherence_rewards_the_intended_zone() {
        let planned = tempo_session();
        let ath = athlete(); // max 185, Z3 floor 0.70 -> 129.5
        let mut in_zone = activity(55, 12.0, 300.0, 150, 185);
        in_zone.laps = executed_tempo_laps();
        let score = zone_adherence(&planned, &in_zone, ath.max_hr).expect("scored");
        assert!(score > 0.9, "got {score}");
    }

    #[test]
    fn zone_adherence_penalises_being_way_off() {
        let planned = tempo_session();
        let ath = athlete();
        let mut too_easy = activity(55, 12.0, 330.0, 110, 185);
        too_easy.laps = vec![lap(55, 110, 12.0)];
        let score = zone_adherence(&planned, &too_easy, ath.max_hr).expect("scored");
        assert!(score < 0.4, "got {score}");
    }

    /// Exceeding the zone is not this component's business. On short intervals
    /// heart rate legitimately runs past the ceiling, and penalising it would
    /// teach athletes to under-run their intervals to protect a number.
    #[test]
    fn running_above_the_zone_is_not_penalised() {
        let planned = tempo_session();
        let ath = athlete();
        let mut hard = activity(55, 12.0, 275.0, 172, 184);
        hard.laps = vec![lap(15, 130, 3.4), lap(30, 172, 6.35), lap(10, 130, 2.25)];
        let score = zone_adherence(&planned, &hard, ath.max_hr).expect("scored");
        assert!(score > 0.95, "over-delivery at intensity, got {score}");
    }

    #[test]
    fn effort_allows_one_point_of_rpe_noise() {
        assert_eq!(effort_adherence(Some(7), Some(7)), Some(1.0));
        assert_eq!(effort_adherence(Some(7), Some(8)), Some(1.0));
        assert!(effort_adherence(Some(7), Some(3)).expect("s") < 0.1);
        assert!(effort_adherence(Some(7), None).is_none());
    }

    #[test]
    fn subjective_maps_one_to_five_onto_the_unit_interval() {
        assert_eq!(subjective_score(Some(1)), Some(0.0));
        assert_eq!(subjective_score(Some(3)), Some(0.5));
        assert_eq!(subjective_score(Some(5)), Some(1.0));
        assert_eq!(subjective_score(None), None);
    }

    #[test]
    fn decoupling_needs_enough_laps() {
        let mut a = activity(60, 12.0, 300.0, 150, 170);
        assert!(decoupling(&a).is_none());
        a.laps = vec![lap(15, 140, 3.0), lap(15, 142, 3.0), lap(15, 141, 3.0)];
        assert!(
            decoupling(&a).is_none(),
            "three laps is not enough to split"
        );
    }

    #[test]
    fn decoupling_detects_a_drift_upward() {
        let mut a = activity(60, 12.0, 300.0, 150, 170);
        a.laps = vec![
            lap(15, 140, 3.0),
            lap(15, 140, 3.0),
            lap(15, 158, 3.0),
            lap(15, 158, 3.0),
        ];
        let drift = decoupling(&a).expect("drift");
        assert!(drift > 0.12, "expected ~13% drift, got {drift}");
    }

    #[test]
    fn decoupling_is_zero_when_the_athlete_closes_the_gap() {
        let mut a = activity(60, 12.0, 300.0, 150, 170);
        a.laps = vec![
            lap(15, 150, 3.0),
            lap(15, 150, 3.0),
            lap(15, 142, 3.0),
            lap(15, 142, 3.0),
        ];
        assert!(decoupling(&a).expect("drift") < 0.0);
    }

    #[test]
    fn decoupling_only_penalises_long_runs() {
        let mut a = activity(60, 12.0, 300.0, 150, 170);
        a.laps = vec![
            lap(15, 140, 3.0),
            lap(15, 140, 3.0),
            lap(15, 165, 3.0),
            lap(15, 165, 3.0),
        ];
        assert_eq!(decoupling_penalty(&a, SessionKind::Tempo), 1.0);
        assert_eq!(decoupling_penalty(&a, SessionKind::Intervals), 1.0);
        let penalty = decoupling_penalty(&a, SessionKind::LongRun);
        assert!(penalty < 0.7, "expected a real penalty, got {penalty}");
        assert!(penalty >= 0.5, "penalty is floored, got {penalty}");
    }

    #[test]
    fn a_small_drift_is_free() {
        let mut a = activity(60, 12.0, 300.0, 150, 170);
        a.laps = vec![
            lap(15, 150, 3.0),
            lap(15, 150, 3.0),
            lap(15, 154, 3.0),
            lap(15, 154, 3.0),
        ];
        assert_eq!(decoupling_penalty(&a, SessionKind::LongRun), 1.0);
    }

    #[test]
    fn a_well_executed_session_scores_high() {
        let planned = tempo_session();
        let mut done = activity(55, 12.0, 284.0, 155, 170);
        done.laps = vec![lap(15, 120, 3.2), lap(30, 165, 7.0), lap(10, 125, 1.8)];
        let feedback = SessionFeedback {
            rpe: Some(7),
            feeling: Some(4),
            note: None,
        };
        let q = score_session(&planned, &done, &feedback, &athlete(), threshold());
        assert!(q.score > 85.0, "expected a high score, got {}", q.score);
        assert!(q.confidence > 0.9);
        assert!(q.executed_quality);
    }

    #[test]
    fn a_jogged_tempo_scores_much_lower_than_an_executed_one() {
        let planned = tempo_session();
        let mut jog = activity(55, 12.0, 330.0, 120, 140);
        jog.laps = vec![lap(55, 120, 12.0)];
        let q = score_session(
            &planned,
            &jog,
            &SessionFeedback::default(),
            &athlete(),
            threshold(),
        );
        assert!(q.score < 60.0, "jogged tempo scored {}", q.score);
        assert!(!q.executed_quality, "a jog is not quality work");
    }

    #[test]
    fn missing_data_lowers_confidence_without_faking_quality() {
        let planned = tempo_session();
        let full = {
            let mut a = activity(55, 12.0, 284.0, 155, 170);
            a.laps = executed_tempo_laps();
            a
        };
        let bare = activity(55, 12.0, 284.0, 0, 0); // no HR at all
        let feedback = SessionFeedback {
            rpe: Some(7),
            feeling: Some(4),
            note: None,
        };
        let ath = athlete();
        let with_hr = score_session(&planned, &full, &feedback, &ath, threshold());
        let without = score_session(&planned, &bare, &feedback, &ath, threshold());
        assert!(without.confidence < with_hr.confidence);
        assert!(without.score <= with_hr.score + 1.0);
    }

    #[test]
    fn no_data_at_all_is_neutral() {
        assert_eq!(quality_score(&QualityComponents::default()), 50.0);
        assert_eq!(QualityComponents::default().coverage(), 0.0);
    }

    #[test]
    fn adherence_is_actual_over_planned_load() {
        let planned = tempo_session();
        let mut done = activity(55, 12.0, 284.0, 155, 170);
        done.laps = vec![lap(55, 165, 12.0)];
        let q = score_session(
            &planned,
            &done,
            &SessionFeedback::default(),
            &athlete(),
            threshold(),
        );
        assert!(q.planned_tss > 0.0);
        assert!(q.actual_tss > 0.0);
        assert!((q.adherence - 1.0).abs() < 0.35, "got {}", q.adherence);
    }

    #[test]
    fn a_rest_day_asks_for_nothing_and_executes_as_nothing() {
        let rest = PlannedSession {
            kind: SessionKind::Rest,
            workout: StructuredWorkout::continuous(BlockTarget::Other, DurationSecs::ZERO),
            target_duration: DurationSecs::ZERO,
            target_volume: VolumeKm::ZERO,
            target_pace: None,
            quality: false,
            ..tempo_session()
        };
        assert_eq!(planned_tss(&rest, threshold()), 0.0);
        let accidental_jog = activity(30, 6.0, 300.0, 160, 175);
        let q = score_session(
            &rest,
            &accidental_jog,
            &SessionFeedback::default(),
            &athlete(),
            threshold(),
        );
        assert_eq!(q.adherence, 1.0, "zero planned load -> no ratio to compute");
        assert!(!q.planned_quality);
    }

    #[test]
    fn an_unplanned_hard_effort_still_counts_as_quality() {
        // Plan said easy; athlete went hard. The mismatch must be visible.
        let easy = PlannedSession {
            kind: SessionKind::Easy,
            quality: false,
            target_pace: Some(Pace::new(330.0)),
            ..tempo_session()
        };
        let hard = activity(55, 14.0, 250.0, 170, 182);
        let q = score_session(
            &easy,
            &hard,
            &SessionFeedback::default(),
            &athlete(),
            threshold(),
        );
        assert!(q.executed_quality, "the athlete clearly did hard work");
        assert!(!q.planned_quality);
    }

    #[test]
    fn planned_tss_uses_target_pace_when_present() {
        let with_pace = tempo_session();
        let without = PlannedSession {
            target_pace: None,
            ..tempo_session()
        };
        let a = planned_tss(&with_pace, threshold());
        let b = planned_tss(&without, threshold());
        assert!(a > 0.0 && b > 0.0);
        assert!((a - b).abs() > 1.0, "pace and kind-factor should differ");
    }

    #[test]
    fn planned_kind_factors_are_ordered() {
        assert!(
            planned_kind_factor(SessionKind::Recovery) < planned_kind_factor(SessionKind::Tempo)
        );
        assert!(
            planned_kind_factor(SessionKind::Tempo) < planned_kind_factor(SessionKind::Intervals)
        );
        assert_eq!(planned_kind_factor(SessionKind::Rest), 0.0);
    }

    #[test]
    fn score_is_always_in_range() {
        let planned = tempo_session();
        let ath = athlete();
        for (mins, km, pace, hr) in [
            (10u32, 2.0f64, 200.0f64, 190u16),
            (180, 30.0, 400.0, 100),
            (55, 12.0, 284.0, 155),
        ] {
            let a = activity(mins, km, pace, hr, 190);
            let q = score_session(&planned, &a, &SessionFeedback::default(), &ath, threshold());
            assert!(
                (0.0..=100.0).contains(&q.score),
                "out of range: {}",
                q.score
            );
            assert!((0.0..=2.0).contains(&q.adherence));
        }
    }

    #[test]
    fn verdict_names_the_weakest_component() {
        let c = QualityComponents {
            duration: Some(0.95),
            structure: Some(0.2),
            pace: Some(0.9),
            ..Default::default()
        };
        assert_eq!(verdict(&c), "the main set was not done");
        let good = QualityComponents {
            duration: Some(0.95),
            distance: Some(0.95),
            ..Default::default()
        };
        assert_eq!(verdict(&good), "executed as prescribed");
        assert_eq!(verdict(&QualityComponents::default()), "no data recorded");
    }

    #[test]
    fn hard_time_share_falls_back_to_the_average() {
        let max_hr = athlete().max_hr; // 185
        let mut a = activity(60, 12.0, 300.0, 160, 185);
        a.laps = vec![];
        // avg 160 of max 185 = 0.865, above the Z3 ceiling of 0.80.
        assert_eq!(hard_time_share(&a, max_hr), Some(1.0));
        let easy = activity(60, 12.0, 330.0, 120, 185);
        assert_eq!(hard_time_share(&easy, max_hr), Some(0.0));
        let no_hr = activity(60, 12.0, 300.0, 0, 0);
        assert!(hard_time_share(&no_hr, max_hr).is_none());
    }

    /// The regression the fixture caught: judging "hard" against the session's
    /// own peak HR lets an easy session certify itself as hard, because easing
    /// off lowers the bar by exactly the amount eased.
    #[test]
    fn hard_time_share_is_judged_against_the_athletes_max_not_the_sessions_peak() {
        // A wholly easy hour: 120 bpm, and the session's own peak is also 120.
        let mut jog = activity(60, 10.0, 360.0, 120, 120);
        jog.laps = vec![lap(60, 120, 10.0)];
        assert_eq!(
            hard_time_share(&jog, athlete().max_hr),
            Some(0.0),
            "120 bpm is not hard work for an athlete with a max of 185"
        );
        // An unrecorded athlete max HR must abstain rather than guess.
        assert!(hard_time_share(&jog, runalytics_core::HeartRate::NONE).is_none());
    }

    #[test]
    fn executed_helpers_pass_through_the_activity() {
        let a = activity(55, 12.0, 284.0, 155, 170);
        assert_eq!(executed_duration(&a), DurationSecs::from_minutes(55));
        assert_eq!(executed_volume(&a), VolumeKm(12.0));
        assert!(executed_load(&a, &athlete(), threshold()) > 0.0);
    }
}
