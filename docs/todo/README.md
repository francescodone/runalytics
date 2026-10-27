# Runna parity backlog

Consolidated, re-verified backlog of everything that would make Runalytics behave
and look as close to Runna as our deliberate-divergence rules allow — features
(`RUNNA-FEATURE-PARITY.md`) and UI/UX (`RUNNA-UIUX-PARITY.md`), desktop and mobile.

Sources: the six research phases in [`../runna-ux/`](../runna-ux/) (each phase's
"What this changes for Runalytics" section), re-checked line-by-line against the
code at HEAD `0e77402` (v0.14.0) on 2026-10-27, plus new findings from a live
desktop inspection (all pages at 1440px, light + dark) and 390×844 mobile
measurements. Runna behaviour is cited to its phase doc §, never re-quoted —
the phase docs carry the evidence and the quote audit.

## Documents

| File | Contents |
| --- | --- |
| [`RUNNA-FEATURE-PARITY.md`](./RUNNA-FEATURE-PARITY.md) | F-nn — logic, state model, plan engine, insights, integrations |
| [`RUNNA-UIUX-PARITY.md`](./RUNNA-UIUX-PARITY.md) | U-nn — layout, copy, navigation, accessibility; Desktop + Mobile delta per item |

## Item schema

Every item carries:

- **Runna** — the behaviour, cited as `(phase §)`; e.g. `(mgmt §10.1)` =
  `RUNNA-PLAN-MANAGEMENT.md` §10 item 1. Phase keys: `ux` = RUNNA-UX,
  `gen` = RUNNA-PLAN-GENERATION, `exec` = RUNNA-PLAN-EXECUTION,
  `mgmt` = RUNNA-PLAN-MANAGEMENT, `prog` = RUNNA-PROGRESS-AND-REVIEW,
  `ins` = RUNNA-INSIGHTS-AND-ADAPTATION.
- **Runalytics today** — file:line evidence, verified 2026-10-27, with status:
  **OPEN** (gap confirmed), **PARTIAL** (some of it exists), **CLOSED**
  (already done — kept only where the closure is load-bearing for another item).
- **Change** — what to build.
- **Desktop / Mobile** — the concrete delta per surface. Mobile claims cite a
  selector or line, not a screenshot.
- **Effort** — S ≤ half a day · M ≤ a few days · L ≥ a week.
- **Depends** — item ids that must land first.
- **Impact** — parity impact, 0–5:
  5 = changes how the product *feels* to a Runna user ·
  4 = closes a headline Runna workflow ·
  3 = visibly closer, not decisive ·
  2 = polish or niche ·
  1 = hygiene ·
  0 = documented, no action.

## Wave ordering (recommended sequence)

1. **Schema unlock** — F-01 (`planned_date` + `WorkoutStatus`) is the keystone: it
   unblocks F-02, F-03, F-06, F-26, F-34, F-43 and U-10. Nothing else in the
   backlog moves as much.
2. **Cheap trust copy** — F-04 receipt, F-05 announced adjustments, F-13/F-17/F-18
   explanations, F-37 + U-14 the `at`-vs-`@` notation, F-38/U-15 disarmed ceilings,
   U-03 down-week badge, U-06/U-07 measured a11y/style fixes, U-12 onboarding
   slides. Mostly rendering, no engine risk.
3. **Insights & review** — F-07 adherence ratio + U-02 one-ratio header, F-08 dated
   contract, F-09 abstain states, F-24 sync pill, F-32 splits, F-33/U-04 verdict
   placement, F-34 link activity + F-35 plan-vs-total breakdown.
4. **Engine knobs** — F-19…F-23 ramp and spacing guards, F-16 knob split, F-14
   ceilings as km, F-10 start-date candidates. Each needs a test pinning the
   promise.
5. **New surfaces & mobile** — F-25…F-31 (RPE, reminders, instant workouts, manual
   logging, heat, elevation, PRs), U-05 the mobile calendar decision, U-08 tap
   targets, U-01 nav labels.

## Implementation status (2026-10-27)

Waves 1–4 are done, plus the mobile items from wave 5. Every change was verified
on desktop (1440) and iPhone (390×844) in the browser harnesses; engine changes
carry tests. `cargo test --lib`: **131 pass / 0 fail / 1 ignored** (the three
pre-existing failures named below are fixed); `npm run build` green.

- **Wave 1 (schema unlock)**: F-01, F-02, F-43, F-03 (Move to… + mobile date
  sheet + Undo + Reset week), F-06 remains OPEN (status lifecycle beyond moved).
