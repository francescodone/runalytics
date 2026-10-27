# Runna — The Progress Tab and the Activity Review: Functionality and UI/UX Analysis

**Subject:** Runna iOS app (`com.runbuddy.prod` v8.59.0), the Progress tab and the
post-run activity review
**Evidence:** five user-supplied screenshots, supplied inline in conversation
**Companions:** [`RUNNA-UX.md`](./RUNNA-UX.md) — session execution and feedback ·
[`RUNNA-PLAN-GENERATION.md`](./RUNNA-PLAN-GENERATION.md) — how the plan is built ·
[`RUNNA-PLAN-EXECUTION.md`](./RUNNA-PLAN-EXECUTION.md) — living with a generated plan ·
[`RUNNA-PLAN-MANAGEMENT.md`](./RUNNA-PLAN-MANAGEMENT.md) — §8 of that doc answers the
plan↔activity join left open here (§8.5): the feature is called **Link Activity** ·
[`RUNNA-INSIGHTS-AND-ADAPTATION.md`](./RUNNA-INSIGHTS-AND-ADAPTATION.md) — §4 and §5 here
resolve the two mysteries left open: the fixed dual-unit PR set (their §4.2) and Runna Score
(it is a rescaled estimated race time, their §5)
**Screens:** [`progress/`](./progress/) — **not archived**, see §1.3 and that folder's README

---

## 1. Method, provenance, and limits

### 1.1 What this phase covers

Phase 1 could reach only two of Runna's five tabs; it recorded the Progress tab as
**never visually documented** and noted that a deep link to `/progress` *falls back to
Today* (`RUNNA-UX.md` §6, `EVIDENCE-deeplink-probe.log`). Phases 2 and 3 came from
screen recordings and covered plan creation and plan execution. These five screenshots
are the first evidence of the **fourth surface**: the historical/analytics tab, plus a
deeper view of an individual activity than phase 1 captured.

### 1.2 Provenance: two capture sessions, not one

Unlike phases 2 and 3, these are stills, not a recording — and they are not all from the
same sitting. The status bar dates them:

| shots | clock | connectivity | battery | matches |
| --- | --- | --- | --- | --- |
| 1–3 (Progress tab) | **07:51** | airplane mode + Wi-Fi | 97–98 % | ~2 min **before** video 2 opens (07:53, airplane, 97 %) |
| 4–5 (Tempo activity) | **23:06** | cellular | 64 % | ~3 min **after** video 1 ends (23:01–23:02, cellular, 63 % charging) |

The status bars were read from the recordings themselves
(`/tmp/runna_sb/strip.png`, cropped from `f0000`/`f0060`/`f0121` of video 2 and
`f0000`/`f0080` of video 1). Video 2's filename stamps it 10-27 07:53:45; video 1 was
recorded the previous evening, 2026-10-26.

Two consequences worth holding on to:

- **The Progress shots are the same morning as the plan-generation video.** The account
  state in shots 1–3 is the state the new plan was generated into — which is why §7 can
  cross-check the PR board against the race-time projection the wizard accepted.
- **The Tempo shots are from the previous evening, and the activity is dated Oct 13.**
  This is an *older* activity from an *older* plan era, opened for review. It is not the
  workout the plan-generation video was about.

### 1.3 The images are not in the repo

The five screenshots were pasted inline. They are **not recoverable from disk**: a
search of `~/Downloads`, `~/Desktop`, `/tmp` and `/var/folders` found nothing; the
session transcript stores no attachment bytes (0 base64/`data:image` hits); VS Code's
storage directories hold no matching images. Every number in this document is therefore
**transcribed, not re-measurable from a repo artefact**. `progress/README.md` lists the
exact filenames to drop in if they are ever saved.

This is a real limit on the phase, and it is stated here rather than hidden behind an
evidence index that points at files which do not exist.

### 1.4 What transcription cannot give

No motion, no scroll behaviour, no tap targets, no state transitions, no error states,
and no way to tell whether a control is disabled or merely unstyled. Where a claim
depends on any of those it is marked. Anything below that says *verified* means it was
checked arithmetically against other numbers in the same or a different capture, not
that it was re-derived from a video.

---

## 2. The Progress tab: what it is for

Two segments at the top — **Workouts** and **Performance** — and they answer different
questions:

