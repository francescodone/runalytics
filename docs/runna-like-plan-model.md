# Training Method Selection and Runna-Like Plans

## Scope

Optional session feedback and the Edit plan easier/harder controls apply to both
training methods. They adjust future tempo/interval work lengths at fixed session
distance, not weekly mileage or the runner's threshold estimate. See
[Optional Session Feedback](norwegian-threshold-model.md#optional-session-feedback)
for the shared rating scale, limits, one-shot adaptation and Undo behavior.

Reviewed the [Plan Creation & Management collection](https://support.runna.com/en/collections/16743424-plan-creation-management) and its 27 linked articles on 2026-11-03, plus the linked recovery article. These are public product/coaching descriptions, not a specification of Runna's proprietary generator. The implementation is an independent Runalytics heuristic. It does not claim identical workouts, equivalent outcomes, medical validation, or affiliation with Runna.

New plans can select **Norwegian method** or **Runna-like**. The method is independent of the existing race, improvement, and maintenance goals. Norwegian remains the default for omitted request fields, existing database rows, and older sync payloads. Previously stored workouts are not regenerated when the app upgrades.

## Implemented Rules

| Concern | Public guidance | Runalytics implementation |
| --- | --- | --- |
| Starting point | Current mileage, recent performances, training experience, and longest run inform setup. | Reuse the explicit weekly mileage or synced-history baseline. Pace comes from the existing threshold/evidence system. Long-run starting distance is estimated from weekly mileage; there is no new longest-run or ability questionnaire. |
| Progression | Mileage and long-run progression are separately constrained. More experience does not automatically permit faster build rates. | Reuse the bounded geometric weekly ramp: the chosen 5/8/12% ceiling and an absolute 4 km ceiling, with a distance/frequency target. Runna-like long runs have a separate build ceiling of 10% or 2 km, whichever is smaller. Distances round up to whole kilometres. |
| Workout variety | Intervals, tempo, easy runs, and long runs vary with training focus and goal. | Norwegian retains controlled sub-threshold repetitions. Runna-like uses the existing base/build/peak rotation of short/long intervals, cruise, continuous tempo, progressive, ladder, and rolling sessions. Longer race goals emphasize threshold/endurance; shorter goals emphasize speed. |
| Recovery | Regular deload weeks reduce demand, especially harder sessions. | Reuse three build weeks followed by a recovery week at 70% of the build-volume baseline. Quality is removed in recovery weeks. |
| Taper | Half marathon and longer, or plans of at least ten weeks, use three weeks; shorter blocks for 10 km or below use two. | Apply those taper lengths to Runna-like race plans, including race week. Reduce quality-work load to 75% in the first week of a three-week taper, then 50%. Taper long runs are easy. Norwegian keeps its previous taper. Improvement and maintenance blocks do not gain a race taper. |
| Intensity and recovery | Training should be sustainable, and pain/illness should reduce demand. | Both methods retain evidence-based quality-duration caps, the easy/hard budget, spacing around quality and long runs, and injury/fatigue/recovery holds. New ambitious Runna-like race/improvement plans request 25% more threshold-equivalent main-set work than moderate plans, before applying the same evidence cap. A second quality day still requires the existing capacity gate. |
| Pace targets | Current fitness determines training pace, not a desired finish time. | Both methods keep the current evidence anchor between assessments. Changing method does not grant an automatic faster threshold or bypass accepted pace-adjustment controls. |
| Frequency | Frequency and preferred long-run day shape scheduling. | Keep the existing 3-7 runs/week input and preferred long-run day. Ambitious Runna-like race/improvement plans can use two quality days from four runs/week during build/peak, provided capacity, mileage, intensity and recovery spacing permit it. A hard long run prevents adding another quality day. Base, recovery, taper and race weeks do not gain a second quality session. Other plans retain the five-run minimum. Runna-like easy runs may be as short as 3 km rather than the Norwegian generator's 6 km floor; sessions that cannot fit the mileage budget are not forced in. |
| Persistence | A plan choice should remain stable across devices. | Store `trainingMethod` on the request/plan, in SQLite, and on the existing authenticated, opt-in plan-sync channel. Calendar/watch steps reconstruct from the stored method. No new account, telemetry, or third-party data sharing is introduced. |

The numerical progression, recovery, session-size, and quality-load values above are local engineering/coaching choices unless explicitly attributed. Runna does not publish enough detail to reproduce its mileage curves or workout library. The independently capped long-run ceiling applies during build weeks; recovery and taper reductions do not lower the retained build baseline.

## Quality Limits and Compatibility

A fast threshold pace is not evidence of high training-load tolerance. Unverified capacity retains a 15-minute threshold-equivalent main-set cap and does not authorize a second quality day. Repeated overshoots, a recent threshold-load spike, or recovery longer than 48 hours can reduce the cap to 12 minutes. Injury, high fatigue, poor recovery, or sleep below six hours retain easy-only training. The Runna-like plan receipt now reports the cap and its limiting evidence.

For example, at a 3:46/km threshold, a 12-minute load cap reproduces a 3.3 km progressive main set: 1 km at 4:06-4:16/km, 1 km at 3:51-4:01/km, and 1.3 km at 3:41-3:51/km. This is about 13 minutes of running but 12 threshold-equivalent minutes, because the opening blocks are slower than threshold. It is reduced work, not a full sustained threshold session or an incorrect pace anchor.

The wearable-history collector does not currently populate all experience and recovery inputs required for high capacity. Missing values are not fabricated and neither ambition nor weekly mileage alone bypasses that gate. Selecting ambitious therefore does not guarantee two quality sessions or a larger main set when evidence is limiting.

The new load allocation is identified by persisted athlete-state version 2. Version-1 and legacy plans retain their existing reconstructed prescriptions, including completed workouts; upgrades do not regenerate schedules. Norwegian and maintenance load behavior remain unchanged. New allocation requires a newly generated race/improvement plan and sufficient capacity evidence, not merely reopening an existing plan.

## Deliberate Limits

- Keep Runalytics' four distances (5 km, 10 km, half marathon, marathon), existing minimum horizon, and 16-week COROS limit on non-race blocks. This change does not adopt Runna's full plan catalogue or general 26-week horizon.
- Do not introduce beginner walk/run, post-injury, postpartum, or post-race recovery programmes. A method selector is not medical clearance or a replacement for those programmes.
- Do not automatically add races. Existing benchmark scheduling remains an explicit action. The Runna-like taper excludes the engine's old automatic marathon tune-up pattern.
- Preserve existing calendar moves, skips, resets, plan-abort confirmation, watch export, and offline generation. No holiday/illness wizard, automatic missed-week realignment, multi-plan queue, or draft-plan lifecycle is added.
- Strength, yoga, Pilates, and other cross-training remain outside this running-plan generator. The articles describe separate workflows, not additional running mileage.
- No exact race-result improvement is promised. Training plans cannot eliminate injury risk.

## Source Index

Original summaries of all 27 collection articles, grouped by topic:

| Article | Relevant takeaway |
| --- | --- |
| [Create a training plan](https://support.runna.com/en/articles/15443877-how-to-create-a-training-plan-in-runna) | Separate race, distance, general fitness, beginner, and specialist goals. Setup combines current fitness with availability. |
| [Build around current fitness](https://support.runna.com/en/articles/15231838-how-does-runna-build-your-training-plan-around-your-current-fitness) | Start from actual capability; independently constrain weekly mileage and long-run increases. |
| [Mileage calculation](https://support.runna.com/en/articles/14666572-how-runna-calculates-your-mileage-and-how-to-adjust-it) | Mileage depends on ability, current training, plan duration, frequency, and preferences, not race pace alone. |
| [Training preferences](https://support.runna.com/en/articles/10393191-how-to-use-training-preferences) | Volume and workout difficulty are distinct controls; fewer repetitions need not change target pace. |
| [Training focus](https://support.runna.com/en/articles/11598487-how-to-choose-your-training-focus) | Speed favors intervals; endurance favors longer running and threshold work; balanced training mixes both. |
| [Running schedule](https://support.runna.com/en/articles/6206024-adjusting-your-running-schedule) | Frequency and long-run day should fit current habits and available recovery. |
| [Running ability](https://support.runna.com/en/articles/6205993-adjusting-your-running-ability) | Ability reflects running experience and load tolerance rather than only speed. |
| [Race times and pace targets](https://support.runna.com/en/articles/6205998-adjusting-your-estimated-race-time-and-pace-targets) | Use current performance to prescribe pace, not an aspirational finish time. |
| [Training units](https://support.runna.com/en/articles/6206133-adjusting-your-training-units) | Distance units and pace-versus-effort presentation are preferences, not reasons to inflate training load. |
| [Deload weeks](https://support.runna.com/en/articles/15013028-why-does-my-new-to-running-plan-go-down-in-distance-a-guide-to-deload-weeks) | Planned reductions are part of progression, not missed progress to make up. |
| [Beginner/return-plan difficulty](https://support.runna.com/en/articles/15012974-how-to-adjust-your-running-plan-s-difficulty-new-to-running-path-to-parkrun-and-return-to-running) | Beginner and returning runners need separate effort-based progression and duration limits. |
| [B-races](https://support.runna.com/en/articles/10856802-how-to-add-and-manage-your-b-race) | Secondary races replace stress rather than stack on it, and need recovery outside the main taper. |
| [Extra, club, and parkrun sessions](https://support.runna.com/en/articles/6206280-how-can-i-adjust-my-plan-to-add-extra-runs-parkrun-and-club-runs) | Match replacement effort to the planned session and protect recovery around extra runs. |
| [Training calendar](https://support.runna.com/en/articles/10137793-how-to-use-your-training-calendar) | Single-session moves differ from permanent schedule changes; avoid stacking planned runs. |
| [Plan realignment](https://support.runna.com/en/articles/10026375-how-to-use-the-plan-realignment-feature) | After substantial missed training, rebuild from current fitness rather than cram missed work into later days. |
| [Skipping sessions](https://support.runna.com/en/articles/15012850-how-and-when-to-skip-a-run-managing-missed-sessions-in-your-training-plan) | Skipping and rescheduling have different meanings; occasional omissions do not justify catch-up overload. |
| [Pause options](https://support.runna.com/en/articles/8895444-need-to-pause-your-plan) | Illness and breaks call for lower demand and a gradual return. |
| [Holidays](https://support.runna.com/en/articles/10225691-how-to-adjust-your-plan-around-holidays) | Temporary availability changes should alter load and return progression, not just hide sessions. |
| [Not feeling 100%](https://support.runna.com/en/articles/13531498-how-to-use-not-feeling-100) | Temporary reductions range from easier sessions to no running, followed by staged return. |
| [Minor aches and pains](https://support.runna.com/en/articles/13225357-adjusting-your-plan-around-minor-aches-and-pains) | Respond early with reduced demand; persistent/worsening pain requires professional advice. |
| [Race/end-date changes](https://support.runna.com/en/articles/14666402-how-to-change-your-race-date-or-plan-end-date) | Moving the goal date should rebuild progression and taper, not merely move the race workout. |
| [Between race blocks](https://support.runna.com/en/articles/14666749-which-plan-should-i-choose-between-race-blocks) | Maintenance and base-building are distinct from a fresh race build; recover after racing. |
| [Activity logging/linking](https://support.runna.com/en/articles/14666681-how-to-log-link-or-manually-add-an-activity) | Recorded history and scheduled prescriptions are separate; linking does not itself rebuild the plan. |
| [Current, upcoming, and draft plans](https://support.runna.com/en/articles/15586762-managing-your-current-upcoming-and-draft-plans) | Resolve overlaps explicitly and preserve history; drafts should not silently replace current training. |
| [Strength training](https://support.runna.com/en/articles/15624879-adding-strength-training-to-your-runna-plan) | Strength is a separately configured supporting programme, outside adaptive running mileage. |
| [Yoga, Pilates, and stability](https://support.runna.com/en/articles/15623522-adding-yoga-pilates-and-stretch-stability-to-your-runna-plan) | Supporting mobility sessions complement running without replacing or increasing prescribed mileage. |
| [Cross-training](https://support.runna.com/en/articles/15899152-how-can-i-incorporate-cross-training-in-runna) | Different sports need different load models; do not equate their distance with running mileage. |

Additional direct source: [How Runna builds recovery](https://support.runna.com/en/articles/15272605-how-does-runna-build-recovery-into-your-training-plan), which supplies the published taper-length distinction and describes independent long-run protection.

## Verification

Backend tests cover omitted/invalid methods, distinct session families, retained injury holds, bounded long-run build, taper quality reductions, and method round-trips through sync, insertion, replacement, listing, and database reopening. Browser tests use synthetic IPC and cover keyboard choice, descriptions, review, Back navigation, failed-submit retention, request payload, and active-plan display in light/dark at 320, 390, 768, and 1440 px. These are not native watch/device integration tests.

Results on 2026-11-03:

- Production build passed. All five focused backend method tests passed; all 13 focused browser tests passed, including a final rerun against the current shared working tree. Chooser screenshots were inspected at phone and desktop sizes.
- Full backend suite: 195 passed, one failed, two ignored. The failure is `calendar::tests::quality_sessions_are_named_and_easy_steps_have_no_upper_bound`, the calendar wording assertion already excluded by the repository's existing race-integration verification task.
- Full browser suite: 242 passed, 28 failed in motion, notifications, palette rules, brand positioning, legacy preview expectations, and week navigation. Concurrent edits/HMR occurred in shared UI files during that run, so these results are not a controlled baseline comparison. No full-suite-green or native-device claim is made.