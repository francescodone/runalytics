# Runna — Plan Generation: Functionality and UI/UX Analysis

**Subject:** Runna iOS app (`com.runbuddy.prod` v8.59.0), plan-creation wizard
**Evidence:** user-supplied screen recording `ScreenRecording_10-26-2026 23-01-59_1.MP4`
(1180×2556, 60 fps, 83.55 s), analysed frame-by-frame
**Companions:** [`RUNNA-PLAN-EXECUTION.md`](./RUNNA-PLAN-EXECUTION.md) — living with the generated plan ·
[`RUNNA-UX.md`](./RUNNA-UX.md) — the execution/feedback side of the same product ·
[`RUNNA-PROGRESS-AND-REVIEW.md`](./RUNNA-PROGRESS-AND-REVIEW.md) — the Progress tab, whose
PR board independently backs the 18:00 5K input this wizard accepts ·
[`RUNNA-PLAN-MANAGEMENT.md`](./RUNNA-PLAN-MANAGEMENT.md) — the same inputs as editable
settings after creation, plus Runna's published progression rules and plan-length caps ·
[`RUNNA-INSIGHTS-AND-ADAPTATION.md`](./RUNNA-INSIGHTS-AND-ADAPTATION.md) — what those inputs
become once the plan is live: the adaptation loop, and why race time is an output, not an input
**Screens:** [`plan-wizard/`](./plan-wizard/) — 28 ordered captures, numbered in wizard order

---

## 1. Method and limits

The recording was decoded at 2 fps (167 frames) and segmented by perceptual
difference: a greyscale proxy resized to 100×200, cropped to drop the top 7 %
(the status-bar clock otherwise produces a false "new screen" every minute),
compared with a thresholded pixel difference. A screen counts as *settled* only
after it stays stable across consecutive frames, and near-identical settled
frames collapse to one capture.

Two passes were needed. A first pass with an aggressive dedupe returned 12
screens; a second pass with a much looser threshold over four time windows
returned 28. The aggressive pass had silently swallowed every screen the user
passed through quickly — which turned out to be exactly the most interesting
ones (running ability, injury history, days per week, the volume presets).
The check that caught this: the final summary screen lists seven inputs the
user claims to have supplied, and only four of them appeared in the 12-screen
set. **A summary screen is a checklist for your own extraction.**

What this means for the claims below:

- **Copy is verbatim.** Every quoted string was read off a full-resolution frame.
- **Ordering is real**, taken from frame timestamps.
- **The generated plan is not observed.** The recording ends 3 s after the
  "Generate my plan" button is visible, still on the summary screen. Everything
  about *what a generated week looks like* comes from the ICS analysis in
  `RUNNA-UX.md`, not from this video.
- **Internal logic is inferred** from inputs and the numbers the UI prints. Where
  a rule is a guess rather than an observation, it is marked *inferred*.

---

## 2. The wizard as a whole

Twenty-eight captured states, but only **fourteen are questions**. The rest are
the empty Plan tab, three loading states, and two modal sheets. The wizard is
short; it just looks long because the app renders a skeleton before every
computed value.

| # | Screen | Type |
|---|--------|------|
| 01 | Plan tab, empty state | entry |
| 02 | "What do you want to do next?" | question — personalised |
| 03–04 | "Do you have your next race planned?" | question |
| 05–06 | "Find your race" | search |
| 07 | Race detail | confirmation |
| 08–09 | "How would you rate your running ability?" | question |
| 10 | "Confirm your injury history" | question |
| 11 | "What's your estimated current race time?" | question |
| 12 | "How many days per week would you like to run?" | question |
| 13 | "Which day do you want to do your long runs on?" | question |
| 14 | "When do you want to start your plan?" | question — derived options |
| 15–17 | "Set your training preferences" | computed card, 3 states |
| 18 | Training Preferences sheet | modal |
| 19–22 | Volume editor | slider + presets |
| 23 | Custom Difficulty | advanced sub-panel |
| 24 | "How far have you been running per week recently?" | question |
| 25 | Preferences, revisited | return visit |
| 26 | "What else are you interested in?" | optional multi-select |
| 27 | Skeleton | loading |
| 28 | "Your plan is nearly ready" | **receipt** |

### 2.1 Chrome rules, observed consistently

- **Back chevron left, ✕ right** on every wizard screen. The ✕ is always present,
  so the wizard is never a trap.
- **A segmented progress bar appears from the second question onward** and is
  absent on the entry screen. It is a thin centred bar, not a labelled stepper —
  it conveys "there is a finite path" without making the user count steps.
