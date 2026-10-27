# Runna — Living With a Generated Plan: Functionality and UI/UX Analysis

**Subject:** Runna iOS app (`com.runbuddy.prod` v8.59.0), the post-generation experience
**Evidence:** user-supplied screen recording `ScreenRecording_10-27-2026 07-53-45_1.MP4`
(1180×2556, 60 fps, 61.24 s), analysed frame-by-frame
**Companions:** [`RUNNA-PLAN-GENERATION.md`](./RUNNA-PLAN-GENERATION.md) — how the plan is built ·
[`RUNNA-UX.md`](./RUNNA-UX.md) — session execution and feedback ·
[`RUNNA-PROGRESS-AND-REVIEW.md`](./RUNNA-PROGRESS-AND-REVIEW.md) — the Progress tab,
whose Workouts list cross-checks the calendar rows in this recording ·
[`RUNNA-PLAN-MANAGEMENT.md`](./RUNNA-PLAN-MANAGEMENT.md) — what Runna's help centre says
the calendar drag is for, and what it explicitly is not for ·
[`RUNNA-INSIGHTS-AND-ADAPTATION.md`](./RUNNA-INSIGHTS-AND-ADAPTATION.md) — what happens to
the plan after execution: Pace/Mileage Insights, heat adaptation, workout stage semantics
**Screens:** [`plan-generated/`](./plan-generated/) — 24 ordered captures, including three mid-drag frames

---

## 1. Method and limits

Same pipeline as the previous phase: decode at 2 fps (122 frames), segment by
perceptual difference on a greyscale 100×200 proxy cropped to drop the top 7 %
(the status-bar clock otherwise fakes a new screen every minute), collapse
near-identical settled frames. The dedupe threshold was kept loose from the
start, on the lesson from phase 2 — an aggressive pass silently eats every
screen the user passes through quickly, and those are usually the interesting
ones. Result: **21 distinct screens** from 61 seconds.

Drag gestures are invisible to screen-dedupe: a drag is *sustained motion
without a screen change*. Finding them needs a different signal — a per-frame
difference profile against the previous frame, printed as a bar chart. Taps show
up as one large spike; a drag shows up as a plateau of moderate motion. That
plateau is what located the two rearrangements in this recording, and frames
`f0106`–`f0117` were then examined individually.

What this means for the claims below:

- **Copy is verbatim**, read off full-resolution frames.
- **Ordering and timing are real**, taken from frame timestamps.
- **Only Week 1 is observed**, and only two rearrangements. Weeks 2–23 appear as
  summary cards, never opened.
- **No workout was started or completed.** The recording is browsing and
  rearranging. Everything about execution comes from `RUNNA-UX.md`.
- **The prediction model in §4 is fitted, not documented.** Runna prints four
  numbers; the exponent is my arithmetic on them. It is a good fit, but it is a
  reconstruction.

---

## 2. The four screens between "generated" and "using it"

The recording begins already inside a **four-screen onboarding tour**, before the
user ever sees the app itself. All four are **dark theme**; everything after them
is light. The theme switch is the product telling you that the ceremony is over
and the tool has started.

| # | Screen | Job |
|---|--------|-----|
| 01 | "Your plan is ready" | identity — *this plan is about your race* |
| 02 | "Your plan, built by real coaches and data" | authority — *named humans made this* |
| 03 | "Getting the most out of your plan" | permission — *you are allowed to break it* |
| 04 | "Tap the graph and types of runs to learn more" | literacy — *here is how to read it* |

Each has a single `Continue`. No skips, no opt-outs, no progress dots. Four
screens is already at the limit of what a user tolerates after an ~84-second
wizard, and the app spends them in a deliberate order. The user moved through all
four in eight seconds — three seconds, two, three — which is the honest measure
of how many people read them.

