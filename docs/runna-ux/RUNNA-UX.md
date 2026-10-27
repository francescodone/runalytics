# Runna (iOS) — UI/UX investigation

**App:** Runna v8.59.0 (`com.runbuddy.prod`) on iPhone 16 / iOS 27.0.1
**Investigated:** 2026-10-26
**Companion evidence:** `screens/`, `EVIDENCE-*.txt` in this folder
**Companion docs:** [`RUNNA-PLAN-GENERATION.md`](./RUNNA-PLAN-GENERATION.md) — how a plan is built ·
[`RUNNA-PLAN-EXECUTION.md`](./RUNNA-PLAN-EXECUTION.md) — browsing and rearranging one ·
[`RUNNA-PROGRESS-AND-REVIEW.md`](./RUNNA-PROGRESS-AND-REVIEW.md) — the Progress tab and activity review ·
[`RUNNA-PLAN-MANAGEMENT.md`](./RUNNA-PLAN-MANAGEMENT.md) — the `Manage Plan` control surface,
from Runna's own help centre rather than from screen ·
[`RUNNA-INSIGHTS-AND-ADAPTATION.md`](./RUNNA-INSIGHTS-AND-ADAPTATION.md) — how the plan
adapts back: Pace/Mileage Insights, heat, the AI commentary layer and Runna Score

---

## 1. Method, and what it could not reach

This matters more than the findings, so it goes first.

The device was connected over the air via `devicectl`. That transport gives **capture
and launch, but no input**:

| Capability | Status |
| --- | --- |
| `capture screenshot` | works, ~0.55 s per frame (1179×2556) |
| `capture screen-record` | **unsupported** — `CoreDeviceError 1001`, feature `com.apple.coredevice.feature.screenrecording` is not offered for this device over OTA |
| Tap / swipe / HID | **no channel exists.** `devicectl` has no input verb; Xcode's `DeviceInteraction*` MCP tools are simulator-only; iPhone Mirroring exits immediately when launched headless; `pymobiledevice3` uninstallable (no outbound network from the shell — even `pypi.org` and `google.com` time out) |
| App container / bundle inspection | blocked (`CoreDevice.ActionError 3`; `appBundle` is not a valid `--domain-type`) |

So every screen below was reached by one of two routes: a **universal link**
(`devicectl device process launch --payload-url …`), or a **modal that happened to be
on screen**. The plan-creation wizard, the Plan tab, the Calendar tab and the Progress
tab were **never seen** and nothing in this document describes them.

A route prober (`EVIDENCE-deeplink-probe.log`) cold-launched the app against 20
candidate paths and diffed each screen against a Today-tab baseline using a
status-bar-cropped greyscale proxy — necessary because the clock changes on every
capture and a naive hash reports "different screen" for all of them.

**Second limitation, and it is a finding in its own right:** Runna's post-run share
card and its strength upsell sheet are **sticky across cold launches** — the upsell
survived `process terminate` followed by a fresh `process launch`. Once one was up, it
stayed up, and it poisoned the probe: six "navigated" results were the same frame, not
six screens. Any automation of this app has to dismiss or kill through modals, not
around them. §6 has the byte-hash evidence for exactly which probes were contaminated.

---

## 2. Navigation model

Five tabs: **Today · Plan · Calendar · Progress · Support**
(`screens/01-today.png`, `screens/04-support.png`)

- **Today is the home screen and the only screen with the wordmark.** Every other tab
  replaces the centred "Runna" logotype with the tab's own name (`Support` in
  `04-support.png`). Branding is reserved for the entry point; elsewhere the header
  spends its centre on orientation.
- Header chrome is constant across tabs: **profile avatar left, title centre,
  notification bell right.** The avatar is a bare glyph in a soft circle — no photo,
  no name — so the account entry point stays quiet and the bell stays loud.
- Tab icons are outline, filled + black when selected, and **every tab is labelled**.
  Icon-only navigation is not used anywhere.
- There is no back stack visible from Today; navigation is tab-first, detail-second,
  and detail screens close with an **✕** (`02-activity-detail.png`) or a **back chevron
  in a dark rounded square** (`05-error-state.png`) rather than the iOS edge-swipe
  title pattern. Runna draws its own chrome instead of borrowing `UINavigationBar`.