- **`Continue` is a full-width filled pill pinned to the bottom**, and it is
  *disabled and greyed* until a selection exists (screens 03, 13, 08). The app
  never shows an error for an unanswered question — the button simply will not
  advance. Secondary actions are plain text links below it ("See all plans").
- **A help affordance appears only on questions requiring self-judgement** —
  `?` on current race time, running ability, days per week, long-run day, start
  date; `ⓘ` on the volume editor. Factual questions (which race, which days)
  have none. The icon is placed top-right, in the header row, never inline.
- **Teal is the single accent** and carries three distinct meanings: *selected*
  (border on the chosen option), *recommended* (the "Today" chip), and
  *engine-derived value* (the `~11km` caps, the `37km` readout). Every other
  colour in the wizard is decorative or semantic-only.
- **Skeletons, not spinners.** Each computed block renders as a grey placeholder
  of its own final shape (screen 15, 27), so the layout never jumps when the
  value lands.

### 2.2 The entry screen is already personalised

Screen 02, "What do you want to do next?", is the first thing the wizard shows —
and it is not a generic menu:

> **Train for a race or distance**
> **Run without a race goal**
> **Focus on rest and recovery** — eyebrow: *"Recommended by the coaches"* —
> "You've gone long. Rest now so you can rebuild strong and stay healthy."

The third card is pre-selected with a teal border. The copy states the reason:
*"You've gone long."* The app has read recent training load and is arguing for
rest before it has asked a single question. Note also that this is the **only**
screen with no back chevron and no progress bar — it is a fork in the product,
not step one of a form.

This is a deliberate contrast with the web onboarding, which opens with a
generic "How can we help you?" / "I want to…" offering four unpersonalised
options (Train for a race, Start running, Return to running, Improve my general
fitness). **The app's first question is answered; the web's first question is
asked.**

---

## 3. The input model

The generator consumes fourteen inputs. Grouped by what they do:

### 3.1 Goal and identity (routes the wizard)

| Input | Screen | Options |
|-------|--------|---------|
| Goal | 02 | race / no race / rest |
| Race already chosen? | 03 | Yes / No (help me find a goal) |
| Race | 05–07 | search + filter by Dates, Distance, Country, City |

The race detail screen (07) is where the generator picks up the physical
characteristics of the target, and it is worth reading closely:

> **Berlin Half Marathon** · `Half Marathon`
> 📅 Apr 4, 2027 · 🇩🇪 Berlin, Germany · **↕ Flat** ⓘ · 👥 2,152 other Runnas
> "The Berlin Half Marathon in Berlin, Germany is a premier international road
> running event featuring only the half marathon distance of 21.0975km. Known
> for its fast, flat course and large global field…"
> VISIT WEBSITE · REPORT PROBLEM

Three things happen here. The **elevation profile is a first-class field on the
race record**, not something the user is asked to declare — it arrives with the
race. The distance is stated to four decimals (21.0975 km), which is what the
pace maths will use. And the social-proof count ("2,152 other Runnas") is
repeated from the search list, reinforcing the choice at the moment of commitment.

### 3.2 Runner self-description (sets the ceiling)

| Input | Screen | Options |
|-------|--------|---------|
| Running ability | 08 | Beginner / Intermediate / Advanced / Elite |
| Injury history | 10 | free-text-ish field, default *"Frequently or recently injured"* |
| Current race time | 11 | 5k / 10k / Half / Marathon + h:m:s wheel |
| Recent weekly volume | 24 | slider, *"past 4 weeks"* |

The ability levels are defined **behaviourally, not by time** — this is the
single most transferable idea in the wizard:

> **Beginner** — "You can complete a 5km run without stopping, in under 60 minutes"
> **Intermediate** — "You regularly run at least 5km but don't structure your training or put thought into a plan"
> **Advanced** — "You regularly run at least 10km and do some structured training e.g. intervals"
> **Elite** — "You regularly run half-marathons or further and are experienced with structured training e.g. intervals"

Each has a small ring gauge that fills with level. Choosing **Elite** reveals an
additional inline toggle: *"Enable Elite+ for extra mileage"* — the paywall is
attached to the top ability tier rather than gating the wizard.

The injury question is framed as safety, not segmentation:

> **Confirm your injury history**
> "Help us ensure your training plan is safe and effective."
> `INJURY HISTORY` → *Frequently or recently injured* ✎
> "You can revisit this later in your profile"