**Screen 01 — identity.** A dusk photo of Berlin under a red `21.1` shield,
"Berlin Half Marathon Plan", and the line *"Whether this is your first
Half-Marathon or your fifth, this plan will get you to that finish line
faster."* Two chips: `📅 23 weeks`, `↕ Flat`. Below, an "Estimated Race Time"
card: `21.1` → **`1:22:00 - 1:25:00`**. The plan is framed as a route to a named
race in a named city, not as a spreadsheet of runs. The race-time range — the
number the user actually cares about — is placed *inside the ceremony*, before
any training content.

**Screen 02 — authority.** "Designed by world-class coaches and built on
insights from millions of real-world training data points, so every session is
informed by proven training principles", an underlined "Learn more", and a card
"Coaches powering Runna" listing **Ben Parker** (Head Coach) and **Steph
Kessell** (Lead Coach) with circular photos and chevrons. Note the ordering of
the sentence: *coaches* first, *data* second. The app claims human authorship
ahead of algorithmic authorship.

**Screen 03 — permission.** This is the most interesting screen in the whole
flow, and it is screen three of four, shown to a user who has not yet missed a
single workout:

> **"Running doesn't always go to plan, and that's okay.** Below is a list of
> tools and features at your disposal to make your plan work for you. And if
> things get off track, we'll reach out to help you readjust."

Two rows follow: **Rearranging Runs** > and **Skipping workouts** >.

Three things are being done here. Deviation is *normalised* before it happens,
so the first missed run is a tool-usage event rather than a failure. The
metaphor is ownership — "tools at your disposal", not "rules to follow". And
"we'll reach out" promises an active coach, which is the thing a static ICS file
cannot offer. **Runna teaches rearrangement before the user has ever needed it.**
By the time the user drags a run in §7, the gesture has already been named.

This is also the **only one of the four screens with a footer** — "You can
revisit this in 'Plan Overview' later" — and the only one with two drill-down
rows rather than prose. The other three are pure ceremony; this one is
documentation, so it gets a bookmark. The app is marking which part of the
onboarding is reference material and which is theatre.

**Screen 04 — literacy.** A full-plan mileage graph with an explicit
explanation: *"This is a mileage graph. It displays your entire plan, detailing
the types of runs that will make up your training as you progress towards your
goal."* Filter chips `All runs` / `Easy Run` / `Tempo` / `Intervals`, a stacked
bar chart in the four session colours, y-axis 0/24/48 km, x-axis weeks 1–13,
"Weeks 1 - 23" / "27 Oct - 4 Apr", and two headline totals: **Distance 905km**,
**Runs 91**. Segmented control Week | Month | All.

The chart is *teachable* — the app assumes a user cannot read it unaided and
says so. The two totals are also the plan's contract: 905 km ÷ 23 weeks =
**39.3 km/week**, 91 runs ÷ 23 = **3.96 runs/week**. Both are the numbers the
user set in the wizard (4 days/week, ~40 km), now expressed as an aggregate
rather than a preference. The plan is showing its work.

---

## 3. The plan home (Plan tab)

The Plan tab leads with the race, not the week:

- "Berlin Half Marathon Plan" / "Your race: **APR 4, 2027**" with the red 21.1 shield
- A row `🏁 Berlin Half Marathon Apr 4, 2027` >
- **Four circular quick actions**: Plan Guide (ⓘ) · **Rearrange Workouts** ·
  Connected Apps · Manage Plan (clipboard ✎)
- "Plan overview" > with a green bar chart
- `Week 1  **0/4 workouts**` > and `Plan distance  **0/905km**` >
- A `▶ Record Run` pill, pinned at the bottom on every tab

Two observations. **Rearrange Workouts is a first-class, named, above-the-fold
action** — not a hidden edit mode. It sits between "read the guide" and "connect
your watch", which is a genuine statement about where the product thinks its
value is. And both progress counters are **fractions against the whole plan**
(`0/4`, `0/905km`), so the empty state at day one is 0 % of 905 km. That is a
long way to go; the counter is designed to be filled slowly, over 23 weeks.

Further down: **"PACE INSIGHTS"** — "We'll monitor your speed sessions to keep
your training on track" with a black `LET'S GO` pill, and a collapsed
"Estimated Race Times ⓘ" card. The monitoring promise is placed on the plan
home, not buried in settings.

