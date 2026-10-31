# Redesign Evidence

Updated 2026-10-30. Workflow references come from the archived iPhone captures
in `docs/runna-ux/`. The current visual direction follows the user's subsequent
request for Apple-inspired typography, shapes, materials and motion on desktop
and mobile. This is a web implementation, not a native Apple UI or a claim of
pixel-exact parity with Runna.

## Product Redesign Audit

The current brief prioritizes a distinctive, Apple-inspired Runalytics experience,
not reference-image parity. Existing working changes are retained. The app is
TypeScript, static HTML and Tauri, not React; no framework migration is needed.

| Surface | Audit finding | Implementation direction |
| --- | --- | --- |
| Shared shell | Four daily destinations and More already reduce mobile navigation noise | Preserve routes, bundled identity, safe areas and keyboard access |
| Shared controls | Disclosure text markers compete with inherited chevrons; repeated CSS declarations obscure ownership | One disclosure grammar, consistent tokens, no decorative generated text |
| Today | Workout-first composition is appropriate; training signals occupy excessive boxed space | Keep the workout dominant and present secondary signals as a quieter group |
| Plan | Mobile week grid consumes space before sessions; plan identity repeats section headings | Compact previous/next week navigation with direct week selection; keep desktop week rail |
| Calendar | Exclusive views hide context on desktop; mobile date cells conceal whether training exists | Keep mobile Agenda/Month; pair date selection and agenda where width permits |
| Progress | History has useful filters but no direct text search; analytics repeat full-width regions | Search/filter together; organize wide analytics into meaningful columns |
| Settings | Seven categories work, but dense advanced forms and vague Save labels need clearer task boundaries | Retain category navigation, improve form grouping and action labels |
| Injuries | Logging is behind a disclosure; empty/history states need a clear next action | Keep accessible region selection and connect journal state to logging |
| Support and overlays | Real destinations exist; disclosure and detail treatments vary | Shared headers, readable line lengths, consistent disclosure and feedback states |

Implementation order: shared controls, Plan, Calendar, Progress, Today and secondary
screens, then cross-screen refinement. Each runtime increment receives a focused
Playwright check before the next change. Final gates are the production build,
complete UI suite, desktop/tablet/mobile layout checks, light/dark accessibility,
keyboard and error/empty-state checks. Native/provider acceptance remains separate.

The design contract retains platform typography, neutral canvas, blue commands,
teal data scales and labelled session colours. Standard spacing uses the existing
4px-based tokens. Sections remain unframed; repeated items may use an 8px radius;
translucency is limited to navigation and overlays. Motion explains state changes.

## Screen Mapping

| Surface | Source | Implemented adaptation |
| --- | --- | --- |
| Today | `plan-generated/05-today-tab-day1.png`, `screens/01-today.png` | One daily session/rest/create action, a current-plan link and three flat training summaries; no duplicate week calendar |
| Primary navigation | Today/Plan/Calendar captures | Four primary destinations plus More below 1024px; 216px desktop sidebar with secondary destinations shown once |
| Plan | `plan-generated/06-plan-tab-top.png`, `10-plan-overview-week1.png`, `16-plan-tab-week-cards.png` | Identity and projections, mobile previous/next controls with native week picker, desktop keyboard week rail, retained selection, workout detail and grouped management |
| Wizard | `plan-wizard/02-goal-personalised.png`, `11-current-race-time.png`, `12-days-per-week.png`, `28-plan-receipt.png` | Five steps using existing goal/target/baseline/schedule/review fields; no fictional race catalogue or upsell ceremony |
| Calendar | `plan-generated/18-calendar-before-drag.png`, `20-toast-rescheduled.png` | Exclusive Agenda/Month below 1280px; month beside agenda on wide screens; mobile scheduled-day indicators; focus-preserving date selection and filter clearing; existing Move to, Undo, reset and desktop drag handlers retained |
| Workout | `plan-generated/11-workout-detail-progressive.png`, `14-workout-detail-longrun.png` | Title, date, distance/duration, original prescription paragraph blocks and management actions; rounded mobile detail with safe-area spacing |
| Activity review | `screens/02-activity-detail.png` plus review transcription | Shared route, stats, laps, verdict and splits from Calendar, Today and Progress; extra wearable/risk details retained |
| Progress | `RUNNA-PROGRESS-AND-REVIEW.md` | Workouts/Performance/Milestones segmented selector; combined text/type/date filters with reset and no-match state; real-route thumbnails; side-by-side wide-screen analytics and expandable recovery/risk/routes |
| Support | `screens/04-support.png` | Real setup, connection, training, recovery and settings destinations plus local troubleshooting; no fictional live chat |
| Profile / injuries | No archived matching screen | Existing settings panes, consent, secure connections and accessible body-region selection retained; explicit save labels; empty journal links directly to logging and failed saves retain the draft |

