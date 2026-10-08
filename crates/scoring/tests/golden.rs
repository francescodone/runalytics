//! Golden fixtures required by the plan's verification step 3.
//!
//! These are *integration* tests on purpose: they exercise the crate through its
//! public API the way the app will, on a scenario a coach could recognise, and
//! they pin the resulting numbers. A unit test proves a formula does what its
//! author intended; a golden test proves the composed model still tells the
//! coaching story. If a retune breaks one of these, the retune changed the
//! advice an athlete would be given — which is exactly when we want to be told.

use runalytics_core::{
    Activity, ActivityId, ActivityLap, ActivitySummary, AthleteSnapshot, BlockTarget, Date,
    DurationSecs, HeartRate, HrZone, Intensity, Pace, PlannedSession, PlannedSessionId,
    SessionKind, StructuredWorkout, TimeOfDay, Timestamp, Tz, VolumeKm, WorkoutBlock,
};
use runalytics_scoring::{
    DailyLoad, SessionFeedback, injury_risk_for, score_session, weekly_totals,
};

fn athlete() -> AthleteSnapshot {
    AthleteSnapshot::placeholder("Europe/Madrid".parse::<Tz>().expect("valid tz"))
}

fn date(y: i32, m: u32, d: u32) -> Date {
    Date::from_ymd_opt(y, m, d).expect("valid date")
}

/// Build a daily load series from a list of weekly totals.
///
/// Each week's load is spread over its seven days so the EMA sees a realistic
/// daily distribution rather than one large value per week — the shape matters,
/// because an EMA over weekly spikes behaves quite differently from a daily one.
fn series_from_weeks(start: Date, weeks: &[f64]) -> Vec<DailyLoad> {
    let mut out = Vec::new();
    for (w, total) in weeks.iter().enumerate() {
        // Six training days, one rest day: a real week, not a flat distribution.
        let per_day = total / 6.0;
        for d in 0..7 {
            let Some(day) = start.checked_add_days(chrono::Days::new((w * 7 + d) as u64)) else {
                continue;
            };
            out.push(DailyLoad {
                date: day,
                load: if d == 6 { 0.0 } else { per_day },
            });
        }
    }
    out
}

/// The series as of the end of week `week_index`, i.e. what the app would show
/// on the last day of that week.
fn up_to_week(series: &[DailyLoad], week_index: usize) -> &[DailyLoad] {
    let end = (week_index + 1) * 7;
    &series[..end.min(series.len())]
}

/// A 10-week block with a deliberate 40 % volume spike in week 9.
///
/// The block is deliberately *well managed* before the spike — steady growth,
/// full rest days, no monotony — so the spike is the only thing that can go
/// wrong. That isolation is the point: if the model cannot see a 40 % jump in an
/// otherwise clean block, it will not see one in a messy one either.
#[test]
fn a_40_percent_spike_in_a_10_week_block_pushes_injury_risk_high() {
    let start = date(2026, 8, 3); // a Monday
    // Weeks 1-8: ~500 -> ~700 TSS of steady, healthy growth. Week 9: +40%.
    let weeks = [
        500.0, 540.0, 580.0, 620.0, 560.0, 640.0, 680.0, 700.0, 980.0, 720.0,
    ];
    let series = series_from_weeks(start, &weeks);
    assert_eq!(series.len(), 70);

    let ath = athlete();

    // Before the spike the block must read as safe, or the fixture is not
    // isolating anything — a model that says "high risk" every week is not
    // detecting the spike, it is detecting everything.
    let before = injury_risk_for(up_to_week(&series, 7), &ath, Some(75.0), None);
    assert!(
        before.score < 45.0,
        "week 8 of a clean block should read safe, got {} ({:?})",
        before.score,
        before.band
    );

    let spike_week = injury_risk_for(up_to_week(&series, 8), &ath, Some(75.0), None);
    assert!(
        spike_week.score > 70.0,
        "the 40% spike week must read high risk, got {} (drivers {:?})",
        spike_week.score,
        spike_week
            .drivers
            .iter()
            .map(|d| (d.code, d.contribution))
            .collect::<Vec<_>>()
    );
    assert_eq!(spike_week.band, runalytics_scoring::InjuryBand::High);

    // And the spike must be *named* as the reason, not merely reflected in a
    // number. An unattributed high score is a warning an athlete cannot act on.
    let top = spike_week.drivers.first().expect("a driver");
    assert!(
        matches!(top.code, "spike" | "acwr"),
        "the spike should be the leading driver, got {}",
        top.code
    );

    // The weekly totals themselves must show the jump the fixture claims.
    let totals = weekly_totals(up_to_week(&series, 8));
    let last = *totals.last().expect("a week");
    let prev = *totals.get(totals.len() - 2).expect("a prior week");
    assert!(
        last / prev > 1.35,
        "fixture does not actually spike: {prev} -> {last}"
    );
}