### The Today tab, for contrast

The other entry point is Today: header person / `Week 1/23 ▼` / bell, a seven-day
strip with **small coloured squares under days that have a workout** (MON grey,
TUE green, THU orange, FRI green, SUN purple), "Today's workouts" with a **live
weather chip (☁ 8°)**, the day's card, a "Week 1 Overview" card, and "My
insights — Monitoring mileage".

The weather chip is worth noting: the app reads the outdoor conditions for the
day you are about to run in. And the day-strip dots are a **colour legend
compressed to 8 px** — the same green/orange/purple grammar as everything else,
readable at a glance without labels.

**The week-progress bar appears twice with two different renderings.** On the
Today tab, `Week 1 Overview` shows four **grey** segments above
`Workouts: 0/4` and `Distance: 0/41KM`. On the Plan Overview screen the same
four segments are **colour-coded by session type** (green / orange / green /
purple). Same component, two states: *nothing done yet* desaturates to grey,
*what's scheduled* shows the session colours. A nice piece of economy — one
widget carries both the plan and the progress.

---

## 4. Estimated Race Times as a progress contract

Tapping the collapsed card expands a table with columns **Current** and
**In 23 weeks**, each row separated by a `···🏃···` glyph, deltas in teal:

| Distance | Current | In 23 weeks | Δ |
|---|---|---|---|
| 5K | 18:00 | 17:30 | −30s |
| 10K | 38:10 | 37:10 | −1m 0s |
| 21.1 | 1:25:48 | 1:23:21 | −2m 27s |
| 42.2 | 3:01:28 | 2:56:32 | −4m 56s |

**The Current column is one power law fitted to one input.** The user supplied
exactly one time in the wizard: an 18:00 5k. Fitting `t = a · d^b` to that 5k
and solving for the exponent implied by each printed time:

| Distance | implied *b* |
|---|---|
| 10K | 1.0843 |
| 21.1 | 1.0847 |
| 42.2 | 1.0834 |

Three independent distances, exponents agreeing to ±0.0007. Checking the fit
forward from 5k = 1080 s at *b* = 1.0843:

| Distance | printed | model | error |
|---|---|---|---|
| 10K | 2290 s | 2290.0 s | +0.0 s |
| 21.1 | 5148 s | 5145.1 s | +2.9 s |
| 42.2 | 10888 s | 10909.4 s | −21.4 s |

A Riegel-style curve (the classic exponent is ~1.06; Runna's is steeper, so it
penalises longer distances harder) reproduces all four times within 21 seconds
at marathon distance. **The Current column is not four estimates, it is one
number and a formula.** The wizard's single 5k input generated the entire table.

**The target column is not the same formula re-applied.** Implied exponents
there are 1.0460 / 1.0646 / 1.0705 — visibly less consistent, and *flatter* than
the current column. What is consistent is the **relative** improvement:

| Distance | Δ | as % of current |
|---|---|---|
| 5K | −30 s | 2.78 % |
| 10K | −60 s | 2.62 % |
| 21.1 | −147 s | 2.86 % |
| 42.2 | −296 s | 2.72 % |

Mean **2.74 %**, spread 0.24 pp. So the projection is: *assume you get uniformly
about 2.7 % faster at every distance, then re-curve.* That is a defensible
honest position — it does not pretend a half marathon improves more than a 5k —
but it also means **the target column carries no distance-specific training
information whatsoever.** It is the same promise four times.

Two more things fall out. The hero range on screen 01 was `1:22:00 - 1:25:00`,
midpoint 1:23:30; the table's "In 23 weeks" half-marathon time is 1:23:21, and
predicting the HM from the current 5k at the textbook *b* = 1.06 gives 1:22:48 —
inside the range. The range is the model's uncertainty band, and it is drawn
around the same arithmetic.

And the loop closes with phase 2: the wizard *echoed back* the 18:00 5k as one of
seven inputs on the receipt. Here it is again as the "Current" 5k, the seed of
four predictions. **One input, one exponent, one percentage — the whole
race-time story of a 23-week plan.**