### Today, top to bottom (`screens/01-today.png`)

1. **7-day strip**, MON–SUN, today as a filled black disc. A small **square dot** sits
   under today — a "something happened / something is scheduled" marker that does not
   compete with the date itself.
2. A **notched drag handle** cut into the bottom edge of the strip's panel. This is a
   sheet affordance: the calendar strip is a pull-down drawer, not a fixed header.
3. A large **animated polygon badge** — concentric translucent teal rings around a
   runner glyph. Animation is *measured*, not assumed: two Today captures 28 minutes
   apart (`screens/07-badge-motion-diff.png`) differ **only** in this region — the
   header wordmark and the workout card are pixel-identical (mean abs diff `0.00`),
   while the badge region measures `1.78` with 3.2 % of pixels above threshold. Within
   the badge, only the **polygon edges and the runner glyph** moved; the ring *fills*
   are unchanged. So it is a slow continuous **rotation of the outline**, not a
   repainting chart. It is the largest element on the screen and it carries no number
   and no label.
4. **Time-of-day greeting**: "Good evening, Francesco". Personalised *and* contextual.
5. Subtitle: **"Start a workout or create a plan"** — two CTAs in the copy, one of which
   is the thing being investigated. This is the empty-state prompt sitting on top of a
   populated screen.
6. **Today's workout card**: route-shaped line icon, "Monday run", "Today · 12:18 PM",
   then Distance / Time / Avg pace. A **grey leading accent bar** marks it as complete.
7. **"Sources: COROS APEX 4 42mm, Strava"** — provenance stated in plain text, on the
   home screen, unprompted.
8. **"Add run"** with a right-aligned **`COLLAPSE ⌄`** toggle and colour-coded tiles:
   Easy Run (green→lime), Tempo (amber), Intervals (red→orange), plus a second row
   (dark green, purple, dark red). Each tile carries a small crisp runner glyph top-left
   *and* a large ghosted silhouette bottom-right — the same pose twice, one as icon and
   one as texture.
9. **"▶ Record workout"** — a black pill FAB. It **overlays** the tiles rather than
   pushing them, and in `01-today.png` it sits directly across the three tile labels,
   cutting "Easy Run", "Tempo" and "Intervals" mid-height.

**Decision worth stealing:** the intensity ramp is encoded as *colour before text*. You
can find "Intervals" by its red without reading. And the ramp is consistent with the
prescription data (§4), where easy work is capped and quality work is targeted.

**Decision worth questioning — the polygon badge:** it is the most visually dominant
thing on the home screen and it is *uninterpretable from a screenshot* — no value, no
unit, no legend. It earns its size only if the user learns its grammar over time. A
first-week user gets a decorative shape.

**Decision worth questioning — the FAB:** recording is the highest-priority action and
Runna makes it unmissable by permanently covering the labels of the *alternative* way
to start a workout. Colour is the only thing that survives the overlap, which suggests
the tiles are legible by hue precisely because their text is not always visible. That
may be deliberate layering rather than a layout bug, but it is the one place on the
home screen where two primary actions fight for the same pixels.

---

## 3. The plan data model (from `docs/Runna-Backup.ics`)

The ICS export is the strongest evidence available, because it is Runna's own
serialisation (`PRODID:-//Runna//EN`, `X-WR-CALNAME:Runna`) rather than a screenshot.
192 events. Full extraction in `EVIDENCE-ics-data-model.txt`.

### The discriminator is the UID

```
COMPLETED_PLAN_WORKOUT-<source-id>        17   (coros 6, strava 5, runna-native uuid 6)
COMPLETED_NON_PLAN_WORKOUT-strava-<id>   175
```

Runna keeps **plan workouts and imported workouts in one timeline** and separates them
by UID prefix, not by calendar, not by a flag field. One stream, typed at the ID.

### The property that actually makes a workout "a plan workout"

| | plan (17) | imported (175) |
| --- | --- | --- |
| 📊 Summary | 11 | 175 |
| **📋 Description (prescription)** | **17** | **0** |
| ♻️ Laps | 10 | 129 |
| 🏆 PBs | 3 | 13 |