- **Workouts** = *the log*. Everything I have ever done, newest first, filterable.
- **Performance** = *the scoreboard*. Aggregates, trends, records.

That split matters because it is the same split Runna makes everywhere else: the thing
that happened, and the thing it means. Phase 3 found it at workout level (a logged run
renders differently from a scheduled one); here it is the top-level organisation of the
entire history surface.

### 2.1 Workouts segment

- **Filter row:** `Workout Type ▼`, `Year ▼`, `Month ▼` — three dropdowns, no search box,
  no date-range picker, no sort control. Type is the primary axis, which tells you what
  Runna considers the meaningful category: session *purpose*, not sport.
- **A summary line above the list, scoped to whatever the filters select:**
  `October 2026 · 112.0 km · 13 activities · 10h 0m 15s`

  Verified: 112.0 km / 13 = **8.62 km per activity**, and 10 h 00 m 15 s / 112 km =
  **5:22/km**. The three visible cards (9.20, 12.01, 11.01 km) are all *above* that mean,
  which is what you expect when the list is newest-first and the month's easy days are
  further down.

- **Cards:** grey vertical bar on the left edge, route **line art** (the actual GPS path
  drawn as a decorative stroke), then `Monday run` / `Oct 26 12:18 PM` / `9.20km` /
  `42:46` / `4:38/km`.

Two observations:

**The route drawing is doing real work.** Every card carries the shape of where the run
went. That is a per-card image render over the whole log, and it turns a list of numbers
into a wall of places. Nobody scrolls a Runna log without seeing that.

**The grey bar is the absence of a colour.** Phase 3 established Runna's session-type
colour grammar (green easy, **orange/amber tempo**, etc.) and that *logged* runs render
differently from scheduled ones. These cards are grey-barred even though the Monday run
was a real run — so the colour slot on a log card is reserved for **plan provenance**,
not session type. A grey bar reads as "this run is not attached to a plan workout",
which is the same 17/175 distinction phase 1 drew from the ICS data model.

### 2.2 Cross-check that pins the data

`Monday run Oct 26 12:18 PM · 9.20km` and `Saturday run Oct 24 11:39 AM · 12.01km`
appear here, and both appear in the video-2 calendar rows with the same distances. The
Workouts list and the Calendar are reading the same store. That is the strongest
internal-consistency signal in this phase, and it is what makes §3's contradiction
notable rather than noise.

---

## 3. Performance segment

### 3.1 All-Time Stats

One card, seven rows, each with a teal line-art icon:

| row | value |
| --- | --- |
| TOTAL DISTANCE | 2,042 km |
| TOTAL ACTIVITIES | 196 |
| TOTAL TIME | 162 h 41 m 15 s |
| LONGEST RUN | 42.6 km |
| PLAN RUNS COMPLETED | 144 |
| PLAN STRENGTH | 0 |
| PLAN YOGA | 13 |
| PLAN PILATES | 0 |
| PLAN STRETCH & STABILITY | 17 |

The five `PLAN *` rows are the interesting part. Derived:

- Plan completions sum to **174** (144 runs + 13 yoga + 17 stretch + 0 + 0).
- Against 196 total activities that leaves **22 non-plan activities** — 11 % of the log.
  Plan runs alone are **73.5 %** of all activities.
- Whether the 30 zero-distance sessions (yoga + stretch) are inside the 196 changes the
  per-activity mean: **10.42 km** if they are, **12.30 km** across the 166
  distance-bearing ones if they are not. The card does not say which. Lifetime pace is
  unambiguous either way: 585,675 s / 2,042 km = **4:47/km**.

**The honest framing is the finding.** Runna separates *plan* completions from *total*
activity, and gives the cross-training rows their own counters even when those counters
are **zero**. `PLAN STRENGTH 0` and `PLAN PILATES 0` are displayed with the same weight
as `PLAN RUNS COMPLETED 144`. The board shows what you skipped as plainly as what you
did — while phase 1 documented a full-screen sheet selling exactly that strength work
(`screens/03-strength-upsell.png`). Zero is not hidden.

### 3.2 Weekly chart and the `VIEW BY` control

- Range label `Oct 26 - Nov 2` with `VIEW BY: WEEK ▼` — the granularity is a control, not
  a fixed choice. (Day/Month/Year options are not visible in the capture; unconfirmed.)