The UX function of the table is a **contract with a date on it**. It is checked
and unchecked, and it is the thing the user will compare against on April 4,
2027. Whether Runna recomputes it as runs come in is not observable here — the
card is labelled "Estimated" and has an ⓘ, and the PACE INSIGHTS copy above it
promises speed sessions are monitored, which implies yes.

---

## 5. Week and workout browsing

Three nested levels, each with its own header treatment:

**Plan tab week cards** (`16`, `17`). Each week is a card: **Week 1** (black
border = selected) `OCT 26 - NOV 1`, `Total Workouts: 4`, `Distance: 41.00km`,
then four compact rows — `Tue Easy Run · 10km`, `Thu Tempo · 10km`,
`Fri Easy Run · 10km`, `Sun Long Run · 11km`. **Week 2** `NOV 2 - NOV 8`, 4
workouts, `Distance: 40.80km`: `Tue Easy Run · 10km`, **`Thu Intervals · 9.8km`
(RED)**, `Fri Easy Run · 10km`, `Sun Long Run · 11km`. **Week 3**, 4 workouts,
`41.00km`.

**Plan Overview** (`10`, `13`, `15`) — reached by tapping a week. Double back
chevron, "Plan Overview", `Week 1 ▼` (so weeks are switchable without going
back), the four segment bars, `Total Workouts: 4`, `Distance: 41.00km`, then a
**coach card — Colleen Quigley, Athlete, US Olympian** — with a personalised
welcome: *"Welcome Francesco to your personalized half marathon plan from Runna.
We are so excited to have as part of the team! This is your…"* truncated with
**See more**. Then the four workout rows with empty checkboxes.

**Workout detail** (`11`, `12`, `14`) — see §6.

The **colour grammar is now fully visible** and consistent across every surface
— day-strip dots, card left-bars, chart stacks, segment bars, week-card rows:

| Colour | Session |
|---|---|
| green | Easy Run |
| orange | Tempo |
| **red** | **Intervals** |
| purple | Long Run |
| grey | completed non-plan import |

Week 2 is where the plan starts to look like a plan rather than a template: the
Thursday quality session **changes type** from Tempo to Intervals, and its
distance becomes an odd **9.8 km** while every easy run stays a round 10 km.
Round numbers for the filler, precise numbers for the work. Week 1 and Week 3
both total exactly 41.00 km while Week 2 dips to 40.80 km — the load is
essentially flat at the start of a 23-week block, which matches the phase-2
finding that the volume curve is gentle and the ramp is back-loaded.

The **named coach per week** is a real differentiator. Not a generic banner: a
specific athlete with a photo, a title, and a message containing the user's
first name. The plan has an author you can tap.

---

## 6. Workout detail — the prescription, verified

Two workout details were captured. Both have the same skeleton, and the hero
gradient is **keyed to the session type** — amber for the Tempo-based
Progressive Run, purple for the Long Run.

**Progressive Run** (`11`, `12`), OCT 29 2026, red `🕐 SCHEDULE` badge,
"Tempo · 10km", `🕐 40m - 50m`, and **"Synced 10/27/2026 at 7:53 AM"** with a red
icon. Then a card "Personalized Workout Briefing / **Available in 19h 5m**".
Four circular actions: **WARM-UP STRETCHES · ADD ROUTE · LINK ACTIVITY · SKIP
WORKOUT**. A "Description" with an **OUTDOOR / TREADMILL** toggle, then the
sectioned step list:

| # | Section | Step |
|---|---|---|
| 1 | Warm-Up (grey) | 2km at a conversational pace ⓘ — **No faster than 4:55/km** |
| 2 | Session (2–4, orange) | 2km at **4:20/km** |
| 3 | Session | 2km at **4:00/km** |
| 4 | Session | 2km at **3:50/km** |
| — | — | 90s walking rest (REST) |
| 5 | Cool Down (grey) | 2km at a conversational pace ⓘ — **or slower!** |