And the current-time question carries an explicit anti-cheat instruction, with
"current" coloured teal in both the title and the body:

> "Choose a time reflective of your **current** fitness level - don't use an out
> of date PB or goal time!"

### 3.3 Schedule constraints (shape the calendar)

| Input | Screen | Options |
|-------|--------|---------|
| Days per week | 12 | 2–7 Days |
| Long-run day | 13 | Monday–Sunday |
| Availability | — | all seven days (echoed on the receipt) |

Days-per-week is the only question in the wizard whose subtitle is a *warning*
rather than a clarification:

> "This should be at most once more than you currently run per week to reduce
> the risk of injury"

The app is telling the user the answer they are about to give can hurt them,
before they give it. It then enforces the consequence elsewhere (see §4.3)
rather than blocking the selection.

### 3.4 Preferences (tune the curve)

| Input | Screen | Options |
|-------|--------|---------|
| Training volume | 19–22 | Gradual / Steady / Progressive, or Custom |
| Difficulty | 23 | preset, or Custom with four dials |
| Start date | 14 | four derived dates, or Custom |
| Interests | 26 | Strength / Stretching / Injury mgmt / Nutrition / Sleep |

Screen 26 is optional multi-select ("Choose as many as you like") and is clearly
adjacent-product surface — it is the same five categories that appear as
upsells elsewhere in the app (see `RUNNA-UX.md` §3). It does not feed the run
plan; it feeds the cross-sell.

---

## 4. Generation logic, inferred from what the UI prints

The wizard never shows a formula. But it prints enough concrete numbers, in
enough related pairs, to reconstruct the shape of the engine.

### 4.1 Plan length is arithmetic on the race date

Screen 14 offers four start dates for a race on **Sun, Apr 4, 2027**:

| Start | Label | Days to race | ÷ 7 |
|-------|-------|-------------|-----|
| Oct 26, 2026 | **Now 23 Week Plan** | 160 | 22.86 → **23** |
| Dec 14, 2026 | 16 Week Plan | 111 | 15.86 → **16** |
| Jan 11, 2027 | 12 Week Plan | 83 | 11.86 → **12** |
| Jan 25, 2027 | 10 Week Plan | 69 | 9.86 → **10** |

All four are exactly `round(days_to_race / 7)`, and all four start dates are
**Mondays** while the race is a **Sunday**. So:

- Plans are **Monday-anchored**; the race lands on the final Sunday.
- Duration is not a menu the user picks from — it is **derived from the start
  date**, and the app presents four candidate start dates so the user chooses
  *when*, and reads back *how long*.
- The first option is labelled **"Now 23 Week Plan"** with a `Today` chip,
  making the default the maximal-training-time choice.

Each option carries a coaching rationale, which is the part worth stealing:

> Now 23 Week Plan — *"Recommended for maximum training time"*
> 16 Week Plan — *"Recommended for longer training time"*
> 12 Week Plan — *"Recommended for balanced training amount"*
> 10 Week Plan — *"Recommended for fast-track training"*

The user is never told a 10-week plan is a bad idea. They are told what each
option *buys*, in order of decreasing preparation, and left to choose. The
framing makes the longest plan the obviously-correct answer without forbidding
the others.

### 4.2 Volume resolves to two hard caps

The volume editor is a slider over a named preset, with a chart of the ramp
curve (x-axis `PLAN STARTS` → `PLAN ENDS`, one teal curve highlighted over
faint grey alternatives — the other presets, visible as context). The slider
snaps between three named positions and the card below restates the consequence
in distance, not effort:

| Preset | Slider | Descriptor | Hard/easy run max | Long run max |
|--------|--------|-----------|-------------------|--------------|
| **Gradual** | left | *"Low volume with a minimal disruption to daily life, offering gentle progression to suit busy schedules and diverse commitments."* | ~9 km | ~16 km |
| **Steady** | middle | *"Medium volume that balances steady progress with sustainable training. Great for staying challenged while maintaining a manageable routine."* | ~11 km | ~19 km |
| **Progressive** | right | *"High volume for those looking to push their limits and achieve peak performance. Best if you're aiming to prioritize running in your schedule."* | ~13 km | ~23 km |

Two design decisions here are unusually good:

1. **The presets are named by temperament, not intensity.** "Gradual", "Steady",
   "Progressive" describe the runner's relationship to training. Nothing is
   called "Easy" or "Hard", so picking the low option is not an admission.