- Bar chart, x-axis `Sep 7 … Oct 26` at weekly ticks, y-axis `0 / 21 / 42` km. The
  leftmost tick is clipped in the render (`p 7`), i.e. the axis does not reserve space for
  its own first label.
- Stat block for the selected range: `KILOMETERS 0.00`, `TIME 0s`, `ACTIVITIES 0`.

### 3.3 The 0.00 contradiction — stated carefully

As displayed, the Performance pane says the week **Oct 26 – Nov 2** contains **0.00 km
and 0 activities**, while the Workouts pane from the *same session, ~the same minute*
lists `Monday run Oct 26 12:18 PM · 9.20km · 42:46 · 4:38/km`. Oct 26 2026 is a Monday,
so that run is inside the stated range.

I am **not** calling this a bug. Two explanations fit the evidence and the captures
cannot separate them:

1. **Aggregation lag.** The run happened 12:18 PM the previous day; the screenshots are
   07:51. If the Workouts list is a direct view of imported activities while the
   Performance aggregates are built by a separate pipeline, a ~20 h lag is possible.
2. **Selection binding.** The stat block may be bound to a *selected* bar rather than the
   range named in the header, and the selection may be a week with no data.

What is defensible from the screenshots alone: **the two panes visibly disagree, and
nothing on screen explains why.** A user who notices has no way to resolve it. That is
worth knowing regardless of which explanation is right, and it is the kind of thing only
a still from a real account reveals — a scripted demo would have shown a clean week.

### 3.4 Personal Records

Six records, each a **hexagon badge** with a tier colour:

| distance | time | date | badge |
| --- | --- | --- | --- |
| 1K | 3:00 | Jul 25 2025 | grey |
| 1MI | 5:18 | Aug 22 2025 | black |
| 2MI | 11:10 | Aug 7 2025 | red |
| 5K | — | — | blue |
| 5MI | — | — | teal |
| 10K | — | — | gold |

Three things here.

**The hexagon is the same shape as the `100 pts` chip on the activity hero (§4).**
Runna has one gamification primitive — the hexagon — reused for earned records and for
points. The visual language says *these are the same kind of thing: things you earned*.

**Every PR is 14–15 months old** (Jul–Aug 2025, viewed Oct 2026). The board is a frozen
trophy case, not a live progress signal. Combined with §3.1's zeros, the Performance tab
in this account is mostly a museum: nothing on it is moving.

**The half-visible rows matter.** 5K/5MI/10K are cut off at the fold, so the board is
scrollable and the *most impressive* distances are the ones you have to reach for.

### 3.5 The PR board validates the wizard's race-time maths

Phase 2 found the wizard accepting a **5K of 18:00** as the race-fitness input, and
phase 3 found the Estimated Race Times screen using a Riegel-style exponent
(**b = 1.0843**). The PR board lets those be checked against each other:

| extrapolated from | 5K prediction at b = 1.0843 |
| --- | --- |
| 1K 3:00 | 17:11 |
| 1MI 5:18 | 18:07 |
| 2MI 11:10 | **18:00** |

The 2MI PR extrapolates to **exactly** the 18:00 the wizard accepted. And the three PRs
are not one clean curve — the implied exponent between 1K and 1MI is **1.196**, between
1K and 2MI **1.124** — so Runna is not fitting the user's own PRs to a personal curve, it
is using a fixed one. The agreement at 18:00 is a coincidence of this athlete's shape,
not evidence of calibration.

That is a useful limit to carry into Runalytics: **a fixed-exponent projection is
defensible but it is not personalised**, and a UI that shows six PRs side by side is
one screen away from being able to notice that.

---

## 4. Activity review, hero section (Tempo 3-2-1, Oct 13 2026)

Top to bottom:

- **Gold/amber gradient header**, keyed to session type — phase 3 established the hero
  gradient is session-coloured and amber is the Tempo colour. Consistent.
- Back chevron in a dark square, and an **`ooo`** overflow button top-right.
- **GPS route map** — a real map tile with the track drawn in red, place-labelled
  *Laim, Bavaria, Germany*. Not line art: this is the map, where the log cards use the
  line-art abstraction of it.