**Long Run** (`14`), NOV 1 2026, "Long Run · 11km", `50m - 55m`, same sync line.
Session (purple): `1` — "11km at a conversational pace ⓘ". Briefing:
"**Available the day before your run (Sat)**".

This is direct confirmation of the prescription grammar that phase 1 reconstructed
from ICS metadata (`RUNNA-UX.md` §4) — now seen in the UI that generates it:

- **Easy steps get a ceiling, not a target.** `No faster than 4:55/km` on the
  warm-up, and on the cool-down not even a number — just **"or slower!"**. The
  app is actively defending against the most common self-sabotage in running:
  doing easy days too hard. The cool-down is the funniest and smartest string in
  the app.
- **Quality steps get an exact pace**, and it ramps: 4:20 → 4:00 → 3:50 across
  three 2 km blocks. A progressive run is *literally* progressive in its
  prescription, not just in its name.
- **Sections are labelled and coloured** — Warm-Up grey, Session in the session
  colour, Cool Down grey — which is the same observation as phase 1's finding
  that warm-up/cool-down are excluded from the pace-target score. The UI
  visually separates what you're graded on from what you're not.
- **The ⓘ on every pace line** is where the derivation lives. Not observed
  open, but its presence on the easy lines specifically means the app expects to
  be asked "why 4:55?".

**The briefing countdown is the retention mechanic.** "Available in 19h 5m" for
a run two days out; "Available the day before your run (Sat)" for a run five
days out. The personalised briefing is deliberately withheld until the run is
imminent. This is a countdown, not a lock — it converts the briefing into a
reason to reopen the app, and it means the content can be regenerated with
whatever the latest runs said. A static ICS export cannot do this, which is
precisely why phase 1 found prescriptions in the ICS but briefings nowhere.

**"Synced 10/27/2026 at 7:53 AM"** on both, with a red icon — the plan was
generated at 07:53 and every workout carries its own sync stamp. It answers "is
this current?" without the user asking, and it is the visible half of the
Connected Apps action.

**Four actions, and one of them is destructive-by-design.** `SKIP WORKOUT` sits
as a first-class circular button on the detail screen — the same permission
screen 03 of the onboarding promised. `LINK ACTIVITY` is how a completed run
from any watch attaches to the slot. `ADD ROUTE` turns a pace prescription into
a course. `WARM-UP STRETCHES` is content, not control — the app monetising
attention on the least interesting part of the workout.

---

## 7. Rearranging workouts — the headline

This is what the recording was made for, and the affordance chain is unusually
complete. Frames `f0104`–`f0117` at 0.5 s granularity.

**Before** (`18`, `f0104`) — Calendar tab, "Calendar" + lightbulb icon, "FRI Rest
Day", header `Oct 26 - Nov 1` `WEEK 1` with `Total: 9.2 km / 41,0 km`. Seven day
rows: MON 26 holds a completed run, TUE 27 (today, black circle) "10km Easy Run /
10km" green + `+ Add`, WED 28 `+ Add` only, THU 29 "Progressive Run / 10km"
orange + `+ Add`, FRI 30 Easy Run green, SAT 31 `+ Add`, SUN 1 Long Run purple.
**There is no Reset button.**

**The drag** (`22`, `23`, `f0106`–`f0109`):

1. **f0106–0107** — the "10km Easy Run" card **lifts off its row with a drop
   shadow** and follows the finger, overlapping the TUE 27 row. The lift is the
   grab confirmation; nothing else on screen moves.
2. **f0108** — the **WED 28 row turns green**, the day label **"WED" becomes a
   teal/green pill**, and a **"+"** appears beside it. Three simultaneous signals
   that this is a legal drop target.
3. **f0109** — green fills the whole WED row as the card settles.

**After** (`19`, `20`, `f0110`–`f0113`):

4. The landed card carries a **↻ circular-arrow icon** on its right edge — a
   *moved* marker, not a *changed* one.
5. A **dark toast** appears at the top: **"✓ Workout rescheduled … Undo"**.
6. The ↻ icon then disappears (`f0112`–`f0113`). It is a transient confirmation,
   not a persistent flag.