**Every plan workout carries a prescription; no imported run carries one.** That is the
definition. A plan workout is *a workout that was prescribed before it was run*. The
plan is not a schedule of slots — it is a queue of intents.

### Section template

```
📊 Summary:      Distance / Time / Avg Pace
📋 Description:  <the prescription — coach-authored>
♻️ Laps:         <recorded execution>
🏆 PBs:          <only if any>
📲 View in the Runna app: https://club.runna.com/n9Tx/activities?activityId=<id>
```

Emoji do double duty: **sport type in `SUMMARY`** (🏃 run, 🧘 mobility) and **section
headers in `DESCRIPTION`**. Note the typing is not airtight — 3 mobility sessions carry
🧘 and 3 "Stretch & Stability" sessions carry no emoji at all.

191 of 192 events end with a deep link back into the app. The calendar export is a
**funnel, not an archive** — every row is an invitation to return.

### Time handling

`X-USER-TIMEZONE` on all 192 events, and five distinct values: Europe/Berlin,
Europe/Lisbon, Europe/Paris, America/Chicago, Europe/Rome. Plus `LOCATION` and `GEO` on
most events. Runna pins the timezone **per workout**, not per account — the plan
survives travel. (Relevant to `runalytics`: our own plan dates are serialised date-only,
which is exactly the trap this avoids.)

---

## 4. The prescription grammar

This is the most interesting thing in the dataset. Full verbatim dump:
`EVIDENCE-ics-prescriptions.txt`.

### Two different prepositions, deliberately

- **Prescription** uses `at`: `400m at 3:40/km`
- **Recorded lap** uses `@`: `0.40 km @ 3:35 /km`

Intent is `at`, reality is `@`. 179 recorded lap lines all match `# km @ #:# /km`;
prescription lines match `#m at #:#/km`. Same quantity, two notations, so a reader can
never confuse the plan for the log.

### Structural primitives, by frequency (n=17)

| primitive | count |
| --- | --- |
| effort cue "conversational pace" | 11 |
| km distance | 11 |
| warm up | 7 |
| `at <pace>` | 7 |
| pace cap "no faster than" | 7 |
| cool down | 6 |
| "or slower!" | 6 |
| `----------` separator | 3 |
| "Repeat the following Nx:" | 3 |
| m-scale distance | 4 |
| "recovery" / "stride" / "tempo" / "threshold" as words | **0** |

### The nesting is real

`400m Repeats` (2025-09-03) prescribes a **repeat block containing a reps block**:

```
4km warm up at a conversational pace (no faster than 4:40/km), 90s walking rest

Repeat the following 2x:
----------
6 reps of:
• 400m at 3:05/km (2:55-3:15/km), 60s walking rest

90s walking rest
----------

3km cool down at a conversational pace (or slower!)
```

So the model is at least: `block { repeat N } > { reps N } > step { distance, pace,
optional pace window, rest }`, with warm-up/cool-down as unmeasured bookends outside the
measured set.

### Pace is expressed as a *limit*, and the copy says so

- `2.4km warm up at a conversational pace (no faster than 4:50/km)` — a **ceiling**, 7/17
- `5.5km easy run at a conversational pace (no faster than 4:45/km). This is a limit, not
  a target - run at whatever pace feels truly easy!` — the app **pre-empts the exact
  mistake** a driven runner makes with an easy day
- `1.8km cool down at a conversational pace (or slower!)` — 6/17, always on cool-down
- quality steps get a **window**, not a point: `1km at 3:35/km (3:25-3:45/km)`

**This is the core UX insight of the product.** Easy work is bounded from above; hard
work is bounded on both sides. The number is never just a number — it always arrives
with its failure mode attached.

*Confirmed in the UI on 2026-10-27:* the workout detail screen renders exactly this —
`2km at a conversational pace` / **`No faster than 4:55/km`** on the warm-up, `2km at
4:20/km` → `4:00/km` → `3:50/km` on the session, and **`or slower!`** on the cool-down.
See [`RUNNA-PLAN-EXECUTION.md`](./RUNNA-PLAN-EXECUTION.md) §6. What was reconstructed
from ICS metadata is what the app prints.

### Rest is inline prose, not a field

