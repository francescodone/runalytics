# Runna UI/UX parity — Runalytics

Layout, navigation, copy, accessibility. Logic/features live in
[`RUNNA-FEATURE-PARITY.md`](./RUNNA-FEATURE-PARITY.md); ID scheme, scoring legend
and deliberate divergences in [`README.md`](./README.md).

Every measurement below was taken live on 2026-10-27 at HEAD `0e77402` (v0.14.0):
all six pages plus the four harnesses, at **1440×1000** (light and dark) and
**390×844**, using a script that walks the DOM for sub-44px targets, uppercase
labels, non-token radii and WCAG contrast (colour walked up the tree, alpha
composited, gradient stops honoured). Mobile claims cite a selector or line.

**Baseline, stated plainly so nothing below reads as a general failure:**
in the light scheme at desktop there are **zero WCAG contrast failures on any
page**, **zero horizontal overflow at any width**, and the heading scale is
consistent (H1 34/700 ls −1.87 · hero H2 38/600 · group H2 17/700 · panel H2
15/650 ls −0.3). The real findings are tap targets, one uppercase rule, two
dark-mode contrast bugs, and the mobile navigation model.

---

## 1. Navigation and structure

### U-01 · Tab model: six destinations vs Runna's five

- **Runna**: five labelled tabs — Today / Plan / Calendar / Progress / Support
  (`ux §7`, `prog §8`).
- **Runalytics today** — **PARTIAL**: six labelled links — Dashboard / Plan /
  Calendar / Injuries / Achievements / Settings (`index.html` nav; measured as six
  links at 1440). The mapping is close: Dashboard≈Today, Plan≈Plan,
  Calendar≈Calendar, Achievements≈(part of)Progress. There is no Support surface,
  and Settings is a destination rather than a leaf.
- **Change**: keep six — do not chase the count. What is worth taking is the
  *ordering discipline*: Runna leads with Today and ends with Support. Ours ends
  with Settings, which is correct for a desktop app. The genuine gap is that
  "Progress" as a *review* surface is split across Dashboard + Achievements, so a
  Runna user looking for their history lands on Achievements and finds badges.
- **Desktop**: consider a "Progress" grouping label over the Dashboard/
  Achievements pair, or rename nothing and let F-32/F-35 give Achievements a
  review dimension.
- **Mobile**: at ≤900px the destinations collapse into a header dropdown
  (`src/lib/navigation.ts:66` `matchMedia("(max-width: 900px)")`, Settings moved
  into the toggle at `:69`). Six items in a dropdown is fine; five tabs would not
  be. **Do not** add a bottom tab bar to match Runna — the dropdown is measured
  clean at 390 and a bottom bar would fight the existing header.
- **Effort**: S (label only) · **Depends**: — · **Impact**: 2

### U-02 · Week header: two tiles → one ratio

- **Runna**: `9.2 km / 41,0 km` in the week header, adjacent to the days it
  describes; two tiles make the user do the division (`exec §10.2`).
- **Runalytics today** — **CLOSED (2026-10-27)**: one tile reads
  `37.0 / 45.0 km` with `ran / planned` under it and the date range in the label
  (`src/calendar.ts renderWeekStats`); avg pace and last published keep their
  tiles. On mobile the same ratio heads each agenda week
  (`.agenda-week-ratio`, `src/lib/schedule.ts`).
- **Change**: one tile reading `30.0 / 38.0 km` with the range in the label,
  keeping the accessible title. Presentation only — the numbers are already
  computed and `planM` already includes banked sessions.
- **Desktop**: replace the first two tiles; keep avg pace and last published.
- **Mobile**: the same four tiles are the "This week" strip; the ratio frees a
  slot at 390 where four tiles are tight.
- **Effort**: S · **Depends**: — · **Impact**: 4

### U-03 · Down-week badge on the week card

- **Runna**: users are told to *guess* which week is the deload; ours is
  deterministic, so the badge is free (`mgmt §10.7`).
