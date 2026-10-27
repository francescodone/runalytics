# Runna — Plan Management: The Control Surface the Videos Never Showed

**Subject:** Runna iOS app (`com.runbuddy.prod`), plan creation and management
**Evidence:** Runna's public help centre — collection
[16743424 "Plan Creation & Management"](https://support.runna.com/en/collections/16743424-plan-creation-management),
**all 28 articles**, fetched and archived complete (see §1 and `support/`)
**Companions:** [`RUNNA-UX.md`](./RUNNA-UX.md) · [`RUNNA-PLAN-GENERATION.md`](./RUNNA-PLAN-GENERATION.md) ·
[`RUNNA-PLAN-EXECUTION.md`](./RUNNA-PLAN-EXECUTION.md) · [`RUNNA-PROGRESS-AND-REVIEW.md`](./RUNNA-PROGRESS-AND-REVIEW.md) ·
[`RUNNA-INSIGHTS-AND-ADAPTATION.md`](./RUNNA-INSIGHTS-AND-ADAPTATION.md) — Phase 6, which
closes every open lead in §11 below
**Screens:** none — this phase is documentation, not captures

---

## 1. Method, and what kind of evidence this is

Phase 1–3 evidence was behavioural: what the app *did* on screen. This phase is
**Runna's own description of what the app is supposed to do**, which is a different
evidence class with different strengths:

| | video/screenshots | help centre |
| --- | --- | --- |
| reveals | layout, motion, colour, gesture, real data | rules, limits, state machines, *why* |
| cannot reveal | the rule behind the behaviour | whether the UI matches the claim |
| failure mode | over-fitting one session | vendor optimism, stale text |

So every rule below is **"as Runna documents it"**, not "as verified on device". Where a
documented rule contradicts what a recording showed, that contradiction is itself the
finding and is called out (§9).

**Coverage:** all 28 articles in the collection, fetched server-side (the pages are
server-rendered; `.article_body` parses cleanly) and archived in this folder:

- `support/raw_articles.json` — structured extraction (headings, paragraphs, list items,
  tables, image alts) for all 28, 236 KB
- `support/all_articles.md` — the same rendered as readable text, 2,131 lines
- `support/to_md.cjs` — the converter

Article 28 (*Do Not Share My Personal Information*) is a privacy-notice formality with no
product content; it is archived but not analysed. **27 articles carry functionality.**

One collection limitation worth stating: this is the *plan* collection. Runna's help
centre has other collections (watches, calendar sync, Pace Insights, Runna Score,
Personal Records, recovery science) which appear here only as *titles* in "related
articles" links. Several are cited below as open leads (§10).

---

## 2. The headline: `Manage Plan` is the product

Across 27 articles, one navigation path appears again and again — **Plan → Manage Plan**.
It is the control surface for essentially every plan parameter, and the videos never
opened it. Reconstructed from the article paths, `Manage Plan` contains:

| section | what it holds |
| --- | --- |
| **Plan Overview** | plan name (pencil icon), end date / race date |
| **Running Ability** | Beginner → Elite Plus; rebuilds the whole plan |
| **Current Race Times** | the estimated *current* race time that sets all pace targets |
| **Current Weekly Mileage / Current Longest Run** | the fitness inputs from setup, editable later |
| **Running Schedule** | Runs per week · Available Days · Long Run Day |
| **Training Preferences** | Training Volume + Difficulty, each 3 levels, + Advanced Customisation |
| **Adapt Your Plan** | Add a B-Race · Holidays · Not Feeling 100% |
| **Strength / Yoga / Pilates / Stretch & Stability** | one tab each, own Set up / Update / Delete |
| **Start New Plan** | create a plan from inside the current one |
| **PDF → Share** | export the whole plan as a PDF |

Plus a separate top-level entry, **Plan → Your Plans**, which holds the plan *lifecycle*
(§3).

**The organising principle is duration of change**, and Runna states it explicitly with
the same two-row table twice (articles 3 and 11):

| I want to… | Where to do it |
| --- | --- |
| Change which day my long run falls on **every week**, runs per week, or available days | Plan → Manage Plan → Running Schedule |
| Move a **single** workout this week, next week or the previous week | Calendar tab → drag and drop |

That is a genuinely clean separation, and it is *stated to the user* rather than left to
be discovered. Runna even warns against the wrong door: *"It's important to make date
changes through Manage Plan, rather than dragging workouts around in the calendar. Your
plan needs to rebuild to maintain safe progression and taper."* (article 15).

**Worth stealing for Runalytics:** the two-verb model — **reconfigure** (rebuilds the
plan) vs **reschedule** (moves one session). Our app has drag-and-drop and a settings
form but never names the distinction, and our users have to infer which one they are in.

---

## 3. Plan lifecycle: five states, drafts, and a conflict screen

Article 27 (*Managing Your Current, Upcoming, and Draft Plans*, app **v8.49.0**, gradual
rollout) documents a plan-lifecycle model with no equivalent in anything we have seen on
screen.

Five auto-assigned categories — *"You don't set these categories yourself"*:

- **Current** — *"You'll only ever have one at a time."*
- **Upcoming** — scheduled to start later; queue **as many as you want**
- **Completed** / **Incomplete** — finished vs started-but-not-finished
- **Drafts** — *"your space to experiment… A draft stays a draft until you choose to add
  it, even once its start date comes around, so it never lands on your calendar on its
  own."*

Three design decisions in that are worth pausing on:

**1. Drafts are a no-commitment sandbox.** *"That's exactly what Drafts are for… You can
create as many Drafts as you want."* Full plan generation, zero consequence. There is
also a **Preview** option — *"a condensed view of a plan, its week cards and a summary, so
you can see what it involves without switching to it."*

**2. Deleting is not deleting.** *"Removed plans won't be deleted, they will simply move
to your Drafts."* The destructive-looking verb has a reversible destination.

**3. Overlaps get a conflict screen, not a silent overwrite.** When a new plan clashes
with a scheduled one, Runna offers **Save it to drafts** (the suggested default) or
**Adjust your training calendar** (new plan takes over, old one is *moved to drafts, not
deleted*). *"The conflict screen shows you what will happen to any affected plans before
you confirm. This check runs whenever you switch your current plan, add an upcoming
plan, or create a new one."*

The default is the interesting part: **when in doubt, the app chooses the reversible
option and says so.** Also: *"Workouts from your Upcoming Plans won't show in your
Training Calendar, only workouts from an active plan will"* — the calendar is a single
plan's view, never a merge.

And a hard edge: *"You cannot make a completed plan your active plan again."* Incomplete
→ active is allowed; Completed is terminal.

**Runalytics:** `PlanStatus` (`src-tauri/src/types.rs:212`) is
`Draft · Active · Pushed · Completed · Aborted`. We already have `Draft` and `Completed`,
but **no Upcoming/queue, no conflict detection, and no "delete lands in drafts"
behaviour**. The conflict screen is the piece worth copying first — it is the cheapest
one to build and the one that prevents the most user regret.

---

## 4. Plan Realignment: a missed-workout state machine

Article 9 is the most algorithmically interesting document in the collection, because it
describes a **trigger condition Runna implements as a pop-up with graded options** — and
the trigger is deliberately *negative*.

> *"The Plan Realignment feature is a pop-up notification that will appear at the bottom
> of your Today or Plan tab when the app detects that you've missed more than three
> workouts, or a week of training. Typically, this appears on a Monday… **To make sure the
> Plan Realignment pop-up appears, don't mark your missed workouts as skipped and leave
> them unticked.**"*

**Doing nothing is the signal.** An explicit "skip" means *"this run isn't happening,
please don't factor it into my plan going forward"* (article 19); an unticked workout
means *"I might still catch up"* and keeps the realignment path alive. Runna distinguishes
*skipped* from *missed*, and the difference changes what the app will offer you.

The options scale with how much was missed:

| gap | options offered |
| --- | --- |
| 1 workout | drag in calendar · Skip · leave unticked if more are coming |
| **3+ workouts or 1 week** | **Skip missed workouts** · **Rearrange missed workouts** |
| **2 weeks** | **Extend plan** by the time away · **Rebuild your plan with your current end date** · **Continue unchanged** (skips the weeks) |
| **4+ weeks** | **Start a new plan** · **Restart current plan** · **Rebuild your plan with your current end date** · **Continue unchanged** |

With the coaching judgement attached: *"our coaches recommend rebuilding or extending"*,
and on continue-unchanged after a month: *"This may increase injury risk as the plan has
progressed beyond what your current fitness is capable of."*

(Note the trigger threshold is stated two ways in the same article: *"missed more than
three workouts"* and *"you'll probably miss more (3+ or a full week)"*. `>3` and `≥3` are
different detectors. Worth pinning with a test if we build it, rather than copying the
prose.)

Two more constraints that reveal the machinery:

- **Irreversible:** *"once confirmed, it can't be undone."* (Contrast §3, where deletion
  is reversible — Runna is consistent about marking which actions are one-way.)
- **No skip-forward:** *"it's not possible to skip forward to following weeks in the
  plan… If you need to skip a specific future week, contact our support team via the
  in-app support channel and we can do this for you."* A capability gap is filled by a
  human, and the help centre says so.
- **The pop-up can be pre-empted by another surface:** if the realignment doesn't appear,
  one check is *"Check whether you have an actionable mileage insight"* — i.e. a mileage
  insight and a realignment offer compete, and only one fires.

**Runalytics:** this is the strongest idea in the whole collection for us. We have
`pending_proposal` on the dashboard payload (`commands.rs`) and a proposal engine with a
7-day dismissal suppression (`proposal.rs:22`) — so the *plumbing* for a
surfaced-decision-with-suppression already exists. What we lack is the **graded ladder**
(1 vs 3+ vs 2wk vs 4wk) and the **unticked-vs-skipped distinction**. We currently have
`PlanWorkout.done: bool` — one bit, no third state — so we structurally *cannot* tell
"skipped" from "missed". That single missing state is what the whole Runna ladder is
built on.

---

## 5. The adjustment features: one shape, three instances

**Not Feeling 100%**, **Holiday Mode**, and **B-Race** are three different features with
an identical skeleton:

```
scope (dates)  →  intensity (how much you keep)  →  [review impact summary]  →  rebuild
                                                    ↑
                             "Training Disruption: low / medium / high"
                             + "Adjustment Insights" (what will change, per week)
```

That third step — **show the predicted consequence before committing** — is the part that
matters, and Runna ships it on all three:

- B-Race: *"Review the summary of plan changes. This is where Runna calculates a
  personalised adjustment and shows you exactly how your B-Race will impact your training
  for your goal race with **Training Disruption** and **Adjustment Insight**."* The
  insights are week-scoped: the week before (taper starts, deload moves forward), B-race
  week (fewer runs, deload aligned, sessions swapped for taper intervals), the week after
  (extra rest days, lighter first session).
- Holiday: *"Review the summary of changes. Runna will show any impact on your race time,
  mileage, or schedule."*
- Not Feeling 100%: adjustment applied, then *"adjusted beyond that to support a safe and
  sustainable return"*.

### 5.1 Not Feeling 100% (app v8.25)

The richest of the three. Parameters:

- **Duration 3–14 days**, start today **or schedule ahead** *"if you know a busy or
  demanding week is coming up"*.
- **Backdating:** start from your most recent completed-or-skipped workout, **or one week
  ago today, whichever is shorter**. Rules: max 7 days back, the period must include
  today, may extend into the future, total (past + future) ≤ 14 days.
- **Level:** Reduced Difficulty (all run types, speed sessions easier, long runs
  conversational) · Easy & Long Runs Only (pace targets removed, mileage down) · Short Easy
  Runs Only · No Running. Plus an option to temporarily remove non-running workouts.
- **Return:** Slowly (aches/pains) · Balanced (cold/short illness) · Quickly (life got
  busy). *"Runna will calculate a safe, sensible progression based on your choice."*
- **Editable** during the adjustment *or* the return period (duration, level, or return
  approach). Push notification **the day before** it ends *"so you can extend it if
  needed"*.
- **Discard is time-boxed:** *"You can discard a 'Not Feeling 100%' adjustment from your
  plan the day that you create it. After that, you'll need to end the adjustment early and
  return to full training."*
- **B-race interaction:** the overlapping B-race is *not* auto-removed; you get a warning
  and a recommendation to switch it to "Run for fun". Skipping the B-race doesn't affect
  the rest of the plan.

And the honest part — **why it may refuse you** (article 14):

> *"If the 'Not Feeling 100%' adjustment is visible but can't be added, it's usually
> because doing so would prevent us from safely rebuilding the required mileage in the
> time remaining before your race."*

Blockers listed: an existing B-race with mileage targets, a reduced-load holiday, and
**multiple existing adjustments** (*"Multiple adjustments can limit how safely we're able
to progress your training"*). Then the admission that the real reason is sometimes
opaque: *"even without obvious constraints, the current phase of your plan may limit
flexibility in some cases — especially if you're close to a goal race."*

That is a **constraint solver talking**, and the docs say so: the features are
mutually-limiting, and the app computes whether it can still hit the goal rather than
applying your request blindly.

### 5.2 Holiday Mode (app v7.2)

3–21 days, named by the user (*"e.g. July Beach Break"*), with a **regional name
variant** — *"Holiday Mode (or Vacation Mode for North American Runnas)"*. Intensity
options are **plan-aware**: "Easy + long runs" only appears for half-marathon-and-longer
plans, "Easy + speed runs" only for shorter race plans. A `Remove non-running workouts`
toggle. Runna *"will automatically rearrange deload weeks"* and sends a push notification
when the holiday ends.

Constraints: no holiday in the past (leave sessions unticked instead → Plan Realignment);
*"deleting a holiday will not necessarily reset your plan back to its original
structure"*; can't edit preferences once it has started; *"won't push a plan beyond 26
weeks"*; no limit on the number of holidays, though *"frequent breaks may reduce how
effective your plan is overall"*.

### 5.3 B-Race (app v7.31.2, plan version v11)

A secondary race inside the A-race build. The rules are unusually specific:

- **Effort is a user choice with three levels** — *Run for fun* (minimal impact) /
  *Strong & steady* (moderate, small taper/recovery tweaks) / *Going all out* (maximum,
  significant taper and recovery). The plan rebuilds accordingly.
- **Must be shorter than the A-race** — *"you cannot add a 10K as a B-race when your plan
  is for a 5K."* (Note: article 23 says B-races *"can be the same distance or shorter"* —
  a direct contradiction between two articles in the same collection; see §9.)
- **Must fall inside the plan window**, and not inside the A-race taper.
- **Coaching guidance is inlined** — 4–8 weeks out for a marathon build, 3–6 for a half;
  *"Avoid hard efforts within 7–10 days of your goal event"*; don't race on untested kit
  or fuelling.
- **Blocked with an explanation:** *"In these cases, the app will guide you with a clear
  explanation"* and *"We sometimes block you to keep your training safe and effective."*
- **Post-hoc editing is limited:** only the *approach* is editable after adding; to change
  the race you delete and re-add; *"you cannot delete a B-race if it's already passed"*
  but you can delete one in the current week.
- **Pace cues adapt to terrain:** flat/rolling B-races get pace alerts and per-km splits;
  *"if your B-race includes hilly or moderate terrain, Runna will not provide specific
  pace target cues… Instead, you'll receive feedback at the end of each kilometre."*
- **Not supported:** multi-day or back-to-back events. The documented workaround is
  explicit — build around the primary day, add the others as Instant Workouts, and
  *"your plan won't automatically adjust for multiple race days, so it's important to
  manage your recovery between races yourself."*

**Runalytics:** we have `PlanProposal` with Accept and a 7-day dismissal suppression,
which is the right primitive — but our proposals are *app-initiated nudges*. Runna's
adjustments are **user-initiated requests that the planner can refuse**, with a
pre-flight impact preview. That's the inverse direction, and it's the one that needs the
planner to be able to *evaluate* a hypothetical plan without committing it.

---

## 6. The progression algorithm, in Runna's own words

Article 22 is the closest thing to a published spec, and article 16 explains the mileage
levers. The load-bearing statements:

**Race/distance plans:**
- Weekly mileage builds *"along a curve that increases week by week, but with a fixed
  ceiling on how much it can go up in any single week. The maximum increase allowed each
  week is calculated based on your current weekly mileage, ensuring very low-mileage
  runners can still progress while higher-mileage runners are protected from increasing
  mileage too quickly."* → **a proportional cap, clamped by absolute safety in both
  directions.**
- *"Your long run is managed separately, following its own progression to avoid increasing
  too quickly even if your overall weekly mileage is building."* → **two independent
  progressions, not one.**
- *"More experienced runners are given higher overall training targets, but not faster
  build rates."* → **level shifts the ceiling, never the slope.**
- Pace targets: *"Based on your completed workouts, Runna may suggest increases or
  decreases to your pace targets throughout the plan. **You can choose whether to accept
  these suggestions.**"*

**Beginner/return plans:** *"a controlled double constraint… Each week, your training
ceiling is set using two limits, and whichever is more conservative applies."*
Walk-to-run is staged: *"Rather than switching abruptly from walk-runs to continuous
running, the proportion of continuous runs in your plan increases gradually across defined
stages"* — Return to Running transitions over **four quarters** of the plan; New to
Running is *"slightly more gradual"*.

**General plans:** *"build your weekly mileage at a fixed weekly percentage rate"* with a
notable inversion — *"if you're running fewer days per week, your plan applies a slightly
higher percentage increase each week. The more days per week you run, the more volume
you're already covering, so the weekly percentage increase is scaled down accordingly."*

**Deloads:** *"Runna doesn't explicitly label a week as a deload week in the app"* — the
user is told to infer it from a mileage dip or a lighter schedule. Article 21 gives the
rationale (*"stress, recover, repeat"*, *"the deload week is what makes the build week
count"*) and says *"You can see your deload weeks in advance in your plan's mileage
graph."*

**Difficulty settings have a quantified effect:** *"Choosing the most demanding settings
can lead to around **10% faster progress** compared to the default plan"* (article 10) —
and the defaults are the aggressive ones: *"Runna recommends selecting 'Progressive' for
Training Volume and 'Challenging' for Plan Difficulty. These are the default settings."*

### 6.1 The knob taxonomy — and the table that resolves it

Article 16 is effectively a decision table for **which knob to turn**, and it is the most
directly reusable artefact in the collection:

| knob | changes | does NOT change |
| --- | --- | --- |
| **Running Ability** | length and load of all runs, weekly mileage, session distances | — |
| **Training Volume** | how fast mileage builds | pace targets |
| **Difficulty** | number/structure of hard runs, how much time at faster pace | pace targets |
| **Current Race Times** | pace targets across the plan | **mileage** (*"⚠ Current Race Times affect your pace targets, not your overall mileage"*) |
| **Runs Per Week** | how mileage is distributed | — |
| **End Date / Race Date** | how fast the build must be | — |

The **volume/difficulty/pace-target separation** is the key insight: three orthogonal
knobs, and the docs repeatedly reassure that turning two of them cannot touch the third
(*"lowering your Difficulty may result in fewer reps in an interval workout, but the
target paces for those intervals will remain the same"*).

**Runalytics:** our `IncreaseMethod` (`types.rs:190`) is a **single** knob with three
stops, and it drives *two* things at once — the peak-volume multiplier
(`engine.rs:74`: ×0.85 / ×1.0 / ×1.15) **and** the weekly growth cap
(`types.rs:198`: +5 % / +8 % / +12 %). Runna separates those deliberately: volume controls
the ramp, difficulty controls the session mix, and neither may touch pace. We do already
match two of their rules: `engine.rs` caps the build-week jump at the selected method's
factor (test at `engine.rs:842`) and caps the long run separately via
`peak_long_run_km`/`long_run_share` — so *"fixed ceiling per week"* and *"long run managed
separately"* are independently confirmed. Our 3:1 build/deload cycle also matches their
*"deload every few build weeks"*. What we don't have is the **orthogonality promise**. In
our engine, raising the increase method happens *not* to move pace targets — the bands are
derived from `threshold_sec_km` (`engine.rs:493`, `easy_pace_s_km`, `tempo_s_km`,
`interval_s_km`), never from `IncreaseMethod` — so we already keep the promise Runna
states. But we keep it by accident of wiring: it is nowhere documented and nowhere
asserted, so a future refactor that routed pace through volume would break it silently.

---

## 7. Plan catalogue, ability ladder, and the hard limits

**Ability ladder** (article 6) — defined by *single-run capability*, not weekly volume,
and the doc says so: *"💡 'Regularly run' refers to the distance you can comfortably
complete in a single run — not your total weekly mileage."*

| ability | definition | runs/week |
| --- | --- | --- |
| New to Running | cannot yet do 5K without stopping | 2–4 |
| Beginner | 5K without stopping, under 60 min | 2–4 |
| Intermediate | regularly ≥5K, no structured plan | 2–5 |
| Advanced | regularly ≥10K + some structure (intervals) | 2–6 |
| Elite | regularly half-marathon+, experienced with intervals | 2–7 |
| Elite Plus | *"consistently running 80–100km per week"* | 2–7 |

**Minimum 2 runs per week, no exceptions** — *"No, we don't have 1-run per week plans."*
The runs-per-week ceiling is **gated by ability**, which is a nice way to bind two knobs
without a settings matrix.

**Hard numeric limits found across the collection:**

| limit | value | source |
| --- | --- | --- |
| Max plan length | **26 weeks** — *"to help avoid mental and physical burnout"* | 12, 15, 18, 23 |
| Max length, beginner/return plans | **16 weeks ahead of you**, measured from *now* | 20 |
| Absolute min, New to Running | **6 weeks** | 20 |
| Absolute min, Return to Running | **4 weeks** (only if away <3 months) | 20 |
| Shortest plan overall | **3 weeks** (Run to Maintain) | 18, 23 |
| Race distances supported | **5K → 50K**, nothing over 50K, nothing under 5K | 23, 27 |
| Holiday length | 3–21 days | 12 |
| Not Feeling 100% | 3–14 days, backdate ≤7 days | 14 |
| Calendar drag range | same week, **±1 week** | 11 |
| Planned runs per day | **1** (Instant Workouts can add more) | 11 |
| Realignment trigger | >3 workouts or 1 week; different set at 4+ weeks | 9 |
| Recommended pace nudge | **15–30 seconds** at a time | 7 |
| Strength sessions/week | max 4; lengths 30/45/60 min | 25 |

Recommended minimum lengths per plan (article 23) are published as a table: New to Running
8 · Return to Running 6 · 5K Improvement 8 · 10K 10 · Half 12 · Fast-Track Half 8 ·
Marathon 16 · Fast-Track Marathon 12 · Ultra 16 · Run to Maintain 3.

Note the **Fast-Track variants** — a documented second tier of the same plan with a
shorter minimum, which is how the app offers compression without breaking the safety
floor.

**Runalytics:** our engine caps a plan without a race date at 16 weeks
(`engine.rs:319`, *"a plan without a race date runs at most 16 weeks — that is the longest
block COROS will hold"*). So our 16-week ceiling is an **upstream provider constraint**
(COROS accepts 4–16 whole weeks and silently truncates longer), while Runna's 26 is a
**coaching choice** and their 16 is *also* a coaching choice, applied to a different plan
family. Worth being explicit about which of our limits are ours, because right now the
code comment is the only place that distinction exists.

---

## 8. Plan ↔ activity reconciliation (answers a Phase 4 open question)

Phase 4 (`RUNNA-PROGRESS-AND-REVIEW.md` §8.5) concluded that Runna's `PLAN RUNS
COMPLETED` counters imply a plan↔activity link and that Runalytics has none. Article 17
confirms the mechanism and names it:

- **Manual activity:** Progress tab → **+** → type → distance/time → save.
- **Link Activity:** Plan tab → open the workout → **Link Activity** → pick an activity
  from the Progress tab. *"This helps keep your plan aligned and ensures completed runs
  connect back to the right scheduled workout."*
- **Many-to-one is allowed:** *"you may still be able to link multiple sessions to one
  planned workout, depending on how they were recorded."*
- **Activities are immutable; the workaround is delete-and-replace:** *"Once a run is
  saved in Runna, you can't directly edit its distance or time"* → delete, add a Manual
  Activity, link it. For Strava-sourced runs: fix at source, pull-to-refresh, and only
  then delete/re-import.
- **Un-completing exists:** tap the ✅ to mark a completed activity incomplete, unlink,
  drag the workout forward up to a week, redo it.
- **Manual activities do not adapt the plan:** *"they don't usually rebuild or adapt your
  plan automatically."*

Two more boundaries that define what "counts":

- **Instant Workouts and Free Runs are outside the plan.** *"Additional runs you do as
  Instant Workouts will not count towards your plan."* A **Free Run** (Today → Record
  Workout) is unstructured and lands in the Progress tab only.
- **Cross-training never counts.** Article 26 states it three times: cycling, swimming,
  rowing, sports *"can't be scheduled as part of your plan, counted toward adherence, or
  used to influence plan adaptations."* Even the app-offered Strength/Yoga/Pilates
  sessions *"sit outside the adaptive running plans… they won't influence adherence,
  intensity guidance, or plan adaptations."*

That last one is a **deliberate scope limit with a stated reason** (article 26):
*"Accuracy. Each sport requires different loading models and recovery timelines. Keeping
plans run-specific ensures correct pacing, progression, and adaptation."*

**Runalytics:** we have `all_activities` (`commands.rs:324`) and `ActivityDetail`, but no
link table between an `Activity` and a `PlanWorkout`, and `PlanWorkout.done: bool` with no
`activity_id`. Runna's model says the link is worth having *and* that it should be
user-initiated (Link Activity) rather than auto-matched — which is a cheaper build than
auto-reconciliation and avoids the false-positive problem entirely.

---

## 9. Where the documentation contradicts the recordings (and itself)

Five discrepancies found. Each is a real finding, not a transcription error.

**1. B-race distance rule — two articles disagree.**
Article 1: *"The B-Race is longer than your goal race distance — the app only supports
B-races that are **shorter** than your A-race distance."*
Article 23: *"B-races must not be longer than your main goal race — they can be **the same
distance or shorter**."*
Article 1 hedges in its own FAQ (*"What if I want to add a B-Race at the same distance?"*
→ coaches recommend shorter, but here are your options), so the truth is probably "same
distance is possible but discouraged and flagged". **Lesson: two articles in one
collection disagree; neither is marked as newer.** Any consumer of vendor docs needs the
article's own update date, which Intercom does not expose in this markup.

**2. Plan length: 23 weeks on screen vs a 26-week cap.**
Phase 3 recorded a wizard offering *"Now 23 Week Plan — Recommended for maximum training
time"*. Four articles state a hard 26-week cap. Both are true — 23 was what the date
math produced, not the ceiling — but it shows the wizard's "maximum" language means
*maximum available for your date*, not *product maximum*. Easy to misread.

**3. Deload weeks are invisible in the UI.**
Article 16: *"Runna doesn't explicitly label a week as a deload week in the app"* — and the
user is instructed to infer it from a mileage dip. Yet article 21 says *"You can see your deload
weeks in advance in your plan's mileage graph."* Phase 3's screenshots show a week
progress bar and a mileage graph but **no deload marker**. So the docs point at a graph
that does not, in fact, label them. This is a genuine UI gap that only surfaces by
comparing claim to capture.

**4. "You can't skip forward" vs "leave them unticked and it self-heals."**
Article 9 forbids skipping ahead to future weeks (support must do it manually), while the
same article's realignment ladder *skips missed weeks* for you under
**Continue-unchanged**. Not a contradiction exactly, but the user-facing rule ("you can't
jump ahead") and the system capability ("the app can jump you ahead") are opposite. The
control is deliberately withheld from the user and retained by the planner.

**5. Feature availability is a version matrix the UI doesn't explain.**
Minimum versions named across the collection: **7.2** (Training Preferences, Holiday
Mode), **7.31.2** (B-Race), **8.25** (Not Feeling 100% editing), **8.49.0** (Your Plans) —
plus a **plan version** axis (*"Your plan must be updated to the latest version (v11)"*)
and a **plan-type** exclusion list that recurs verbatim: *New to Running, Return to
Running, Path to parkrun, Hyrox, Functional Fitness, Post-Natal, Post-Injury, Post-Race
Recovery*. Three independent gates (app version × plan version × plan type) and several
articles say *"it is being gradually rolled out"* — so two users on the same build see
different feature sets. Our Phase 3 capture (v8.59.0) is past all four thresholds, yet
the onboarding in that recording still had to *teach* the features, which suggests
rollout-era confusion is real.

---

## 10. What this changes for Runalytics

Ranked by how much user-visible value per unit of work.

**1. Adopt the unticked / skipped / missed distinction (one enum, unlocks a ladder).**
`PlanWorkout.done: bool` (`types.rs:402`) cannot express what Runna's entire Realignment
feature depends on. A three-state `WorkoutStatus { Planned, Skipped, Missed }` — where
*Missed* is derived (date passed, not done, not skipped) rather than stored — is a small
change that makes the graded 1 / 3+ / 2wk / 4wk ladder buildable on top of the proposal
engine we already have (`proposal.rs`, with its 7-day dismissal suppression). Runna's
key trick costs nothing to implement: **the trigger is the absence of an action**, so
there is no UI to build for it, only a detector.

**2. Ship a conflict screen before shipping multi-plan.**
If we ever allow a second plan, the overlap screen (§3) must exist first. Runna's pattern
is copyable exactly: compute the overlap, show what happens to each affected plan,
default to the reversible option, and never delete — move to drafts. Cheap, and it is the
difference between a plan manager and a plan destroyer.

**3. Name the two verbs: reconfigure vs reschedule.**
Runna prints the same two-row table in two different articles to stop users dragging a
race into the wrong week. Our app has both mechanisms and no statement of which to use —
and our `move_workout` guards (`calendar.rs:394`: race day fixed, completed rejected,
no move into the past, no move past the plan's end) already *enforce* a boundary silently.
Note the ranges differ in the opposite direction from what you'd expect: Runna allows a
drag of only **±1 week**, while ours allows any day from today to the end of the plan —
we are more permissive and explain less. A one-line hint at the
top of the calendar ("to change your plan, not just this week, use Plan settings") would
make our existing guards legible instead of mysterious.

**4. Pre-flight impact preview on plan mutations.**
Runna's *"Training Disruption: low / medium / high"* + per-week *"Adjustment Insights"*
before committing is the single best pattern here, and it is exactly what our planner can
already do — `engine.rs` is a pure function from request to weeks, so evaluating a
hypothetical and diffing it against the current plan is arithmetic, not new machinery.
We currently apply plan changes and let the user notice the consequences.

**5. Split our one volume knob into volume / difficulty / pace-target.**
Runna's orthogonality promise (§6.1) — *"lowering Difficulty may result in fewer reps, but
the target paces will remain the same"* — is a testable contract. Our `IncreaseMethod`
moves peak volume and growth rate together (§6). If we split the knobs, we can make the
promise *and* assert it: a test that changes difficulty and asserts
`pace_lo_s_km`/`pace_hi_s_km` (`types.rs:410`) are unchanged. That is the kind of
guarantee that makes a plan feel trustworthy rather than arbitrary.

**6. Keep the long run on its own progression — and say so.**
We already do (`peak_long_run_km`, `long_run_share`, the `+8 %` build cap at
`engine.rs:842`), and Runna's independent documentation of the same two rules is useful
confirmation. But we never *tell the user* the long run has its own governor. Runna
doesn't either (§9.3) — which is precisely why we should.

**7. Label the deload week.**
Runna's own docs tell users to guess. Our 3:1 cycle is deterministic, so a "recovery week"
badge on the week card is free, and it turns a scary mileage dip into an explained one.
This is the cheapest UX win available from the whole collection.

**8. Add `Link Activity` rather than auto-reconciliation.**
Per §8: a user-initiated link (activity ↔ planned workout) gives us the
`PLAN RUNS COMPLETED`-style adherence view without building a matcher, and matches how Runna
actually ships it. Note their extra affordances worth keeping: un-complete (tap ✅),
many-activities-to-one-workout, and manual activities that deliberately do **not**
trigger adaptation.

**9. Publish the limits.**
Runna's help centre states every cap with a reason (*26 weeks "to help avoid mental and
physical burnout"*; ability-gated runs-per-week; minimum 2 runs). Our caps live in code
comments. A short "how Runalytics builds your plan" panel — levers, what each changes,
what each can't — would be the single biggest trust gain, and it is documentation, not
engineering.

---

## 11. Open leads (other collections, not crawled here)

> **CLOSED.** All nine were fetched and analysed in Phase 6 —
> see [`RUNNA-INSIGHTS-AND-ADAPTATION.md`](./RUNNA-INSIGHTS-AND-ADAPTATION.md).
> The Riegel question resolved as "no formula is disclosed; the evidence points to a fixed
> cohort curve", Runna Score turned out to be a rescaled estimated race time, and the
> fixed-distance PR categories are a deliberate dual-unit product decision. The list is kept
> as written for the record.

Titles surfaced as related links inside this collection, each a likely spec document:

- *How Is My Target Race Time Predicted?* — would confirm/deny the Riegel b = 1.0843
  finding from Phase 3
- *What Are Pace Insight Recommendations and How Do They Work?* — the pace-suggestion
  loop referenced repeatedly in article 7 and 22
- *What Are Mileage Insights?* — competes with Plan Realignment for the same pop-up slot
- *What Is My Runna Score?* — unexplained in every phase so far
- *How Do Personal Records Work In The Runna App?* — would explain the fixed-distance PR
  categories seen in Phase 4 (5K/10K/HM/M **and** 5mi/10mi, regardless of unit setting)
- *How Does Runna Build Recovery Into Your Training Plan?* and
  *How Does Runna Reduce the Risk of Injury and Overtraining?*
- *How to Use Instant Workouts* and *How to Use and Manage Your Target Mileage Settings
  on Run Further / Train Your Way / Run to Maintain Plans* — the target-mileage plans are
  a whole plan family we have not seen on screen
- *Syncing Runna to Your Calendar & Scheduling Your Workouts* — pairs with our ICS work

## 12. Evidence index

| artefact | what it is |
| --- | --- |
| `support/raw_articles.json` | all 28 articles, structured (headings/paras/lists/tables/img alts), 236 KB |
| `support/all_articles.md` | the same as readable text, 2,131 lines, numbered 01–28 with URLs |
| `support/to_md.cjs` | the JSON→text converter |
| collection root | https://support.runna.com/en/collections/16743424-plan-creation-management |

Article numbering used above: 01 B-Race · 02 Minor Aches · 03 Running Schedule ·
04 Extra/parkrun/Club runs · 05 Pause Your Plan · 06 Running Ability · 07 Estimated Race
Time · 08 Training Units · 09 Plan Realignment · 10 Training Preferences · 11 Training
Calendar · 12 Holidays · 13 Training Focus · 14 Not Feeling 100% · 15 Race/End Date ·
16 Mileage Calculation · 17 Log/Link Activity · 18 Between Race Blocks · 19 Skip a Run ·
20 Difficulty (beginner plans) · 21 Deload Weeks · 22 Build Around Fitness · 23 Create a
Plan · 24 Yoga/Pilates/Stretch · 25 Strength · 26 Cross-Training · 27 Your Plans ·
28 Do Not Share My Personal Information (no product content).
