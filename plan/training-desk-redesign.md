# Training Desk: Independent Product Redesign

Date: 2026-10-30
Status: replacement implemented, browser-verified and migrated to the original routes

## Instruction Audit

No AGENTS.md, CLAUDE.md, .cursorrules, .cursor/rules files, nested agent
instructions, or .github/copilot-instructions.md were found in the repository.
The repository contains extensive previous design specifications and memory.
They are historical evidence, not a reference for this replacement.

| Source | Conflicting constraint | Treatment |
| --- | --- | --- |
| README.md | Four primary destinations, More, desktop sidebar, separate Plan and Calendar | Updated after route migration; functional/setup/release guidance retained |
| plan/design-runna-redesign-1.md | Match Runna screen hierarchy; retain controller IDs; 216px rail; five bottom tabs | Superseded for the parallel frontend; original document retained |
| docs/todo/RUNNA-UIUX-PARITY.md | Historical dropdown mandate, parity targets, reuse existing layout overrides | Not applicable to the new presentation; accessibility and safety remain applicable |
| plan/runna-redesign-evidence.md | Preserve week rail and settings categories; existing shell and tokens | Verification of the previous implementation only |
| Repository memory | Several incompatible paper, violet, Runna and Apple-inspired systems | Do not use visual prescriptions; verify technical claims against current source |
| General reuse guidance and design skills | Extend existing components and visual language | This request explicitly requires an independent presentation implementation |

No instruction file is deleted. Privacy, secure credentials, medical disclaimers,
destructive-action confirmation, backend training rules, data contracts, user
preferences and deep-link compatibility continue to apply.

## Product Inventory

Audience: runners using COROS and a Mac/iPhone to prepare for races, build fitness
or maintain consistent training. Desktop supports planning and investigation;
mobile supports checking a session, changing a day and logging symptoms.

| User objective | Required information | Preserved capabilities |
| --- | --- | --- |
| Start training | Connection, sync recency, available history, baseline | Connect/disconnect, API diagnostics, sync progress/results, setup states |
| Build a realistic plan | Goal, date, threshold, mileage, frequency, long-run day, active pain | Race/improvement/maintenance, baseline defaults, validation, receipt, watch publishing, cancellation |
| Organize the week | Planned vs completed/skipped/missed, original dates, recovery weeks | Week/month navigation, reschedule with Undo, reset preview, skip/restore, manual completion matching |
| Execute a session | Date, prescription, distance/duration, pace ranges, adjustment reason | Warm-up/work/recovery/cool-down from backend notes; session-level actions |
| Learn from a run | Route, laps, splits, actual vs target, health/load/risk at the time | Activity review, detail, run scores, filtering by name/sport/provider/date |
| Understand adaptation | Load, recovery, fitness, mileage, predictions, pacing evidence | Range selection, backend bands/thresholds, trends, routes/heatmap, streaks/milestones, pacing offers |
| Respond to pain | Location, intensity, onset, notes, status, proposal changes | Log/delete/resolve/reopen/follow-up; accept/dismiss changes; optional assistant and conversation clearing |
| Control the app | Appearance, zones, credentials, sync direction, publishing, map source, version | All existing settings, consent, secure key operations, update flow, troubleshooting |

All data continues through src/lib/ipc.ts and src/lib/types.ts. Rust remains
authoritative for scoring, prescription generation, adaptation and move guards.
No health data is sent to external design services or included in fixtures.

## Concepts Compared

1. Training Desk: top-level Training / Journal / Health workspaces; combined
   plan/calendar; desktop week lanes and session inspector; mobile date ribbon,
   itinerary and full-screen detail. Strongest for frequent plan/run/review tasks.
2. Coach Briefing: decision-led daily stream, separate plan editor, mobile task
   stack. Clear next action, weaker week comparison and historical investigation.
3. Training Atlas: season timeline with filters and inspection, mobile chronological
   log. Strong analytical continuity, higher learning cost for ordinary daily use.

Selected: Training Desk. The runner's time and selected session organize the
interface, not the old page taxonomy. Primary actions sit with the object they
change; account and help are utility destinations, not training destinations.