- **Runalytics today** — **CLOSED (2026-10-27)**: a "Recovery week" badge
  (`.week-badge`, tooltip "Your plan cuts volume this week on purpose.") renders
  on the desktop week strip, the grid week header, and the agenda week header at
  mobile — driven by the `downWeek` flag already in the payload
  (`src/calendar.ts:346`, `:413`).
- **Change**: a small "Recovery week" badge on the `#week` card and the calendar
  week header, driven by the flag already in the payload.
- **Desktop**: badge on the week card + the calendar week strip.
- **Mobile**: agenda-list week header (the grid is hidden, see U-05).
- **Effort**: S · **Depends**: — · **Impact**: 4

### U-04 · Put the lap verdict on the activity, not only on Plan

- **Runna**: the bar-against-cap verdict is on the activity you just opened
  (`prog §8.2`).
- **Runalytics today** — **CLOSED (2026-10-27)**: the workout detail sheet now
  renders the same verdict (`session_review` → `.workout-verdict` with the shared
  `paceTrack` bar and the accessible sentence), alongside the Plan page's
  adaptation view. One component (`src/lib/pace-track.ts`) serves both; the sheet
  is a bottom sheet on both surfaces.
- **Change**: also render it in the workout/activity detail sheet
  (`src/lib/workout-detail.ts`). See F-33.
- **Desktop / Mobile**: the detail sheet is a bottom sheet on both
  (`src/lib/overlay.ts:89` switches axis on `matchMedia`), so one component serves
  both.
- **Effort**: S · **Depends**: — · **Impact**: 3

### U-05 · The calendar grid disappears on iPhone

- **Runna**: the phone is the primary device; the grid is the plan.
- **Runalytics today** — **CLOSED (2026-10-27)**: option (a) shipped. The agenda
  carries the week ratio (U-02), the Recovery-week badge (U-03), the moved marker
  (U-10) and the **"Move to…"** action (F-03) in the workout sheet — verified at
  390×844 including the back-to-back long-run refusal. The grid stays hidden on
  phones; the agenda is the plan view there.
- **Change**: either (a) accept the agenda-first phone model and make the agenda
  carry the week ratio (U-02), the down-week badge (U-03) and a "Move to…" action
  (F-03); or (b) restore a compact 7-column grid with no chips, just dots. Option
  (a) is cheaper and already half-built.
- **Desktop**: unchanged.
- **Mobile**: this is the single biggest mobile decision in the backlog.
- **Effort**: M · **Depends**: F-03 · **Impact**: 5

---

## 2. Accessibility (measured)

### U-06 · Dark mode: MapLibre attribution is near-black on a dark panel

- **Runalytics today** — **CLOSED (2026-10-27)**, root cause as diagnosed. The
  link rule now carries the specificity/`!important` treatment its sibling
  already had (`app.css:3182-3193`, with a comment naming the bundle-order
  cause), and the attribution control itself is tinted to the theme with
  `!important` (`app.css:3162`). Re-measured in dark mode: the links clear 4.5:1.
- **Change**: raise specificity (`.maplibregl-ctrl-attrib.maplibregl-ctrl-attrib a`)
  or add `!important`, matching the neighbouring overrides.
- **Desktop / Mobile**: both; the attribution is present on every map, and the
  mobile variant already grows the tap area (`app.css:7557-7566`).
- **Effort**: S · **Depends**: — · **Impact**: 3
- **Note**: this is the only real contrast defect found. Do not "fix" the harness
  readings that looked worse — several pages show spurious failures if measured
  *mid theme-transition*; settled, they are clean.

### U-07 · One uppercase rule violates the repo's own no-uppercase rule

- **Runalytics today** — **CLOSED (2026-10-27)**: the transform is gone —
  `.plan-fact dt` keeps `letter-spacing: 0.04em` and `--text-faint` with a
  comment recording the no-uppercase rule (`app.css:3669-3675`). Zero uppercase
  elements app-wide.
- **Change**: drop the transform; keep the `letter-spacing: 0.04em` and
  `--text-faint` so the label still reads as a label.
- **Desktop / Mobile**: identical rule, one edit.
- **Effort**: S · **Depends**: — · **Impact**: 2