- A **`100 pts` hexagon chip** — the gamification primitive again, on a completed run.
- `Tempo` as an **amber pill** · `9.01km` · `Oct 13 2026 9:23 AM` · **`View on Strava`**.
- Seven metrics: `DISTANCE 9.01km` · `TIME 38:01` · `AVG PACE 4:13/km` ·
  `ELEVATION ▲25m` · `AVG HR 148` · `CADENCE 154` · `CALORIES 653`.
- A black **Share** pill.
- **Description**, with a **copy icon**:
  > 1km warm up at a conversational pace (no faster than 4:55/km)

Verified: 9.01 km in 38:01 is **4:13/km** — the printed average is correct.

Four things worth pulling out.

**`View on Strava` is an admission.** Runna displays the activity but points outside
itself for the canonical record. The activity data is imported, and the app says so on
the surface where you would most expect ownership. Contrast the phase-1 share card,
which is fully Runna-branded and designed to be posted *outside* the app. Same data,
opposite posture: one screen concedes provenance, the other claims it.

**The Description carries the prescription.** The session the athlete *ran* is titled
`Tempo 3-2-1`; the text under it is the coach's *instruction*, verbatim, including the
pace ceiling. Plan and log share a surface — and phase 1 found Runna keeps them
typographically separate (`at` for intent, `@` for actual). Here the intent is presented
as prose under the actual, which is the same separation achieved by layout instead of
glyph.

**The copy icon is the most interesting control on the screen.** It exists so a runner
can paste their workout into a chat, a coach's email, or a notes app. It treats the
prescription as *the user's text*, not app content.

**`no faster than 4:55/km` is the same ceiling as the brand-new plan** (phase 2's
wizard, and phase 3's first week). Same athlete, different plan era, same easy-day
governor — which is a good sign for the algorithm's consistency and a bad sign for
anyone hoping the ceiling adapts to fitness.

---

## 5. Activity review, laps

A **bar chart** with a `TARGET` legend entry.

- Green bars, labelled with a pace: `4:40` (warm-up), `3:55`, `3:50`, `3:39`.
- **Dark target caps sit above the hard laps** — a horizontal marker at the required pace,
  so the bar's height against the cap *is* the pass/fail verdict.
- The warm-up bar has **no cap**: ungraded laps are unmarked, exactly as phase 1 found in
  the zigzag lap-row view (`RUNNA-UX.md` §5: "Rows that carry no target carry no check").
- Thin, unlabelled bars between the hard ones = recoveries, present but visually
  demoted.

This is the **third rendering of the same lap data** across the phases, and it confirms
phase 1's "decision worth stealing": shape (segmented bar) → per-lap success (bar chart
with caps/ticks) → numbers (splits table). Three reading depths, no extra data.

The new detail is *how* the pass/fail is encoded here. Phase 1's share card used **check
marks above met bars**. This screen uses **a target line the bar must reach**. The check
mark is a verdict handed to you; the cap is a verdict you read yourself. Same
information, one step more or less explicit — and the more explicit one is on the
in-app review screen, while the check-mark version is on the share card built for other
people to look at.

---

## 6. Activity review, splits — the `+/-` column is not what it looks like

The table is `KM | AVG PACE | +/-`:

| km | avg pace | delta |
| --- | --- | --- |
| 1 | 4:39 | — |
| 2 | 3:52 | +0:47 |
| 3 | 3:56 | −0:04 |
| 4 | 3:58 | −0:02 |
| 5 | 4:28 | −0:30 |
| 6 | 3:48 | +0:40 |
| 7 | 4:30 | −0:42 |

The natural reading — "delta against the target pace" — is **wrong**, and the
arithmetic proves it. If `+/-` were vs target, each row would imply a target:

- km2 3:52 with +0:47 ⇒ target 3:05/km
- km3 3:56 with −0:04 ⇒ target 4:00/km
- km6 3:48 with +0:40 ⇒ target 3:08/km

Those are not one session's targets; they are nonsense for a tempo run.

Against the **previous kilometre**, every row matches exactly:

| km | computation | delta |
| --- | --- | --- |
| 2 | 3:52 − 4:39 | **+0:47** ✓ |
| 3 | 3:56 − 3:52 | **−0:04** ✓ |
| 4 | 3:58 − 3:56 | **−0:02** ✓ |
| 5 | 4:28 − 3:58 | **−0:30** ✓ |
| 6 | 3:48 − 4:28 | **+0:40** ✓ |
| 7 | 4:30 − 3:48 | **−0:42** ✓ |