7. **The `⟳ Reset` button now exists in the week header** (`f0110`, and still
   there at `f0117`). It was absent at `f0104`.

**Second drag** (`24`, `f0114`–`f0117`): the same sequence for **Progressive Run
THU 29 → TUE 27**, with the TUE row highlighting green mid-gesture. Result: TUE
27 now holds "Progressive Run / 10km" with the ↻ marker, THU 29 is back to
`+ Add`, and the toast is still up.

Read the two moves together and they are **one decision, not two**: the easy run
that was scheduled for today (TUE 27) was pushed to tomorrow, and the tempo
session from Thursday was pulled forward into today. Net state after 61 seconds:

| Day | Before | After |
|---|---|---|
| TUE 27 (today) | 10km Easy Run | **Progressive Run** |
| WED 28 | — | **10km Easy Run** |
| THU 29 | Progressive Run | — |

That is the canonical rearrangement, and it is the one a plan cannot predict:
*"I feel good today, give me the hard one."* A generated plan is a proposal about
which day the quality session lands on, and the user's read on their own legs
that morning is better information than the generator had. The whole
rearrange-with-undo-and-reset apparatus exists to serve this single, extremely
common edit.

The design points worth stealing:

- **Reset is conditional on a deviation existing.** It does not clutter the
  header of a week that has never been touched, and it appears the moment one
  exists. A control that only exists when it can do something.
- **Two layers of undo.** The toast's `Undo` reverts one move; the header's
  `Reset` reverts the week to the generated plan. Fine-grained and coarse-grained
  recovery, separately.
- **Drop-target confirmation is loud.** Green row + pill-shaped day label + "+"
  is more feedback than most apps give for a drag. Rearranging a training plan is
  consequential — it can put intervals on a work day — so the app makes the
  landing unambiguous.
- **The week total is unchanged by rearrangement.** Moving runs changes
  placement, never volume. `Total: 9.2 km / 41,0 km` is a **completed / planned**
  ratio (9.2 km done of 41.0 planned), not a sum of what's currently on screen.
  Rearranging cannot accidentally edit your volume — the two are separate
  quantities in the UI.
- **The plan never loses the original.** The ↻ marker and the Reset button both
  imply the generated layout is retained alongside the user's edits. This is what
  makes "we'll reach out to help you readjust" (screen 03) operationally
  possible: the app can see what it proposed versus what you did.

---

## 8. Completed runs render differently

Two non-plan runs are visible on the Calendar and they are **visually distinct
from plan cards**:

- **MON 26** — "Monday run / **9.2 km • 42m 46s**" with a **dark grey left bar**
  and a **black ✓ circle**
- **SAT 24** (previous week) — "Saturday run / **12.0 km • 56m 55s**" ✓

These are the imported, non-plan workouts phase 1 identified in the ICS export
(`COMPLETED_NON_PLAN_WORKOUT-strava-*`). In the UI they get: grey instead of a
session colour, a **duration** where plan cards show a distance, and a check
instead of a checkbox or `+ Add`. Plan slots say *what will be run*; imports say
*what was run*, and the typography separates them without a label. Note also
that MON 26 sits inside Week 1 and contributes the 9.2 km in the week total —
**unplanned running counts toward the week**, which is the correct behaviour for
a runner who went out for a jog on a rest day.

Empty days show only `+ Add`, so a free day is an invitation rather than a
failure state. And "FRI Rest Day" is rendered as a first-class item at the top of
the calendar view — rest is a scheduled thing, not an absence.

---

## 9. Inconsistency: one quantity, three formats

The week's planned distance is written three different ways on three surfaces
reached within eight seconds of each other:

| Surface | Rendering |
|---|---|
| Calendar week header | `Total: 9.2 km / **41,0 km**` — comma decimal, 1 dp, space before unit |
| Plan Overview | `Distance: **41.00km**` — period decimal, 2 dp, no space |
| Today tab | `Distance: **0/41KM**` — integer, no space, uppercase unit |

