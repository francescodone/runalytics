# Runna feature parity — Runalytics

Logic, state model, plan engine, insights and integrations. UI/UX lives in
[`RUNNA-UIUX-PARITY.md`](./RUNNA-UIUX-PARITY.md); the ID scheme, scoring legend
and the deliberate-divergence list live in [`README.md`](./README.md).

Every "Runalytics today" line was re-checked against the code at HEAD `0e77402`
(v0.14.0) on 2026-10-27. Status: **OPEN** gap confirmed · **PARTIAL** some of it
exists · **CLOSED** already done (kept only where load-bearing for another item).

---

## 1. State model — the keystone

### F-01 · `planned_date` + `WorkoutStatus` (the one schema change that unlocks the ladder)

- **Runna**: every workout keeps its generated date beside the edited one, and a
  three-state status (planned / skipped / missed, where *missed* is derived)
  powers Realignment, deviations, and the adherence view (`mgmt §10.1`, `exec §10.1`).
- **Runalytics today** — **CLOSED (2026-10-27)**: `PlanWorkout` carries
  `planned_date` and a `WorkoutStatus` (`planned`/`skipped`, `missed` derived at
  read) — `planned_date`/`status` in `src-tauri/src/types.rs`, migration +
  `is_moved()` in `src-tauri/src/store.rs`. `move_workout` keeps `planned_date`
  immutable and sets `moved`, so a deviation is visible, resettable, and auditable.
- **Change**: add `planned_date TEXT` alongside `date`, and a `status` column
  (`planned` / `skipped`; `missed` derived at read = date passed, not done, not
  skipped). Keep `done` as a compatibility view over `status != planned` for one
  release, then drop it.
- **Desktop**: makes Reset (F-02), the moved marker (U-10), adherence (F-07) and
  Realignment (F-06) all buildable.
- **Mobile**: same column; iOS is pull-only (`config.can_publish = !mobile()`,
  `src-tauri/src/plan_sync.rs:603`), so it consumes the status without writing it.
- **Effort**: M · **Depends**: — · **Impact**: 5

### F-02 · Reset a week back to the plan

- **Runna**: `⟳ Reset` appears only once a deviation exists and returns the week
  to the generated dates (`exec §7`, `exec §10.1`).
- **Runalytics today** — **CLOSED (2026-10-27)**: `week_deviations` +
  `reset_week` commands (`src-tauri/src/calendar.rs`) reuse the move guards (never
  race day, never completed); a confirm sheet lists each day it will move back and
  marks non-restorable sessions. Desktop: header button per week. Mobile: the same
  control sits in the agenda week header (`src/lib/schedule.ts
  openResetWeekSheet`), verified at 390×844.
- **Change**: a `reset_week(week_start)` command restoring `date = planned_date`
  for that ISO week (never for race day, never for completed sessions — reuse the
  `move_workout` guards), plus a header button that renders only when
  `date != planned_date` exists in the visible month.
- **Desktop**: button in the calendar header; confirm sheet lists the days it will
  move back.
- **Mobile**: the grid is hidden ≤560px (`src/styles/app.css:5430`), so the reset
  control belongs in the agenda list header, not the grid.
- **Effort**: M · **Depends**: F-01 · **Impact**: 4

### F-03 · Undo a move + a moved marker (finish the reschedule chain)

- **Runna**: lift → loud drop target → moved marker → toast + Undo → conditional
  Reset. Runalytics has the first two and nothing after the drop (`exec §10.1`).
- **Runalytics today** — **CLOSED (2026-10-27)**: the full chain exists.
  `move_workout` guards unchanged; the drag keeps its ghost chip + `.drop-target`;
  after a drop `moveSession()` (`src/lib/schedule.ts`) raises a toast with **Undo**
  (moves back to the date the frontend already holds); a moved day carries the
  `.moved-day` corner-fold marker (U-10); and the touch route is the **"Move to…"**
  action in the workout sheet (`openMoveSheet`), which is the whole mobile
  reschedule path — verified at 390×844 in the calendar harness (refusal and
  success paths both exercised).
- **Change**: (a) toast with Undo after a drop, using the previous date the
  frontend already holds in `dragged.date`; (b) a small marker on any cell where
  `date != planned_date` (needs F-01); (c) a touch rearrange path — long-press to
  pick up, or a "Move to…" action in the workout sheet, which is the cheaper and
  more accessible route.
- **Desktop**: toast + marker only (drag already works).
- **Mobile**: the "Move to…" action is the whole feature; without it iOS has zero
  reschedule.
- **Effort**: M (S for toast+marker) · **Depends**: F-01 (marker) · **Impact**: 5

### F-04 · The plan receipt (highest value-per-line in the whole analysis)

- **Runna**: before showing a generated plan it echoes the inputs back in second
  person as the stated basis (`gen §5`, `gen §6.2.1`).