2. **The consequence is stated as a distance ceiling before you commit.** "Your
   plan will typically include: Long run max ~19km" is the single most honest
   thing in the wizard — it answers "how long will my longest run be?" at
   selection time, which is the question every runner actually has.

**Advanced Customization** (a toggle, off by default) replaces the presets with
two direct sliders, `Hard/easy run max` and `Long run max`, and the note
*"Disable Advanced Customization to select a preset volume"* — the two modes are
mutually exclusive and the UI says so. In the recording, Custom at max produced
`~11km` / `~16km`, i.e. the user's custom curve was *lower* than Steady's long
run. The final receipt confirms both fields resolved to "Custom".

### 4.3 Difficulty is four separate dials

Screen 23, **Custom Difficulty**, is the most revealing screen in the recording
because it exposes the engine's actual control surface:

> A **custom difficulty** tailored to your own needs. Adjust these settings to
> get more control over how you train towards your goal.
>
> | Dial | Subtitle | Value |
> |------|----------|-------|
> | **Number of Hard Runs** | "Frequency of tempo, hills and interval workouts" | **0 or 1 per week** |
> | **Hard Run Difficulty** | "Proportion of speed mileage in hard runs" | **Reduced** |
> | **Long Run Structure** | "Frequency of long runs with pace targets" | **Greatly reduced** |
> | **Long Run Difficulty** | "Difficulty of long runs with pace targets" | **Reduced** |

Read as a set, this is a *de-loading* configuration: one hard session a week,
less speed inside it, fewer long runs carrying pace work, and less pace inside
those. Combined with the injury answer in §3.2 and the banner in §4.4, the
recording is showing the injury adjustment applied and made inspectable.

The vocabulary is also notable. Difficulty is expressed as **Reduced / Greatly
reduced**, not numbers. The engine has internal magnitudes but the UI reports
them as qualitative states on a discrete slider — four positions, three of which
have names the user can reason about.

### 4.4 The engine mutates the plan, and says so

Screens 15 → 16 → 17 are the same screen in three states, and the sequence is
the clearest UX pattern in the whole flow:

1. **15 — pending.** The card is an empty grey block. Nothing is claimed yet.
2. **16 — computed.** The card fills: four icon tiles, `TRAINING VOLUME: Custom`,
   `DIFFICULTY: Custom`, `EDIT PREFERENCES`.
3. **17 — adjusted.** A solid teal banner appears above the card:
   **"✓ Adjusted for your injury history"**

> "We've customized your training volume and difficulty for you. You can revisit
> this later in 'Manage Plan'."

The banner is the whole point. The engine did something the user did not ask
for, and the app **volunteers the reason, unprompted, in a positive frame, at
the moment the value appears.** It does not bury it, and it does not silently
lower the training. This is what makes an automatic adjustment trustworthy
rather than suspicious — and it is cheap to implement: one conditional banner
with a reason string.

Note the ordering: the adjustment is announced *before* the user can inspect the
numbers, so when they open the sheet they are already primed to expect values
that look conservative.

### 4.5 The current-volume question is a slider, prefilled

Screen 24, asked *after* the preferences have already been computed:

> **How far have you been running per week recently?**
> "Select your average weekly mileage from the past 4 weeks and would feel
> comfortable running this week. You can update this anytime in 'Manage Plan'."
> `CURRENT WEEKLY MILEAGE` — slider → **37km** · button reads **Update**

