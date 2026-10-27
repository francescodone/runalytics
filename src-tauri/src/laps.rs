//! Per-lap and per-kilometre breakdown of a recorded run.
//!
//! The plan prescribes a session as a list of laps — warm-up, work, recovery,
//! cool-down — and the app has only ever judged a quality session with a single
//! number ([`crate::plan::pacing::rep_pace`]). That is the right shape for
//! adapting a plan and the wrong shape for reviewing a run: a runner who opens a
//! tempo session wants to see each rep's pace sitting next to the range it was
//! asked for, not one average that the warm-up drags down.
//!
//! Two readings are produced from the same track, because they answer different
//! questions:
//!
//! * **Laps** follow the *prescription*, so every bar has a target to be read
//!   against. This is the "did I hit it" view.
//! * **Kilometre splits** follow the *clock*, so they exist for every run
//!   whether or not a plan was behind it. This is the "how did my pace move"
//!   view, and its delta is against the previous kilometre — labelled as such in
//!   the UI, because "delta" alone is read as "against target" and the two are
//!   opposite on a surge.
//!
//! Where there is no prescription to line up with — a standalone run, a track
//! that has not synced, a run whose shape does not match the session planned for
//! it — the laps fall back to per-kilometre laps carrying no target. Ungraded
//! laps are drawn unmarked rather than guessed at. Only the work is graded:
//! a warm-up, a cool-down, or a jog recovery has no range to hit or miss, so
//! the plan's easy paces are not drawn as caps and never enter a verdict — the
//! chart answers "did I do the work", not "did I jog correctly".

use crate::plan::pacing::{distance_ratio_ok, metres_between, span_stats};
use crate::plan::session;
use crate::types::*;

/// Track points pulled for the lap maths. Higher than the sheet's map fetch:
/// laps are cut at 200 m granularity, so a thin sample smears the recoveries
/// into the reps and every bar reads as one long easy effort.
pub const TRACK_POINTS: i64 = 6_000;

/// A split shorter than this is GPS tail, not a kilometre worth printing.
const MIN_SPLIT_M: f64 = 200.0;

/// Below this fraction of its planned length a lap has too little track under
/// it to report a pace for. Better an em-dash than a pace invented from jitter.
const MIN_LAP_COVERAGE: f64 = 0.35;

/// A lap the plan asked for, in the order the run was supposed to unfold.
#[derive(Debug, Clone)]
struct Slot {
    label: String,
    kind: LapKind,
    meters: f64,
    /// The range to grade against — set on the work only. An easy or recovery
    /// slot leaves this empty, which is what stops a cap being drawn over it.
    band: (Option<i64>, Option<i64>),
}

/// Cut a recorded run into prescribed laps and kilometre splits.
///
/// `prescribed` is the session the plan generated for this run, when there is
/// one and its shape is recognisable inside the recorded distance. Without it
/// the laps are per-kilometre and carry no target — the honest answer for a
/// run the plan never asked for.
pub fn build(
    points: &[TrackPoint],
    activity: &Activity,
    prescribed: Option<(&TrainingPlan, &PlanWorkout)>,
    tolerance: f64,
) -> (Vec<LapSlice>, Vec<KmSplit>) {
    let splits = kilometre_splits(points);
    let Some((plan, workout)) = prescribed else {
        return (splits_as_laps(&splits), splits);
    };
    let details = session::details_for_plan(workout, plan);
    let planned_m = details.distance_m();
    let run_m = activity.distance_m;
    // The layout is only trustworthy when the recorded run is the session that
    // was planned. Same tolerance the grading uses, so the chart and the
    // verdict can never disagree about whether this run counts.
    if planned_m <= 0.0 || run_m <= 0.0 || !distance_ratio_ok(run_m / planned_m) {
        return (splits_as_laps(&splits), splits);
    }
    let Some(slots) = slots(&details.steps) else {
        return (splits_as_laps(&splits), splits);
    };
    let scale = run_m / planned_m;
    let laps = prescribed_laps(points, &slots, scale, tolerance);
    if laps.is_empty() {
        return (splits_as_laps(&splits), splits);
    }
    let splits = band_splits(splits, &laps);
    (laps, splits)
}