**All six rows match; the first km has no delta because there is nothing before it.**
Green = faster than the previous km, red = slower.

So the column is **segment-to-segment change**, not fitness-to-plan. Two consequences:

1. **The sign reads backwards from intuition.** On a tempo run, km2 at 3:52 is the *best*
   split so far and it is marked **red +0:47**. Colour here means "the pace dropped", not
   "you did badly". A user scanning for red/green judgement gets the opposite of the
   truth on the fastest surge of the session.
2. **It is the right choice for the laps chart to exist alongside.** The chart answers
   "did I hit the target" (bar vs cap). The table answers "how did my pace move". If the
   table *also* showed vs-target it would duplicate the chart at a worse resolution. The
   column is well-designed and badly-signposted: the header `+/-` carries no hint that
   the reference is the previous kilometre.

**Also verifiable from the same table:** km1 was run at **4:39/km** against a prescribed
ceiling of *"no faster than 4:55/km"* — **16 s/km faster than the easy-day limit**, on a
run whose warm-up bar carries no target cap. The athlete broke the ceiling on the
warm-up, and because the lap is ungraded, nothing on the screen notices. Phase 1's
principle — *never grade the ungraded* — is implemented consistently enough that a
genuine easy-day violation is invisible. That is the cost of a good rule.

---

## 7. Cross-checks against the earlier phases

| this phase | earlier phase | result |
| --- | --- | --- |
| Mon Oct 26 9.20 km, Sat Oct 24 12.01 km in the Workouts list | same rows in the video-2 calendar | **agree** — one store |
| `no faster than 4:55/km` in the Oct 13 description | phase 2 wizard, phase 3 week 1 | **same ceiling**, different plan era |
| amber hero on a Tempo session | phase 3 colour grammar (orange/amber = Tempo) | **agree** |
| ungraded warm-up has no target cap | phase 1 "rows that carry no target carry no check" | **same rule, new medium** |
| `100 pts` hexagon | PR hexagons §3.4 | **one gamification primitive** |
| PR board → 5K 18:00 at b = 1.0843 | phase 2 wizard input, phase 3 Riegel finding | **agrees exactly via the 2MI PR** — but the PRs imply b = 1.12–1.20, so the curve is fixed, not personal |
| Tempo 3-2-1 dated Oct 13 | new plan generated Oct 27 | **predates the plan** — this is review of an old-plan activity |

---

## 8. What this changes for Runalytics

Checked against the code, not assumed.

**1. We have no per-km splits anywhere.** `ActivityDetail` (`src/lib/types.ts:42`) carries
`track`, `health`, `load`, `fitness`, `riskFactors`, `injuryRisk` — and no laps or splits.
Grepping `src/**` for lap/split usage returns only *prescription* lap text in
`calendar_publish.rs` and unrelated `split_once` calls. We hold the raw `TrackPoint[]`
(`ts, lat, lon, hr`) and therefore already have everything needed to compute per-km
splits; we simply never present them. Runna's review screen is *built* around them, and
§6 shows the design trap: pick a reference (previous km, or the target) and **label it in
the header**, because the two readings are opposite on a hard surge.

**2. We already do Runna's lap verdict — better-labelled, less visible.**
`SessionReview` (`src-tauri/src/types.rs:363`) grades the pace held over the main set
against the plan's band and returns `Ahead / InRange / Behind` ("too fast / in range /
too slow"), rendered by `renderPacing()` in `src/plan.ts:613` with an accessible label
that spells out the range and the held pace. That is *the same judgement* as Runna's
bar-against-cap, and ours is more explicit. The gap is placement: ours lives on the Plan
page as a pacing-adaptation signal; Runna puts it on the activity you just opened. Worth
surfacing `SessionReview` in the activity detail sheet, where the athlete is looking for it.

**3. `SessionReview`'s doc comment is the argument against Runna's splits table.**
It says the overall average pace is *not* a substitute for the main-set pace because
"it is dragged down by the warm-up and cool-down and would read as 'too slow' for a
session that was executed perfectly." Runna's hero shows exactly that misleading number
(`AVG PACE 4:13/km` over a session whose hard laps were 3:39–3:55). Our reasoning is
ahead of their UI here — keep it, and make it visible where Runna's users are being
misled.