/// The same block with the spike removed must stay low throughout — the control
/// that proves the previous test is measuring the spike and not the block.
#[test]
fn the_same_block_without_the_spike_stays_low() {
    let start = date(2026, 8, 3);
    let weeks = [
        500.0, 540.0, 580.0, 620.0, 560.0, 640.0, 680.0, 700.0, 720.0, 700.0,
    ];
    let series = series_from_weeks(start, &weeks);
    let ath = athlete();
    for week in 3..10 {
        let risk = injury_risk_for(up_to_week(&series, week), &ath, Some(75.0), None);
        assert!(
            risk.score < 55.0,
            "week {} of a clean block scored {}",
            week + 1,
            risk.score
        );
    }
}

// ---------------------------------------------------------------------------
// Session quality
// ---------------------------------------------------------------------------

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
        id: ActivityId::new(),
        account: runalytics_core::ProviderAccountId::new(),
        provider_activity_id: "fixture".into(),
        name: "fixture run".into(),
        started_at: Timestamp::default(),
        local_date: date(2026, 10, 6),
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

/// Interval 6x800 m with float recovery: 15' warm-up, 6 x (3' hard / 2' easy),
/// 10' cool-down. A real, common quality session.
fn interval_session() -> PlannedSession {
    let mut workout = StructuredWorkout {
        blocks: vec![WorkoutBlock::new(
            BlockTarget::Warmup,
            DurationSecs::from_minutes(15),
        )],
    };
    for _ in 0..6 {
        workout.blocks.push(
            WorkoutBlock::new(BlockTarget::Hard, DurationSecs::from_minutes(3))
                .with_pace(Pace::new(225.0))
                .with_hr_ceiling(HeartRate::new(180), HrZone::Z4),
        );
        workout.blocks.push(WorkoutBlock::new(
            BlockTarget::Recovery,
            DurationSecs::from_minutes(2),
        ));
    }
    workout.blocks.push(WorkoutBlock::new(
        BlockTarget::Cooldown,
        DurationSecs::from_minutes(10),
    ));
    PlannedSession {
        id: PlannedSessionId::new(),
        date: date(2026, 10, 6),
        start: TimeOfDay::from_hms_opt(7, 0, 0).expect("time"),
        kind: SessionKind::Intervals,
        title: "Interval 6x800m".into(),
        intent: "VO2max".into(),
        workout,
        target_volume: VolumeKm(11.0),
        target_duration: DurationSecs::from_minutes(55),
        // `target_pace` is the prescribed *hard* pace, which is what plan-engine
        // sets for an interval session — not the whole-session average, which
        // includes the warm-up and the floats.
        target_pace: Some(Pace::new(225.0)),
        rpe_target: Some(8),
        quality: true,
        external_id: None,
    }
}