/// Turn the plan's steps into the run's shape: work laps, with a recovery slot
/// between each repeat. `None` when the session has no measurable structure.
///
/// Only the work is given a band. The plan does state easy paces for the
/// warm-up, cool-down, and jog recoveries, but they exist so the time and load
/// maths has something to integrate over — they are not a range the runner is
/// asked to hit. Drawing them as caps would put four unmissable targets on the
/// chart next to the one that matters, and grade a jog as though it were a rep.
fn slots(steps: &[session::WorkoutStep]) -> Option<Vec<Slot>> {
    if steps.is_empty() {
        return None;
    }
    let mut out = Vec::new();
    for step in steps {
        let kind = if step.open_ended {
            LapKind::Easy
        } else {
            LapKind::Work
        };
        for rep in 0..step.repeats.max(1) {
            out.push(Slot {
                label: step.label.clone(),
                kind,
                meters: step.distance_m,
                band: if kind == LapKind::Work {
                    (step.pace_lo_s_km, step.pace_hi_s_km)
                } else {
                    (None, None)
                },
            });
            if rep + 1 < step.repeats.max(1) && step.recovery_m > 0.0 {
                out.push(Slot {
                    label: "Recovery".into(),
                    kind: LapKind::Recovery,
                    meters: step.recovery_m,
                    band: (None, None),
                });
            }
        }
    }
    out.iter().any(|s| s.meters > 0.0).then_some(out)
}

/// Walk the prescription across the recorded track, one span per slot.
fn prescribed_laps(
    points: &[TrackPoint],
    slots: &[Slot],
    scale: f64,
    tolerance: f64,
) -> Vec<LapSlice> {
    let cum = cumulative(points);
    let total = *cum.last().unwrap_or(&0.0);
    if total <= 0.0 {
        return Vec::new();
    }
    let mut laps = Vec::with_capacity(slots.len());
    let mut cursor = 0.0f64;
    for (i, slot) in slots.iter().enumerate() {
        let want = slot.meters * scale;
        let from = cursor.min(total);
        let to = (cursor + want).min(total);
        cursor += want;
        let stats = span_stats(points, &cum, from, to);
        let thin = want <= 0.0 || to - from < want * MIN_LAP_COVERAGE;
        let (duration_s, pace, hr) = match stats {
            Some((secs, metres)) if !thin && metres > 0.0 && secs > 0.0 => (
                Some(secs),
                Some(secs / metres * 1000.0),
                mean_hr(points, &cum, from, to),
            ),
            _ => (None, None, None),
        };
        let verdict = pace.and_then(|p| grade(p, slot, tolerance));
        laps.push(LapSlice {
            index: (i + 1) as u32,
            label: slot.label.clone(),
            kind: slot.kind,
            start_m: from,
            meters: to - from,
            duration_s: duration_s.unwrap_or(0.0),
            avg_pace_s_km: pace,
            avg_hr: hr,
            band_lo_s_km: slot.band.0,
            band_hi_s_km: slot.band.1,
            verdict,
        });
    }
    laps.retain(|l| l.meters > 0.0);
    // A layout where nothing was actually measured is worse than no layout:
    // the caller falls back to per-kilometre bars, which at least say what they
    // are. Only a track that produced at least one real pace keeps the
    // prescription.
    if laps.iter().any(|l| l.avg_pace_s_km.is_some()) {
        laps
    } else {
        Vec::new()
    }
}