`41,0` is a locale slip (comma decimal in an otherwise en-GB app that writes
`Oct 27`, `4:55/km`, and `1:25:48` with periods). The other two disagree on
precision and unit spacing. Small, but it is the same number a user watches for
23 weeks, and the plan tab and calendar tab are one tap apart.

One more, softer: the coach welcome reads *"We are so excited to have as part of
the team"* — a dropped word. Personalised copy assembled from fragments will
occasionally mis-join.

---

## 10. What this changes for Runalytics

Runalytics already has the hard part: it generates the plan and writes real
prescriptions into the calendar. Verified in this session:

- `src-tauri/src/calendar.rs` — `no_faster_than(low)` at line 70 emits
  `"no faster than {}/km, at conversational pace 🗣️"`, and `lap_pace(step)` at
  line 74 attaches it to easy steps via `(true, Some(low), _)`. The test
  `quality_sessions_are_named_and_easy_steps_have_no_upper_bound` (line 828)
  pins exactly the ceiling-vs-target distinction Runna renders as
  `No faster than 4:55/km` / `or slower!`. **That part is aligned.**
- `src-tauri/src/plan/engine.rs` — `peak_long_run_km` (line 34) and
  `long_run_share` (line 50), applied at lines 439/451 as
  `ceil_km((target * long_run_share(req.distance)).min(long_cap)).max(MIN_RUN_KM)`.
  Runna's Week 1 long run of 11 km against a 41 km week is consistent with a
  share-based cap of this shape.
- `src-tauri/src/plan/adaptive.rs` already mutates plans, which is the module the
  phase-2 doc flagged as needing an announced-adjustment pattern.

This phase exposes four concrete gaps, in the order I would take them. Each was
checked against the code rather than assumed.

**1. Finish the reschedule affordance chain.** Runalytics already has the hard
half: `calendar::move_workout` (`src-tauri/src/calendar.rs:394`) with real
guards — race fixed, completed rejected, `to < today` and `to > plan.target_date`
refused — and a pointer-driven drag in `src/calendar.ts` `initDragAndDrop()`
with a ghost chip and `.drop-target` highlighting. Runna's chain is *lift → loud
green drop target → moved marker → toast + Undo → header Reset appearing only
once a deviation exists*. Runalytics has the first two and nothing after the
drop. What is actually missing:

- **Undo.** `finish()` calls `moveWorkout` and fires `calendar-updated`. There is
  no inverse offered, so a mis-drop is a silent edit the user has to notice and
  manually undo. Runna's toast costs one string and the previous date, which the
  frontend already has in `dragged.date`.
- **A conditional Reset.** This needs a schema change, not just UI:
  `PlanWorkout` (`src-tauri/src/types.rs:402`) stores only the *current* `date`.
  The generated date is overwritten, so "reset this week to the plan" is
  unrecoverable — there is no original to return to. Runna's `⟳ Reset` button is
  only possible because it keeps the proposal beside the edit. A
  `planned_date` column alongside `date` is the whole change, and it is what
  makes a deviation *measurable* rather than invisible.
- **A moved marker.** Minor, but it is how the user learns which days they have
  touched.

The stronger argument for this is `plan/adaptive.rs`. If Runalytics re-derives a
plan from what it *thinks* the schedule is, while the user has moved a run in
their calendar app instead, the adapter optimises against a plan nobody is
following. And a drag inside Runalytics that destroys the original date means the
adapter cannot see the deviation either — it sees a plan that apparently always
said Tuesday. Note also that `initDragAndDrop` returns early on
`pointerType === "touch"`, and `#cal-grid` is hidden under 560px, so on iPhone
there is currently **no way to rearrange at all** — which is precisely the device
Runna's flow was recorded on.

**2. Show the completed / planned week total as one ratio.** Runalytics already
computes both halves — `renderWeekStats()` (`src/calendar.ts:223`) renders
`km ran` and `km planned` as two separate tiles, and its `planM` deliberately
includes banked sessions, so unplanned running counts exactly as Runna's does.
The only difference is presentation: Runna compresses it to `9.2 km / 41,0 km` in
the week header, adjacent to the days it describes. Two tiles ask the user to do
the division; one ratio answers the question where it is asked.