/// The session executed as prescribed: reps at 3:45/km at Z4 heart rate, jogs
/// genuinely easy, whole session averaging out to the planned duration.
fn executed_intervals() -> Activity {
    let mut a = activity(55, 11.0, 300.0, 152, 178);
    a.laps = vec![
        lap(15, 125, 3.4), // warm-up
        lap(3, 172, 0.80), // rep 1
        lap(2, 132, 0.40), // float
        lap(3, 173, 0.80),
        lap(2, 133, 0.40),
        lap(3, 174, 0.80),
        lap(2, 134, 0.40),
        lap(3, 175, 0.80),
        lap(2, 133, 0.40),
        lap(3, 174, 0.80),
        lap(2, 132, 0.40),
        lap(3, 173, 0.80),
        lap(2, 131, 0.40),
        lap(10, 122, 2.6), // cool-down
    ];
    a
}

/// The same session with only 60 % of the prescribed zone work: two reps short,
/// and the remaining reps run below threshold. Same duration, same distance — the
/// shape of the session is what changed.
fn diluted_intervals() -> Activity {
    let mut a = activity(55, 11.0, 300.0, 141, 176);
    a.laps = vec![
        lap(15, 125, 3.4), // warm-up
        lap(3, 171, 0.80), // only two real reps
        lap(2, 132, 0.40),
        lap(3, 172, 0.80),
        lap(2, 133, 0.40),
        lap(3, 138, 0.70), // the rest drift well below threshold
        lap(2, 134, 0.40),
        lap(3, 136, 0.70),
        lap(2, 133, 0.40),
        lap(3, 135, 0.70),
        lap(2, 132, 0.40),
        lap(3, 134, 0.70),
        lap(2, 131, 0.40),
        lap(10, 122, 2.6),
    ];
    a
}

/// Verification step 3, second half: a fully adhered interval session scores at
/// least 90, the same session with 60 % of the zone work scores below 65.
///
/// The gap is the whole point of the quality score. Duration and distance are
/// identical in both activities, so any app that scores sessions on volume
/// reports these two runs as the same session — and an athlete who skipped the
/// main set gets a green tick for it.
#[test]
fn an_executed_interval_session_scores_high_and_a_diluted_one_does_not() {
    let planned = interval_session();
    let ath = athlete();
    let threshold = Pace::new(255.0);

    let done = score_session(
        &planned,
        &executed_intervals(),
        &SessionFeedback {
            rpe: Some(8),
            feeling: Some(4),
            note: None,
        },
        &ath,
        threshold,
    );
    let diluted = score_session(
        &planned,
        &diluted_intervals(),
        &SessionFeedback {
            rpe: Some(6),
            feeling: Some(3),
            note: None,
        },
        &ath,
        threshold,
    );

    assert!(
        done.score >= 90.0,
        "a fully adhered interval session scored {} (components {:?})",
        done.score,
        done.components
    );
    assert!(
        diluted.score < 65.0,
        "a 60%-zone session scored {} (components {:?})",
        diluted.score,
        diluted.components
    );
    assert!(done.executed_quality, "the real session is quality work");
    assert!(!diluted.executed_quality, "the diluted one is not");
    assert!(done.confidence > 0.9, "full data means high confidence");

    // The verdict sentence must name the actual weakness, which is the only
    // part of this score an athlete can act on.
    let verdict = runalytics_scoring::verdict(&diluted.components);
    assert_ne!(verdict, "");
}

/// A session cannot certify itself: the same diluted run reported as a 10/10
/// effort must not be promoted to quality work by self-report alone.
#[test]
fn self_reported_effort_cannot_certify_a_diluted_session() {
    let planned = interval_session();
    let ath = athlete();
    let q = score_session(
        &planned,
        &diluted_intervals(),
        &SessionFeedback {
            rpe: Some(10),
            feeling: Some(5),
            note: Some("brutal, felt amazing".into()),
        },
        &ath,
        Pace::new(255.0),
    );
    assert!(
        !q.executed_quality,
        "a session below threshold intensity is not quality, whatever it felt like"
    );
}