### U-08 · Tap targets below 44px (full inventory)

- **Runna**: every control on the phone is a full-height row.
- **Runalytics today** — **CLOSED (2026-10-27)**, all four priorities shipped:
  1. **Body map** — a `bodymap-list` fallback renders under the map at ≤560px
     (`src/lib/bodymap.ts:164-197`): full-width 44px row buttons per region, so
     the primary input has no sub-44 target on phones.
  2. **Calendar chips** — `.workout-chip` min-height raised 40 → 44
     (`app.css:5191-5193` with the reason in a comment; mobile query at `:7984`).
  3. **Carousel / icon buttons** — `.carousel-btn` 44×44 (`app.css:7978`),
     `button.icon-btn` sized from `--control-h` (44px, `app.css:3467`).
  4. **Inline links** — the `::after` hit-area recipe is reused for the MapLibre
     attribution (`app.css:3193`) and Plan's calendar link.
- **Change**, in priority order:
  1. **Body map (Injuries)** — 49 sub-44 targets on the *primary input* of the
     page, on the device where it matters most. Mitigate with a list fallback
     under the map at ≤560px, or enlarge the hit area with a transparent
     `stroke-width`/`<path>` halo rather than the visible geometry.
  2. **Calendar chips h=40** — 4 px short; bump the chip min-height to 44 in the
     grid.
  3. **Carousel buttons 40×40 and 32×32 icon buttons** — raise to 44.
  4. **Inline links** (Plan's "open the calendar", MapLibre attribution) — the
     app already solves this pattern once: `#plan-view .panel-meta a` grows its tap
     target through a pseudo-element without moving layout (`app.css:3656-3672`,
     the `::after` at `:3669`). Reuse that recipe.
- **Desktop**: items 2–4.
- **Mobile**: item 1 is the mobile finding; items 2–3 are moot where the grid is
  hidden.
- **Effort**: M (map) / S (the rest) · **Depends**: — · **Impact**: 4

### U-09 · Disabled controls read as faint text, not as disabled

- **Runalytics today** — **CLOSED (2026-10-27)**: disabled buttons get a visible
  muted treatment — `background: var(--glass-bg-2)`, `border-color: var(--line)`,
  no shadow (`button:disabled:not(.btn-busy)`, `app.css:3381-3386`) — so the
  state reads as disabled, not as a rendering bug. The Plan page's prose still
  says *why* ("Use Abort plan above before creating another").
- **Change**: give disabled buttons a visible-but-muted fill/border so the state is
  unmistakable, and keep a tooltip or hint stating *why* it is disabled (the Plan
  page already does this in prose: "Use Abort plan above before creating another").
- **Effort**: S · **Depends**: — · **Impact**: 2

---

## 3. Copy and literacy

### U-10 · Moved-session marker

- **Runna**: a marker shows which days the user has touched (`exec §10.1`).
- **Runalytics today** — **CLOSED (2026-10-27)**: a moved session carries a
  corner-fold marker (`.cal-cell.moved-day`, `app.css:5709-5713`) on desktop and
  a "Moved" chip on the agenda row; both read the `moved` flag that `move_workout`
  now sets without touching `planned_date` (F-01/F-43). Undo and Reset week clear it.
- **Change**: a small corner marker on any cell where `date != planned_date`.
- **Desktop**: grid cell. **Mobile**: agenda row (grid hidden).
- **Effort**: S · **Depends**: F-01 · **Impact**: 3

### U-11 · Say the two verbs at the top of the calendar

- **Runna**: repeats a two-row table distinguishing reconfigure from reschedule
  (`mgmt §10.3`).
- **Runalytics today** — **CLOSED (2026-10-27)**: `.cal-hint` under the calendar
  header states both verbs in one line ("…to reschedule the week you're in. To
  change the plan itself, use Plan…") — visible on both surfaces (F-45).
- **Change**: one line above the grid/agenda. See F-45.
- **Effort**: S · **Depends**: — · **Impact**: 3

### U-12 · Two onboarding slides: permission to deviate, and chart literacy

- **Runna**: spends a whole screen explaining what a mileage graph *is*, and tells
  the user deviation is normal *before* it happens (`exec §10.4`).
- **Runalytics today** — **CLOSED (2026-10-27)**: `src/lib/onboarding.ts` is now
  six slides. Slide 5 ("Read your charts") names the ramp, the deliberate dips,
  and the taper; slide 6 ("Plans meet real life") grants permission to deviate.
  The storage key bumped to `runalytics.onboarding.v2` so existing users see the
  new slides once. Verified at 1440 and 390×844 (dots, nav, completion).
  Still one overlay component on both surfaces — no per-generation ceremony
  (divergence #6 intact).
- **Change**: replace slide 5's body with the literacy framing (what the ramp is,
  what a down week is) and add a sixth slide granting permission to deviate
  ("plans meet real life — moving a run is not failing it"). Zero engineering.
  Do **not** build a four-screen ceremony per generation (divergence #6).
- **Desktop / Mobile**: same overlay component on both.
- **Effort**: S · **Depends**: — · **Impact**: 4

### U-13 · Teach the plan timeline

- **Runna**: the graph is explained, not just shown (`exec §10.4`).
- **Runalytics today** — **CLOSED (2026-10-27)**: the dashboard timeline legend
  now names the three shapes ("the ramp building week by week, dips where the
  plan cuts volume on purpose, and the taper beside the flag",
  `src/dashboard.ts`), and the Recovery-week badge (U-03) makes the dip
  self-explaining on the calendar.
- **Change**: one caption under the timeline naming the three shapes it contains,
  plus the down-week badge (U-03) so the dip is self-explaining.
- **Effort**: S · **Depends**: U-03 · **Impact**: 3

### U-14 · `at` vs `@` in the UI (F-37's visible half)

- **Runalytics today** — **CLOSED (2026-10-27)**: `fmtPaceBand()`
  (`src/lib/ipc.ts`) is the single prescribed-band formatter (`at lo–hi/km`),
  shared by the dashboard timeline, calendar chips, plan pacing, and the detail
  sheet; `fmtPaceHeld()` keeps `@` for recorded pace only (F-37/F-48).
- **Change**: prescribed becomes `at`, actual stays `@`; one shared formatter so
  notes and timeline cannot drift (F-48).
- **Effort**: S · **Depends**: F-37 · **Impact**: 3

### U-15 · Disarm the ceiling in the UI copy

- **Runna**: "This is a limit, not a target — run at whatever pace feels truly
  easy!" (`ux §7.2`).
- **Runalytics today** — **CLOSED (2026-10-27)**: the sentence now grants
  permission — "a limit, not a target: slower is fine, and often better"
  (`calendar.rs`, F-38). Copy flows to both surfaces through the notes.
- **Effort**: S · **Depends**: — · **Impact**: 3

---

## 4. Visual system (measured)

### U-16 · Radius ladder drift

- **Runalytics today** — **PARTIAL**: the intended allowlist is
  0 / 8 / 10 / 18 / 22 / 999px / 50%. Measured non-token radii, by frequency:
  **12px** on `.nav-link` (10), `.cal-cell` (40+14+8+6 across states),
  `.panel` (4), `.skip-link` (2), `.chip`; **20px** on `.panel`, `.notice.warn`,
  `.hero-brief`, `.today-action`, `.decision-lead`, `.pillar`, `TABLE`,
  `.danger`; **7px** on `.bar-fill` (12), `.meter` variants (12+4+2);
  **7.5px** on `.brand-tile` (2).
- **Change**: either promote 12 and 20 onto the ladder (they are used widely and
  look intentional) or fold them to 10/18. The 7px and 7.5px cases are genuine
  drift — 7px on bars/meters should be `--radius-sm`, and a 7.5px brand tile is
  almost certainly an accident.
- **Desktop / Mobile**: shared tokens, one pass.
- **Effort**: S · **Depends**: — · **Impact**: 2

### U-17 · Workout-kind colour grammar (deliberate divergence, recorded)

- **Runna**: green=easy, orange=tempo, red=intervals, purple=long, grey=imported
  (`ux`).
- **Runalytics today** — **BY DESIGN**: one hue per palette with per-kind
  lightness steps (`--workout-easy` … `--workout-race`, `app.css:76-80` dark and
  `:310-314` light, plus four palette overrides). Completed sessions switch from
  outline to fill (`.chip-done`, `app.css:3581`), so *fill* carries "banked" and
  hue carries kind within one hue.
- **Change**: none — see divergence #11 in `README.md`. Recorded here because a
  Runna user will notice the difference immediately, and the answer is a
  deliberate one, not an oversight. The one real defect in this area is already
  fixed in code: the `chip-longRun` / `chip-longrun` alias (`app.css:3558-3560`)
  exists because the calendar emits `longRun` while serde lowercases it — keep the
  alias.
- **Impact**: 0

### U-18 · Heading scale is sound; keep the panel H2 out of the outline

- **Runalytics today** — **CLOSED**: measured H1 34/700 ls −1.87px, hero H2
  38/600, `.group-head` H2 17/700, panel H2 15/650 ls −0.3px. Consistent, and the
  skip link is present on every page.
- **Change**: none. One note for U-01: with panel headings at H2, the document
  outline for the dashboard is 1 H1 + 10 H2, which is noisy for screen-reader
  navigation. Consider demoting panel titles to H3 under their group H2.
- **Effort**: S · **Impact**: 1

### U-19 · The mobile header is a dropdown, and that is fine

- **Runalytics today** — **CLOSED**: at 390×844 the nav collapses to
  brand + Sync + "Open navigation" (`navigation.ts:66-69`), no overflow
  (`scrollWidth == innerWidth == 390` on all five pages), and the iOS status bar is
  coloured per palette via `theme-color` (`paintStatusBar()`,
  `src/lib/theme.ts:64-66`, called from `setPalette()` at `:76`).
- **Change**: none. Do not add a bottom tab bar to imitate Runna (see U-01).
- **Impact**: 0

---

## 5. Mobile-specific summary

Measured at 390×844, all pages: **no horizontal overflow**, **no contrast
failures**, **no uppercase labels** (except Plan's, U-07). The mobile gap list is
short and specific:

| # | Finding | Item |
| --- | --- | --- |
| 1 | No way to rearrange a plan at all (grid hidden + touch drag disabled) | F-03, U-05 |
| 2 | 49 sub-44px targets on the injury body map | U-08 |
| 3 | No month view; agenda-first, so the week ratio and badges must move to the agenda header | U-02, U-03, U-05 |
| 4 | iOS is pull-only for plans (`config.can_publish = !mobile()`, `plan_sync.rs:603`), so every "your plan changed" surface must be written for a reader, not an editor | F-24 |
| 5 | The assistant rejects non-https endpoints except loopback (`src-tauri/src/assistant.rs:43-60`), so a local model on the phone needs the desktop — worth stating in Settings rather than leaving as a silent failure | — |

> Update 2026-10-27: gaps 1–3 closed in the implementation pass (Move to… +
> date sheet on mobile, bodymap list fallback, week ratio and Recovery badge in
> the agenda header). 4 and 5 stand.

---

## Status rollup (2026-10-27, after the implementation pass)

| Status | Count | Items |
| --- | --- | --- |
| OPEN | 0 | — |
| PARTIAL | 2 | U-01 (six tabs kept by design; Progress grouping undecided), U-16 (radius ladder) |
| CLOSED / by design | 17 | U-02…U-15, U-17, U-18, U-19 |

**Closed in this pass (2026-10-27)**, all verified at 1440 and 390×844: U-02
(single ratio + mobile agenda header), U-03 (Recovery-week badge), U-04 (verdict
in the detail sheet), U-05 (agenda-first accepted, Move to… shipped), U-06
(attribution contrast), U-07 (uppercase removed), U-08 (44px targets incl.
bodymap list), U-09 (disabled fill), U-10 (moved marker), U-11 (calendar hint),
U-12 (onboarding slides), U-13 (timeline caption), U-14 (`at`/`@`), U-15
(limit-not-target copy).