## Layout And Colour Contract

These values are implementation choices, not fabricated screenshot measurements.

- Light canvas: `#f5f5f7`; panels: white; text: `#1d1d1f`; actions: `#0066cc`.
- Dark canvas: `#161618`; panels: `#242426`; text: `#f5f5f7`; action fill:
  `#0071e3`. Translucency is reserved for navigation and overlay backdrops.
- Easy/tempo labels: `#396c1b` / `#8d5006` on light surfaces. Dark mode uses
  brighter category accents. Session category and chart scales are independent.
- Repeated items: 8px radius; controls: 8px; overlay corners: 24px. Today's
  training summaries are unframed, separated by rules. Activity
  sheets open from the bottom at 760px and below and from the side above it;
  desktop sheets are 480px wide.
- Fixed text roles: 14px body, 13px metadata, 30px desktop / 28px mobile page
  heading, 32px desktop / 28px mobile workout
  heading. Platform fonts use `-apple-system` and `BlinkMacSystemFont` on Apple
  devices with system fallbacks elsewhere; no Apple font files are distributed.
- Mobile gutters: 16px; tablet: 24px; desktop: 32px. Bottom navigation reserves
  64px plus the bottom safe area. Header respects the top safe area.
- System is the default appearance; its stored ID remains `reference` for compatibility.
  All five legacy palette IDs remain supported; persisted preferences are not
  deleted or rewritten to force the new default.
- Press feedback scales controls immediately, without moving navigation icons
  or lifting rows on hover. Shared sheets have one interruptible spring owner
  in `src/lib/overlay.ts`, not a competing CSS entrance animation.
- Reduced motion removes optional animation. Reduced transparency and increased
  contrast use solid navigation surfaces; increased contrast strengthens borders.

## Verification

Run `npm run build` and `npm run test:ui` from the repository root.

The current final gate is the existing **Verify redesign** task. It runs the
production TypeScript/Vite build and 50 Playwright tests. New checks cover shared
disclosure names and keyboard operation, mobile/desktop week selection, Calendar
focus and small-screen date-cell geometry, combined history search/reset, and
injury logging with a failed-save draft. These are synthetic browser checks,
not live-provider or native-device certification.

Final run on 2026-10-30: TypeScript and the production Vite build passed;
**50 Playwright tests passed in 28.4 seconds**. Editor diagnostics reported no
new errors in the edited source/test files. Vite retains its warning for the
1,066.27kB minified workout-detail chunk (286.76kB gzip); this is not a failed
build. Screenshot artifacts are generated for all seven screens at 390px and
1440px in both themes, plus expanded Performance. Layout assertions additionally
check 320px and 768px, and the primary shell at 1920px. Visual approval and manual
assistive-technology acceptance remain separate from these automated results.

The earlier workflow redesign passed a production build and browser checks.
Its unsigned iOS simulator build produced
`src-tauri/gen/apple/build/arm64-sim/Runalytics.app`; this predates the current
Apple-inspired appearance and is not a native verification of the new styles.

The Playwright suite uses existing synthetic harness data, never user health
records. It covers 320/390/768/1440/1920px shell layouts; every production page
in both themes; hidden panels, duplicate IDs and overflow; real history
filtering; race/improvement/maintenance payloads; invalid wizard input and back
navigation; week selection; prescription detail; calendar expansion/date
selection; no-plan access; and saved system/legacy palettes. Appearance tests
also cover platform typography, immediate press feedback, reduced motion,
solid material fallbacks, and sheet geometry at 390/700/1440px.

The automated WCAG AA gate scans all seven production pages in light and dark
mode. It identified and drove fixes for badge foregrounds, light-surface session
labels, dark-mode session colours and connected-status contrast. Automated
checks do not replace manual screen-reader, zoom, keyboard and device testing.

Screenshots are generated into ignored `test-results/` files. Browser preview:
`http://localhost:1500`. Real data and provider operations require the Tauri app.

## Remaining Sign-Off Limits

- No Runna desktop screenshots exist in this reference set.
- Progress reference screenshots remain unavailable; their source is the
  research transcription, not measurable image evidence.
- The 4px reference-overlay target has not been certified. Shared fonts, actual
  personal data, missing references and platform typography prevent a claim of
  pixel-exact parity from the automated regression suite alone.
- Real account sync, device publishing, physical-iPhone keyboard behaviour and
  credential-dependent flows require native acceptance testing; fixtures prove
  presentation and request contracts, not provider availability.
- Native UI automation was attempted, but the Xcode bridge could not register
  the generated workspace and the required device-interaction skill was not
  available locally. The device session was closed; no native interaction pass
  is claimed.
- Dependency audit reports the pre-existing MapLibre sanitizer advisory
  `GHSA-jrc7-96c5-q579`. Its proposed fix is a major-version upgrade and is not
  bundled into this UI redesign.