/// Grade one lap against the range it asked for. Only the work asks for a
/// range, so an easy or recovery slot (band empty) is never graded.
fn grade(pace: f64, slot: &Slot, tolerance: f64) -> Option<LapVerdict> {
    let (Some(lo), Some(hi)) = slot.band else {
        return None;
    };
    if pace < lo as f64 - tolerance {
        Some(LapVerdict::Ahead)
    } else if pace > hi as f64 + tolerance {
        Some(LapVerdict::Behind)
    } else {
        Some(LapVerdict::InRange)
    }
}

/// Per-kilometre splits, whole kilometres plus a trailing part-kilometre.
pub fn kilometre_splits(points: &[TrackPoint]) -> Vec<KmSplit> {
    let cum = cumulative(points);
    let total = *cum.last().unwrap_or(&0.0);
    if total < MIN_SPLIT_M {
        return Vec::new();
    }
    let mut out: Vec<KmSplit> = Vec::new();
    let mut km = 0u32;
    let mut from = 0.0f64;
    while from < total {
        let to = (from + 1000.0).min(total);
        if let Some((secs, metres)) = span_stats(points, &cum, from, to) {
            if metres >= MIN_SPLIT_M && secs > 0.0 {
                out.push(KmSplit {
                    km: km + 1,
                    meters: metres,
                    duration_s: secs,
                    avg_pace_s_km: Some(secs / metres * 1000.0),
                    avg_hr: mean_hr(points, &cum, from, to),
                    delta_prev_s_km: None,
                    band_lo_s_km: None,
                    band_hi_s_km: None,
                });
            }
        }
        from = to;
        km += 1;
    }
    for i in 1..out.len() {
        out[i].delta_prev_s_km = match (out[i].avg_pace_s_km, out[i - 1].avg_pace_s_km) {
            (Some(now), Some(prev)) => Some(now - prev),
            _ => None,
        };
    }
    out
}

/// Attach the prescribed band covering each split, so the table can show a
/// pace against its target without re-walking the track.
fn band_splits(mut splits: Vec<KmSplit>, laps: &[LapSlice]) -> Vec<KmSplit> {
    for split in &mut splits {
        let mid = (split.km as f64 - 0.5) * 1000.0;
        if let Some(lap) = laps
            .iter()
            .find(|l| mid >= l.start_m && mid < l.start_m + l.meters)
        {
            split.band_lo_s_km = lap.band_lo_s_km;
            split.band_hi_s_km = lap.band_hi_s_km;
        }
    }
    splits
}

/// When nothing was prescribed, the laps are the kilometres: still worth
/// drawing, just with no caps to read against.
fn splits_as_laps(splits: &[KmSplit]) -> Vec<LapSlice> {
    splits
        .iter()
        .map(|s| LapSlice {
            index: s.km,
            label: format!("{} km", s.km),
            kind: LapKind::Kilometre,
            start_m: (s.km as f64 - 1.0) * 1000.0,
            meters: s.meters,
            duration_s: s.duration_s,
            avg_pace_s_km: s.avg_pace_s_km,
            avg_hr: s.avg_hr,
            band_lo_s_km: None,
            band_hi_s_km: None,
            verdict: None,
        })
        .collect()
}

/// Cumulative metres along the track, same length as `points`.
fn cumulative(points: &[TrackPoint]) -> Vec<f64> {
    let mut cum = Vec::with_capacity(points.len());
    let mut total = 0.0f64;
    cum.push(0.0);
    for pair in points.windows(2) {
        total += metres_between(&pair[0], &pair[1]);
        cum.push(total);
    }
    cum
}