Two things. The button says **Update**, not Continue, because this screen is a
correction pass — the app prefilled 37 km from the connected device history and
is asking the user to confirm or fix it. And the phrasing admits the question is
unanswerable precisely ("…from the past 4 weeks and would feel comfortable
running this week"), so it uses a slider rather than buckets. A slider with a
prefilled value is the right control for a guess.

The Training Preferences sheet (18) then reports the two numbers the engine
derived from everything above:

> `CURRENT` — **Weekly Mileage 37km** · **Longest Run 14km**

`Longest Run 14 km` against a Steady preset whose long-run *cap* is ~19 km is
the distinction between **current** and **ceiling**: the engine knows where you
are and where the plan is allowed to go, and the two are never conflated in the UI.

### 4.6 A visible bug

On screen 11, the sentence above the wheel reads *"I can currently run a 5k in
**45:00**"* while the wheel itself was scrolled from 45m down through 16m, 17m
to **18m**. The sentence did not update during the scroll. The receipt on
screen 28 states **"Your estimated current 5km time is 18:00"**, so the *stored*
value tracked the wheel correctly — only the summary sentence above it is stale.

Worth recording because it is the specific failure mode of mirroring a picker's
value in a sentence: the sentence is a second render of the same state, and if
it is not on the same update path as the control, it lies. The consequence here
is mild (the receipt is right), but in a plan generator a stale fitness input
displayed at confirm-time would be a trust problem.

---

## 5. The plan receipt — the headline pattern

Screen 28, "Your plan is nearly ready", is the reason this wizard works:

> *[full-bleed dusk photograph of Berlin, red shield badge reading **21.1**]*
> **Berlin Half Marathon Plan**
> ⇅ 23 weeks · 21.1 km
> 📅 Sun, Apr 4, 2027
>
> **Your plan is customized based on these details**
> · Your estimated current 5km time is **18:00**
> · You are available to run on Mon, Tue, Wed, Thu, Fri, Sat, Sun
> · You'd like your long run to be on Sunday
> · Your plan starts on Mon, Oct 26, 2026
> · You are training for a race with a **Flat elevation profile**
> · You have selected a Custom training volume
> · You have selected a Custom difficulty
>
> **[ Generate my plan ]**

Observations:

- **Not one workout is shown.** The user answers fourteen questions and is
  offered zero exercise previews before committing. What they get instead is
  *proof of listening*.
- **Every input is echoed, in the user's own grammatical person** — "You'd like
  your long run to be on Sunday", not "Long run day: Sunday". The receipt reads
  as a coach restating a conversation.
- **The echo list is the justification.** "Your plan is customized based on
  these details" makes the bullets do argumentative work: this plan is not a
  template, here is the evidence.
- **The order of the bullets is not the order of the questions.** Fitness comes
  first, then availability, then long-run day, then start date, then elevation,
  then the two preference fields. It is ordered by *how much each input shaped
  the output*, which is a claim the engine is confident enough to make.
- **The elevation profile appears on the receipt but was never asked.** It came
  from the race record. Putting a derived input in the same list as chosen ones
  quietly communicates that the app knows things it wasn't told.
- **The badge is the race distance, not a level.** `21.1` in a red shield over a
  photograph of the host city — the plan is branded as the *event*, which makes
  the plan feel like a ticket rather than a spreadsheet. The same shield appears
  on the share card documented in `RUNNA-UX.md` §5, so the visual identity is
  consistent across generation, execution, and sharing.
- **"Nearly ready"** is exactly right. It promises the plan exists and is one
  tap away, without claiming it has been delivered.

The button is the standard filled pill. There is no "Edit these details" link on
this screen — going back is done with the chevron, and the receipt does not
invite second thoughts.

---

## 6. What this changes for Runalytics

Runalytics already implements the *output* half of Runna's model. The gap this
recording exposes is entirely on the *input* side.

### 6.1 Already ours

`src-tauri/src/calendar.rs` has the ceiling/window grammar that Runna's
prescriptions use (see `RUNNA-UX.md` §4):

- `no_faster_than(low)` → `"no faster than {}/km, at conversational pace 🗣️"` —
  the cap form, including the disarming tone.
- `lap_pace(step)` matching `(open_ended, pace_lo_s_km, pace_hi_s_km)` →
  `" @ {lo}-{hi}/km"` — the window form.
- The test `quality_sessions_are_named_and_easy_steps_have_no_upper_bound`
  already encodes the rule that easy days get a ceiling and not a target.

`src-tauri/src/plan/engine.rs` is already calibrated against Runna's published
marathon builds (peak 76–80 km/week, biggest long runs 32.4 km and 35.0 km),
with `peak_long_run_km` = 35.0 and `long_run_share` = 0.44 for K42.

### 6.2 Gaps worth closing, in priority order

1. **The plan receipt.** This is the highest value-per-line change in the whole
   analysis. Runalytics generates a plan and shows it. Before showing it, echo
   the inputs back in second person as the stated basis. The data is already in
   the engine's input struct; this is a rendering task, not a modelling one.
   Pattern: `plan/receipt.rs` → a list of `(input, sentence)` pairs, ordered by
   influence, rendered above the first workout.

2. **Start date as plan length.** Runalytics takes a plan length. Runna takes a
   start date and derives length as `round(days_to_race / 7)` from a set of
   Monday-anchored candidates, each labelled with what it buys. If Runalytics
   knows a race date, offering "Now 23 Week Plan / Recommended for maximum
   training time" is strictly better UX than a number field — and it is nine
   lines of date arithmetic, verified above against four real examples.

3. **Announce automatic adjustments.** The `✓ Adjusted for your injury history`
   banner is the trust mechanism. Runalytics' `plan/adaptive.rs` already mutates
   plans; it should surface *that it did* and *why*, at the moment the value
   renders, in a positive frame. An unannounced adjustment reads as a bug.

4. **Separate current from ceiling.** Runna keeps `Weekly Mileage 37km` /
   `Longest Run 14km` (where you are) visibly distinct from `Long run max ~19km`
   (where the plan may go). Runalytics' `long_run_share` produces the ceiling;
   the current value should be displayed beside it, not merged into it.

5. **Ceilings as the unit of choice.** "Your plan will typically include: Long
   run max ~19km" answers the user's real question at preference time. Any
   volume control in Runalytics should state its consequence in km, not in
   percentages or "moderate/high" labels.

6. **Behavioural ability levels.** If Runalytics ever asks a self-assessment
   question, define the options by behaviour ("you regularly run at least 10km
   and do some structured training e.g. intervals"), never by pace or by
   adjectives.

7. **Name volume presets by temperament.** Gradual / Steady / Progressive —
   nobody is choosing to be the lazy one.

### 6.3 Explicitly not copying

- **The upsell surface.** Screen 26's five interest categories are cross-sell
  for strength/nutrition/sleep products. Runalytics is a single local app with
  no adjacent products; the screen has no purpose here.
- **The web/app divergence.** The web onboarding asks a generic first question;
  the app's first question is already answered from training data. Runalytics
  has the training data locally and no reason to open with a generic menu.
- **The stale-picker bug** (§4.6). If a Runalytics control mirrors its value in
  a sentence, put both on the same render path.

---

## 7. Evidence index

`plan-wizard/` — 28 frames in wizard order, filenames encode the screen.

| File | t | Shows |
|------|---|-------|
| `01-plan-tab-empty.png` | 0.0 s | entry point, empty Plan tab |
| `02-goal-personalised.png` | 2.0 s | rest recommended from real load |
| `03`/`04-race-planned*.png` | 6.0 / 8.0 s | disabled → enabled Continue |
| `05`/`06-find-your-race*.png` | 9.5 / 10.5 s | skeleton → populated, social proof |
| `07-race-detail.png` | 13.0 s | Flat profile, 21.0975 km, 2,152 Runnas |
| `08`/`09-running-ability*.png` | 14.5 / 16.0 s | behavioural levels, Elite+ toggle |
| `10-injury-history.png` | 18.0 s | safety framing |
| `11-current-race-time.png` | 20.0 s | wheel, stale sentence bug |
| `12-days-per-week.png` | 26.5 s | injury warning |
| `13-long-run-day.png` | 30.5 s | Monday–Sunday |
| `14-start-date.png` | 33.5 s | four derived durations + rationale |
| `15`→`16`→`17-preferences*.png` | 36.5 / 38.0 / 40.0 s | pending → computed → **injury banner** |
| `18-preferences-sheet.png` | 44.0 s | CURRENT 37 km / 14 km |
| `19-volume-custom-advanced.png` | 47.5 s | custom caps |
| `20`/`21`/`22-volume-*.png` | 60.0 / 61.0 / 63.0 s | Progressive / Steady / Gradual |
| `23-custom-difficulty.png` | 67.0 s | the four dials |
| `24-how-far-running.png` | 69.5 s | prefilled slider, "Update" |
| `25-preferences-revisit.png` | 73.5 s | return visit |
| `26-what-else-interested.png` | 75.5 s | optional upsell multi-select |
| `27-skeleton-loading.png` | 77.5 s | pre-receipt skeleton |
| `28-plan-receipt.png` | 80.5 s | the receipt |

Raw pipeline artefacts (frame diffs, scene lists, contact sheets) were kept
out-of-tree under `/tmp/runna_vid/`; the 28 frames above are the durable
evidence.

**Not covered by this document:** the generated week view, dragging workouts to
rearrange them, plan management ("Manage Plan"), and adaptive re-planning after
missed sessions. The recording ends 3 s short of the first of these.

The generated-plan side is documented in
[`RUNNA-PLAN-EXECUTION.md`](./RUNNA-PLAN-EXECUTION.md), from a second recording
that starts one second after "Generate my plan". It confirms the receipt's
`18:00` 5k echo as the single seed of the whole Estimated Race Times table
(§4 there), and it covers the rearrange flow that "Rearranging Runs" on the
post-generation tour promises.