**3. A predicted-race-time progress table.** The model is trivially
reproducible — one power law from one input (`b ≈ 1.08`), then a uniform
percentage improvement — and its value is not accuracy but **contract**: a dated
promise the user can be measured against, in teal deltas. Runna's weakness is
that the target column is distance-blind (§4); Runalytics could do better by
re-deriving the projection from actual session performance, which
`plan/pacing.rs` already tracks — it stores how far the quality-pace ranges have
been moved and from which day, which is precisely the evidence a real projection
needs. Same UI, honest version.

**4. Teach the chart and say the plan will drift.** Two cheap copy wins. Runna
spends a whole onboarding screen explaining what a mileage graph is, and screen
three of four tells the user that deviation is normal before it happens. Both are
zero-engineering. The corresponding Runalytics surfaces are `plan.html` and
`src/lib/plan-bands.ts` for the graph, and `src/lib/onboarding.ts` for the
permission-to-deviate framing — which is already one-idea-per-slide, so the
"plans meet real life" framing belongs there rather than in a new tour.

And one thing to *not* copy: the four-screen dark-theme ceremony. Runna can
afford it because a plan generation happens a few times a year and each one is an
event. Runalytics regenerates plans adaptively; a four-screen tour per
regeneration would be hostile. Take the *content* — identity, authority,
permission, literacy — and put the permission and literacy parts where they will
be read once, not four times per year.

---

## 11. Evidence index

[`plan-generated/`](./plan-generated/) — 24 captures, numbered in viewing order.

| File | Screen |
|---|---|
| `01-plan-is-ready.png` | "Your plan is ready" hero + Estimated Race Time |
| `02-coaches-powering-runna.png` | Ben Parker / Steph Kessell |
| `03-getting-the-most-out-of-plan.png` | "Running doesn't always go to plan…" |
| `04-mileage-graph-tutorial.png` | 905 km / 91 runs, stacked graph |
| `05-today-tab-day1.png` | Today tab, day strip, weather, Week 1 Overview |
| `06-plan-tab-top.png` | Four quick actions, 0/4, 0/905km |
| `07-plan-tab-scrolled.png` | PACE INSIGHTS, LET'S GO |
| `08-race-times-expanded.png` | Current / In 23 weeks table |
| `09-plan-tab-lower.png` | Week 1 card |
| `10-plan-overview-week1.png` | Colleen Quigley welcome, segment bars |
| `11-workout-detail-progressive.png` | Amber hero, warm-up + session steps |
| `12-workout-detail-progressive-scrolled.png` | Step 4, REST, cool-down "or slower!" |
| `13-plan-overview-list.png` | Week 1 four rows |
| `14-workout-detail-longrun.png` | Purple hero, briefing "day before (Sat)" |
| `15-plan-overview-back.png` | Return from detail |
| `16-plan-tab-week-cards.png` | Weeks 1–3 cards |
| `17-plan-tab-week-cards-scrolled.png` | Scrolled |
| `18-calendar-before-drag.png` | Full week rows, MON 26 completed, **no Reset** |
| `19-calendar-after-drag1.png` | **Reset appears**, ↻ on WED 28 |
| `20-toast-rescheduled.png` | "✓ Workout rescheduled … Undo" |
| `21-calendar-after-drag2.png` | Second move landed on TUE 27 |
| `22-drag-lift-t54.0s.png` | **Mid-gesture** — WED row green, day pill + "+" |
| `23-drag-hover-t54.5s.png` | **Mid-gesture** — green fills the row |
| `24-drag-second-t57.5s.png` | **Mid-gesture** — second drag |

Raw frames, segmentation scripts, and the motion profile that located the drags
are in `/tmp/runna_v2/` (ephemeral). The three mid-drag frames are the only
evidence for §7 steps 1–3 and cannot be recovered from the settled screens.
