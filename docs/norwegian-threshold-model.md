# Norwegian-Inspired Threshold Engine

New plans default to this method and can instead select the independent [Runna-like heuristic](runna-like-plan-model.md). The selected method is stored with the plan; the shared evidence and recovery safeguards apply to both.

## Purpose and Evidence

### Optional Session Feedback

Completed tempo, interval, benchmark and long-run sessions can receive an optional
rating relative to their prescription: 1 much too easy, 2 slightly easy, 3 about
right, 4 slightly hard, or 5 much too hard. Easy runs and ordinary races are not
rated. Recent newly completed recordings receive a nonblocking reminder; Not now
suppresses it without recording a score. Existing recordings are not backfilled.

The first non-neutral submission requests a small change to future tempo/interval
work only. Ratings can subsequently be edited or cleared without applying another
adjustment. Training > Plan > Edit plan offers the same easier/harder actions
manually. Neither action changes weekly mileage, session dates, run frequency,
long-run targets, benchmark targets, or the athlete's threshold estimates.

Adjustments resize work in 100 m increments, exchanging work metres with easy
warm-up metres within each session. They target approximately 5% of quality load,
allow at most 10% for a rounded step, and stay within 80%-120% of the original
quality prescription. A single repetition can be adjusted when changing every
repetition would be too large. Pace and recovery targets stay unchanged. Minimum
warm-up/cooldown distances, quality-time caps and harder-change weekly intensity
limits remain enforced. Harder changes also require suitable pain, recovery and
training-load evidence and do not increase recovery-week or taper work. Missing
load evidence blocks harder changes, not feedback submission.

Distance rounding and safety limits can leave some or all sessions unchanged;
the action reports that result. Undo restores only unchanged, still-future,
uncompleted sessions and does not erase the feedback or rearm its automatic
adjustment. Adjusted steps and feedback survive plan sync and restart. Both
devices must support plan wire version 4; older clients refuse the new format.
Subjective ratings are not included in calendar descriptions or watch workouts.

This is a conservative individualized training heuristic, not an implementation of an elite athlete's schedule, a lactate meter, an injury diagnosis, or a claim that Norwegian training is universally superior. Its scores and policy cutoffs are engineering/coaching assumptions, not clinically validated probabilities or physiological laws.

The useful principle is repeatable work at controlled internal intensity, supported by substantial easy running, recovery, consistency and longitudinal measurement. Double-threshold days are one high-load organizational strategy, not the definition of the method.

| Evidence category | Interpretation in this engine |
| --- | --- |
| Scientific evidence | Endurance training research supports individualized intensity and a substantial low-intensity component. Comparative intensity-distribution results depend on population, periodization and measurement method. |
| Observed elite practice | Some Norwegian runners combine frequent lactate-guided intervals with roughly 150-180 km/week and extensive recovery support. Their success does not establish causality or recreational safety. |
| Coaching methodology | Repeatable sessions, controlling rather than chasing pace, individual lactate responses, and hard/easy separation inform the policy. |
| Plausible rationale | Avoiding excessive metabolic strain may permit more repeatable work. Proposed molecular mechanisms do not prove that doubling sessions is better. |
| Anecdote | Individual training logs can generate hypotheses; they cannot calibrate a universal dosage. |
| Social-media simplification | Neither "double threshold for everyone" nor universal 2/3/4 mmol/L targets are accepted. |

Sources checked for this implementation:

- Casado, Foster, Bakken and Tjelta (2023), [Does Lactate-Guided Threshold Interval Training within a High-Volume Low-Intensity Approach Represent the Next Step in the Evolution of Distance Running Training?](https://www.ebi.ac.uk/europepmc/webservices/rest/PMC10000870/fullTextXML), DOI 10.3390/ijerph20053782. Sections 4, 6 and 7 distinguish thresholds, describe elite practice and explicitly limit the review to observational studies/reports. Its applicability beyond the studied athletes, including marathon runners, is uncertain. Its numerical examples are not universal zone boundaries.
- Stoggl and Sperlich (2015), [The Training Intensity Distribution among Well-Trained and Elite Endurance Athletes](https://www.frontiersin.org/journals/physiology/articles/10.3389/fphys.2015.00295/full), DOI 10.3389/fphys.2015.00295. Reviews experimental and retrospective evidence, methodological differences, and individual variation. It does not identify a universally optimal distribution.

These are a bounded evidence review, not an exhaustive systematic review of all research through 2026. The engine's exact confidence caps, sampling tolerances, scoring weights and eligibility gates have not been validated by these papers.

## Threshold Definitions and Estimation

`lt1Estimate` represents an athlete-specific first lactate/aerobic threshold. `lt2Estimate` represents a second lactate/anaerobic threshold estimate. Each can contain pace (seconds/km), HR (beats/min), power (watts), lactate (mmol/L) and post-repetition sampling delay (seconds). Every estimated channel contains `value`, `confidence` (0-1), `relativeUncertainty`, and `source`. These describe evidence quality and a heuristic relative error allowance, not a statistical confidence interval.

There is deliberately no third, ambiguous `lactateThreshold` scalar, or duplicate threshold pace/HR/power state: those channels live under LT1/LT2. LT2, maximal lactate steady state and critical speed/power are related but not interchangeable measurements. LT1 is not fabricated as a percentage of LT2 or maximum HR.

| Lactate availability | Behavior |
| --- | --- |
| Regular testing | Consume dated athlete-specific LT1/LT2 interpretations and the associated sampling protocol. Preserve each channel's confidence and uncertainty. Raw absolute concentrations alone do not locate either breakpoint. A qualified physiological test interpretation is required; the engine does not fit arbitrary breakpoints to a few samples. |
| Occasional testing | Retain the profile with confidence decaying by a 56-day half-life, increase relative uncertainty, and reject observations older than 180 days. A fresh lower-confidence wearable observation can replace a sufficiently stale channel. Retesting is preferable to assuming the old lactate-to-HR relationship never changes. |
| No lactate | Use a watch-reported threshold, a recent maximal race converted to a one-hour pace proxy, or the requested pace as a low-confidence fallback. Never synthesize lactate concentrations. HR/power proxies require at least three distinct recent, repeatable pace/RPE-conditioned threshold sessions. Missing corroboration leaves them unknown. |

Watch anchors are capped at 0.55, field-test anchors at 0.65 and lactate-test anchors at 0.90 before aging. Race pace proxies start at 0.40; unverified requested pace at 0.30. Pace-only session classification is capped at 0.45; classification without usable lactate cannot exceed 0.65. Confounded sessions have confidence halved; flagged sensor failures provide no classification evidence. More repetitions do not count as independent physiological modalities.

Critical speed and power use two recent maximal performances, 3-20 minutes long, separated by at least a twofold duration difference and no more than 21 calendar days, under matched conditions. The distance-time slope estimates critical speed; work-time slope estimates critical power. Positive reserve/intercept checks reject nonsensical fits. Two-point estimates remain low-confidence (0.50), do not replace LT2, and are not inferred from ordinary workout labels. Additional maximal tests and fit diagnostics would be needed for a stronger estimator.

`continuousThresholdDurationLowerBoundMin` is the longest observed controlled repetition, not an assumed one-hour endurance limit. `thresholdDurationTolerance` measures accumulated work in a controlled session. Neither is a validated time-to-exhaustion prediction.

## Session Response and Lactate

Each repetition can supply duration, recovery, pace, HR, power, RPE (0-10), talk-test comfort, lactate, and sampling delay. Unknown channels stay null. HR is not used to locate intensity in repetitions shorter than three minutes because of response lag. Repetition/recovery duration also determines what comparisons are admissible; pace alone is not treated as a metabolic measurement.

`intendedSessionType` and `observedSessionType` remain separate: `SUB_THRESHOLD`, `THRESHOLD`, `HIGH_THRESHOLD`, `ABOVE_THRESHOLD`, or `UNCERTAIN`. Above-threshold does not assert that VO2max itself was measured. The intended label never forces the observed classification.

The intensity estimate is a conservative ratio-based warning index relative to individual LT2 channels. A discordant high signal is retained rather than averaged away. Numeric ratio boundaries are heuristic flags, not sharp biological borders. Inspect the associated confidence and conditions before interpreting a domain.

Lactate comparisons distinguish:

- Absolute concentration: `lactateMeanMmol` is a descriptive reading, not a universal threshold.
- Change within a session: `lactateDeltaMmol` is the last-minus-first sample when timing is comparable. Both absolute and proportional increase are required for the accumulation penalty.
- Accumulation: a rise greater than 1 mmol/L and 25% at matched timing triggers a warning, not a universal diagnostic criterion. Workload and recovery changes must still be considered.
- Response at standardized pace: `lactateAtStandardizedPace` conditions samples on the athlete's reference workload.
- Workload at standardized lactate: `paceAtStandardizedLactate` and `powerAtStandardizedLactate` condition workload on the athlete's own reference concentration.

Sampling delay must match the reference within 15 seconds for lactate-based intensity. Unmatched/unknown delays cannot increase classification confidence. Raw concentrations without a profile may still flag accumulation, but cannot establish a threshold. Recent comparable sessions supply longitudinal context rather than assuming an identical concentration has the same meaning between athletes.

`thresholdControl` is a 0-100 score. It penalizes intensity overshoot, workload escalation or deterioration, HR progression at stable pace/power, rising lactate, high RPE, difficult breathing and increasing recovery requirements. Incomplete work, delayed post-session recovery or deteriorating subsequent performance further reduce it. Below 65 flags overshooting. Each additional recent overshoot after the first lowers aggregate control by another ten points. Fast repetitions are not automatically good performances.

The model can analyze 10 x 3 minutes, 5 x 6 minutes or 3 x 10 minutes as 30 minutes of work while preserving differences in HR latency, recovery and observed response. Equal work duration alone does not prove equal stimulus. Generated controlled intervals use 3-, 6- or 10-minute nominal repetitions, rounded down to distance-based steps for existing watch exports, with approximately one-minute easy recoveries. Low/unknown capacity starts with short repetitions and a smaller total budget, not an elite 30-minute prescription by default.

## Load, Capacity and Adaptation

Threshold load is separate from distance: session load is `2 * workMinutes * intensityRatio^3`, with the ratio bounded to 0.5-1.5. These are arbitrary load units, not TSS, TRIMP or a measured physiological cost. Unknown intensity contributes no quantified load; zero recorded load is not proof of no exposure. The report includes observed threshold-family minutes over 7/28 days, sessions in 7 days, 7-day load, load trend against the previous three weeks, recovery hours, control and tolerance. Above-threshold/overshot exposure is included conservatively rather than disappearing from the load budget.

The capacity gate combines aerobic context (easy-time share), current/historical volume, consistency, training age, threshold experience, session frequency, sleep, injury and recovery. Missing required capacity evidence yields `UNKNOWN`, not permission to increase density. Low or impaired recovery is `LOW`. Complete but insufficient history is `MODERATE`.

For the ordinary high-tolerance gate, current defaults require at least a year of training, eight consistent weeks, 35 current/30 historical km/week, four sessions/week, three months' threshold experience, at least 80% easy time, no known injury/fatigue, adequate sleep/recovery and controlled sessions on at least three distinct days. Current volume must not exceed historical volume by more than 10%. These conservative gates are not population-derived physiological limits.

Double-threshold tolerance uses stricter defaults: three years of training, 24 consistent weeks, 80 current/historical km/week, eight weekly sessions, twelve months' threshold experience, six distinct controlled days, recovery time availability and no reported injury in the last year. Performance level alone neither qualifies nor disqualifies an athlete. Even `HIGH` with its confidence score is not approval to perform doubles. The scheduler continues to allow only one run per day.

New plan policy:

- Preserve the existing bounded mileage ramp, recovery weeks and taper. Do not import elite mileage targets or training frequency.
- Hold threshold pace between assessments rather than applying a calendar-driven fitness gain. A requested faster pace does not override a slower evidence anchor.
- Use controlled threshold intervals, initially capped at 15 minutes when capacity is unknown. Proven tolerance can permit at most 30 minutes and at most 5% above the largest recent controlled session.
- Two recent overshoots, a threshold-load increase greater than 20%, or recovery longer than 48 hours reduce the cap to 12 minutes. Repeated overshooting does not earn faster targets.
- A second quality day requires proven ordinary threshold tolerance, stable density and observed recovery within 36 hours. It is on a separate day, spends the existing easy/hard budget, and can be race-specific faster work rather than another threshold session.
- Injury, high fatigue, poor recovery or severe sleep restriction suppress quality and test/race prescriptions. Reassess with the athlete rather than treating this as medical clearance to keep running.
- After wearable sync, the existing soften-only adaptive path can replace the next eligible quality session with easy running for repeated overshooting, rising threshold density or recovery warnings. It never retrospectively changes completed prescriptions or automatically raises intensity.

Eight efficiency outputs are supported when their input channels exist: threshold pace/power at standardized HR; pace/power at standardized lactate; HR/lactate at standardized pace; accumulated duration tolerance; and session stability/control. Missing channels return null. Workload matching uses a 2% neighborhood, lactate matching 10%, not extrapolation far outside observed data.

Fitness trend is `IMPROVING`, `STABLE`, `DECLINING` or `UNCERTAIN`. It requires at least two matched comparisons on distinct recent days and distinct baseline days, separated by 14-42 days, with equivalent repetition counts, durations and recoveries. Temperatures must match within 3 C, climbing within 3 m/km and altitude within 200 m. Both sessions must explicitly be non-fatigued, non-tapered and sensor-valid. Missing environmental/fatigue context prevents an adaptation verdict. Changes below 2% are treated as stable; mixed directions remain uncertain. This observational association does not establish that threshold training caused the adaptation. Race/benchmark evidence informs pace proxies, but one good race does not establish a longitudinal trend.

## Backend Contract and Limits

`create_plan` accepts its existing `request` plus optional `thresholdEvidence`. `threshold_report` accepts optional `thresholdEvidence` and computes a read-only current report. Both merge supplied observations/sessions/performances with wearable evidence. Supplied capacity cannot erase recorded pain, replace observed running volume/frequency, or conceal shorter recorded sleep. Supplemental raw evidence is not stored separately; only the derived plan snapshot is persisted.

Evidence fields are camelCase. Dates are ISO dates. Observation sources are `LACTATE_TEST`, `WATCH` or `FIELD_TEST`; source text inside individual estimates should name the test method. A `ThresholdPoint` accepts `paceSKm`, `hr`, `powerW`, `lactateMmol`, and `lactateDelayS`. `Evidence` accepts `observations`, `sessions`, `performances`, and `capacity`; the Rust serializable types are the authoritative contract.

Current sync supplies watch threshold pace, activity pace/track HR, climbing and sleep/HRV/resting-HR history. Matched quality sessions use the existing unscaled completed-lap reader. Standalone sessions can supply low-confidence pace-conditioned splits without trusting their label. Shortened or missing tracks are not scaled up to make repetitions appear complete. Matched recovery durations not exposed by that reader remain unknown. Wearable physiological warning trends require several recent and baseline health records.

Lactate, running power, RPE, talk-test, temperature, altitude, explicit training age, recovery-time availability and full injury history are not currently imported by the wearable adapters. They can be supplied through optional backend evidence; there is no new clinical-data entry screen or claim of automatic blood-lactate estimation. Missing these data normally prevents high-tolerance and confident adaptation classifications. Missing sleep/HRV is not treated as evidence of good recovery.

Every new plan stores a versioned `thresholdState` so calendar reconstruction and watch export use the same policy. Legacy plans without it keep their previous reconstruction behavior. Database replacement and opt-in encrypted cross-device plan sync preserve the snapshot. The snapshot contains derived summaries, not raw GPS or lactate samples; it follows the existing plan deletion lifecycle and does not enable a new external data destination. New medical data collection or clinical deployment would require a separate privacy/legal review.

This change does not automatically raise an active plan's threshold pace, optimize elite doubles, correct unknown weather, identify laboratory breakpoints from raw curves, validate sensors, or infer maximum sustainable duration. Those require better evidence and, where appropriate, qualified coaching/physiological assessment. The model should be calibrated against athlete-specific longitudinal outcomes before increasing the allowed density.

## Verification

Run `cargo test --manifest-path src-tauri/Cargo.toml --lib plan:: --offline` for threshold, pacing, session and generation regressions. The sync suite tests both JSON and database snapshot round-trips. The broader repository verification task also builds the frontend and exercises benchmark, plan creation, pacing and schedule workflows.