`, 90s walking rest` is appended to the step it follows. Human-parseable, machine-lossy.
Runna's calendar surface is *written for a runner reading it on a phone*, and the
structured form is kept server-side.

### Coach voice

- Named human: **"Your coach Torzi will be with you every step of the way!"** — appears
  in every Mobility Session.
- Mobility sessions have **no structure at all** — pure prose, sensory copy: *"You'll
  leave this one standing taller and breathing deeper."* / *"Perfect as a reset after a
  long day, or a light flush post-run."*
- Practical constraints stated as copy: **"No wall or equipment needed."**
- Coaching tips embedded mid-structure: `add 3x 15s fast bursts towards the end of your
  warm up jog to inject some pace into your legs before the main session!`

**Decision:** Runna does not render strength/mobility work in the same schema as runs.
Runs get a table; recovery work gets a paragraph. Forcing one template on both would
make the yoga session look like a workout, which is the wrong feeling.

---

## 5. Execution feedback: the loop that closes

### Activity detail (`screens/02-activity-detail.png`)

- A **segmented bar across the top** that tracks scroll position — the first two dashes
  are lit, the rest dimmed — so the session reads as a shape and a position at once.
- Lap rows **zig-zag left / right**, which is what makes the alternating hard-easy
  structure of a "rolling" session legible at a glance. Each row shows **one number** —
  the actual pace — plus a **check mark when that lap met its target**, and the distance
  in a fixed right column. The target itself is *not* repeated per row; it lives in the
  prescription, and the check is the comparison.
- Rows that carry no target carry **no check** (`4:45`, and the dimmed `4:35` at the
  bottom). Ungraded laps are visibly ungraded rather than shown as passes.
- Headline verdict: **"You completed 14 laps, achieving 11/12 pace targets"**
- Followed by: **"A solid session, keep building"**

Note the arithmetic: **14 laps, 12 pace targets.** Warm-up and cool-down are excluded
from the score. You are never graded on the parts that were never graded — which is the
same principle as the "limit, not a target" copy, expressed in scoring instead of prose.

### Share card (`screens/06-share-card.png`)

- Full-bleed **tinted** background (gold here) — a different visual register from the
  app's light UI, i.e. this surface is designed to look good *outside* the app.
- **`Generali Köln Marathon Plan | Week 1`** as a pill chip — plan name *and* week number,
  on the artefact the user posts. The share carries its own progress context.
- Runna logotype top-right: every share is branded.
- GPS route as line art, then a **lap bar chart with check marks above the met bars** —
  and warm-up/cool-down rendered as **flat rounded blocks, not bars**, so unmeasured
  laps are visually distinct without a legend.
- 3×2 metric grid: Distance / Pace / Time / Heart rate / Elevation gain / Cadence.
- **Share** (filled) over **Maybe later** (text). Skippable, and the skip is cheap.

**Decision worth stealing:** the same lap data is shown three ways at three zoom levels —
segmented bar (shape), bar chart with ticks (per-lap success), numeric grid (totals).
Same numbers, different reading depths, no extra data.

**Decision worth questioning:** this modal is **blocking and persistent** — it survived
an app relaunch and covered the entire UI until the process was killed. A share prompt
that outranks the home screen is a growth-loop decision that costs the user time on
every open.

### Error state (`screens/05-error-state.png`)

Question-mark glyph, *"Something went wrong, please try again or contact us if this
problem persists"*, then **Refresh** (filled) and **Contact us** (outline).

**Decision worth stealing:** the error screen's secondary action is a **human**, not a
retry. And the copy names the escalation condition ("if this problem persists") instead
of just apologising.

### Upsell sheet (`screens/03-strength-upsell.png`)

Full-bleed photo, ✕, H1 "Add strength workouts", one sentence of benefit copy —
*"…build muscle, reduce injury risk, and support your running performance"* — then
**Continue** (filled) and **Learn more** (outline). Note the framing: the add-on is sold
as *supporting the thing you already do*, never as a separate product.

---

## 6. Integration surface

