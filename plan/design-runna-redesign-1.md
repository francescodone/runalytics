---
goal: Redesign Runalytics mobile and desktop around the Runna reference experience
version: 1.0
date_created: 2026-10-30
last_updated: 2026-10-30
owner: Runalytics
status: In progress
tags: [design, runna, mobile, desktop, accessibility]
---

# Introduction

![Status: In progress](https://img.shields.io/badge/status-In_progress-yellow)

Replace the current analytics-first, violet interface with a Runna-matched training experience across the entire app. This is an implementation plan, not an implemented redesign. Keep Runalytics branding, real integrations, training safeguards, and existing user data. Match Runna's screen hierarchy, navigation, session presentation, visual language, and interaction patterns wherever the repository contains evidence.

Implementation is underway as of 2026-10-30: the five-destination shell,
reference/legacy appearances, Today/Progress split, history filtering and route
thumbnails, plan wizard/week browser, calendar date selection, Support and
shared detail presentation are implemented. See [implementation evidence](runna-redesign-evidence.md)
for verification and explicit sign-off limits. This plan remains **In progress**
until reference-overlay and native/provider acceptance gates are satisfied.

### Implementation Ledger

The task table's completion gates include acceptance, not just code delivery.
The following ledger separates delivered code from outstanding sign-off.

| Tasks | Code status | Acceptance status |
| --- | --- | --- |
| TASK-001, TASK-003 | Reference-to-screen mapping and responsive contracts documented | Pixel measurements and approved reference overlays pending; desktop/Progress evidence gaps remain |
| TASK-002 | Superseded palette/navigation rules recorded | Done |
| TASK-004, TASK-005, TASK-006 | Reference and legacy palettes, five destinations, Support, ungated navigation delivered | Build and browser layout/accessibility checks pass |
| TASK-007 | Shared presentation delivered through a dedicated stylesheet and existing component utilities | Legacy CSS consolidation remains pending |
| TASK-008, TASK-009, TASK-010 | Today/Progress separation, preserved analytics, purpose-aware filters, totals and route thumbnails delivered | Browser workflows pass; screenshot-exact Progress sign-off blocked by missing captures |
| TASK-011, TASK-012, TASK-013 | Five-step creation, all three plan kinds, week browser and grouped management delivered | Fixture payload/validation tests pass; real provider publishing requires account/device acceptance |
| TASK-014, TASK-015, TASK-016 | Date grid/agenda, rescheduling, prescription blocks and shared completed review delivered | Browser detail/focus/reschedule/Undo tests pass; native gestures not certified |
| TASK-017, TASK-018, TASK-019 | Existing grouped settings, consent and accessible injury paths retained under shared styling; Support delivered | Seven-page light/dark accessibility and overflow gates pass |
| TASK-020 | Playwright and axe regression suite delivered | 21 tests pass |
| TASK-021 | Responsive screenshots generated | Layout checks pass; 4px reference-overlay gate remains open |
| TASK-022 | Frontend build and iOS simulator compilation exercised | Physical-device, real-provider and manual accessibility acceptance remain open |

**Working hypothesis:** changing colours alone cannot produce parity. The controlling differences are the six-destination shell in `src/lib/navigation.ts`, the dashboard composition in `index.html` and `src/dashboard.ts`, and the shared tokens in `src/styles/app.css`. The cheapest initial implementation check is a static-data shell showing the five target destinations at 390px and 1440px, with Today and Progress separated and all existing routes reachable.

**Evidence limits:** archived screenshots are iPhone captures, not desktop references. Desktop layouts below are explicit adaptations. Progress screenshots are not archived; their research document is transcription evidence only. Settings, injuries, and several failure states lack screenshot references. No claim of full pixel parity is valid for those surfaces until additional references exist.

## 1. Requirements & Constraints

- **REQ-001**: Use Today, Plan, Calendar, Progress, and Support as the five primary destinations on both platforms. Mobile uses a persistent labelled bottom tab bar; desktop uses a labelled left rail. Settings is reached through the profile control. Injuries is reached through Today body signals and Progress, with its existing URL retained.
- **REQ-002**: Make Today workout-first. Move long-term analytics and workout history to Progress without deleting them. Show an urgent pain or safety warning before the workout when applicable.
- **REQ-003**: Match archived mobile references in content order, grouping, alignment, navigation, typography hierarchy, session colours, borders, and control shapes. Do not reproduce the Runna logo, branded coach portraits, marketing artwork, premium claims, or unsupported capabilities.
- **REQ-004**: Replace the violet brand canvas with white and pale neutral surfaces, near-black typography and primary actions, and restrained teal accents. Introduce separate easy/tempo/interval/long/imported session colours. Chart scales remain independent from session and warning colours.
- **REQ-005**: Cover all existing screens, plan creation and management, scheduled workout detail, completed activity review, rescheduling, onboarding, settings, injuries, achievements, integrations, and overlays. Include loading, empty, stale, offline, error, busy, success, and partial-data states.
- **REQ-006**: Preserve current IPC contracts, plan computation, local storage, provider credentials, activity provenance, opt-in assistant behaviour, monotonic publishing, and pace/scheduling safeguards. Add backend work only for a separately identified missing capability.
- **REQ-007**: Treat this request as superseding deliberate divergence 11 in `docs/todo/README.md`: session-type colours now follow Runna rather than a single-hue workout ramp. Do not reinterpret it as permission to add upsells, punitive scores, mandatory AI, or unsafe training behaviour. Record this policy change before implementation.
- **REQ-008**: Preserve light and dark appearance choices. Light is the reference review baseline; dark is an accessible adaptation, not a screenshot-backed Runna claim. Preserve supported palette preferences through an explicit migration to the reference palette plus optional legacy choices; never silently discard persisted settings.
- **REQ-009**: Keep all five primary destinations visible in the new shell. Replace current plan-dependent hiding/redirect behaviour with useful no-plan states for Calendar and Progress, while retaining command eligibility checks. A disconnected provider is a connection state, not a reason to conceal activity history.
- **SEC-001**: Do not include real health data, routes, names, credentials, or provider tokens in committed visual fixtures. Preserve confirmation for destructive actions and existing secure credential storage.
- **CON-001**: Stay with TypeScript, Vite, Tauri, existing Tabler icons, MapLibre, and existing motion/overlay/feedback utilities. Do not introduce a framework rewrite as part of the redesign.
- **CON-002**: Reuse existing page entry points and deep links. Keep DOM IDs used by controllers until each consumer has been migrated and tested. Do not rename `achievements.html` merely to obtain the Progress label.
- **CON-003**: Do not use screenshots as production backgrounds or invent a notification inbox, GPS recorder, race catalogue, coach chat, or manual activity persistence. Missing commands require an explicit functional backlog item; unavailable actions are omitted rather than displayed as working controls.
- **GUD-001**: Use matching copy structure without copying proprietary long-form coaching or marketing text. Keep visible instructions where they explain real training or safety decisions; remove decorative introductory descriptions from the shell.
- **PAT-001**: Deliver one shared component vocabulary and responsive layout system, not separate mobile/desktop implementations. Isolate page styles from primitives; remove superseded CSS instead of appending another override layer.

### Reference Register

All paths in this table are relative to `docs/runna-ux/`. These are evidence sources, not reusable production assets.

| Surface | Reference | Evidence level | Required design read |
| --- | --- | --- | --- |
| Today with plan | `plan-generated/05-today-tab-day1.png` | Archived image | Header, date strip, greeting, workout priority, bottom tabs |
| Today without plan | `screens/01-today.png` | Archived image | Empty-plan invitation, completed activity, provenance, available actions |
| Plan hub | `plan-generated/06-plan-tab-top.png`, `07-plan-tab-scrolled.png`, `08-race-times-expanded.png`, `09-plan-tab-lower.png` | Archived images | Plan identity, projections, expandable information, management entry |
| Week browsing | `plan-generated/10-plan-overview-week1.png`, `13-plan-overview-list.png`, `16-plan-tab-week-cards.png`, `17-plan-tab-week-cards-scrolled.png` | Archived images | Week selection, workout rows, completion and recovery-week treatment |
| Scheduled workout | `plan-generated/11-workout-detail-progressive.png`, `12-workout-detail-progressive-scrolled.png`, `14-workout-detail-longrun.png` | Archived images | Title, summary, prescription blocks, effort and pace hierarchy |
| Calendar | `plan-generated/18-calendar-before-drag.png`, `19-calendar-after-drag1.png`, `20-toast-rescheduled.png`, `22-drag-lift-t54.0s.png`, `23-drag-hover-t54.5s.png` | Archived images | Date/agenda relationship, lift/drop state, reschedule receipt |
| Plan wizard | `plan-wizard/01-plan-tab-empty.png` through `28-plan-receipt.png` | Archived sequence | One decision per step, choices, preferences sheet, loading, receipt |
| Support and failure | `screens/04-support.png`, `05-error-state.png` | Archived images | Support grouping and recoverable failure presentation |
| Completed activity | `screens/02-activity-detail.png` | Archived image | Activity hierarchy; additional review detail comes from research |
| Progress and review | `RUNNA-PROGRESS-AND-REVIEW.md`, `progress/README.md` | Transcription, images missing | Workouts/Performance split, filters, summary, records, laps/splits |
| Management and insights | `RUNNA-PLAN-MANAGEMENT.md`, `RUNNA-INSIGHTS-AND-ADAPTATION.md` | Research | Management grouping, adaptation explanations, pending/abstain states |

### Visual Contract

These are initial implementation values, not purported pixel measurements of Runna. Phase 1 measures the actual screenshots and replaces provisional values where necessary. Convert source-image pixels to logical layout units using the captured viewport scale; do not equate 1179 image pixels with CSS pixels.

| Role | Initial contract |
| --- | --- |
| Light surfaces | Canvas `#ffffff`, secondary `#f5f5f5`, control fill `#eeeeee`, border `#dedede` |
| Text/actions | Main `#111111`, secondary `#595959`, primary black with white text |
| Accent | Teal `#087f78`; decorative teal fills must not become low-contrast text |
| Session categories | Easy `#b9e986`, tempo `#f4b45f`, intervals `#f18a72`, long `#b9a2de`, imported `#b6b6b6`; dark text on these light fills; validate actual composites |
| Typography | Retain vendored Geist for the first comparison; regular body, strong headings, tabular metric numerals. Start at 16px body, 13px metadata, 24px section/page headings, 32px workout titles. Fixed role sizes, no viewport-scaled fonts or negative tracking. Substitute only a licensed font after measured comparison |
| Spacing | 4/8/12/16/24/32px scale, 16px phone gutters, 24px tablet gutters, 32px desktop gutters |
| Shape | 8px repeated workout cards, 8px fields, 16px sheet top corners, circular icon buttons and pill primary commands where references support them; no universal 22px radius |
| Chrome | Mobile 56px app header plus safe area, 64px bottom tabs plus safe area; desktop 216px rail, 64px header. Confirm against image measurements before locking |
| Motion | Brief state transitions and clear drag feedback; no decorative page-wide stagger. Reduced motion removes nonessential rotation and translation |

Do not copy the observed floating-action overlap or sticky upsell behaviour. A real primary workout action must reserve enough content space to leave every workout label readable. A decorative badge must not replace the day's actionable workout or imply a fabricated readiness score.

### Responsive Composition

| Width | Shell | Content | Detail interaction |
| --- | --- | --- | --- |
| 320-599px | Profile/title header, five bottom tabs | One column, 16px gutters; short tab labels fit at 320px | Full-height detail with explicit back/close; compact action sheet for options |
| 600-1023px | Same five bottom tabs, no navigation dropdown | Centred reading column up to 760px; calendar can expand independently | Modal/detail surface up to 640px with keyboard dismissal |
| 1024px and above | 216px rail and page header | Main workspace up to 1440px; Today uses main column plus 320px week/context column; Progress uses grouped chart regions | 420-480px inspector beside Calendar/history when at least 640px of main content remains; otherwise modal |

Desktop is not a blown-up phone: Plan places week navigation beside the selected week; Calendar places the date grid beside the agenda; Progress keeps filter controls in one row and uses available chart width. Preserve the same reading order when columns collapse. At 200% zoom, responsive transitions must remain usable.

### Screen Blueprint

| Destination | Mobile redesign | Desktop adaptation | Existing ownership |
| --- | --- | --- | --- |
| Today | Profile/title header, selectable seven-day strip, contextual greeting, next workout or rest day, completed workout with source, compact body signals, week preview | Workout-first main column; selected week and sync status in context column; avoid a wall of gauges | `index.html`, `src/dashboard.ts` (`renderTodayHero`, `trainingDecision`, `renderWearables`) |
| Plan | Active plan identity; week browser/list; projections and mileage; Manage Plan; no-plan create entry | Week list beside selected week; projections below plan summary; management inspector | `plan.html`, `src/plan.ts` (`estTimesHtml`, `estChartHtml`, `applyKind`), `src/lib/plan-bands.ts` |
| Create plan | Goal, distance/date or block length, ability/baseline, injury constraints, frequency, long-run day, start/preferences, review and receipt; back preserves answers | Centred step content with persistent summary beside it; same step sequence, not a giant form | Existing `#create-plan` form and `src/plan.ts` validation; `src/lib/onboarding.ts`, `src/lib/setup.ts` |
| Calendar | Expandable date region with agenda, purpose-coloured scheduled rows and neutral imported rows; Move to date sheet, Undo, Reset week | Month/week workspace and selected-day agenda; pointer drag remains an enhancement to Move to | `calendar.html`, `src/calendar.ts` (`isMovable`, `eventStateClasses`), `src/lib/schedule.ts` |
| Workout detail | Session title/summary followed by structured warm-up, repeated work, recovery and cooldown; pace limits distinct from targets; actual available management actions | Same detail in inspector; keep selected workout and calendar context visible | `src/lib/workout-detail.ts`, `src/lib/overlay.ts`, `src/lib/publishing.ts` |
| Progress / Workouts | Workouts/Performance segmented control; type/year/month filters; filtered totals; route thumbnails, date and three primary metrics | Filter toolbar, activity list with detail inspector; don't substitute a dense table for the reference cards by default | `achievements.html`, `src/achievements.ts`, activity/history rendering currently in `src/dashboard.ts`, `src/lib/route-map.ts` |
| Progress / Performance | Lifetime summary, weekly totals, fitness/race projections, training load/recovery/risk, records and existing badges in separate sections | Meaningful two-column chart groups; shared range selection where data semantics agree | Existing dashboard chart renderers, `src/achievements.ts`, `src/lib/pace-track.ts` |
| Activity review | Route hero, distance/time/pace, prescription versus execution, verdict near laps, splits and source; no-data states for missing GPS or HR | Wide route and metrics with readable lap/split region; inspector may expand to full detail | `src/lib/run-review.ts`, `src/lib/route-map.ts`, `src/lib/workout-detail.ts` |
| Support | Search only if local content is actually indexed; help categories, connection troubleshooting, app/version diagnostics and real external help links | Constrained help reading pane with category navigation | Proposed `support.html`, `src/support.ts`; reuse existing opener/IPC functions rather than inventing chat |
| Profile / Settings | Group account/connections, appearance/units, notifications where supported, assistant consent, publishing, advanced thresholds, app/update information | Grouped settings navigation and one active form pane; keep advanced controls out of Today | `settings.html`, `src/settings.ts`, `src/lib/theme.ts`, `src/lib/sync.ts`, `src/lib/plan-sync-pill.ts` |
| Injuries / Body | Keep body map and accessible region list, severity/history, save/edit/delete, training effect and warnings | Body selector beside entry/history; no tiny map-only input targets | `injury.html`, `src/injury.ts`, `src/lib/bodymap.ts` |
| Setup / Feedback | Connection, first sync and plan creation remain an explicit recoverable path; shared loading/error/confirmation/toast grammar | Same states and progression; keyboard-complete interaction | `src/lib/setup.ts`, `src/lib/navgate.ts`, `src/lib/feedback.ts`, `src/lib/flow.ts` |

## 2. Implementation Steps

Tasks are sequential within each phase unless marked parallel. Each phase depends on the preceding phase's acceptance gate. No runtime edits are authorized by this planning document alone.

### Implementation Phase 1

- **GOAL-001**: Establish measurable references, policy changes, and complete screen coverage before building.

| Task | Description | Completed | Date |
| --- | --- | --- | --- |
| TASK-001 | Create a reference measurement table beside this plan: image path, viewport scale, crop, spacing, type hierarchy, colour samples, radius and component anatomy. Cover Today, plan hub, week, detail, calendar, wizard and Support. Mark Progress, Settings, injuries and desktop as inferred/adapted. Gate: every blueprint row has an evidence class. | No | |
| TASK-002 | Update the redesign-specific decision in `docs/todo/README.md` and cross-reference `docs/todo/RUNNA-UIUX-PARITY.md`. Supersede the single-hue session rule and old dropdown-navigation target; retain the other safeguards. Re-check old backlog statuses against current source instead of treating 2026-10-27 line numbers as current. Gate: no contradictory active palette/navigation requirements. | No | |
| TASK-003 | Produce annotated mobile 390x844 and desktop 1440x1000 layouts for every blueprint row, plus 320px navigation and 768px layout. Use archived references, not invented Runna screens. If producing Figma layouts, obtain the target file/team, create a file session, search the design system first, and follow the required Figma skills. Gate: layouts and state matrix reviewed before runtime changes. | No | |

### Implementation Phase 2

- **GOAL-002**: Build the shared visual primitives and responsive shell without losing routes or eligibility checks.

| Task | Description | Completed | Date |
| --- | --- | --- | --- |
| TASK-004 | Replace theme, type, spacing, radius, session and elevation roles in `src/styles/app.css`; update `src/lib/theme.ts` and theme-colour metadata in existing page heads. Inventory hard-coded CSS/TS/SVG/MapLibre colours. Gate: light/dark swatch fixture passes contrast and no violet primary canvas remains in reference mode. | No | |
| TASK-005 | Rework `src/lib/navigation.ts` and all six existing page nav blocks to the five-destination shell; add proposed `support.html` and `src/support.ts`, registering the HTML entry in `vite.config.ts`. Map Progress to `achievements.html`; preserve settings/injury deep links and move their navigation to profile/body contexts. Gate: every primary route and secondary entry is reachable by pointer, touch and keyboard. | No | |
| TASK-006 | Refactor `applyNavGate`, `initNavGate` and `requirePlanOrRedirect` in `src/lib/navgate.ts` to separate route visibility from operation eligibility. Update harness states accordingly. Gate: no plan, disconnected, imported-history-only, backend-unavailable and active-plan fixtures all retain a usable shell and truthful actions. | No | |
| TASK-007 | Consolidate session cards, metric rows, date strip, tabs, sheet headers, empty states and toasts using existing libraries; add a focused shared module only where at least two controllers need the same rendering. Gate: shared samples render at 320/390/768/1440px with no overlaps and correct hidden states. | No | |

### Implementation Phase 3

- **GOAL-003**: Separate Today from Progress while preserving every existing analytics capability.

| Task | Description | Completed | Date |
| --- | --- | --- | --- |
| TASK-008 | Recompose `index.html` and `src/dashboard.ts` around `renderTodayHero` and `trainingDecision`: date strip, workout/rest state, completed activity, source, safety and week context. Remove the long analytics stack only after TASK-009 owns it. Gate: useful first viewport for workout, rest, completed, no-plan and safety-warning fixtures. | No | |
| TASK-009 | Expand `achievements.html` and `src/achievements.ts` into Progress with Workouts and Performance. Move history, maps, mileage, load/recovery/risk, fitness and wearable renderers out of the Today initialization path; extract their actual shared data/loading boundary before moving functions. Gate: inventory proves every former dashboard metric is still reachable and no route initializes hidden maps/charts unnecessarily. | No | |
| TASK-010 | Implement purpose/year/month filtering and filtered totals from real activity data. Use actual route geometry for thumbnails, a neutral no-GPS fallback, and distinct plan/import provenance. Preserve existing badges and honest running-only distance semantics. Gate: filters, totals and list agree for empty, mixed-sport and partial-route datasets. | No | |

### Implementation Phase 4

- **GOAL-004**: Match the full plan creation, browsing and management journey.

| Task | Description | Completed | Date |
| --- | --- | --- | --- |
| TASK-011 | Replace the large `#create-plan` presentation with stepwise views in `plan.html` and `src/plan.ts`. Reuse `selectedDistance`, `selectedKind`, `applyKind`, `effectiveTargetDate` and existing validation; preserve values on back and report field errors in the owning step. Gate: race, improvement and maintenance flows create equivalent payloads to the existing form. | No | |
| TASK-012 | Redesign the active-plan view and week browser around plan-generated references. Retain `estTimesHtml`, `estChartHtml` data semantics, sync pill and down-week labels; separate plan identity, week navigation, workouts and projections. Gate: week selection, current week, completed/moved sessions and no-plan state work on both layouts. | No | |
| TASK-013 | Group real management commands into a Manage Plan surface; show current values, pending changes, consequences and receipt. Use `src/lib/plan-sync.ts` and `src/lib/publishing.ts` for real device states. Gate: cancellation does not mutate, save survives refresh, refusal is actionable, publishing remains monotonic. | No | |

### Implementation Phase 5

- **GOAL-005**: Unify Calendar, scheduled details and completed activity review.

| Task | Description | Completed | Date |
| --- | --- | --- | --- |
| TASK-014 | Recompose `calendar.html` and `src/calendar.ts` as mobile date/agenda and desktop grid/agenda, using shared session rows and explicit date selection. Preserve existing Move to, drag, undo and reset handlers. Gate: touch requires no drag, selection survives closing details, guarded dates stay refused. | No | |
| TASK-015 | Redesign `src/lib/workout-detail.ts` prescription hierarchy and actions using progressive/long-run references. Preserve repeated block nesting, effort cues, pace ceilings, limits and planned-versus-actual labels. Gate: easy, interval, progressive, long and cross-training sessions show correct quantities without flattening repetitions. | No | |
| TASK-016 | Redesign `src/lib/run-review.ts` and `src/lib/route-map.ts` for route/metrics, execution, verdict, laps and splits. Keep GPS attribution legible, handle no-route/no-HR/no-lap data, and restore map size on surface changes. Gate: one completed activity opens consistently from Today, Calendar and Progress. | No | |

### Implementation Phase 6

- **GOAL-006**: Bring secondary journeys and all feedback states into the same design language.

| Task | Description | Completed | Date |
| --- | --- | --- | --- |
| TASK-017 | Rework `settings.html` and `src/settings.ts` into grouped profile/settings; preserve provider, units, thresholds, assistant, calendar publishing, update and export controls that currently exist. Gate: saves do not overwrite backend-owned fields or erase result messages during refresh. | No | |
| TASK-018 | Adapt `injury.html`, `src/injury.ts` and `src/lib/bodymap.ts`; provide a 44px-minimum accessible region list alongside anatomical visuals. Gate: severity, entry history, edit/delete and training warnings retain their current meaning and functionality. | No | |
| TASK-019 | Populate Support with actual connection/help/diagnostic destinations and adapt setup/onboarding plus `src/lib/feedback.ts`, `src/lib/overlay.ts` and `src/lib/flow.ts`. Do not ship empty chat/search affordances. Gate: connection failure, first sync, empty plan, offline, retry, confirmation and success states are navigable and truthful. | No | |

### Implementation Phase 7

- **GOAL-007**: Verify visual parity, accessibility, behaviour and native packaging before declaring completion.

| Task | Description | Completed | Date |
| --- | --- | --- | --- |
| TASK-020 | Extend existing harness pages with deterministic privacy-safe fixtures and add repeatable browser assertions for primary navigation, wizard, filters, rescheduling, detail and settings states. Add a browser-test dependency only after checking available tooling; keep tests scoped to behaviour rather than internal markup. Gate: TEST-001 through TEST-006 pass. | No | |
| TASK-021 | Capture every blueprint surface at 390x844 and 1440x1000, with additional 320x740, 768x1024 and 1920x1080 checks. Compare mobile screenshots to normalized reference crops; review desktop against approved adaptations. Gate: zero unexplained major layout differences, overflow, clipped labels or unreachable actions. | No | |
| TASK-022 | Run `npm run build` and backend tests when contracts/commands change; exercise the iOS build and desktop app for safe areas, keyboard, back/close, dragging and provider/publishing flows. Use TEST-007 and TEST-008. Gate: record commands/results and residual evidence gaps, then mark the plan completed only when all implemented surfaces pass. | No | |

## 3. Alternatives

- **ALT-001**: A CSS-only reskin is rejected because it leaves analytics as the home screen, six primary destinations, a mobile dropdown and a large plan form.
- **ALT-002**: Literal duplication of the iPhone layout on desktop is rejected because it wastes the main workspace and obscures calendar/detail context. Desktop uses the same visual grammar and information architecture with explicit adaptations.
- **ALT-003**: A React/SPA rewrite is rejected because it does not improve reference fidelity by itself and adds routing/state risk to a working multipage application.
- **ALT-004**: Keeping the existing monochrome workout palette is rejected for the reference mode because it directly conflicts with the requested Runna appearance. It may remain an optional legacy preference, not the default redesign target.
- **ALT-005**: Adding Runna's entire service offering is outside this visual redesign. Functional gaps are individually assessed rather than represented by nonfunctional controls.

## 4. Dependencies

- **DEP-001**: Archived images and research in `docs/runna-ux/` are the source of truth. Progress image recovery/new captures are required for pixel-level Progress sign-off, but not for a transcription-backed functional layout.
- **DEP-002**: Approved phase-1 layouts and measurements precede shell implementation. Figma is optional for this planning deliverable; target file/team and required skills/session are prerequisites for any later Figma authoring.
- **DEP-003**: Existing IPC methods and backend types determine which commands are real. Inspect `src/lib/ipc.ts`, `src/lib/types.ts` and the corresponding backend owner before adding any capability or changing a payload.
- **DEP-004**: Existing browser harnesses, Vite build and Rust tests support verification. Native iPhone/device access is required for the final keyboard/safe-area/provider smoke test; browser-only checks cannot certify it.

## 5. Files

Paths marked proposed do not exist yet. Other paths are existing ownership anchors, not instructions to edit every file regardless of need.

- **FILE-001**: `src/styles/app.css`, `src/lib/theme.ts`, `src/lib/icons.ts`: tokens, component states, responsive shell, existing icon vocabulary.
- **FILE-002**: `src/lib/navigation.ts`, `src/lib/navgate.ts`, `index.html`, `plan.html`, `calendar.html`, `achievements.html`, `settings.html`, `injury.html`: navigation, page labels and preserved routes.
- **FILE-003**: `src/dashboard.ts`, `src/achievements.ts`: Today/Progress ownership split and existing analytics migration.
- **FILE-004**: `src/plan.ts`, `src/lib/plan-bands.ts`, `src/lib/onboarding.ts`, `src/lib/setup.ts`: plan wizard, week view, setup and receipts.
- **FILE-005**: `src/calendar.ts`, `src/lib/schedule.ts`, `src/lib/workout-detail.ts`, `src/lib/run-review.ts`, `src/lib/route-map.ts`: scheduling and session surfaces.
- **FILE-006**: `src/settings.ts`, `src/injury.ts`, `src/lib/bodymap.ts`, `src/lib/feedback.ts`, `src/lib/overlay.ts`, `src/lib/flow.ts`, `src/lib/sync.ts`, `src/lib/plan-sync-pill.ts`, `src/lib/publishing.ts`: secondary workflows and feedback.
- **FILE-007**: `support.html`, `src/support.ts` (proposed), `vite.config.ts`: real fifth destination and production build entry.
- **FILE-008**: `__harness.html`, `__calendar_harness.html`, `__plan_harness.html`, `__review_stub.js`, `__settings_harness.html`, `__injury_test.html`: extend existing fixtures before adding redundant test pages.
- **FILE-009**: `docs/todo/README.md`, `docs/todo/RUNNA-UIUX-PARITY.md`, `README.md`: reconcile superseded decisions and document new navigation only when implemented.

## 6. Testing

- **TEST-001**: Navigation matrix: all five destinations, profile/settings, injuries, old URLs, back/close and active-state semantics work with keyboard/touch. No-plan, imported-history-only, disconnected and backend-unavailable states never strand the user or conceal existing history.
- **TEST-002**: Visual matrix: all blueprint rows at 390x844 and 1440x1000; shell and long-text cases at 320x740, 768x1024 and 1920x1080. Assert `scrollWidth <= innerWidth`, main content clears fixed tabs/actions, selected dates remain visible and every `[hidden]` surface has computed `display: none`. Dark mode has a separate adaptation review.
- **TEST-003**: Accessibility: WCAG AA text contrast (4.5:1 normal, 3:1 large), 3:1 meaningful controls/graphics, visible focus, labelled icons, 44px minimum actionable targets, colour-independent session labels, reduced motion, 200% zoom, screen-reader tab/segment state and modal focus restoration. Permit actual route graphics to remain noninteractive rather than pretending tiny geometry is a tap target.
- **TEST-004**: Training flows: create all three plan kinds with identical valid input payload semantics; wizard back preserves values; invalid dates/pace/frequency are rejected; plan week changes and nested prescriptions remain correct. Pin displayed absolute distances/pace, not just ratios or round trips.
- **TEST-005**: Scheduling/review: touch Move to, pointer drag, refusal, Undo and Reset week preserve original dates/status and backend guards. Exercise date-only labels under Europe/Berlin and America/Chicago. Completed activity metrics/provenance agree across all entry points; missing route/HR/laps never crash.
- **TEST-006**: Persistence/feedback: units, appearance, provider state, thresholds and assistant consent survive reload; configuration patches preserve backend-owned fields; action receipts survive refresh; busy buttons restore correctly. Test pending/failed publishing and stale/offline data without fabricated success.
- **TEST-007**: Build: `npm run build` must pass after every phase touching runtime files. For backend changes run `cargo test --manifest-path src-tauri/Cargo.toml --lib` plus tests scoped to the changed command. Do not reformat unrelated Rust files or treat historical test counts as current results.
- **TEST-008**: Native smoke: desktop launch/window resize/keyboard and iOS launch/safe areas/onscreen keyboard/back navigation; real provider sync, workout publication and opener links where credentials/devices are available. Record blocked checks explicitly, not as passes.

Visual sign-off is structural and measured: match section order, component anatomy and interaction states; target key alignments/gaps within 4 logical pixels of normalized mobile references after excluding OS chrome and dynamic data. Use screenshot overlays for review, not a blind global pixel-difference threshold across different fonts, personal data and maps. Missing references remain marked unverified even when the implementation looks consistent.

## 7. Risks & Assumptions

- **RISK-001**: A cross-page analytics move can break DOM bindings, duplicate data loads or initialize maps in hidden containers. Migrate one render/data boundary at a time and retain a feature inventory until Progress is verified.
- **RISK-002**: Navigation changes can invalidate gate behaviour and harness assumptions. Update eligibility and fixtures together; do not weaken mutation safeguards merely to display a tab.
- **RISK-003**: Literal Runna colours may fail contrast for small text or in dark mode. Use labels plus accents and independently tested foregrounds; fidelity does not justify inaccessible controls.
- **RISK-004**: Current colour/radius overrides and hard-coded map/SVG colours may outlive token changes. Remove obsolete rules by component rather than stacking global overrides.
- **RISK-005**: Unsupported recording, notifications, chat or race-search affordances can make a visual clone misleading. Distinguish visual work from backend/platform projects and omit unavailable commands.
- **RISK-006**: Full visual parity cannot be certified for missing Progress screenshots or unobserved screens; desktop has no captured Runna target. Keep an evidence-gap register until those references or adapted layouts are approved.
- **ASSUMPTION-001**: "Fully match Runna" means maximum evidenced interface/workflow fidelity, not impersonating Runna or replacing Runalytics' safe training model with unrelated commercial features.
- **ASSUMPTION-002**: Retain all current useful functionality, including injuries, analytical scores, achievements, opt-in assistant and publishing. Moving an entry point is allowed; deleting a capability is not.
- **ASSUMPTION-003**: Plan duration is not estimated until phase 1 validates component scope and functional gaps. A phase is complete only at its acceptance gate, not when a screenshot has been restyled.

## 8. Related Specifications / Further Reading

- [Runna UI/UX research](../docs/runna-ux/RUNNA-UX.md)
- [Plan generation](../docs/runna-ux/RUNNA-PLAN-GENERATION.md)
- [Plan execution](../docs/runna-ux/RUNNA-PLAN-EXECUTION.md)
- [Plan management](../docs/runna-ux/RUNNA-PLAN-MANAGEMENT.md)
- [Progress and activity review](../docs/runna-ux/RUNNA-PROGRESS-AND-REVIEW.md)
- [Insights and adaptation](../docs/runna-ux/RUNNA-INSIGHTS-AND-ADAPTATION.md)
- [Existing parity decisions and status](../docs/todo/README.md)
- [Existing UI/UX backlog](../docs/todo/RUNNA-UIUX-PARITY.md)
- [Existing functional backlog](../docs/todo/RUNNA-FEATURE-PARITY.md)