## Structural Contract

| Former screen | Old architecture | New architecture and interaction | Benefit |
| --- | --- | --- | --- |
| Today | One workout with a neighboring stack of three signals; sidebar/bottom tabs | Training opens the current week; day lanes and session selection on desktop, date ribbon and itinerary on mobile; readiness is contextual | Daily execution and schedule changes share one workspace |
| Plan | Identity, week selector/rail, session list, management disclosure | Plan is a horizon within Training; cumulative schedule, plan basis and actions share the workspace | No duplicate plan/calendar browsing |
| Calendar | Month grid and agenda with view toggle | Time-horizon toolbar switches week/month/plan; session inspector remains the action surface | Move, inspect and review without separate page transitions |
| Progress | Workouts/Performance/Milestones tabs under page header | Journal is a searchable master/detail activity library; analytics and milestones belong to Health | A run stays selected while comparing its route, laps and context |
| Injuries | Body map, collapsed form, journal below, proposals | Health combines evidence and an episode journal; explicit Log pain task, selected episode conversation and reviewed changes | Symptom action is adjacent to recovery context |
| Settings | Second sidebar or category selector, one pane | Searchable settings directory with focused inline task editors and preserved hashes | Faster discovery without nested navigation |
| Support | Destination rows followed by FAQs | Searchable help topics with contextual actions and app diagnostics | Answers route directly to the relevant task |

Desktop: top application bar, a context toolbar, a work canvas and contextual
inspector. No global sidebar. Mobile: three bottom destinations, utility access
in the header, selected-day itinerary, and full-screen detail with explicit Back.
The mobile schedule is not a compressed seven-column desktop board.

## Independent Presentation Boundary

- New React entry at desk.html; source under src/desk/; standalone desk.css.
- No imports of old controllers, navigation, sheets, bodymap rendering, page CSS
  or presentation components. Shared IPC, types and non-presentational utilities
  are permitted. Vendored fonts and the bundled identity are assets, not layouts.
- Original HTML source was retained while the replacement was built and tested.
   After those checks passed, Vite entry transformation migrated development and
   production URLs to the independent shell without overwriting that source.
- Deep links map to the corresponding new workspace/task, not a generic home.
- Old presentation is available to the regression suite using the development-only
   `X-Runalytics-Legacy: 1` header. It is not included in the production entry graph.
   No pre-existing user work was reverted or deleted.

## New Visual Language