From the AASA at `club.runna.com/.well-known/apple-app-site-association`, saved verbatim
as `EVIDENCE-aasa.json`: paths under `/n9Tx/*` route to `L27UJU3HDT.com.runbuddy.prod`
and `ZR4D267DH2.com.runbuddy.prod`, plus App Clip variants of both. **One path pattern
total** — the association itself is the strongest possible statement of "we deep-link
entities, not screens."

Probe results (`EVIDENCE-deeplink-probe.log`, 20 candidates):

| Route | Result |
| --- | --- |
| `/n9Tx/activities?activityId=<id>` | **works** — opens activity detail; 191/192 exported events carry one |
| `/activities` (no id) | reaches a **real error screen** (`05-error-state.png`) — the route is recognised, it just has nothing to show |
| `/strength` | navigates (upsell sheet) |
| `/support` | navigates (Support tab) |
| `/plan`, `/plans`, `/plan/current`, `/training-plan` | fall back to Today |
| `/calendar`, `/progress`, `/today`, `/workouts`, `/workout-library` | fall back to Today |
| `/profile`, `/settings`, `/goals`, `/races`, `/events`, `/subscription`, `/routes`, `/history` | **not real routes** — see below |

**Conclusion: Runna deep-links *data*, not *navigation*.** One entity type (`activity`)
is addressable — and even that only with an id: `/activities` without one lands on the
error screen rather than a list. There is no link to a plan, a week, a day, or a tab.

That is a deliberate moat and it has a concrete consequence for anyone integrating with
Runna: **you can link a user to a completed workout, but you cannot drive the app to a
plan screen.** Plan management is app-only and un-orchestratable from outside. If we
want Runalytics to interop with a user's Runna plan, the only reliable channel is the
**ICS export** — which is also why the export is so well-formed: it *is* the public API.

### How to audit the log yourself

The probe's own summary line claims **11 navigated routes. Only 3 are real.** The diff
score alone is not trustworthy here — a modal that never goes away reports a big diff
forever. The reliable signal is **byte-identical screenshots across routes that cannot
plausibly be the same screen**:

| diff | routes | verdict |
| --- | --- | --- |
| `0.001–0.004` ×9 | plan, plans, plan/current, training-plan, calendar, progress, today, workouts, workout-library | fell through to Today |
| `0.799` ×6 | profile, settings, goals, races, events, subscription | **one identical upsell-sheet frame** — sticky modal, not navigation |
| `0.518` ×3 | support, routes, history | `/support` genuine; `routes` + `history` are **byte-identical to it** — tab-restore, the app reopened on Support |
| `0.478` ×2 | strength, activities | two *different* screens that happen to score alike: `/strength` is the upsell, `/activities` is the error state |

Two failure modes to design around if you automate this app:

- **Sticky modal** — the upsell sheet survived `process terminate` + relaunch and
  covered the UI for six consecutive probes.
- **Tab restore** — the app reopens on the last-used tab, so after reaching Support,
  every later probe landed there too.

The nine low-diff rows are the important *negative* result: they are not probe failures,
they are the app **refusing** the link and landing on home.

---

## 7. What this changes for Runalytics

Our `src-tauri/src/plan/engine.rs` is already calibrated against Runna's *numbers* —
the module doc cites two completed Runna marathon builds aimed at 2:50 that peaked at
76-80 **km/week** with biggest long runs of 32.4 km and 35.0 km, and `long_run_share()`
holds the long run at 0.44 of weekly volume at K42. (Careful: that "~80" is kilometres,
and the separate 80/20 in the same file is an **intensity** budget. Two different 80s.)
The gaps this investigation exposes are all about **representation, not arithmetic**:

1. **Prescription is the identity of a plan workout.** Our model should make
   "was this prescribed?" a first-class, non-nullable property, and treat an imported
   run as structurally incapable of having one — matching Runna's 17/175 split.
2. **The bound model is already right — the copy around it is not.** `calendar.rs`
   already renders exactly the two shapes Runna uses: `no_faster_than()` for
   `open_ended` steps ("no faster than 5:30/km, at conversational pace 🗣️") and a
   `@ lo-hi/km` window for quality steps. So the ceiling-vs-window idea is
   **independently confirmed** by a competitor, not a change to make. What Runna adds
   on top is the sentence that disarms the athlete: *"This is a limit, not a target -
   run at whatever pace feels truly easy!"* and *"(or slower!)"* on cool-down. Our
   `no_faster_than` states the cap and the effort cue but never tells the reader the cap
   is permission rather than an objective. That is a copy change, not a schema change.