- **Runalytics today** — **CLOSED (2026-10-27)**: `src-tauri/src/plan/receipt.rs`
  builds ordered `(input, sentence)` pairs by influence and `plan.html` renders
  them as "What this plan was built from" above the plan — one line per input
  (distance/date, baseline km, threshold, frequency, method, long-run day, shape),
  each naming what it did to the plan. Text block; flows identically at 390.
- **Change**: `plan/receipt.rs` → ordered `(input, sentence)` pairs by influence,
  rendered above the first workout on `plan.html`. Rendering only, no modelling.
- **Desktop**: block above `#plan-view`.
- **Mobile**: same block; it is text, so it just flows.
- **Effort**: S · **Depends**: — · **Impact**: 5

### F-05 · Announce automatic adjustments

- **Runna**: a `✓ Adjusted for your injury history` banner is the trust
  mechanism; an unannounced adjustment reads as a bug (`gen §6.2.3`).
- **Runalytics today** — **CLOSED (2026-10-27)**: the calendar renders a
  positive-framed `#cal-adjusted` banner ("Adjusted for you — recovery score
  38/100. 1 session softened…") in the same frame the softened values render,
  and each softened session carries an `Adjusted` chip plus the reason in its
  detail sheet (`src/calendar.ts`, `src/lib/workout-detail.ts`).
- **Change**: render the reason inline on the affected workout (calendar chip
  tooltip + detail sheet) and as a positive-framed banner, in the same frame the
  softened value appears.
- **Desktop / Mobile**: shared component; on mobile the detail sheet is the
  primary surface since the grid is hidden.
- **Effort**: S · **Depends**: — · **Impact**: 4

### F-06 · Realignment ladder (missed-workout escalation)

- **Runna**: graded 1 / 3+ / 2wk / 4wk ladder; the trigger is the *absence* of an
  action, so there is no UI to build, only a detector (`mgmt §10.1`).
- **Runalytics today** — **OPEN**: no missed detector. The offer machinery exists
  (`proposal.rs` tiers + 7-day dismissal suppression), only the volume/miss trend
  does not.
- **Change**: a detector over F-01's derived `missed` status feeding
  `proposal.rs`'s existing offer path. Adopt Runna's two rules verbatim: announced
  resets, and "the governor outranks the suggestion" (accept fails when the target
  exceeds the ability × run-days range) (`ins §11.2`).
- **Desktop**: proposal card on Plan + a calendar notice.
- **Mobile**: read-only card (iOS cannot publish).
- **Effort**: M · **Depends**: F-01 · **Impact**: 4

### F-07 · Adherence: planned-vs-actual as one ratio

- **Runna**: `9.2 km / 41,0 km` in the week header; Mileage Insights compares
  planned to actual *including* runs not linked to the plan (`exec §10.2`,
  `ins §11.2`).
- **Runalytics today** — **CLOSED (2026-10-27)**: one ratio per ISO week. The
  desktop week strip reads `37.0 / 45.0 km` with `ran / planned` under it
  (`src/calendar.ts renderWeekStats`), and the agenda week header carries the same
  `ran / planned` ratio (`src/lib/schedule.ts`). The plan↔activity join
  (`calendar_completions`, `src-tauri/src/store.rs`) feeds the planned side.
- **Change**: expose `planned_km` / `actual_km` per ISO week from the store;
  present as one ratio (see U-02).
- **Desktop**: header ratio + a small adherence strip on the dashboard.
- **Mobile**: the ratio is the header of the agenda list.
- **Effort**: S · **Depends**: — · **Impact**: 4

---

## 2. Plan generation — the input side

### F-08 · Dated race-time contract (the honest version of Runna's progress table)

- **Runna**: a predicted-race-time table is a *contract* — a dated promise with
  teal deltas — not an accuracy claim; its weakness is a distance-blind target
  column (`exec §4`, `exec §10.3`).
- **Runalytics today** — **PARTIAL**: `plan.ts` already renders three estimates on
  a shared axis (watch / threshold / at-race-day) using a Riegel projection
  (`src/plan.ts:82`, `:162-168`), but the rows are **undated** — there is no
  "by 15 Dec" promise the user can be measured against, and the projection is not
  re-derived from the session evidence `plan/pacing.rs` already stores.
- **Change**: date each row (today / now / race day) and re-derive the
  at-race-day row from `quality_offset_sec_km` / `quality_offset_from`
  (`src-tauri/src/types.rs:325`,`:331`) so the promise tracks real execution.
- **Desktop**: dates + delta under each row of `#pacing-section`'s sibling chart.
- **Mobile**: same rows stack; keep the accessible group label.
- **Effort**: M · **Depends**: — · **Impact**: 4

### F-09 · Pacing: say the ceiling, add the abstain states

- **Runna**: five-state Pace Insights card, including "mixed results" and "not
  enough data"; offset is unbounded because it is expressed as a race-time edit
  (`ins §11.1`).
- **Runalytics today** — **PARTIAL**: `plan/pacing.rs` grades `SessionReview`s into
  `Ahead/InRange/Behind` with `STEP_SEC_KM = 5`, `MAX_OFFSET_SEC_KM = 20`,
  `LOOKBACK = 8` (`src-tauri/src/plan/pacing.rs:39`,`:46`,`:49`). `streak()`
  (`:313`) returns a count
  plus a direction, so a contradictory history and an empty one look identical to
  the user. The ±20 s ceiling is the right call (see divergence #13) but is silent.
- **Change**: (a) surface abstain states — "mixed", "not enough sessions yet";
  (b) one line stating the ±20 s/km bound and why, so a runner genuinely 30 s/km
  faster understands the wall. Keep the ceiling.
- **Desktop**: the Pacing panel's lead sentence.
- **Mobile**: same panel; it is already in the stacked flow.
- **Effort**: S · **Depends**: — · **Impact**: 3

### F-10 · Start date as plan length (candidates, not a number field)

- **Runna**: takes a start date and derives length as `round(days_to_race / 7)`
  from Monday-anchored candidates, each labelled with what it buys
  (`gen §6.2.2`).
- **Runalytics today** — **OPEN** for race/improvement: race builds derive weeks
  from the target date server-side (`plan/engine.rs:401` `first_monday`), but the
  wizard offers no candidate choice; maintenance takes a raw weeks `<select>`
  read client-side by `maintenanceTargetDate()` (`src/plan.ts:251-254`).
- **Change**: offer 2–3 Monday-anchored candidates ("Now · 23 weeks — maximum
  training time" / "In 2 weeks · 21 weeks"), each labelled with the trade.
- **Desktop**: radio row replacing the length control.
- **Mobile**: same radios; they stack cleanly.
- **Effort**: S · **Depends**: — · **Impact**: 3

### F-11 · Separate current from ceiling

- **Runna**: `Weekly Mileage 37km` / `Longest Run 14km` (where you are) stays
  visibly distinct from `Long run max ~19km` (where the plan may go)
  (`gen §6.2.4`).
- **Runalytics today** — **PARTIAL**: the engine computes the ceiling
  (`peak_long_run_km`, `long_run_share`, `plan/engine.rs:34`,`:50`) and the form takes
  current mileage, but the review never shows current beside ceiling.
- **Change**: in the plan preview, print current long run and the plan's ceiling
  side by side.
- **Desktop / Mobile**: one row in the preview card.
- **Effort**: S · **Depends**: — · **Impact**: 3

### F-12 · Announce the long-run governor

- **Runna**: doesn't document its own governor — which is why we should say it
  (`mgmt §10.6`, `mgmt §9.3`).
- **Runalytics today** — **OPEN**: `peak_long_run_km`, `long_run_share` and the
  per-method weekly cap (`IncreaseMethod::weekly_factor()`,
  `src-tauri/src/types.rs:201`, applied at `plan/engine.rs:369`) exist and are never
  explained to the user. The `+8 %` figure is asserted, not stated, in
  `weekly_volume_respects_increase_cap` (`plan/engine.rs:842`, `:866`).
- **Change**: one sentence in the receipt (F-04) and the "how we build your plan"
  panel (F-18).
- **Effort**: S · **Depends**: F-04, F-18 · **Impact**: 2

### F-13 · Name volume presets by temperament

- **Runna**: Gradual / Steady / Progressive — "nobody is choosing to be the lazy
  one" (`gen §6.2.7`).
- **Runalytics today** — **PARTIAL**: the wizard already offers
  Conservative (+5 %/wk) / Moderate / Ambitious (+12 %/wk) as `<option>`s in
  `plan.html:161-163`. "Ambitious" is temperament already; "Conservative" is the one
  that reads as the cautious choice.
- **Change**: rename to Gradual / Steady / Progressive, keep the % hints.
- **Effort**: S · **Depends**: — · **Impact**: 2

### F-14 · Ceilings as the unit of choice

- **Runna**: states each volume control's consequence in km, not percentages or
  adjectives (`gen §6.2.5`).
- **Runalytics today** — **OPEN**: the increase control speaks in %/wk; the
  preview estimates peak km (`updatePlanPreview()`, `src/plan.ts:391`) but the
  control's own hint does not.
- **Change**: each increase option's hint names the resulting peak km for the
  current inputs (the preview math already exists).
- **Effort**: S · **Depends**: — · **Impact**: 3

### F-15 · Behavioural ability levels (only if we ever ask)

- **Runna**: defines self-assessment options by behaviour, never pace or adjectives
  (`gen §6.2.6`).
- **Runalytics today** — **N/A**: we ask no self-assessment question.
- **Change**: none now. Recorded as the rule to follow if a gate ever appears.
- **Effort**: — · **Impact**: 0

### F-16 · Split the volume knob: volume / difficulty / pace-target

- **Runna**: orthogonality promise — "lowering Difficulty may result in fewer
  reps, but the target paces will remain the same" — a testable contract
  (`mgmt §10.5`, `mgmt §6.1`).
- **Runalytics today** — **OPEN**: one `IncreaseMethod` moves peak volume and
  growth rate together.
- **Change**: split into volume and difficulty; add a test that changes difficulty
  and asserts `pace_lo_s_km`/`pace_hi_s_km`
  (`src-tauri/src/types.rs:410-411`) are unchanged.
- **Desktop**: two controls in the wizard where one is now.
- **Mobile**: same, stacked.
- **Effort**: L · **Depends**: — · **Impact**: 3

### F-17 · Pre-flight impact preview on plan mutations

- **Runna**: "Training Disruption: low/medium/high" + per-week Adjustment Insights
  before committing (`mgmt §10.4`).
- **Runalytics today** — **PARTIAL**: the wizard already previews shape
  (weeks/sessions/estimated peak) before generation (`updatePlanPreview()`,
  `src/plan.ts:391`), but *mutations* to a live plan apply silently — that
  preview covers the wizard only.
- **Change**: `plan/engine.rs` is a pure function request→weeks, so evaluate a
  hypothetical and diff against the current plan; show the disruption level +
  per-week deltas before applying.
- **Effort**: M · **Depends**: — · **Impact**: 4

### F-18 · Publish the limits ("how Runalytics builds your plan")

- **Runna**: the help centre states every cap with a reason (26 weeks "to help
  avoid mental and physical burnout", ability-gated run days, minimum 2 runs)
  (`mgmt §10.9`).
- **Runalytics today** — **OPEN**: our caps live in code comments. Settings has
  zone copy (`settings.html:124`) but no planner explainer.
- **Change**: a short panel — levers, what each changes, what each can't.
  Documentation, not engineering.
- **Desktop**: a Settings pane or an expandable on Plan.
- **Mobile**: same pane; it is already a scrollable list.
- **Effort**: S · **Depends**: — · **Impact**: 4

---

## 3. Plan engine rules

### F-19 · Add the absolute term to the ramp

- **Runna**: `min(fixed_step, multiple)` closes "big % jump from a low base"
  (`ins §11.4.1`, `ins §7.3`).
- **Runalytics today** — **CLOSED (2026-10-27)**: the ramp step is now
  `(week_volume * ratio).min(week_volume + MAX_WEEKLY_STEP_KM).min(peak)`
  (`plan/engine.rs`, `MAX_WEEKLY_STEP_KM = 4.0` — the conventional coaching
  ceiling, above every step the reference 2:50 marathon builds needed). The
  receipt's "at most X% a week" line stays true: the absolute cap only lowers
  steps. Pinned by `the_ramp_never_adds_more_than_the_absolute_step`.
- **Change**: cap the step by an absolute km term as well as the ratio.
- **Effort**: S · **Depends**: — · **Impact**: 3

### F-20 · Cut hard sessions harder in a down week

- **Runna**: intensity-weighted reduction, not proportional (`ins §11.4.2`,
  `ins §7.1`).
- **Runalytics today** — **OPEN**: `week_volume * 0.7` is distributed
  proportionally (`plan/engine.rs:409-425`).
- **Change**: take the down-week reduction preferentially from quality sessions.
- **Effort**: M · **Depends**: — · **Impact**: 3

### F-21 · Reconsider frequency scaling

- **Runna**: *decreases the rate* as frequency rises (`ins §11.4.3`, `ins §7.6`).
- **Runalytics today** — **OPEN** (opposite): `peak_weekly_km` *increases* peak
  with frequency, ±7.5 %/session away from 4 days, clamped 0.85–1.3
  (`plan/engine.rs:72`, comment at `:63`).
- **Change**: at minimum, a 6–7 day plan must not also get the fastest ramp.
- **Effort**: S · **Depends**: — · **Impact**: 2

### F-22 · Back-to-back long-run guard

- **Runna**: a documented guard before anything can move a long run
  (`ins §11.4.4`, `ins §7.2`).
- **Runalytics today** — **CLOSED (2026-10-27)**: `move_workout` refuses to place
  a long run adjacent to another long run (`calendar.rs` — "That would put two
  long runs back to back — pick another day"), checked before anything can move
  one. Pinned by `a_long_run_cannot_be_moved_next_to_another_long_run`; the
  refusal surfaces through the move error path on both surfaces (desktop toast,
  mobile sheet status line — both verified in the harness).
- **Change**: add the long-run↔long-run adjacency guard now, before reschedule
  ships.
- **Effort**: S · **Depends**: — · **Impact**: 3

### F-23 · Minimum session floor inside taper

- **Runna**: a floor so taper weeks never produce a token run (`ins §11.4.5`,
  `ins §7.4`).
- **Runalytics today** — **CLOSED (2026-10-27)**: the floor holds by construction
  (easy runs are dropped, not shrunk, when the remainder falls under
  `MIN_RUN_KM`), and the promise is now pinned for every taper session — not just
  the long run — by `taper_weeks_keep_every_session_above_the_floor`
  (`plan/engine.rs`, K10/K21/K42).
- **Change**: assert the floor across every taper session, not just the long run.
- **Effort**: S · **Depends**: — · **Impact**: 2

---

## 4. Sync, insights, review

### F-24 · Sync status pill + update summary

- **Runna**: "Plan Up To Date" vs "New Plan Version Available" plus a summary of
  what the update includes (`ins §11.5`).
- **Runalytics today** — **CLOSED (2026-10-27)**: `plan_sync_check` +
  `pill_status` (`src-tauri/src/plan_sync.rs`) classify current / updateAvailable
  / importAvailable / otherPlan / unpublished / off, and `src/lib/plan-sync-pill.ts`
  renders one pill with tone, a summary line ("New plan version · 36 workouts ·
  2 hours ago") and a single action. Importing someone else's plan needs a
  two-step confirm ("Replace this plan?", 6 s expiry). The monotonic rule stands
  (divergence #12) — no revert. Desktop: pill in the Plan actions panel head.
  Mobile: `place()` moves the same element into the page header ≤900px — both
  verified at 1440 and 390×844, including the writer+importAvailable hiding rule.
- **Change**: a pill that says "your plan changed, here's what changed". Keep the
  monotonic rule (divergence #12) — no revert.
- **Desktop**: pill in the Plan actions header.
- **Mobile**: pill in the plan header (iOS is pull-only, so this is where it
  matters most).
- **Effort**: S · **Depends**: — · **Impact**: 3

### F-25 · RPE (Modified Borg 0–10 with talk-test bands)

- **Runna**: an effort channel for when pace data is least trustworthy, and it
  then *disables* pace grading rather than grading a meaningless band
  (`ins §11.8`, article [01]).
- **Runalytics today** — **OPEN**: nothing in the repo.
- **Change**: capture RPE on completion; when RPE is present and pace is
  unreliable, disable pace grading for that session.
- **Effort**: M · **Depends**: — · **Impact**: 3

### F-26 · Reminders (with the suppression rule)

- **Runna**: six fixed times, one global setting, one combined reminder per day,
  timezone-aware, suppressed if the workout moved/completed/skipped first — the
  suppression rule is the whole feature (`ins §11.8`, [18]).
- **Runalytics today** — **OPEN**: no notifications/reminders anywhere.
- **Change**: Tauri notification plugin; suppression keyed on F-01's status.
- **Desktop**: native notifications.
- **Mobile**: APNs via the Tauri plugin; needs an entitlement — check before
  scoping.
- **Effort**: M (desktop) / L (iOS) · **Depends**: F-01 · **Impact**: 3

### F-27 · Instant / ad-hoc workouts

- **Runna**: logged to the calendar, "designed to complement your plan, not
  replace it", excluded from Pace Insights but **included** in Mileage Insights
  (`ins §11.8`, [09]).
- **Runalytics today** — **OPEN**: no ad-hoc session creation.
- **Change**: if added, honour that split — counts toward mileage, never toward
  pace grading.
- **Effort**: M · **Depends**: F-07 · **Impact**: 2

### F-28 · Manual workout logging (a correction path)

- **Runna**: allows logging a workout by hand for GPS inaccuracy, user-initiated,
  deliberately not triggering adaptation (`ins §11.8` [10], `mgmt §8`).
- **Runalytics today** — **OPEN**: no manual entry.
- **Change**: a manual activity that can be linked (F-34) but is excluded from
  `plan/adaptive.rs` evaluation.
- **Effort**: M · **Depends**: F-34 · **Impact**: 2

### F-29 · Heat / humidity adaptation

- **Runna**: a feels-like-temperature × duration trigger with four escalating
  adaptation types, plus a staleness warning when dependencies are invalidated —
  the staleness pattern is the most copyable part (`ins §11.8`, §8).
- **Runalytics today** — **OPEN**: no weather code at all (`heat_points` is an
  unrelated GPS heatmap).
- **Change**: self-contained feature; promise that plan arithmetic is untouched.
- **Effort**: L · **Depends**: — · **Impact**: 2

### F-30 · Elevation as a plan input

- **Runna**: Elevation Profile (Flat/Rolling/Moderate/Hilly) is a plan input that
  rebuilds automatically and controls whether hill sessions exist (`ins §11.8`,
  [24]).
- **Runalytics today** — **OPEN**: `elevation_gain_m` is display-only
  (`src-tauri/src/fit.rs:83-85`); we generate no hill sessions.
- **Change**: only worth taking if we add a hill session pattern; otherwise the
  input has nothing to control.
- **Effort**: L · **Depends**: session patterns · **Impact**: 2

### F-31 · PR engine (milestones, not history)

- **Runna**: a complete small spec — elapsed time, outdoor only, fixed dual-unit
  milestone set, milestones not history, a PR may trigger a plan update; the
  fixed set is a product decision, not a bug (`ins §11.3`, [13]).
- **Runalytics today** — **OPEN**: `personal_best`/`pr_` grep is empty. Our badges
  are progress badges (`src/lib/flow.ts`), which we keep deliberately (divergence #8).
- **Change**: if added, state elapsed-vs-moving in the UI (we read durations from
  FIT, so whichever is chosen must be labelled).
- **Effort**: M · **Depends**: — · **Impact**: 2

### F-32 · Per-km splits (we hold the data and never present it)

- **Runna**: the review screen is *built* around splits; the design trap is to pick
  a reference (previous km or the target) and **label it in the header**, because
  the two readings are opposite on a surge (`prog §8.1`, §6).
- **Runalytics today** — **OPEN**: `ActivityDetail` (`src/lib/types.ts:42`) carries
  `track`, `health`, `load`, `fitness`, `riskFactors`, `injuryRisk` — no laps or
  splits. `src-tauri/src/fit.rs` decodes SESSION + RECORD only (no LAP messages), but the raw
  `TrackPoint[]` is enough to compute per-km splits.
- **Change**: derive per-km splits from `track` in the activity detail sheet; label
  the reference in the column header.
- **Desktop**: a splits table in the detail sheet.
- **Mobile**: same table; it is the screen Runna users expect on the phone.
- **Effort**: M · **Depends**: — · **Impact**: 4

### F-33 · Surface the lap verdict where the athlete looks for it

- **Runna**: the bar-against-cap verdict sits on the activity you just opened
  (`prog §8.2`).
- **Runalytics today** — **CLOSED (2026-10-27)**: the verdict now renders in the
  workout/activity detail sheet as well as on Plan. `src/lib/workout-detail.ts`
  fetches `session_review` for a completed tempo/interval and paints
  `.workout-verdict` with the shared `paceTrack` bar (`src/lib/pace-track.ts`) and
  the accessible sentence ("Planned at 4:40–4:50/km, reps held @ 4:28/km — too
  fast"). The Plan page keeps its adaptation view; the reasoning behind the
  reps-only grading is unchanged (F-39).
- **Change**: also render the verdict in the activity detail sheet.
- **Effort**: S · **Depends**: — · **Impact**: 3

### F-34 · "Link activity" as a named, user-initiated verb

- **Runna**: Plan → workout → Link Activity → pick an activity; user-initiated,
  many-activities-to-one-workout, manual activities do not re-adapt
  (`mgmt §10.8`, `mgmt §8`, `prog §8.5`).
- **Runalytics today** — **PARTIAL**: the join and the manual matcher exist
  (`calendar_completions`, `match_activity`; form in
  `src/lib/workout-detail.ts:34-80` via `tauri.matchCalendarActivity`), and
  `reconcile` auto-matches strict 1:1 same-day at distance ratio 0.5–1.5. Missing:
  the *named* affordance, un-complete (tap ✅), and many-to-one.
- **Change**: name the control "Link activity", add un-complete, allow multiple
  activities per workout.
- **Desktop / Mobile**: same sheet; on mobile this is the primary completion path.
- **Effort**: M · **Depends**: F-01 (un-complete needs status) · **Impact**: 4

### F-35 · PLAN RUNS COMPLETED breakdown (adherence made legible)

- **Runna**: the All-Time card's `PLAN *` breakdown separates *plan* completions
  from *total* activity, including zeros for skipped modalities (`prog §8.4`).
- **Runalytics today** — **OPEN**: lifetime totals exist and are more honest about
  units (`gamificationStats()` `src/lib/flow.ts:235`; the "all-time distance would
  be a lie" comment at `src/achievements.ts:81`) — keep that — but there is no plan-vs-total
  split.
- **Change**: add a plan-completions line beside the totals, using F-34's links.
- **Effort**: S · **Depends**: F-34 · **Impact**: 3

---

## 5. Prescription grammar and export

### F-36 · `pace_target` beside the band (a named target *and* its tolerance)

- **Runna**: `400m at 3:05/km (2:55-3:15/km)` — a target point plus its band; on a
  watch face mid-interval a centre is easier to aim at than a range
  (`ux §7.3`).
- **Runalytics today** — **OPEN**: we write `@ 4:45-4:55/km`, the band alone —
  `lap_pace()` at `src-tauri/src/calendar.rs:77`, documented by the example at `:82`.
- **Change**: add `pace_target_s_km` alongside `pace_lo_s_km`/`pace_hi_s_km`;
  render `at {target} ({lo}-{hi}/km)`.
- **Effort**: M · **Depends**: — · **Impact**: 3

### F-37 · Two notations, one meaning — `at` for intent, `@` for actual

- **Runna**: reserves `at` for prescribed and `@` for recorded, so plan and log can
  never be confused when they share a surface — most important in the ICS export
  where planned and completed text sit side by side (`ux §7.5`).
- **Runalytics today** — **CLOSED (2026-10-27)**: prescribed laps read
  `at 4:45-4:55/km` (`calendar.rs` lap-line builder) and recorded pace keeps `@`
  (`Ran: 10.0 km @ 5:00/km`). The prescribed-side doc-comment example and
  `quality_sessions_are_named_and_easy_steps_have_no_upper_bound` were updated in
  the same commit; the recorded-pace `@` assertions still pass.
- **Change**: prescribed laps become `at lo-hi/km`; `@` stays for actual. The
  `@` occurrences in the tests (`calendar.rs:906`, `:910`, `:926`, `:955`, `:1027`) are
  all *recorded* pace and stay; the doc-comment example at `:82` and
  `quality_sessions_are_named_and_easy_steps_have_no_upper_bound` (`:828`) are the
  prescribed-side assertions to update.
- **Effort**: S · **Depends**: — · **Impact**: 3

### F-38 · Disarm the ceiling in copy

- **Runna**: adds the sentence that releases the athlete — *"This is a limit, not a
  target — run at whatever pace feels truly easy!"* and *"(or slower!)"* on
  cool-down (`ux §7.2`).
- **Runalytics today** — **CLOSED (2026-10-27)**: `no_faster_than()` now reads
  "no faster than 5:30/km, at conversational pace 🗣️ — a limit, not a target:
  slower is fine, and often better" (`src-tauri/src/calendar.rs`), with a doc
  comment stating why (easy-day pace is the one number an athlete is tempted to
  chase). Pinned by the warm-up assertions in `calendar.rs`.
- **Change**: extend the ceiling string with the permission sentence.
- **Effort**: S · **Depends**: — · **Impact**: 3

### F-39 · Never grade the ungraded

- **Runna**: excludes warm-up/cool-down from the pace-target denominator
  ("14 laps, 11/12 pace targets") so easy days are not punished (`ux §7.4`).
- **Runalytics today** — **CLOSED (load-bearing)**: `plan/pacing.rs` grades 200 m
  windows inside the main set only, reps never averaged (`WINDOW_M`, `:54`), and
  jog recoveries never count — stated in the Pacing panel copy. Keep; cite when
  F-32's splits table adds any per-lap score.
- **Impact**: 0 (guard, not a gap)

### F-40 · Per-workout timezone on the export

- **Runna**: pins `X-USER-TIMEZONE` on every event across five zones
  (`ux §7.6`).
- **Runalytics today** — **OPEN**: plan events are all-day
  (`DTSTART;VALUE=DATE:`, `calendar.rs:573`) with no `VTIMEZONE`/`X-USER-TIMEZONE`
  anywhere in `calendar_publish.rs`. Compounds the known date-only mislabel for
  west-of-UTC users.
- **Change**: pin the zone on the workout, not the profile.
- **Effort**: M · **Depends**: — · **Impact**: 3

### F-41 · Different schema per session type

- **Runna**: runs get structure; recovery/mobility gets prose — never force
  non-run work into the lap table (`ux §7.7`).
- **Runalytics today** — **N/A**: we emit no strength work. Rule to follow if we
  ever do.
- **Impact**: 0

### F-42 · The export is the moat

- **Runna**: the calendar export is well-formed *because* it is the only
  integration surface (`ux §7.8`).
- **Runalytics today** — **PARTIAL**: our ICS is disciplined (brand → title →
  `Plan:` laps → `Why:` → score lines, `calendar.rs`) and publishable
  (`calendar_publish.rs`). F-37 and F-40 are the two open defects against this
  principle.
- **Impact**: 0 (covered by F-37/F-40)

---

## 6. New findings (not in the six phase docs)

### F-43 · `move_workout` should not destroy the generated date

- **Runalytics today** — **CLOSED (2026-10-27)**: `move_workout` writes `date`
  only; `planned_date` stays exactly as generated and `moved` is set, so the
  deviation is measurable (the moved marker, Reset, and the week-deviations list
  all read from it) instead of invisible.
- **Change**: with F-01, keep `planned_date` immutable and log the move; the
  deviation becomes measurable rather than invisible.
- **Effort**: S · **Depends**: F-01 · **Impact**: 3

### F-44 · Conflict screen before any second plan

- **Runna**: an overlap screen that computes the overlap, shows what happens to
  each affected plan, defaults to the reversible option, and never deletes — moves
  to drafts (`mgmt §10.2`, §3).
- **Runalytics today** — **OPEN**: single active plan enforced
  (`plan.html`: "You already have an active plan. Use Abort plan above…").
- **Change**: none now; the rule is *ship the conflict screen before shipping
  multi-plan*. Recorded so a future multi-plan PR cannot skip it.
- **Impact**: 0 (gate)

### F-45 · Name the two verbs: reconfigure vs reschedule

- **Runna**: prints the same two-row table in two articles to stop users dragging
  a race into the wrong week (`mgmt §10.3`).
- **Runalytics today** — **CLOSED (2026-10-27)**: the calendar carries a
  persistent hint under the header (`calendar.html .cal-hint`): "Drag a session
  (or open it and use **Move to…**) to reschedule the week you're in. To change
  the plan itself, use Plan — that rebuilds every week, this only moves one run."
  Same copy on both surfaces; on mobile it is the only guidance since the grid is
  hidden.
- **Change**: a one-line hint at the top of the calendar ("to change your plan, not
  just this week, use Plan settings").
- **Desktop / Mobile**: same hint; on mobile it is the only guidance since the grid
  is hidden.
- **Effort**: S · **Depends**: — · **Impact**: 3

### F-46 · Label the deload week (cheapest UX win in the collection)

- **Runna**: its own docs tell users to *guess*; our cycle is deterministic, so a
  badge is free and turns a scary mileage dip into an explained one
  (`mgmt §10.7`).
- **Runalytics today** — **CLOSED (2026-10-27)**: a "Recovery week" badge
  (`.week-badge`, title "Your plan cuts volume this week on purpose.") renders on
  the desktop week strip and grid week header (`src/calendar.ts:413`) and on the
  agenda week header at mobile (`:346`). The dashboard sentence remains.
- **Change**: a "Recovery week" badge on the week card and a marker on the calendar
  week.
- **Desktop**: badge on `#week` card + calendar week header.
- **Mobile**: agenda list week header (grid hidden).
- **Effort**: S · **Depends**: — · **Impact**: 4

### F-47 · The copy icon (the prescription is the user's property)

- **Runna**: the only control on the review screen that treats the prescription as
  the user's property; makes a single plan composable with the outside world
  (`prog §8.7`, `ux §7.8`).
- **Runalytics today** — **CLOSED (2026-10-27)**: the workout detail sheet has a
  **Copy** button in its header (`src/lib/workout-detail.ts`) that puts the full
  prescription text (brand line, plan laps, Why line) on the clipboard — same
  sheet on desktop and mobile.
- **Change**: a copy button on the workout detail sheet emitting the prescription
  text.
- **Effort**: S · **Depends**: — · **Impact**: 2

### F-48 · `at` vs `@` in the plan timeline (F-37's frontend twin)

- **Runalytics today** — **CLOSED (2026-10-27)**: one shared formatter —
  `fmtPaceBand()` (`src/lib/ipc.ts`) renders `at lo–hi/km` and is the only pace-
  band renderer used by the dashboard timeline, the calendar chips, the plan
  pacing panel, and the detail sheet (`fmtPaceHeld()` renders held pace). Notes
  and timeline cannot drift; no frontend site emits `@` for a prescribed band.
- **Change**: single pace-formatting helper shared by notes and timeline.
- **Effort**: S · **Depends**: F-37 · **Impact**: 2

### F-49 · Garmin remains a stub

- **Runalytics today** — **OPEN (by design)**: "Garmin Connect has no official
  public API. Support is feature-gated and not enabled in this build"
  (`settings.html`, Setup pane). Runna's integration surface is Strava/Apple Health
  class importers, not a watch vendor.
- **Change**: out of scope for parity; noted so the Setup pane's "not built" copy
  is understood as a boundary, not a gap to close.
- **Impact**: 0

---

## Status rollup (2026-10-27, after the wave-1…4 implementation pass)

| Status | Count | Items |
| --- | --- | --- |
| OPEN | 19 | F-06,10,14,16,18,20,21,25,26,27,28,29,30,31,32,34*,35,36,40 |
| PARTIAL | 6 | F-08,09,11,13,17,42 |
| CLOSED | 19 | F-01,02,03,04,05,07,19,22,23,24,33,37,38,39,43,45,46*,47,48 |
| Boundary / gate / not-now | 5 | F-15,41,44,49, F-30 (deferred until hill sessions exist) |

\* F-34 and F-46 are OPEN for the *missing half* only — the join
(`calendar_completions`) and the `downWeek` data respectively already exist.
(F-46's badge shipped 2026-10-27; F-34's named verb / un-complete / many-to-one
remain open.)

**Closed in this pass (2026-10-27), all verified on desktop (1440) and iPhone
(390×844) unless the item is generation-side only:** F-01, F-02, F-03, F-04,
F-05, F-07, F-19 (engine + test), F-22 (guard + test + harness refusal check),
F-23 (test), F-24 (pill, both placements), F-33, F-37, F-38, F-43, F-45, F-46,
F-47, F-48. `cargo test --lib`: 131 passed / 0 failed / 1 ignored.