Geist provides a compact readable interface and tabular numbers. A neutral
porcelain canvas (#f6f8f7), white work surfaces, near-black ink (#18231f), forest
commands (#16634b), blue comparison data (#2968ba), and amber warnings (#946000)
differentiate action, evidence and caution. Dark mode uses neutral charcoal,
not navy. Legacy palette preferences remain supported as explicit choices.

Information is organized by aligned lanes, lists and working surfaces, not a
grid of metric cards. Controls are 44px touch targets. Fixed type roles and
4/8/12/16/24/32 spacing; letter spacing is zero. Motion explains selection and
drill-in; reduced motion removes spatial transitions. Real activity routes and
training charts are the visual assets, not decorative stock photography.

### Material Refinement (2026-10-31)

The subsequent visual request retains this workspace structure while replacing
Geist with local Source Serif 4 headings and Source Sans 3 interface text.
Rounded toolbars, dialogs, inspectors and navigation use restrained translucent
materials; palette tints reach the canvas and selected chrome. Health chart
areas and lap bars fade toward their baseline, and on-target lap bands receive
an explicit highlight. Workspace, dialog and inspector entries use short
springs, with tactile press states and focus feedback. Reduced motion disables
spatial transitions; reduced transparency or increased contrast uses solid
surfaces. User data and business logic remain local.

### Interaction System (2026-10-31)

The active interaction layer is independent of the earlier presentation rules.
It is owned by src/desk/interaction.tsx and src/desk/interaction.css. The existing
Motion dependency supplies presence, shared-layout selection and gesture physics;
no dependency was added and no training or IPC payload was changed.

| Role | Timing And Behavior |
| --- | --- |
| Press | 80ms; compact controls compress to 0.96, rows use an inset selection wash without moving columns |
| Feedback | 140ms; input borders, selection emphasis and state icons |
| Content change | 220ms; short opacity transition, no initial page-load cascade and no outgoing screen blocking the next screen |
| Surface | Critically settled, zero-bounce spring with 280ms visual response; desktop uses 12-18px offsets |
| Exit | 180ms acceleration; same spatial path as entry, surface stays mounted until completion |
| Reduced motion | No spatial movement, layout travel, scale feedback or gesture animation; brief 80ms opacity feedback, static selection and full keyboard controls |

MotionSystem configures the root policy. Presence wraps conditional dialogs and
inspectors, including async saves. SelectionGroup namespaces SelectionMarker so
workspace, horizon, workout-mode, date and chart-session indicators cannot cross
between unrelated controls. Change refreshes a selected content region without
replaying surrounding chrome or initial page-load effects. The JS tempo values
and CSS --motion-* tokens describe the same timing roles and easing family.

Shared buttons keep their label footprint through pending and result states.
The initiating control receives the result even when a form is submitted with
Enter or a touch browser does not focus the tapped button. Busy controls reject
duplicate submissions; success/error icons are supplementary to readable status
or error content. Result icons clear after one second, with timers cleaned up on
unmount. Loading content reserves space immediately and reveals after 120ms so
fast IPC reads do not flash a spinner. Notifications have retained exits, explicit
dismissal and no timeout that can steal an Undo opportunity.

Desktop dialogs settle into the center. Mobile dialogs rise from the bottom and
track downward gestures only from a dedicated grip; horizontal inspector return
is recognized only on the noninteractive header area. Controls, sliders, maps
and scrollable content keep their native gestures. Constrained drags resist the
wrong direction, cancel to rest, and dismiss by distance or release velocity.
Back, Escape and Cancel remain available without a gesture. Focus restoration,
mobile inert background, modal scroll ownership and viewport changes remain part
of the surface lifecycle, including its exit.

Native details retain browser semantics and progressively enhance block-size
transitions where supported. No scroll listeners, cursor tracking, decorative
parallax, blanket will-change or idle animation loop was introduced. Transform
and opacity carry movement; disclosure/episode height is the deliberate layout
animation exception. Chart values are never interpolated into invented readings.
Native schedule dragging highlights the candidate day and still opens the review
surface before a move is saved; cancellation clears feedback through React state.

Motion regression coverage lives in the existing tests/desk.spec.mjs suite:
pending/result geometry, failures and keyboard retry, retained exits, focus and
scroll restoration, real touch tracking/cancellation/dismissal, reduced motion,
selection interruption, preference/viewport changes, tooltips and motion-enabled
12-view tours in both themes at 390px and 1440px. Test-clock callers explicitly
advance through a surface exit before asserting resource cleanup. Browser
screenshots are written to ignored test-results/motion-*.png. Browser emulation
does not replace physical iOS or live COROS acceptance.

## Verification And Migration Gates

1. Build isolated shell; prove no legacy presentation imports and reachable
   destinations before adding workspace behavior.
2. Validate each task against the same IPC payloads and backend guards.
3. Test loading, empty, error, success, confirmation, Undo and draft preservation.
4. Validate 320/390/768/1440px layouts, both themes, keyboard/focus, reduced motion
   and automated accessibility. Inspect rendered screenshots of every workspace.
5. Migrate the seven existing entry routes only after parallel checks pass; run
   production build and the replacement regression suite again after migration.
6. Record anti-anchoring assessment: DOM composition, navigation, information
   hierarchy and user tasks must change, not just style tokens.

Native provider availability and physical-device operation require separate
acceptance; synthetic tests must never be presented as live account verification.

## Acceptance Record

Latest verification, 2026-10-31: production build and all 100 selected UI tests
pass after the interaction-system implementation. Ten known rest-day-dependent
cases remain excluded, not repaired or counted as passing. Ten new motion tests
cover 12-view light/dark tours at 390/1440px, automated accessibility, real touch
tracking/cancellation/dismissal, stable pending/result geometry, keyboard retry,
retained exits, focus/scroll ownership, interruptible selection, Escape-dismissed
tooltips, live preference changes and resizing. The tours generate 56 screenshots.
Existing three-second route animation and map resource-cleanup tests still pass;
their fake clocks now explicitly advance through inspector exit.

Run the VS Code task "Verify adaptive glass UI" to reproduce the production
build and selected 100-test gate. The prior adaptation gates also covered 19
pacing/adaptation, 15 lap and 13 scoring Rust cases; Rust was not modified or
retested for this interaction-only work. Native iPhone/WebKit performance and
live provider acceptance remain separate. The original record below is historical.

Verified 2026-10-30:

- `npm run build && npm run test:ui`: production build and **89 tests passed**,
   comprising 39 replacement tests and 50 retained legacy regressions.
- Ten views (Training week/month/plan, Journal list/map, Health overview/pain/
   milestones, Settings and Help) captured at 320/390/768/1440px in light/dark
   themes. Matrix checks cover document overflow, runtime errors and automated
   WCAG A/AA checks. Session inspectors additionally exercise Tab containment,
   Escape and background restoration. Activity/proposal detail is checked at
   320/1440px in both themes.
- Interactive route heatmaps pass actual WebGL pixel checks and zoom-response
   checks at 390/1440px, not just canvas-existence assertions.
- Creation payloads and failed drafts, move/Undo, skip/restore, reset preview,
   manual matching, native sync events, watch progress, reviewed proposal decisions,
   pain lifecycle/chat, end-plan confirmation, settings preservation, credential
   retention/removal and explicit device-import replacement are exercised.
- Chrome smoke-tested all eight generated HTML entries directly from `dist`.
   They render the appropriate React workspace using one shared entry bundle;
   the output contains no synthetic fixture or old navigation. The temporary
   smoke-test server was closed afterward.
- Existing settings aliases and Progress performance/recovery/risk/milestones/
   routes links resolve to their replacement destinations. Canonical activity
   links use `#journal/<activityId>`.

Rendered captures are generated under ignored `test-results/desk-*.png`. Browser
snapshots and geometry checks supplement the automated capture matrix; these
checks are not a manual assistive-technology certification or physical-device audit.

## Anti-Anchoring Assessment

| Dimension | Evidence of structural replacement |
| --- | --- |
| Navigation | Seven separate page destinations become three workspaces with contextual horizons; top bar replaces the desktop sidebar and three mobile tabs replace the old primary/More split |
| Composition | Desktop schedule uses seven working lanes and an inspector; mobile uses selected-day content and full-screen detail, not scaled-down lanes |
| Information hierarchy | Plan/Calendar/Today become Training; activity investigation becomes Journal; recovery, symptoms and milestones become Health |
| Actions | Session operations live in the selected session, proposal decisions in review dialogs, and settings in dedicated task editors |
| Implementation | Separate React tree and stylesheet; network tests reject old page controllers, styles, navigation and overlay imports |
| Visual language | Aligned work surfaces, flat rows and contextual tools replace the prior card/rail composition; no decorative metric-card dashboard |

## Remaining Acceptance Limits

No backend logic, OAuth contract or credential storage was changed. Browser tests
use synthetic IPC responses, not a live COROS account. Physical iPhone/Safari,
live OAuth, real watch publishing, updater installation and a real offline PMTiles
archive still need native acceptance. Backend rejection and partial-publishing
semantics remain authoritative; fixture behavior is not evidence of remote success.

Vite reports the lazy MapLibre chunk size (about 1.05 MB minified). The production
dependency audit reports the existing critical MapLibre sanitizer advisory
GHSA-jrc7-96c5-q579; its suggested v5-to-v6 upgrade is outside this redesign and
must be assessed before release. No automatic major dependency upgrade was made.

Preview with synthetic data: `http://localhost:1500/__desk_preview.html`.
Normal entry routes use the native bridge for actual training data.