3. **Quality steps carry a window but no target point.** Runna writes
   `400m at 3:05/km (2:55-3:15/km)` — a named target *and* its tolerance band. We write
   `@ 4:45-4:55/km`, the band alone. On a watch face mid-interval, "3:05, band 2:55-3:15"
   is easier to aim at than a range with no centre. Worth adding `pace_target` alongside
   the existing `pace_lo_s_km`/`pace_hi_s_km`.
4. **Never grade the ungraded.** Runna's "14 laps, 11/12 pace targets" excludes
   warm-up/cool-down from the denominator. Any completion score we show should do the
   same, or it will punish easy days.
5. **Two notations, one meaning — we do not do this yet.** Runna writes `at` for intent
   and `@` for actual, so plan and log can never be confused when they share a surface.
   Our plan notes use `@` for the planned window, which is the same glyph Runna reserves
   for recorded laps. Cheap to separate, and it matters most in the ICS export where
   planned and completed text sit side by side.
6. **Per-workout timezone.** Runna pins `X-USER-TIMEZONE` on every event across five
   zones. Our plan dates are date-only strings — which we already know mislabels by a
   day for west-of-UTC users. Pin the zone on the workout, not the profile.
7. **Different schema per session type.** Runs get structure; recovery/mobility gets
   prose. If Runalytics ever emits strength work, don't force it into the lap table.
8. **The export is the moat.** Runna's calendar export is well-formed *because* it's the
   only integration surface. If we want our own plans to be composable with external
   tools, the export needs to be as disciplined as the UI.

---

## 8. Evidence index

| File | What it is |
| --- | --- |
| `screens/01-today.png` | Today tab — full home screen |
| `screens/02-activity-detail.png` | Activity detail — lap ladder, pace-target checks |
| `screens/03-strength-upsell.png` | Strength cross-sell sheet (`/strength`) |
| `screens/04-support.png` | Support tab — the only other tab reachable by link |
| `screens/05-error-state.png` | Error screen with Refresh / Contact us |
| `screens/06-share-card.png` | Post-run share card — plan + week chip, lap chart |
| `screens/07-badge-motion-diff.png` | Thresholded pixel diff of two Today captures 28 min apart — proves the badge animates and nothing else does |
| `EVIDENCE-ics-data-model.txt` | Population, section template, lap grammar, coach-voice frequency |
| `EVIDENCE-ics-prescriptions.txt` | Every prescription verbatim, prescription-vs-execution pairing, PB and timezone tables |
| `EVIDENCE-deeplink-probe.log` | 20-route deep-link probe with per-route pixel diffs |
| `EVIDENCE-aasa.json` | Apple App Site Association, retrieved and re-verified 2026-10-26 |

**Not captured (no input channel):** Plan tab, Calendar tab, Progress tab, plan-creation
wizard, plan editing, workout pre-run screen, notification list, profile/settings.
Re-running this investigation with the phone unlocked and someone tapping — or with a
simulator build, where Xcode's `DeviceInteraction*` tools do work — would close those.

*Update:* the Plan tab, Calendar tab, plan-creation wizard, and plan editing are now
documented in [`RUNNA-PLAN-GENERATION.md`](./RUNNA-PLAN-GENERATION.md) and
[`RUNNA-PLAN-EXECUTION.md`](./RUNNA-PLAN-EXECUTION.md); the Progress tab and a deeper
activity review in
[`RUNNA-PROGRESS-AND-REVIEW.md`](./RUNNA-PROGRESS-AND-REVIEW.md). Still unreached **on
screen**: workout pre-run screen, notification list, profile/settings.

The `Manage Plan` control surface behind them — plan editing, B-races, holidays,
"Not Feeling 100%", ability and preference knobs, the plan lifecycle — is documented from
the other direction in [`RUNNA-PLAN-MANAGEMENT.md`](./RUNNA-PLAN-MANAGEMENT.md), built
from Runna's own help centre. Its rules are therefore *documented*, not *observed*: the
gap this note records is closed as to behaviour, not as to layout.