- **Wave 2 (cheap trust copy)**: F-04, F-05, F-37, F-38, U-03, U-06, U-07, U-12,
  U-14, U-15, F-45/U-11, F-47. F-13/F-17 partially; F-18 OPEN.
- **Wave 3 (insights & review)**: F-07 + U-02, F-24 (sync pill, both
  placements), F-33 + U-04, U-08, U-09, U-10, U-13. F-08, F-09, F-32, F-34, F-35
  remain OPEN/PARTIAL as marked in the feature doc.
- **Wave 4 (engine knobs)**: F-19 (absolute +4 km weekly step cap, tested),
  F-22 (back-to-back long-run guard, tested, harness refusal verified on
  mobile), F-23 (taper floor, tested). F-10, F-14, F-16, F-20, F-21 OPEN.
- **Wave 5 (partial)**: U-05 resolved agenda-first; F-26 (reminders) deferred by
  decision — iOS notifications need APNs and this app has no push server.
  F-25, F-27…F-31 OPEN.

Deliberate divergences were respected throughout — none of the 13 below were
closed.

## Deliberate divergences (do NOT close these)

Stated once here so six phases of admiration don't drift into uncanny-valley
imitation. These Runna behaviours are *known and rejected*; any future item that
would reintroduce them contradicts this list.

1. **Score demotion** — Runna punishes inactivity in a health product (`ins §12.1`).
   Our streak celebrates continuity and never revokes. Keep.
2. **AI you can't opt out of** — ours is opt-in (`ins §12.2`). Keep.
3. **Labs + Premium gating of basics** — a local app has no reason to fragment its
   own feature surface (`ins §12.3`). Keep everything on by default.
4. **Silent outlier removal** — if we ever trim chart data, mark the trimmed
   points (`ins §12.4`).
5. **A metric the vendor admits is weak** — our four scores carry `notes`; the
   notes are the honest part (`ins §12.5`).
6. **The four-screen generation ceremony** — take the *content* (permission,
   literacy) into onboarding once, not four times a year (`exec §10`, `gen §6.3`).
7. **The upsell surface** — five interest categories cross-selling products we
   don't have (`gen §6.3`).
8. **PR board over progress badges** — Runna's board is 14 months frozen; our
   progress badges are a moving target, the better choice for a training tool
   (`prog §8.6`).
9. **Ambiguous lifetime totals** — Runna's All-Time card can't tell you whether
   yoga is inside the distance; ours counts running-only for distance and says so
   (`prog §8.4`, `achievements.ts:82` comment).
10. **Point economy (Levels)** — farmable without caps, demotion on inactivity;
    gated behind Labs *and* Premium in Runna (`ins §11.7`). Not copied.
11. **Runna's chip colour grammar** (green=easy / orange=tempo / red=intervals /
    purple=long / grey=imported, `ux`) — Runalytics deliberately uses one hue
    ramp per palette with per-kind lightness steps (`app.css:76-80` and the
    palette blocks). The grammar is carried by labels + fill (done), not hue.
12. **Plan Versions revert** — our sync is monotonic by design (`installed_rev`,
    `plan_sync.rs`); Runna's revert is unavailable exactly when needed
    (`ins §11.5`, `ins §10.7`). Keep monotonic; adopt only the status pill (F-24).
13. **Unbounded pace drift** — our ±20 s/km pacing ceiling is safer than Runna's
    race-time-edit model; keep the ceiling, *say it* (F-09) (`ins §11.1`).

## Verification state of this audit

- Gates at HEAD (2026-10-27, after the implementation pass): `cargo test --lib`
  **131 pass / 0 fail / 1 ignored** · `npm run build` green · `tsc --noEmit`
  clean. (The audit's original gate noted 108 pass / 3 fail and fmt/clippy debt;
  the three failures are fixed. The repo is still not `cargo fmt --all --check`
  clean at large — ~50 pre-existing diffs — so fmt only over files you touch.)
- Desktop: all six pages + four harnesses measured at 1440px in light and dark —
  zero WCAG contrast fails in light; two real dark-mode fails (U-06); one
  uppercase rule (U-07); tap-target inventory in U-08.
- Mobile 390×844: no horizontal overflow on any page; nav collapses to a header
  dropdown (`navigation.ts:66`); the calendar grid stays hidden ≤560px by design
  (U-05, agenda-first) and the rescheduling gap is closed — **Move to…** opens a
  date sheet on touch, with the backend guards surfaced and Undo available.