/// Mean heart rate over a stretch of track, weighted by the time in each
/// sample. `None` when no sample in the stretch carries a heart rate.
///
/// The weighting is seconds and not metres on purpose: a lap's fast half covers
/// more ground per second than its slow half, so weighting by distance would
/// pull the average toward exactly the samples that were already hardest.
fn mean_hr(points: &[TrackPoint], cum: &[f64], from: f64, to: f64) -> Option<i64> {
    let start = cum.partition_point(|&c| c < from).saturating_sub(1);
    let mut weighted = 0.0f64;
    let mut seconds = 0.0f64;
    let mut i = start;
    while i < points.len() && cum[i] < to {
        if let Some(hr) = points[i].hr {
            // This sample's own segment, clipped to the span: the share of the
            // segment inside the stretch owns the same share of its seconds.
            let segment_s = match points.get(i + 1) {
                Some(next) => (next.ts - points[i].ts).max(0) as f64,
                None => 0.0,
            };
            let seg_lo = cum[i];
            let seg_hi = cum.get(i + 1).copied().unwrap_or(seg_lo);
            let inside = if seg_hi > seg_lo {
                (to.min(seg_hi) - from.max(seg_lo)).max(0.0) / (seg_hi - seg_lo)
            } else {
                0.0
            };
            let dt = segment_s * inside;
            if dt > 0.0 {
                weighted += hr as f64 * dt;
                seconds += dt;
            }
        }
        i += 1;
    }
    (seconds > 0.0).then(|| (weighted / seconds).round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    /// A track built from `(metres, seconds per km)` legs, sampled every 20 m
    /// walking due north so cumulative distance is real.
    ///
    /// Seconds accumulate as a float and only the absolute timestamp is
    /// rounded: rounding each step's duration would bias every synthetic track
    /// (20 m at 5:45/km is 6.9 s, and fifty 7 s steps silently become a
    /// 7.0 s/km leg).
    fn track(segments: &[(f64, f64)]) -> Vec<TrackPoint> {
        const STEP_M: f64 = 20.0;
        let mut paces = Vec::new();
        for &(metres, pace) in segments {
            let steps = (metres / STEP_M).round() as usize;
            paces.extend(std::iter::repeat_n(pace, steps));
        }
        let mut out = Vec::with_capacity(paces.len() + 1);
        let mut ts_f = 0.0f64;
        let mut lat = 32.7f64;
        out.push(TrackPoint {
            ts: 0,
            lat,
            lon: -96.8,
            hr: None,
        });
        for pace in paces {
            ts_f += STEP_M / 1000.0 * pace;
            lat += STEP_M / 111_320.0;
            out.push(TrackPoint {
                ts: ts_f.round() as i64,
                lat,
                lon: -96.8,
                hr: None,
            });
        }
        out
    }

    fn run(distance_m: f64) -> Activity {
        Activity {
            id: 1,
            provider: ProviderKind::Coros,
            provider_record_id: "1".into(),
            sport: "running".into(),
            start_time: Utc::now(),
            duration_s: 0.0,
            distance_m,
            avg_hr: None,
            max_hr: None,
            avg_pace_s_km: None,
            elevation_gain_m: None,
            has_track: true,
        }
    }

    fn slot(kind: LapKind, meters: f64, band: (Option<i64>, Option<i64>)) -> Slot {
        Slot {
            label: "Rep".into(),
            kind,
            meters,
            band,
        }
    }

    /// `repeats` x 1 km at 4:40-4:50/km with `recovery_m` of jog recovery.
    /// The recovery carries an easy band on the step — the plan states one for
    /// the load maths — which the layout must not turn into a graded cap.
    fn step(repeats: u32, distance_m: f64, recovery_m: f64) -> session::WorkoutStep {
        session::WorkoutStep {
            label: "Intervals".into(),
            repeats,
            distance_m,
            pace_lo_s_km: Some(280),
            pace_hi_s_km: Some(290),
            recovery_m,
            recovery_pace_lo_s_km: Some(390),
            recovery_pace_hi_s_km: Some(420),
            open_ended: false,
        }
    }

    #[test]
    fn splits_report_the_real_pace_and_a_delta_against_the_previous_kilometre() {
        // Pinned to an absolute number on purpose: a round-trip or a ratio test
        // keeps passing while every pace is 1000x off, because the error
        // cancels. 5:00/km must read as 300 seconds per kilometre.
        let points = track(&[(1000.0, 300.0), (1000.0, 290.0)]);
        let splits = kilometre_splits(&points);
        assert_eq!(splits.len(), 2);
        assert_eq!(splits[0].km, 1);
        assert!(
            (splits[0].avg_pace_s_km.unwrap() - 300.0).abs() < 0.5,
            "a 5:00/km kilometre must report 300 s/km, got {:?}",
            splits[0].avg_pace_s_km
        );
        assert!(
            (splits[1].avg_pace_s_km.unwrap() - 290.0).abs() < 0.5,
            "got {:?}",
            splits[1].avg_pace_s_km
        );
        // Negative means faster, which is the convention the UI's sign follows.
        let delta = splits[1].delta_prev_s_km.unwrap();
        assert!(
            (delta + 10.0).abs() < 1.0,
            "a negative split is -10, got {delta}"
        );
        assert_eq!(splits[0].delta_prev_s_km, None);
    }

    #[test]
    fn splits_keep_a_usable_part_kilometre_and_drop_gps_tail() {
        let points = track(&[(2000.0, 300.0), (400.0, 300.0)]);
        let splits = kilometre_splits(&points);
        assert_eq!(splits.len(), 3, "the trailing 400 m is still a split");
        assert!((splits[2].meters - 400.0).abs() < 1.0);
        assert_eq!(splits[2].km, 3);

        // A track shorter than the minimum split is noise, not a run.
        assert!(kilometre_splits(&track(&[(100.0, 300.0)])).is_empty());
    }

    #[test]
    fn heart_rate_is_weighted_by_time_not_by_distance() {
        // 1 km at 10:00/km with a low heart rate, then 1 km at 5:00/km with a
        // high one. The slow kilometre burns twice the seconds per metre, so a
        // time-weighted mean sits nearer the easy number: (120*600 + 170*300)
        // / 900 = 137. A distance-weighted mean would report 145, pulled toward
        // exactly the samples that were hardest — the wrong way for a lap read.
        let mut points = track(&[(1000.0, 600.0), (1000.0, 300.0)]);
        let cum = cumulative(&points);
        for (i, p) in points.iter_mut().enumerate() {
            p.hr = Some(if cum[i] < 1000.0 { 120 } else { 170 });
        }
        let total = *cum.last().unwrap();
        assert_eq!(mean_hr(&points, &cum, 0.0, total), Some(137));
    }

    #[test]
    fn heart_rate_is_absent_when_no_sample_in_the_span_carries_one() {
        let points = track(&[(1000.0, 300.0)]);
        let cum = cumulative(&points);
        assert_eq!(mean_hr(&points, &cum, 0.0, 1000.0), None);
    }

    #[test]
    fn repeats_expand_with_a_recovery_between_each() {
        // 4 reps with a jog between: 4 work slots and 3 recoveries, never 4
        // recoveries — a trailing jog belongs to the cool-down, not the set.
        let jog = slots(&[step(4, 1000.0, 400.0)]).unwrap();
        assert_eq!(jog.len(), 7);
        assert_eq!(jog.iter().filter(|s| s.kind == LapKind::Work).count(), 4);
        let recoveries: Vec<&Slot> = jog.iter().filter(|s| s.kind == LapKind::Recovery).collect();
        assert_eq!(recoveries.len(), 3);
        // A recovery is rest. The step states an easy pace for the load maths,
        // but it is not a range the runner is asked to hit, so no cap and no
        // verdict — even for a float run at quality pace.
        assert!(recoveries.iter().all(|s| s.band == (None, None)));

        // Only the work carries the band it is graded against.
        let work: Vec<&Slot> = jog.iter().filter(|s| s.kind == LapKind::Work).collect();
        assert!(work.iter().all(|s| s.band == (Some(280), Some(290))));
    }

    #[test]
    fn an_easy_step_is_never_graded_even_though_it_states_a_pace() {
        // A warm-up step comes through with an easy pace band on it; the chart
        // must not draw that band as a target the runner could miss.
        let warm = session::WorkoutStep {
            label: "Warm-up".into(),
            repeats: 1,
            distance_m: 2000.0,
            pace_lo_s_km: Some(390),
            pace_hi_s_km: Some(420),
            recovery_m: 0.0,
            recovery_pace_lo_s_km: None,
            recovery_pace_hi_s_km: None,
            open_ended: true,
        };
        let laid = slots(&[warm]).unwrap();
        assert_eq!(laid.len(), 1);
        assert_eq!(laid[0].kind, LapKind::Easy);
        assert_eq!(laid[0].band, (None, None));
    }

    #[test]
    fn a_step_with_no_length_is_not_a_layout() {
        assert!(slots(&[]).is_none());
        assert!(slots(&[step(3, 0.0, 0.0)]).is_none());
    }

    #[test]
    fn laps_are_cut_where_the_prescription_says_the_shape_changes() {
        let points = track(&[
            (2000.0, 380.0), // warm-up, run faster than the easy pace it states
            (1000.0, 270.0), // rep 1, under the band
            (400.0, 400.0),  // jog recovery
            (1000.0, 285.0), // rep 2, inside the band
        ]);
        let slots = vec![
            slot(LapKind::Easy, 2000.0, (None, None)),
            slot(LapKind::Work, 1000.0, (Some(280), Some(290))),
            slot(LapKind::Recovery, 400.0, (None, None)),
            slot(LapKind::Work, 1000.0, (Some(280), Some(290))),
        ];
        let laps = prescribed_laps(&points, &slots, 1.0, 4.0);
        assert_eq!(laps.len(), 4);
        assert_eq!(
            laps.iter().map(|l| l.kind).collect::<Vec<_>>(),
            vec![
                LapKind::Easy,
                LapKind::Work,
                LapKind::Recovery,
                LapKind::Work
            ]
        );
        // Spans tile the run without gaps or overlap.
        assert_eq!(laps[0].start_m, 0.0);
        for pair in laps.windows(2) {
            assert!(
                (pair[0].start_m + pair[0].meters - pair[1].start_m).abs() < 0.5,
                "lap {} does not hand over to lap {}",
                pair[0].index,
                pair[1].index
            );
        }
        assert!(
            (laps[1].avg_pace_s_km.unwrap() - 270.0).abs() < 1.0,
            "rep 1 pace {:?}",
            laps[1].avg_pace_s_km
        );

        // The verdicts are the whole point of the chart: each work bar reads
        // against its own range, and nothing else is graded at all.
        assert_eq!(
            laps.iter().map(|l| l.verdict).collect::<Vec<_>>(),
            vec![
                // A warm-up has no range to hit or miss: no cap, no verdict —
                // even though it was run quicker than the easy pace it states.
                None,
                Some(LapVerdict::Ahead),
                // A jog is rest: ungraded, so the UI draws no cap over it.
                None,
                Some(LapVerdict::InRange),
            ]
        );
        assert_eq!(laps[0].band_lo_s_km, None);
        assert_eq!(laps[2].band_lo_s_km, None);
    }

    #[test]
    fn a_lap_too_thin_to_measure_reports_no_pace_rather_than_a_guess() {
        // The prescription asks for 1 km of work but the run stopped after
        // 200 m of it. A pace from that is GPS jitter with a label on it.
        let points = track(&[(1000.0, 300.0), (200.0, 300.0)]);
        let slots = vec![
            slot(LapKind::Work, 1000.0, (Some(280), Some(290))),
            slot(LapKind::Work, 1000.0, (Some(280), Some(290))),
        ];
        let laps = prescribed_laps(&points, &slots, 1.0, 4.0);
        assert_eq!(laps.len(), 2, "the short lap is still drawn");
        assert!(laps[0].avg_pace_s_km.is_some());
        assert_eq!(laps[1].avg_pace_s_km, None);
        assert_eq!(laps[1].verdict, None);
    }

    #[test]
    fn a_layout_that_measured_nothing_at_all_is_dropped() {
        // Every lap empty is worse than no layout: the caller falls back to
        // per-kilometre bars, which at least say what they are.
        let points = track(&[(1000.0, 300.0)]);
        let slots = vec![slot(LapKind::Work, 5000.0, (Some(280), Some(290)))];
        assert!(prescribed_laps(&points, &slots, 1.0, 4.0).is_empty());
    }

    #[test]
    fn grade_respects_the_tolerance_on_both_ends() {
        let band = slot(LapKind::Work, 1000.0, (Some(280), Some(290)));
        let g = |pace: f64, s: &Slot| grade(pace, s, 4.0);
        assert_eq!(g(275.0, &band), Some(LapVerdict::Ahead));
        assert_eq!(g(276.5, &band), Some(LapVerdict::InRange));
        assert_eq!(g(285.0, &band), Some(LapVerdict::InRange));
        assert_eq!(g(293.5, &band), Some(LapVerdict::InRange));
        assert_eq!(g(295.0, &band), Some(LapVerdict::Behind));
        // No band means no opinion, which the UI draws as an unmarked bar.
        // That covers every easy, recovery, and cool-down slot.
        assert_eq!(
            g(285.0, &slot(LapKind::Recovery, 400.0, (None, None))),
            None
        );
        assert_eq!(g(200.0, &slot(LapKind::Easy, 2000.0, (None, None))), None);
    }

    #[test]
    fn a_run_with_no_plan_behind_it_becomes_per_kilometre_laps() {
        let points = track(&[(2400.0, 300.0)]);
        let (laps, splits) = build(&points, &run(2400.0), None, 4.0);
        assert_eq!(laps.len(), 3);
        assert!(laps.iter().all(|l| l.kind == LapKind::Kilometre));
        // No prescription means no target: no caps, no verdicts.
        assert!(laps.iter().all(|l| l.band_hi_s_km.is_none()));
        assert!(laps.iter().all(|l| l.verdict.is_none()));
        assert_eq!(laps[0].label, "1 km");
        assert_eq!(splits.len(), 3);
        assert!(splits.iter().all(|s| s.band_hi_s_km.is_none()));
    }

    #[test]
    fn splits_under_a_prescription_carry_only_the_work_band() {
        // The splits table has to be able to show a pace against its target
        // without walking the track again, so the band is stamped per split —
        // but only where the lap above it was work. A kilometre under an easy
        // lap stays unmarked, same as the chart.
        let points = track(&[(1000.0, 300.0), (1000.0, 285.0)]);
        let slice = |index: u32,
                     kind: LapKind,
                     start_m: f64,
                     meters: f64,
                     band: (Option<i64>, Option<i64>)| LapSlice {
            index,
            label: "Rep".into(),
            kind,
            start_m,
            meters,
            duration_s: 0.0,
            avg_pace_s_km: Some(300.0),
            avg_hr: None,
            band_lo_s_km: band.0,
            band_hi_s_km: band.1,
            verdict: None,
        };
        let splits = band_splits(
            kilometre_splits(&points),
            &[
                slice(1, LapKind::Work, 0.0, 1500.0, (Some(280), Some(290))),
                slice(2, LapKind::Easy, 1500.0, 500.0, (None, None)),
            ],
        );
        assert_eq!(splits[0].band_lo_s_km, Some(280));
        assert_eq!(splits[0].band_hi_s_km, Some(290));
        // The second kilometre's midpoint falls in the easy lap: no band.
        assert_eq!(splits[1].band_lo_s_km, None);
        assert_eq!(splits[1].band_hi_s_km, None);
    }
}