**4. Lifetime totals: we have them, and we are more honest about the units.**
`gamificationStats()` (`src/lib/flow.ts:235`) computes `totalKm`, `sessions`,
`totalHours`, `crossHours`, and `achievements.ts:82` carries the comment *"'all-time
distance' would be a lie next to a cross-training card"* — distance counts running only,
sessions and hours count every sport. Runna's All-Time Stats card prints `TOTAL DISTANCE`
and `TOTAL ACTIVITIES` with no such statement, and §3.1 shows you cannot tell whether the
30 yoga/stretch sessions are inside the 196. **Our ambiguity-avoidance is the better
design; do not trade it for a tidier card.** What Runna has and we do not: the
`PLAN *` breakdown that separates *plan* completions from *total* activity, including
zeros for skipped modalities. That is a genuinely useful idea — it makes adherence
legible without a separate adherence page.

**5. No plan↔activity link exists in our store.** Grepping `store.rs` and
`*.rs` for `adherence`, `plan_workout_id`, `completed_workout`, `was_prescribed` returns
nothing. `PlanWorkout` (`types.rs:402`) has a `done: bool` and nothing connecting it to an
`Activity` row. So we cannot currently answer Runna's most basic Progress-tab question —
*was this run the workout that was planned?* — even though `done` implies something sets
it. If the `PLAN RUNS COMPLETED` idea is worth having (§4), a join between a completed
plan workout and the activity that satisfied it is the prerequisite, and it is also the
missing piece behind the `planned_date` gap phase 3 recorded in
`/memories/repo/runalytics-drag-drop.md`.

> **Answered after this doc was written.** Runna's help centre names the feature
> **Link Activity** (Plan → open the workout → Link Activity → pick an activity from the
> Progress tab), confirms it is *user-initiated* rather than auto-matched, allows several
> activities to link to one workout, and states that manual activities deliberately do not
> re-adapt the plan. Full analysis in [`RUNNA-PLAN-MANAGEMENT.md`](./RUNNA-PLAN-MANAGEMENT.md) §8.

**6. Stale records are a design decision, not a data fact.** Runna's PR board is 14
months frozen (§3.4). Our badges in `flow.ts` are *progress* badges (`earned`,
`progress`, tier by % of goal) — a moving target rather than a trophy case. That is the
better choice for a training tool, and worth defending if anyone asks for a PR board:
the honest version of a PR board is one that also shows how long it has been since
anything moved.

**7. The copy icon.** Small, and the only control on Runna's review screen that treats
the prescription as the user's property. Cheap to add to our workout detail; it also
makes the plan composable with the outside world, which is phase 1's "the export is the
moat" point applied to a single workout.

---

## 9. Evidence index

**The five screenshots are not in this repository** — see §1.3. The table below is what
each one contains, so the analysis stays auditable against the transcription even
without the files.

| intended file | content | session |
| --- | --- | --- |
| `progress/01-performance-all-time.png` | Progress → Performance, All-Time Stats card | Oct 27 07:51, airplane, 97 % |
| `progress/02-performance-week-prs.png` | Performance scrolled: range header, weekly chart, Personal Records | Oct 27 07:51, airplane, 98 % |
| `progress/03-workouts-october.png` | Progress → Workouts: filters, October summary, activity cards | Oct 27 07:51, airplane, 97 % |
| `progress/04-activity-laps-splits.png` | Tempo 3-2-1: laps bar chart with target caps, splits table | Oct 26 23:06, cellular, 64 % |
| `progress/05-activity-hero.png` | Tempo 3-2-1: map, `100 pts`, metrics, Strava link, description | Oct 26 23:06, cellular, 64 % |

Supporting artefacts that *are* reproducible:

| artefact | what it establishes |
| --- | --- |
| `/tmp/runna_sb/strip.png` | status bars cropped from video 1 and video 2 frames — the dating in §1.2 |
| `/tmp/runna_prog_calc.py` | every arithmetic claim in §3.1, §3.5, §4, §6, run and verified |
| `EVIDENCE-deeplink-probe.log` | `/progress` falls back to Today — the gap this phase closes |

**Still not captured:** the `VIEW BY` dropdown's options, the `Workout Type` filter's
values, any Progress-tab empty or error state, scroll and tap behaviour, whether the
`ooo` overflow menu on an activity contains anything beyond share/delete, and the
5K/5MI/10K PR times below the fold.
