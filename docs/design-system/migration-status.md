## Quality Day Selection, 2026-11-11

Plan creation offers localized weekday checkboxes for tempo and interval availability.
Days less than two days from the long run are disabled; at least one available day
is required. Training and recovery constraints may produce fewer quality sessions.
Older requests without a selection retain the existing automatic scheduling behavior.

Verification: the production build, catalog check, 30 plan-engine tests, and
10 focused Chromium creation checks passed, as did 35 Prism route and intermediate
viewport checks. The chooser workflow was exercised at
320, 390, 768 and 1440px in light and dark. Browser tests use synthetic IPC;
native device and live watch scheduling were not verified.

## Optional Session Feedback And Plan Difficulty, 2026-11-10

Completed tempo, interval, benchmark and long-run reviews offer an optional,
initially unselected 1-5 relative-difficulty fieldset. Recent newly recorded
completions appear in a nonblocking reminder list; dismissal is persisted.
Easy sessions and ordinary races are excluded by the backend. Review-tab changes
and failed saves retain the rating draft. Training > Plan now includes two
explicit easier/harder actions, visible result feedback and guarded shared Undo.
The controls reuse Prism buttons, native radio semantics, existing icons and
unframed sections. All eight locale catalogs include the new controls and results.

Feedback and manual actions share a revision-guarded transaction and resize future
quality work without changing any session's total distance. Adjusted prescriptions
are versioned snapshots shared by calendar, review and provider rendering.

Verification: production build, TypeScript and catalog validation passed. The
50-check Chromium acceptance run passed the feedback workflows, schedule/benchmark
regressions, Prism width/theme matrix from 320-2560px and intermediate/short
viewports. Feedback-specific screenshots were captured at 320/390/820/1440px in
light and dark. All 37 localization browser checks passed. Eight focused Rust
difficulty tests, a customized sync/restart regression, 18 lap tests and 12 COROS
unit checks passed. The final plan suite passed 98 tests, sync passed 34, and the
18 unaffected calendar checks passed. Browser checks use synthetic IPC, not native
device or live provider integration. Native binaries have not been rebuilt.

Two calendar unit checks still fail on generated-distance/description expectations:
`benchmark_replaces_session_and_eases_adjacent_load` and
`quality_sessions_are_named_and_easy_steps_have_no_upper_bound`. An isolated copy
with the pre-existing training edits and without this feature reproduced the same
failures, so those unrelated expectations were not changed.

## Native Transition Stability And Calendar Morphs, 2026-11-10

Native Tauri appearance updates no longer take document View Transition snapshots.
Native views, including macOS, reuse the immediate stable-host handoff instead
of waiting for outgoing leaf cascades. Dialog top-layer activation, focus and
scroll locks now run before paint. Native overlays use the existing opaque
surface fallback while the stationary scrim retains its backdrop blur.

Calendar session inspectors again expand from their initiating tile with the
shared 560ms Bezier timing. A single native transform animation owns the entrance
and is cancelled on dismissal, teardown or reduced-motion changes. Other
inspector entrances remain unchanged. iPhone navigation retains acknowledged
ownership across theme and route updates, blocks intents until acknowledgement,
and retains workspace spacing while an overlay suspends the native island.
Failed attachment still restores web navigation.

The ownership regression failed before the bridge fix, reporting a zero inset
and fallback during an ordinary update. All 13 bridge unit checks, production
build, editor diagnostics, 33 focused Chromium checks, 33 focused WebKit checks
and all 121 motion acceptance checks passed. Calendar frame checks cover
320/390/820/1440px in light and dark with browser and synthetic Tauri branches;
the acceptance task also covers the Prism 320-2560px width/theme matrix,
keyboard/touch, nested dialogs, reduced motion and workflow regressions.
The 390px light and 1440px dark calendar screenshots were inspected.

These are synthetic-IPC browser checks, not native-device frame-rate proof.
The device-interaction tool rejected the connected physical iPhone and offered
only simulators. Installed macOS/iPhone binaries were not rebuilt or replaced;
they need a new native build containing these assets. Physical flicker acceptance
remains unverified. The referenced foundations and components documents remain
absent from this checkout.

## Viewport Counters And Native Island Colors, 2026-11-10

Shared counters now wait for viewport entry instead of counting while offscreen.
They play once per view visit, rearm on retained-view arrivals and retain
immediate final values for screen readers and reduced motion. The native island
now receives resolved surface, accent and muted-text RGBA tokens, including live
light/dark and palette changes. UIKit applies them to the island background and
normal/selected icons and labels in all three tab layouts, rather than relying
on system tint defaults. Existing attachment guards and web fallback are retained.

Verification: 12 bridge unit checks, TypeScript, 12 focused Chromium checks and
17 focused WebKit checks passed. Browser checks cover counter entry at
390/1440px, reduced motion, retained-view replay, packaged production assets and
light/dark sakura/glacier native color snapshots. The signed iOS package built
successfully, passed signature verification and was installed and launched on
the connected physical iPhone without an uninstall or data-clear operation.
Physical visual acceptance remains unverified because the interaction tools
cannot capture this phone; browser results are synthetic IPC, not native UI proof.

## Packaged Motion Time Units, 2026-11-10

Production CSS minification rewrites `560ms`, `500ms` and `650ms` tokens as
`.56s`, `.5s` and `.65s`. The shared reader previously divided every numeric
value by 1,000, making native packaged entrances, transitions and counters run
1,000 times too fast. It now honors the token's `ms` or `s` suffix. Motion
preferences, choreography and design tokens are unchanged.

The production-asset regression reproduced a 0.56ms entrance and a counter with
no visible progression before the fix. Production build, editor diagnostics,
five focused Chromium checks and the same five WebKit checks now pass at
1440x900 for packaged behavior, with existing 390x844 counter controls and
reduced-motion coverage. Checks observe real 560ms entrance and 500ms exit
durations, intermediate counter/exit frames and clean page-error collection.
The wider motion acceptance task passed 112 of 113 checks; its desktop-exit
frame sampler was interrupted by a page reload. That exact test and both
packaged regressions passed on a sequential rerun (3/3).
These checks use synthetic IPC and do not verify the installed macOS binary.
The installed app must be rebuilt or updated to include this frontend fix.

Physical-device follow-up: a packaged frontend build and development-signed
iOS build succeeded for the connected iPhone 16 on iOS 27.2. Signature
verification passed, and `devicectl` installed and launched
`com.fradone.runalytics` without an uninstall or data-clear operation. The
device-interaction tools rejected the physical phone and exposed only
simulators; on-phone entrance, counter and transition visibility therefore
remain unverified. No simulator result is substituted for that missing check.

## DOM-Ordered Bezier Choreography, 2026-11-10

This refinement supersedes the timing and morph behavior in the earlier motion
entries below. Content, surface and layout travel now share
`cubic-bezier(.22, 1, .36, 1)`, 560ms entrances, 500ms exits, 14px vertical
movement and exact 60ms DOM-order staggering without randomized directions or
simultaneous capped groups. Startup uses one page-wide sequence; returned views
replay only their own leaves. Desktop exits keep an opaque, untransformed host
until the final leaf departs. Compact navigation still updates immediately.

Activity and calendar inspectors enter at full size instead of stretching from
a row or tile. Row presence/layout and disclosure children use the same cadence.
Counters use a 650ms Bezier count-up, with final values always available to screen
readers and reduced motion. Pointer feedback remains immediate. Notifications
and pointer pulses use the same easing. Dialogs capture their initiating button
across the event turn, preserving nested-dialog focus return in WebKit even when
pointer activation leaves a previous textarea focused. Drafts, scroll ownership,
Escape and preference-change cancellation remain intact. No spacing, typography,
palette or static component geometry was redesigned.

Production build, editor diagnostics, whitespace checks, all 113 scoped
synthetic-IPC Chromium checks and all 31 focused WebKit checks passed after the
final refinements. Chromium covered thirteen routes in both themes at 320, 360,
375, 390, 414, 430, 600, 768, 820, 1024, 1280, 1366, 1440, 1600, 1920 and 2560px,
plus four motion-enabled accessibility tours at 390/1440px, keyboard/touch input,
retained filters, pending actions, live preferences, route/row exits and workflows.
WebKit covered exact keyframes and startup order, counters, retained view replay,
activity/calendar panels and nested-dialog focus/draft/scroll behavior. The
390px light Pace insights, 1440px dark Training and 1440px light activity review
screenshots were inspected. The live 528x829 activity review filled the viewport
with no overflow, residual transform or running animation after settling.
An initial shared-server run was interrupted by concurrent preview teardown;
the final suites ran sequentially on an isolated port. Native iOS/macOS provider
integration and physical-device frame-rate acceptance remain unverified.

## Softer View Landings And Activity Reviews, 2026-11-10

Startup and view entrances now use a 520ms zero-bounce landing response,
44ms stagger spacing and smaller scale changes. Retained compact views replay
their own landing without remounting or animating the persistent navigation.
Rapid switching, keyboard focus, touch and reduced-motion cancellation remain
immediate. Activity rows use the same soft staggered fade while keeping layout
projection separate from entrance transforms.

Number counters use a 1100ms cubic-Bezier ease and restart with returned views;
ordinary data rerenders do not restart them. Screen readers and reduced motion
still receive final values immediately. Activity reviews no longer stretch a
short row into a tall panel. They use a near-full-size, 480ms soft panel arrival
and a 220ms exit, retaining focus return and compact scroll ownership. Calendar
session morph behavior is unchanged. Its regression checks now evaluate source
continuity from actual geometric progress rather than first-frame timing.

Production build, editor diagnostics, all 112 scoped synthetic-IPC Chromium
checks and fifteen isolated WebKit checks passed. Chromium coverage includes
the light/dark 320-2560px route matrix, motion-enabled accessibility tours,
keyboard/touch, retained state, view replay, measured slower counters and
activity opening at 390, 820 and 1440px. Desktop light activity review and
320px light Pace insights screenshots were inspected. The live compact review
settled at 390x844 with no overflow or leftover animations. An initial parallel
WebKit run lost its shared preview server; the sequential rerun passed.
Native iOS/macOS and physical-device frame-rate acceptance remain unverified.

## Shared Motion Lifecycle, 2026-11-10

The existing Motion dependency now drives shared token-based presets, a bounded
one-shot unlock entrance, component reveals, directional desktop departures,
scoped selection indicators and activity-row presence/layout transitions.
Compact navigation remains immediate and retains mounted state. Responsive
breakpoint changes preserve the active workspace and open inspector. Focus,
pointer-down, early navigation and live reduced-motion changes settle reveals
without leaving stale transforms. The existing atmospheric background is static
instead of running a perpetual repaint animation. No dependency, spacing,
typography, palette or static component geometry was changed for this work.

Production build, editor diagnostics and whitespace checks passed. All 108
scoped synthetic-IPC Chromium acceptance checks passed. The route matrix covered
13 views in light and dark at 320, 360, 375, 390, 414, 430, 600, 768, 820, 1024,
1280, 1366, 1440, 1600, 1920 and 2560px. Four motion-enabled accessibility tours
covered eleven views at 390 and 1440px in both themes. Separate reduced-motion
keyboard and held-touch checks passed. Other coverage includes startup timing,
one-shot behavior, cancellation, retained exits, rapid compact navigation,
filter retention, morph geometry, notifications and pending-button geometry.
Light 320px Pace insights and dark 1440px Training screenshots were inspected.
The settled 1440px preview had no running animations or horizontal overflow.
All thirteen focused synthetic-IPC WebKit checks also passed, covering startup,
live motion preferences, viewport reveals, activity removals, filtered views,
keyboard behavior and compact/desktop inspector morphs.
Native iOS/macOS integration and physical-device 60 FPS remain unverified.
See `motion.md` and the `Verify iOS-inspired motion` VS Code task.

## Toolbar Tooltip Clearance, 2026-11-10

Toolbars no longer clip control tooltips and are raised while a tooltip-bearing
control is hovered or keyboard-focused. Synthetic-IPC Chromium and WebKit
regressions passed at 1440px for hover, focus and Escape dismissal. The training
preview was inspected at desktop size. Other widths, themes and native-device
behavior remain unverified for this change.

## Entry Counters, 2026-11-10

Calendar summaries, activity overview metrics and health verdict metrics count
up on entry using the existing Motion library. Screen readers receive the final
value and reduced motion displays it immediately. Synthetic-IPC Chromium checks
passed at 390px for all three views with reduced motion and normal motion,
including intermediate health values. Production build and TypeScript passed.
Other widths, themes, screenshots and native device behavior remain unverified.

## Compact Activity Morph, 2026-11-10

Activity reviews now morph from their source row at compact widths when reduced
motion is off. Synthetic-IPC browser checks passed at 390px in Chromium and
WebKit, including dismissal and focus return. Existing WebKit checks also passed
for the 390px calendar session morph, the 1440px desktop activity morph, and
reduced-motion calendar behavior. Native iOS and macOS behavior, other widths,
themes, screenshots and the full Prism matrix remain unverified for this change.

## Native iPhone Chrome Hosting Verification, 2026-11-09

The local iOS Tauri plugin hosts a real UIKit tab bar above the existing
WKWebView without replacing Tauri's root controller or WebView delegates.
Its versioned attach command validates document sessions, revisions, tab IDs,
localized labels and resolved theme. The shared adapter subscribes before
attachment, validates acknowledgements, serializes updates and teardown, and
rejects stale or duplicate intents. Listener cleanup supports both the installed
Swift API's camelCase commands and the JavaScript API's snake-case commands.

Production activation remains off. An explicit development-only URL parameter,
nativeChromeProbe=1, loads a small hosting probe. After a matching attachment
acknowledgement it hides the web island, reserves measured native clearance,
and forwards native selections through the existing React hash navigation.
It suspends tabs while visible web dialogs or inspectors are present. This is
a verification probe, not the final shared modal registry or phone-layout
integration. Native Back/Close and keyboard handling are not implemented.

Real runtime verification passed on iPhone 18 Pro simulator, iOS 27.0, at
402x874 points in portrait, with the real empty local backend rather than
synthetic IPC. Native hierarchy captures and LLDB inspection confirmed one
WKWebView and one UIKit tab bar, identified as runalytics.workspaceTabs.
Training, Activities and Health taps changed React headings and native selection
together. Repeated selection of the current tab remained usable after fixing
the probe to publish a fresh acknowledgement snapshot. Help and Settings
utility taps passed through the transparent native host; Settings had no false
primary-tab selection. Opening Help's recovery/risk information dialog removed
the native tabs, and closing it restored them and subsequent native navigation.

Runtime inspection found that UITabBar.sizeThatFits already included the bottom
safe-area allowance. Removing an additional 34pt reduced its frame from 117pt
to 83pt. The rebuilt simulator app confirmed a frame of (0,791,402,83), an 83pt
bridge inset, and 131px workspace bottom padding including existing 48px content
spacing. Native buttons remained within the viewport. UIKit hit-testing outside
the tab bar returned nil from the native overlay and reached WebView content.
Settings' final item was geometrically reachable above the bar at maximum
scroll; trusted touch scrolling itself was not verified. A visible home
indicator was absent from captures, so visual home-indicator clearance is not
claimed. VoiceOver operation, enlarged native text, keyboard, landscape and
other widths, physical-device behavior and native Back/Close remain unverified.

The simulator build succeeded through the existing generated Xcode project,
using the shell's npm PATH and Tauri's live IDE build coordination service.
The initial development launch displayed Vite HTML at tauri://localhost and
could not resolve its module URLs. Loading the explicit IPv4 development URL,
http://127.0.0.1:1500/?nativeChromeProbe=1, through the debugger corrected that
verification launch. Normal bundled cold-start behavior is not established by
this workaround. GUI-only Xcode builds still need npm available in their
build-script environment. The earlier CoreSimulator version mismatch is
resolved: Xcode MCP can discover and interact with the installed simulators.
The connected physical iPhone was not installed or modified.

Verification gates passed: Swift/Rust device-target plugin compile, a fully
linked arm64 simulator build, 12 protocol/lifecycle unit tests, eight scoped
synthetic-IPC Chromium checks, TypeScript, editor diagnostics and whitespace.
The browser checks cover transport compatibility, probe activation/fallback,
tab reselection, modal suspension/restoration and existing navigation/dialog
regressions; they are distinct from the native simulator evidence above.
The production frontend build passed, and an asset scan confirmed the
development probe is absent from shipped JavaScript. Relevant reusable VS Code
tasks are Build native chrome simulator, Verify native chrome hosting probe,
and Verify native chrome production isolation. Simulator screenshots,
hierarchies and LLDB logs are temporary session artifacts, not release assets.

## Application Localization

The production React interface uses bundled i18next catalogs for English,
Spanish, French, German, Italian, Portuguese, Japanese and Simplified Chinese.
Each language has its own JSON file with 1,514 matching messages. The system's
primary language selects the catalog before rendering; unsupported languages
and missing messages fall back to English. Navigation, workspaces, settings,
help, forms, dialogs, notices, accessibility labels and known first-party
generated training text use the shared runtime. Dates and numeric readouts use
the selected locale without changing ISO inputs or stored values.

User notes, custom names, assistant conversations, external diagnostics and
provider identifiers are preserved. Calendar exports and provider-side
descriptions retain their original language. Catalogs work offline; translation
services are not called at runtime. Initial translations are machine-generated
and still require native-speaker review, especially training and medical terms.

Verification: production build, catalog parity and interpolation checks, editor
diagnostics and whitespace checks passed. All 144 selected synthetic-IPC
Chromium checks passed, including seven translated plan submissions, nineteen
routes in each non-English language, English workflow regressions, locale
fallback, date-only handling west of UTC, translated settings at 320, 390 and
1440px, and the Prism light/dark route matrix from 320 to 2560px. German mobile
and Chinese desktop screenshots were inspected. An earlier French test was
interrupted by a development-server page reload; its isolated rerun and final
suite passed. Native macOS/iPhone locale behavior and linguistic proofreading
were not verified. See `docs/i18n.md` for maintenance and verification commands.

## Mobile Tap Haptics, 2026-11-07

Completed touch clicks on buttons, clickable cards, navigation and choices request
one light native impact on iOS/Android. Mobile browsers use an 8ms vibration when
supported. Feedback is shared by MotionSystem; mouse, keyboard, disabled controls,
cancelled gestures and noninteractive content do not trigger it. Unsupported or
failed haptics do not block actions. Native registration and impact permission
are mobile-only; no extra provider permissions are granted.

Verification: iOS arm64 and macOS cargo checks, the production frontend build,
editor diagnostics and whitespace checks passed. All 15 focused synthetic-IPC
Chromium checks passed, including native requests, browser fallback, unsupported
hardware, button/card taps, touch cancellation, selection, navigation and pending
button geometry. Android compilation and physical-device vibration strength were
not tested. Mobile installation must be rebuilt to include the native plugin;
iOS Safari does not support the browser vibration fallback.

## Reference Button Material Correction, 2026-11-07

Action buttons, icon actions, navigation and review tabs, date controls, pace
controls and setting choices share one reference material. Inactive faces have
a neutral sheen and raised edge; selected faces use a solid muted fill without
an inset bevel. The default palette uses blue selected labels and interaction
outlines; custom palette accents and destructive colors remain distinct.
Speed/Long's transparent-face exception and mobile-only shadow overrides are
removed. Notification actions and compact month dates now use shared button
classes. Touch release restores the selected fill without sticky hover styling.
High-contrast and reduced-transparency fallbacks retain opaque, undecorated faces.

Verification: the production build and 68 scoped synthetic-IPC Chromium checks
passed, covering light/dark route layouts from 320 to 2560px, material families
at 320, 390, 768 and 1440px, pointer outlines, held touch and release, keyboard
activation, pending/result geometry, notifications and Undo. After correcting the
default accent, all 14 reference, pointer, keyboard and touch checks passed again.
Selected, hovered and mobile states were captured for comparison. Existing 320px
enlarged-text roadmap overflow and obsolete motion-test expectations are outside
this verification run. Native macOS/iPhone rendering and physical touch were not
tested.

## Reference Button States, 2026-11-07

Shared action and selection controls use softer raised shadows, filled selected
states without a persistent accent border, and a continuous accent outline on
hover or held press. Selection remains distinct from hover; existing palette,
keyboard focus, disabled, reduced-motion and high-contrast behavior is retained.

Verification: production build, editor diagnostics and whitespace checks passed.
All 17 focused synthetic-IPC Chromium checks passed, including Speed/Long states
at 320, 390 and 1440px in light/dark, continuous outlines at 320 and 1440px,
selection switching, keyboard activation, held-touch feedback and shared control
geometry. Desktop light/dark and mobile dark state screenshots were inspected.
The earlier broader matrix passed 46 of 47 checks; the existing 320px enlarged-text
roadmap overflow remains. A separate 19-check run passed 17, with two existing
motion tests failing because they seek Log pain on Pace insights instead of the
pain journal. Native macOS/iPhone rendering and physical touch were not tested.

## Immediate Compact Navigation, 2026-11-06

At widths up to 800px, shared view changes update their contents in place without
waiting for exits, remounting the transition container or replaying staggered
reveals. A brief opacity-only fade remains nonblocking and is omitted for reduced
motion. Visited compact workspaces retain state through React Activity; hidden
effects are suspended, and resumed routes and data revalidate. Week, Month and
Plan share a bounded calendar request; summaries still use the visible dates.
Desktop view transitions and overlay motion remain on their existing paths.

Verification: production build, TypeScript diagnostics and whitespace checks
passed. The isolated synthetic-IPC Chromium regression run passed 85 of 87
checks, including the route matrix at 320, 360, 375, 390, 414, 430, 600, 768,
820, 1024, 1280, 1366, 1440, 1600, 1920 and 2560px in light/dark. Touch
navigation reached Week within two animation frames at 320, 390 and 768px in
both themes and motion preferences. Coverage also included rapid switching,
mounted filter retention, held-request revalidation, remote plan endings,
calendar totals, Undo and calendar overlay focus. Separate held-touch press and
cancel checks passed. Light/dark 390px screenshots were inspected.

The two regression timeouts expected a Log pain action on Health > Pace insights;
that action is in the pain journal. Earlier shared-port runs were interrupted by
preview-server conflicts; the reported run used an isolated port. Native iPhone
WebView rendering, physical touch latency and provider integration were not tested.

## Setup And Proposal Messages, 2026-11-06

Setup and proposal messages use shared neutral parent material, the palette
aura, a full subtle border, 8px corners and consistent token spacing. Message
text wraps without shrinking actions. Six synthetic-IPC Chromium setup checks
passed at 320, 390 and 1440px in light and dark, including high-contrast fallback.
The production build passed and the 390px light screenshot was inspected.
Native-device rendering and proposal workflows were not tested.

## Mobile Parent Auras, 2026-11-06

Mobile training controls share an 8px parent inset across both rows rather than
padding only the horizon row. Six synthetic-IPC Chromium checks passed at 320,
390 and 768px in light and dark, asserting every button stays inside that inset.
The production build passed and the 390px light screenshot was inspected.

Grouped toolbar, choice and pace-mode parents clip overflow to their rounded
boundaries at every width. Inner health tabs remain horizontally scrollable;
six synthetic-IPC Chromium checks passed at 320, 390 and 768px in light and
dark, including keyboard access to the final tab.

Compact training and health toolbar parents retain the shared material aura,
matching the activity filter toolbar. High-contrast and reduced-transparency
fallbacks remain unchanged. Six synthetic-IPC Chromium mobile surface checks
passed at 320, 390 and 768px in light and dark, including parent aura presence
and the health toolbar's high-contrast fallback. The 390px light training
screenshot was inspected. Native iPhone rendering and physical touch were not
tested.

## Map Startup And Attribution, 2026-11-06

Raster paint and the themed background are included in the initial MapLibre
style, avoiding a default-color frame before load. Native attribution starts
collapsed once its credits populate; its toggle and contributor link remain
available. The production build and two focused synthetic-IPC Chromium checks
passed at 390 and 1440px, including initial paint, attribution toggling and all
six palettes in light/dark. Light desktop and dark mobile screenshots were
inspected. A later rerun failed on a concurrently added attribution-icon CSS
assertion; the broader map regression run was not confirmed. Native macOS/iOS
and offline archive loading were not tested.

## Map Attribution Icon, 2026-11-06

Shared glass background-image rules exclude MapLibre's attribution toggle so
its built-in info icon remains visible at rest, hover, keyboard focus and press.
Two synthetic-IPC Chromium map checks passed at 390 and 1440px, including
attribution expansion/collapse and light/dark map checks across all six palettes.
Native-device rendering and physical touch were not tested.

## Desktop Activity Morph, 2026-11-06

Clicking an activity uses the shared inspector source morph into the right-hand
review pane; Back and Escape reverse it and restore row focus. The desktop row
remains visible, and the exiting inspector retains its grid cell. Compact
activity inspectors keep their existing behavior; reduced motion omits the morph.
The production build and five synthetic-IPC Chromium activity checks passed at
820 and 1440px in light and dark, including reduced motion and keyboard return.
A 1440px light screenshot was inspected. The broader route/calendar morph run
passed 42 of 43 checks; the 320px light calendar closing-width assertion failed
in that run and passed on an isolated rerun.
Native macOS rendering was not tested.

## Mobile Activity Cards, 2026-11-06

Activity rows on compact screens use the same card material, radius and spacing
as plan weeks. The production build and six synthetic-IPC Chromium checks passed
at 320, 390 and 768px in light and dark themes. A 320px light screenshot was
inspected. The broader route matrix did not return a final result; native iPhone
rendering and physical touch behavior were not tested.

## Edge-to-Edge Mobile Shell, 2026-11-06

The compact workspace and header extend to the viewport edges, and the
navigation island sits 4px above the bottom when no safe-area inset is present.
The existing safe-area adjustment remains in place. Fifteen synthetic-IPC
Chromium geometry and workflow checks passed at 320, 390 and 768px, plus
740x390 landscape, in light and dark themes; a 34px bottom inset was simulated.
The broader responsive task passed 31 checks before 15 connection failures
when its preview server stopped; the full matrix remains unverified.
Native iPhone rendering and physical home-indicator interaction were not tested.

## Grouped Control Aura, 2026-11-06

Grouped control surfaces use the notification's shared material aura. Compact
training and health toolbars remain transparent, and accessibility fallbacks
remain opaque. The production build and focused synthetic-IPC Chromium aura
checks passed in light and dark; the light screenshot was inspected. The route
matrix passed in the initial run, but a repeat lost its preview server connection.
Native rendering was not tested.

## Mobile Control Depth, 2026-11-06

Shared mobile action and selection controls use a stronger raised bevel while
retaining their existing pressed, disabled and high-contrast treatments.
The production build passed. Synthetic-IPC Chromium checks covered the raised
controls at 320, 390 and 768px in light and dark, plus navigation, touch and
route layouts. A 390px training screenshot was inspected. The existing 320px
200% text roadmap-row overflow still fails; native iPhone rendering was not tested.

## Full-Width Mobile Header, 2026-11-06

The compact header background extends to both viewport edges while the white
workspace retains its 8px inset and 48px corners. Six synthetic-IPC Chromium
checks passed at 320, 390 and 768px in light and dark themes; a 390px training
screenshot was inspected. Native iPhone rendering was not tested.

## Mobile Training Controls, 2026-11-06

At widths up to 480px, the training horizon uses three equal-width controls
above a full-width date-navigation row. The production build and synthetic-IPC
Chromium control and navigation checks passed at 320 and 390px. A 320px week
screenshot was inspected. The broader route matrix did not return a final result;
native iPhone rendering and physical touch behavior were not tested. An existing
today-first check also failed on its unrelated 4px bottom-navigation inset
expectation, which currently measures 16px.

## Mobile Shell Top Inset, 2026-11-06

The compact shell top inset is 16px; its bottom inset, 48px corners and island
position are unchanged. Header blur still reaches the viewport top. Production
build, four synthetic-IPC header and geometry checks, and 37 responsive route,
short viewport and navigation checks passed. A 390px screenshot was inspected;
native iPhone rendering and physical display clearance were not verified.

## Mobile Shell Safe Areas, 2026-11-05

The compact workspace and topbar share a 48px corner radius. The topbar glass
extends to the viewport edge and the navigation island sits lower while retaining
some clearance for the bottom safe area. Reduced-transparency fallback stays opaque.
The production build passed. Synthetic-IPC Chromium checks passed for 41 of 42
selected cases, including route layouts at 320, 360, 375, 390, 414, 430, 600, 768,
820, 1024, 1280, 1366, 1440, 1600, 1920 and 2560px in light/dark, plus compact
navigation and topbar behavior. A 34px bottom inset was simulated; native iPhone
rendering and home-indicator interaction were not tested. The 200% text check at
320px failed on plan roadmap row overflow; 820px and 1440px passed.

## Notification Alignment, 2026-11-05

Compact notifications keep their message and dismiss control vertically centered,
including multiline progress messages. Synthetic-IPC browser geometry and
dismissal checks passed at 320, 390, 768 and 1440px. Native rendering was not tested.

## Appearance Transitions, 2026-11-05

Theme and palette changes cross-fade the rendered view when the browser supports
view transitions. Reduced motion and unsupported browsers apply changes immediately;
initial appearance remains pre-painted. Rapid updates supersede older transitions.

Verification: TypeScript, production build and two synthetic-IPC Chromium appearance
checks passed at 390px, covering light, dark, palette and reduced motion. The route
matrix did not pass: 31 cases reported a missing app heading with blank screenshots;
the broader layout matrix and native macOS/iOS rendering remain unverified.

## Pointer-Lit Glass Button Borders, 2026-11-05

Shared action buttons, navigation, tabs and option controls now have a masked
glass rim: a white reflection follows the existing pointer coordinates while
a palette-tinted reflection catches the opposite edge. Hover and keyboard
focus fade the rim in; press strengthens it at the interaction origin. The
mask leaves labels and fills untouched and does not change control geometry.
Shared pointer-down/up/cancel handling supplies immediate held-touch feedback.
Reduced motion retains non-spatial feedback; contrast, reduced transparency
and forced colors omit the reflective layer. Disabled controls omit it too.

Verification: production build and 50 distinct synthetic-IPC Chromium checks
passed. Border, keyboard, disabled and contrast states passed at 320 and 1440px
in light/dark; held touch and release passed at 390px. Shared controls, button
pending/result geometry and the route matrix passed at 320, 360, 375, 390, 414,
430, 600, 768, 820, 1024, 1280, 1366, 1440, 1600, 1920 and 2560px. Desktop,
compact and touch screenshots were inspected. Native macOS/iOS rendering was
not tested. Existing card fills and tilt strengths are unchanged.

## Long Card Depth, 2026-11-05

Plan weeks, activity rows and settings cards share the shallower 3 tilt
strength and 1200px perspective, while calendar session tiles retain strength
20 and 650px perspective. Pointer, press and reduced-motion behavior are
otherwise unchanged. This supersedes the matching-depth decision below.

Verification: build and 43 synthetic-IPC browser checks passed, including
light/dark pointer and press behavior, reduced motion, touch, card clearance,
button workflows and the route matrix at 320, 360, 375, 390, 414, 430, 600,
768, 820, 1024, 1280, 1366, 1440, 1600, 1920 and 2560px. Native rendering
was not tested.

## Training Context Control Clearance, 2026-11-05

The activity review's context range control is inset from the inspector edge
to preserve its focus outline. The production build passed; browser geometry
and focus were checked at 320, 390 and 1440px. The broader route matrix did not
produce a final result. Native rendering was not tested.

## Plan Card Edge Clearance, 2026-11-05

The plan-week scroller reserves additional horizontal space for card tilt and
shadows without changing vertical scrolling or the cards' interaction physics.
The build and 39 synthetic-IPC browser checks passed, including pointer probes
at both card edges at 320 and 1440px in light and dark themes, screenshots, and
the route matrix at 320, 360, 375, 390, 414, 430, 600, 768, 820, 1024, 1280,
1366, 1440, 1600, 1920 and 2560px. Native rendering was not tested.

## Calendar Tile-to-Modal Morph, 2026-11-05

Week and month session items now expand from the activated tile's position
and bounds into the shared session Inspector, with its contents fading in.
Dismissal reverses the same path; the original tile remains hidden until
the Inspector unmounts. Compact itineraries use the actual activated tile,
not the hidden desktop calendar copy. Shared spring and exit tokens govern
the transition. Reduced motion retains the existing non-spatial behavior.
Focus return, Escape, background inertness, scroll locking and session actions
remain intact; other Inspectors retain their existing entrance motion.

Verification: production build, TypeScript, whitespace checks and 59 distinct
synthetic-IPC browser checks passed across focused and regression runs. Morph
geometry, reverse motion and keyboard reopening passed at 320, 390, 820 and
1440px in light/dark, including interrupted dismissal and reduced motion.
Month-view touch activation passed at 390 and 1440px. Existing session editing,
Undo, activity matching, stale drafts and drag-to-reschedule passed, together
with Prism's width/theme route matrix at 320, 360, 375, 390, 414, 430, 600, 768,
820, 1024, 1280, 1366, 1440, 1600, 1920 and 2560px. Desktop and compact
screenshots were inspected. Compact overlay tests now select a scheduled day
instead of assuming today has a session. Native macOS/iOS rendering remains
unverified. The referenced foundations, components and motion documents were
absent from this checkout; current shared primitives and tokens were used.

## Matching Activity and Settings Motion, 2026-11-05

Activity and settings cards now use the calendar session tiles' 20 tilt
strength and 650px perspective for pointer hover and press. Plan roadmap rows
retain their restrained motion. Reduced-motion and touch behavior are unchanged.
This supersedes the activity and settings row motion described below.

Verification: build and 41 synthetic-IPC browser checks passed, including
light/dark pointer and press behavior, reduced motion, touch, button workflows,
and route layouts at 320, 360, 375, 390, 414, 430, 600, 768, 820, 1024,
1280, 1366, 1440, 1600, 1920 and 2560px. Native rendering was not tested.

## Consistent View Controls and Restrained Rows, 2026-11-05

View tabs, navigation, pace controls, date/lap controls and option labels now
share action-button sheen, resting edge depth and interaction-only blur.
Selected/current tint and selection indicators remain intact. Wide settings,
activity and roadmap rows use a tilt strength of 2 instead of 20 and 1200px
perspective, bounding hover rotation to 1.2 degrees per axis. Training tiles
retain their existing behavior; reduced motion and opaque fallbacks remain.

Verification: build, whitespace checks and 46 synthetic-IPC browser tests passed.
Coverage includes both themes at 320, 360, 375, 390, 414, 430, 600, 768, 820,
1024, 1280, 1366, 1440, 1600, 1920 and 2560px; cross-view material consistency,
tab navigation, wide-row press/release, keyboard focus, contrast, reduced motion,
touch and pending/success/error geometry. Toolbar and settings screenshots were
inspected. Native macOS/iOS rendering remains unverified; the enlarged-text and
ambient-animation limitations recorded below were not changed by this work.

## Action Button Glass Refinement

Shared action buttons retain a translucent resting fill, restrained sheen and
inset edge depth. Hover, keyboard focus and press activate 12px backdrop blur;
press increases fill density. Existing pointer aura, tilt, control dimensions
and pending/result behavior are preserved. Reduced motion removes movement;
contrast and reduced-transparency preferences use opaque, unfiltered surfaces.

Verification: production build passed. The final synthetic-IPC browser run
passed 44 of 45 checks, including light/dark route matrices at 320, 360, 375,
390, 414, 430, 600, 768, 820, 1024, 1280, 1366, 1440, 1600, 1920 and 2560px;
button hover/press material, keyboard activation/focus return, contrast fallback,
touch aura and pending/success/error geometry. Enlarged text passed at 820 and
1440px; 320px reported plan-roadmap row overflow outside this button change.
Desktop and 390px mobile screenshots were inspected in light/dark.

The two existing settled-rendering tests time out because the page's infinite
`atmosphere-aurora` animation prevents their all-animations-quiet guard. A
separate settled desktop probe found one filtered surface (app-bar pseudo-element)
and no resting button blur. Native macOS/iOS rendering remains unverified.

## Watch-Gated Connection Transfer, 2026-11-04

Transfer connections requires a connected COROS watch account. A visible
disclaimer asks runners to sign in with the same account on both devices;
both transfer actions stay disabled while disconnected, loading or unavailable,
with a link to Watch connection. Export, import review and import confirmation
recheck provider status before reading credentials or writing settings.

Expanded encrypted transfers include assistant endpoint/model/API key, COROS
account fingerprint and optional Strava athlete ID. Known account mismatches
block import; missing identity requires explicit acknowledgement. Assistant use
remains disabled after import. These are local account comparisons, not physical
watch identity or live authentication verification.

Verification: TypeScript and 19 synthetic-IPC transfer tests passed, including
disconnect-before-export/import guards, account mismatches and QR presentation
at 320, 390 and 1440px in light/dark. Native watch/Keychain behavior was not tested.

## Centered Cursor Weight, 2026-11-05

Supersedes pointer-anchored depth: the pivot remains at the surface center.
Both tilt axes are reversed so the cursor-facing side recedes and the opposite
side rises, with pressing deepening that same response. Aura, fade and strength
are unchanged. Nine focused browser checks passed, including centered pivot,
direction signs, press depth, list clearance, touch and reduced motion.
Native rendering remains unverified.

## Pointer-Anchored Depth, 2026-11-05

Hover and press transforms pivot at the pointer position rather than the center.
The area near the pointer stays relatively anchored while farther edges move
more. Aura shading, color fade and existing motion/accessibility fallbacks are
unchanged. Nine focused browser checks passed, including corner pivot geometry,
press depth, list clearance, fade, touch and reduced motion. Native rendering
remains unverified.

## Hover Aura Color Transition, 2026-11-05

Shared controls interpolate registered aura strength over the existing 240ms
motion-change timing. The transparent gradient stays mounted so pointer entry
and departure both fade smoothly and can reverse mid-transition. Original card
fills and pointer tracking remain unchanged. Reduced motion applies immediate
color feedback; contrast/transparency preferences still omit the aura.

Verification: explicit fade-in/fade-out, light/dark shared material, touch and
contrast checks passed. Native rendering remains unverified.

## Inverted Pointer Aura, 2026-11-05

Hovered and pressed controls now carry a 9% accent wash away from the pointer,
with a neutral 7% foreground-tone patch at the pointer: darker in light mode,
lighter in dark mode. Glass sheen, original card fills, tilt, keyboard focus
and accessibility fallbacks are unchanged. Eight focused light/dark interaction
checks passed, including card-color preservation, pointer tracking, touch and
contrast fallback. Native rendering remains unverified.

## Shared Glass Button Depth, 2026-11-04

Shared buttons use pointer-position palette aura, brighter glass edges and
layered hover/pressed shadows. Non-card controls tilt up to 2.4 degrees with
800px perspective; action buttons regain translucent glass fills. Content
cards retain their original background colors while aura overlays them.
Reduced motion removes spatial movement and transparency/contrast preferences
retain opaque surfaces. Plan rows share activity-style surfaces; compact
date/status typography preserves their 320px footprint.

Verification: production build, whitespace validation, eight focused light/dark
interaction checks and eight plan workflow checks passed. These cover original
card fills, pointer/press feedback, touch, reduced motion and plan layouts at
320, 390, 768 and 1440px. Native rendering remains unverified.

## Stronger Hover and Press Depth, 2026-11-04

Card-only tilt now reaches 6 degrees on hover and 8.1 degrees while pressed,
with 900px perspective and stronger contact shading. Press rotation now consumes
the same card-strength multiplier as hover instead of dropping to a weaker
fixed angle. Ordinary controls retain restrained feedback. Eight focused
light/dark checks passed, including measured press depth greater than hover,
Training cards, activity-list clearance, touch and reduced-motion behavior.

## Visible Training Card Tilt, 2026-11-04

Live Training inspection confirmed the transform worked but the previous 1.2
degree cap was too subtle on narrow session tiles. Card-only hover tilt now
reaches 3.6 degrees per axis; ordinary controls remain unchanged. Explicit
Training regressions verify pointer tracking, a perceptible angle and return
to flat in both themes. Eight focused browser checks passed, including existing
activity-list clearance, reduced-motion and touch checks.

## Reference-Inspired Card Tilt, 2026-11-04

Activity, session, roadmap and settings-entry surfaces use a whole-card,
pointer-directed tilt inspired by DaisyUI Hover 3D, bounded to 1.2 degrees per
axis. Transform transitions now ease pointer movement and return to flat rather
than snapping between orientations. Ordinary buttons retain their quieter
response. Existing tint, local shading, scroll clearance and reduced-motion
fallbacks are preserved; no hover-zone overlays or dependency were added.

Verification: six focused light/dark pointer, press, list-edge clearance, touch
and reduced-motion browser checks passed. Native rendering remains unverified.

## Subtle Tinted Paper Feedback, 2026-11-04

Controls retain a quiet palette tint at rest. Pointer lighting now shifts the
local tone by only 4% of the theme foreground rather than adding a saturated
accent glow. Directional hover dips the pointer-facing edge by at most 0.6
degrees; pressing deepens that angle to at most 0.9 degrees. Softer contact
shadows and a slight hover contraction keep the response restrained. Activity
and roadmap scroll containers reserve 8px internal clearance for transformed
rows while retaining contained scrolling and edge fades.

Verification: TypeScript, production build and focused light/dark material,
touch and directional-geometry checks passed. Geometry assertions cover inward
tilt, bounded angles, distinct press deformation and at least 4px clearance
inside the activity list. Native/device rendering remains unverified.

## Pointer-Responsive Aura and Depth, 2026-11-04

Shared controls and interactive rows position their radial aura at the pointer
or press origin. Fine-pointer hover adds bounded directional tilt (up to 2.5
degrees per axis); presses briefly expand the light before settling. Pointer
updates are frame-coalesced without React renders or idle animation loops.
Keyboard focus centers the light. Leaving, cancellation, window blur and
unmount clean up tracking or pulses. Reduced motion disables tilt and press
animation; contrast/transparency fallbacks omit decorative light.

Verification: production build, TypeScript, whitespace checks and 18 selected
synthetic-IPC browser checks passed. Coverage includes pointer-origin tracking,
directional tilt, finite press pulses, return to flat, reduced motion, real
browser touch taps, keyboard/hover clearance and navigation, plus light/dark
route checks at 320, 820 and 1440px. Native rendering and physical-device feel
were not exercised. Existing localhost preview is retained.

## Flat Surfaces and Engaged Aura, 2026-11-04

Shared navigation, buttons, selectors and interactive rows now rest without
sheen or elevation. Floating toolbars use an opaque flat surface instead of
permanent aura and blur. Pointer hover and keyboard focus introduce the same
palette-driven aura and shallow contact shadow; pressing compresses the shadow.
Perspective tilt and hover enlargement are removed. Selection retains its tint
and border independently of hover. Shell/overlay glass and semantic colors are
preserved. Reduced motion retains feedback without movement; increased contrast,
forced colors and reduced transparency disable decorative aura and elevation.

Verification: production build and 70 selected synthetic-IPC browser checks
passed. The route matrix covers 320, 360, 375, 390, 414, 430, 600, 768, 820,
1024, 1280, 1366, 1440, 1600, 1920 and 2560px in light/dark, with additional
short/landscape and enlarged-text checks. Coverage includes plan/health workflows,
navigation input, hover clearance, resting/hover/press/keyboard-focus materials,
high-contrast fallback and real browser touch taps without sticky hover.
Desktop 1440px and mobile 390px captures were recorded. The existing localhost
preview listens on IPv6, so the matrix used a temporary localhost test config;
repository test infrastructure was unchanged. Native rendering, physical touch
devices and the complete UI suite were not exercised.

## Standalone Connection Transfer, 2026-11-04

QR generation/import moved out of Calendar apps and Device sync into the
searchable Transfer connections settings destination. Encrypted exports now
include saved Strava application Client ID/Secret, explicitly excluding OAuth
tokens and athlete sessions. Older version-1 QR images remain compatible and
leave Strava unchanged when that optional field is absent. Import review shows
actual calendar URLs, username, credential presence and Strava Client ID, masks
all secrets and states replacement/removal effects before any save.

Verification: native Rust check, production build and 30 synthetic-IPC
transfer/calendar/device tests passed, including standalone routing, legacy
payload compatibility, malformed Strava rejection and explicit import review.
Existing QR color/decoding/spacing checks cover 320, 390 and 1440px light/dark.
Real Keychain export/import, Strava authorization and physical camera were not
exercised. A rebuilt native app is required for the expanded export command.

## Connection QR Presentation, 2026-11-04

Transfer dialogs list included settings as three bullets and separate excluded
account/history data from the privacy notice. Token-based gaps separate the
summary, QR image, download control and instructions. QR pixels use the current
theme's ink and surface colors, including inverted dark codes, and regenerate
on theme/palette changes without changing the encrypted payload.

Verification: TypeScript, 10 transfer workflow checks and six explicit QR
decoding/color/spacing/axe checks passed at 320, 390 and 1440px in light/dark.
Inspected the dark mobile capture. Physical-camera readability remains unverified.

## Encrypted Connection QR Transfer, 2026-11-04

Calendar apps and Device sync share a transfer dialog. QR images contain only
calendar hosting settings/credentials and the GitHub device-sync token, encrypted
locally with AES-256-GCM and PBKDF2-SHA256 (600,000 iterations, random salt/nonce).
Generation requires an explicit action and a 12-character minimum passphrase.
Import supports image upload and camera decoding, validates the decrypted shape
and HTTPS addresses, and requires review before replacing connections. Sync is
paused before credential replacement; both publishing and sync remain paused.
No account sessions, training history, persistence or network upload are added.
Camera streams stop on scan, close or image selection. Native iOS camera usage
metadata is configured; native macOS keeps image upload as the supported path.

Verification: native Rust check, production build, TypeScript and 41 combined
synthetic-IPC connection checks passed, including 10 transfer checks. Tests cover encrypted PNG round-trip, wrong
passphrases, tampering, schema/URL rejection, explicit confirmation, paused
imports, denied camera fallback, successful synthetic capture and track cleanup.
QR dialogs passed axe/overflow at 320, 390 and 1440px in light/dark; inspected
mobile-light and desktop-dark captures. Physical camera and native Keychain
transfer were not exercised. Existing critical MapLibre dependency advisory
remains outside this change; QR libraries introduced no reported advisories.

## Calendar Apps and Device Sync, 2026-11-04

Split calendar subscriptions from editable plan sync into separate settings
destinations. Calendar hosting, link copying and local file access are disclosed
under options; subscriptions remain read-only and calendar clients control fetch
timing. Device sync has independent token setup and state-specific actions.
Different-plan replacement still requires confirmation. Existing publishing
routes, saved credentials, automatic sync and failed drafts are preserved.
Publishing retries inspect backend error status before reporting success.

Verification: production build and 31 focused synthetic-IPC browser checks
passed. Connection layouts and axe checks cover 320, 390, 768, 820 and 1440px
in light/dark. Inspected mobile and desktop captures. The full UI run also
exercises both routes in Prism's width/theme, short-height and enlarged-text
matrix, which passed. The full 426-test run recorded 39 failures outside these
connection workflows, including motion, training, branding and native-window
checks. Their pre-existing status was not independently established.
Native calendar subscription handoff and real hosting uploads are not verified.

## Sync Verdict Icons, 2026-11-04

Completed sync steps use full-opacity foreground checkmarks; failed steps use
X icons. Working steps retain their spinner. Timing and dismissal are unchanged.

Verification: production build and 18 selected synthetic-IPC Strava browser
checks passed, including verdict icons at 320 and 1440px in light and dark.
Inspected the light 320px progress capture. Native rendering and the full UI
suite were not exercised.

## Transparent macOS Title Strip, 2026-11-04

Removed the native-only opaque background band so the title strip shares the
page's atmospheric background. The 32px control clearance and title remain.

Verification: production build, three synthetic-IPC macOS titlebar browser
checks at 1180x780 (light, dark, system), and whitespace checks passed. Inspected
the light-theme screenshot. Native traffic-light rendering still requires
macOS device verification. Mobile styles and behavior are unchanged.

# Project Prism Verification Ledger

## Theme-Tinted Notification Glass, 2026-11-04

Notifications share navigation's 80% surface glass, palette-driven aura and
floating elevation, with normal theme foreground text. Unsupported blur,
reduced transparency, increased contrast and forced colors retain an opaque
surface. Queueing, placement, countdown, dismissal and Undo are unchanged.

Verification: production build, whitespace validation and 11 synthetic-IPC
notification checks passed. Material checks cover light/dark at 390 and 1440px,
palette changes, alpha, blur, foreground and increased-contrast fallback;
workflow checks cover 320, 390, 768 and 1440px, modal dismissal and Undo.
Inspected light-mobile and dark-desktop captures. Native rendering and the
complete UI suite were not exercised.

## Notification Typography, 2026-11-04

Message and progress notifications share the detail text size and control
line height. Multiline progress rows align their icons with the first line,
and dismiss controls use the same desktop alignment across notification types.

Verification: production build and 49 selected synthetic-IPC notification and
responsive checks passed. Coverage includes 320, 390, 768 and 1440px notification
layouts, long status wrapping, light/dark Strava progress, the route matrix,
short viewports and enlarged text. Inspected the dark desktop progress capture.
Native rendering and the full UI suite were not exercised.

## Activity Inspector Hover Clearance, 2026-11-04

The desktop activity inspector reserves top clearance for its Back control's
hover and focus treatments, including when the browser scrolls it into view.
Leading inspector tooltips align with their controls instead of extending left.

Verification: production build and 42 selected synthetic-IPC browser checks
passed, including light/dark activity Back hover and focus at 1440x900, scroll
control clearance, the route/width matrix, short viewports and enlarged text.
Inspected the dark activity inspector capture. Native app rendering and the
full UI suite were not exercised.

## macOS Outer Background Dragging, 2026-11-04

Primary mouse presses on the empty document background, root padding and shell
gap request native window dragging. Shell content, controls and secondary mouse
buttons do not. The handler is macOS-only and uses the scoped Tauri window drag
permission; browser and mobile behavior remain unchanged.

Verification: TypeScript, production build, one focused synthetic-IPC browser
regression at 1440x900 and the native macOS debug app build passed. Actual native
window movement was not exercised. No visual geometry or styling changed.

## macOS Title Contrast and Control Inset, 2026-11-04

Native traffic lights use a 24px left inset. The native title is hidden in
favor of a noninteractive 13px header label at 104px, explicitly black in light
appearance and white in dark appearance. The 32px header clearance is unchanged.

Verification: TypeScript and both focused synthetic-IPC macOS header checks
passed at 1180x780, covering title contrast/position, palette tint and the native
theme command. Inspected the dark header capture. Native traffic-light position,
dragging and fullscreen transitions require a macOS runtime visual check.

## Overlay Controls and Notification Alignment, 2026-11-04

Modal Session inspectors now use the shared icon-only Close control at the
trailing edge, matching information dialogs. Non-modal inspectors retain an
icon-only Back control. Information dialogs initially focus their heading;
forms focus their first available field. Keyboard focus outlines, Escape,
focus return, scroll locking and notification draft retention remain intact.
Desktop notifications follow the centered, width-capped workspace with 24px
clearance from its right and bottom edges, including above native dialogs.
The adjustment empty state is a compact checked "No pending adjustments" row
using the existing detail typography, and is hidden when its request fails.

Verification: production build and whitespace validation passed. Ten focused
synthetic-IPC overlay checks passed in light/dark at 320, 390, 768, 1440 and
2560px. Another 38 responsive route, short-viewport and enlarged-text checks
passed, plus 19 notification, modal, inspector and proposal workflow checks.
Inspected light desktop dialog, dark 2560px notification and dark 390px
inspector captures. Physical devices, native integration, all palettes and the
complete UI suite were not exercised. The referenced foundations, components
and motion documents are absent from this checkout.

## Health Information Note, 2026-11-04

The overview medical disclaimer uses a neutral information icon, soft surface,
8px corners and consistent 14px text instead of the generic amber warning stripe
and compact caption size. The medical guidance is unchanged; other context and
warning notices retain their existing styling.

Verification: production build and eight synthetic-IPC Health workflows passed
in light/dark at 320, 390, 768 and 1440px, including note styling, text wrapping
and scoped axe accessibility checks. Inspected the 320px light and 390px dark
note captures. Physical devices and the complete UI suite were not exercised.

## Compact Island Motion, 2026-11-04

Mobile workspace navigation is content-sized rather than stretched across the
viewport. Selection uses an opt-in shared spring highlight and a subtle icon
lift; existing press feedback, accessible link names and keyboard focus remain.
Reduced motion keeps static selection feedback without spatial animation.
Later workspace adjustments to button width and safe-area clearance are retained.

Verification: production build, whitespace validation and 46 selected browser
checks passed on the current checkout. Coverage includes light/dark route-width
layouts from 320px through 2560px, intermediate/short viewports, enlarged text,
keyboard selection, rapid retargeting, measured spring travel and synthetic
touch cancellation/activation. Inspected 320px dark and 390px light screenshots.
An earlier broader run reported eight training-toolbar alignment failures;
that separate layout was not changed for this request. Browser checks use
synthetic IPC; physical iPhone safe areas and VoiceOver were not verified.

## Bounded Pace History, 2026-11-04

Pace on point now explicitly limits the chart and date selector to the five
newest eligible sessions, ordered chronologically, regardless of outdated status.
The summary and default selection use the same visible set. Outdated sessions
retain their labels and remain excluded from the current-status count.

Verification: four focused synthetic-IPC Pace tests passed. Oversized, unsorted
histories cover Speed and Long at 320px and 1440px in light/dark, including an
empty-rep entry, mixed outdated status, selection and document overflow. Existing
rep geometry, accessibility, activity navigation and empty/error recovery checks
also passed. Physical devices and the full UI suite were not exercised.

## Scrollable Health Tabs and Unified Pace View, 2026-11-04

Compact Health navigation uses a horizontally scrollable flex row with full
labels instead of squeezing four equal-width columns. Health and Training
selectors retain 8px interior clearance for shared hover/focus effects.
The mobile island's buttons are 64px wide (previously 56px), retaining their
44px height and the existing shell clearance.

Pace uses one 20px section rhythm, consistent section headings and metadata,
a responsive chart capped at 240px high, and a horizontal dated-session
selector with shared transparent fades. Selected session metadata and its
review action share a wrapping header above the repetitions table. Removed
stacked margins and redundant section dividers; pace calculations, eligibility,
outdated status, actions and error/empty states are unchanged.

Verification: production build passed. Eight focused light/dark layout checks
passed at 320, 390, 768 and 1440px, including actual Health horizontal scrolling,
keyboard selection, Pace spacing/type and chart dimensions. Two normal-motion
checks verified first/last hover and focus clearance on Training, Health, Pace
session selectors and activity tabs. Ten existing Health/navigation checks
passed, including pointer, keyboard and synthetic touch; three Pace behavioral
checks passed for session/mode selection and empty/error recovery. Twenty shell,
fade and text-audit checks passed, including the 130-combination route matrix.
Inspected 390px dark Pace overview/session and 1440px light Pace captures.
Browser tests use synthetic IPC. Physical devices, all palettes, native scrolling
and the full UI suite were not exercised.

## Consistent Scroll Fades and Plan Details, 2026-11-04

Horizontal tabs, tables, horizon controls and lap scrollers now use transparent
edge masks rather than painted ink-gradient strips, matching the vertical
lists. Fade strength follows scroll position and clears at either end;
high-contrast and forced-color modes remove masks. All tracked scrollers are
resize-observed so newly revealed regions receive current fade state.

Plan details no longer uses native details/summary. Desktop shows the section
inline; compact layouts use an explicit Show details / Hide details button with
aria-expanded, aria-controls, a chevron and the shared Disclosure animation and
focus contract. Wide tables stay inside their local scroller: the disclosure's
grid child now has min-width:0 instead of expanding to its min-content width.

Verification: TypeScript and the production build passed. Twenty focused
synthetic-IPC checks passed: horizontal tab/table fades in both themes,
keyboard tab activation, high-contrast mask removal, plan workflows at 320,
390, 768 and 1440px in light/dark, and the 130-combination text-layout matrix.
Shared filter/health disclosure and adaptive-plan-height regressions passed,
including normal/reduced motion and draft/focus preservation. Inspected the
dark tab fades, 390px dark plan details and 1440px light plan details captures.
Physical devices, all palettes and the complete UI suite were not exercised.

## Contained Mobile Shell and Control Spacing, 2026-11-04

The bottom navigation island sits inside the rounded workspace with at least
8px of bottom clearance, including the existing safe-area allowance. Mobile
header content no longer translates or hides during scrolling; only the
workspace content scrolls, preserving frame and header bounds. Removed the
obsolete header scroll listener and state. Utility and date controls retain
8px gaps even on narrow screens, and mobile action groups use 12px gaps.

Activity filters explicitly retain their flex layout inside Disclosure: the
shared flow-root rule previously disabled field wrapping and gap spacing.
Filters now wrap with 12px separation, including the Reset filters action.

Verification: production build and TypeScript passed. Eight light/dark shell
checks at 320x640, 390x844, 768x844 and 740x390 measured stationary shell,
header and island geometry, filter/action separation and dialog Escape focus
return. Eight navigation/surface checks and eleven plan/header checks passed,
including pointer, keyboard, synthetic touch, normal/reduced motion and
high-contrast fallback. The 130-combination text clipping matrix passed again.
Inspected 390px dark scrolled and 320px light open-filter screenshots. Browser
checks use synthetic IPC; physical iPhone safe areas, keyboard resizing and
native scrolling remain device acceptance items. The full UI suite was not run.

## Text Clipping Audit, 2026-11-04

Removed rounded clipping from the plan and activity scroll boundaries so
flush-aligned headings such as Most recent first remain intact. Individual
row corners and transparent scroll-edge fades are retained. Activity purpose
text now wraps fully instead of being truncated after two lines.

Verification: the production build and six compact surface checks passed.
Ten synthetic-IPC clipping checks covered 13 routes each in light/dark at
320, 390, 768, 820 and 1440px with reduced motion (130 combinations). Checks
cover rendered text bounds, page/control overflow, unrounded activity scroll
boundaries and unclamped descriptions. Intentional screen-reader-only captions
and content reachable through local scrolling are excluded. Lazy activity maps
are awaited before measurement. Inspected the 320px light and 1440px dark
journal screenshots and confirmed the heading geometry in the live preview.
Physical devices, enlarged text, all palettes and every dialog/error state
were not audited; these checks are not native integration tests.

## Flat Mobile Surfaces, 2026-11-04

Compact controls use flat selection fills without glossy sheens or raised
control shadows. The rounded shell remains visible; the compact header and
navigation no longer cast floating shadows. Activity history uses continuous
rows with small selected corners and no final divider. Plan-row horizontal
spacing accommodates the inset shell at 320px without reducing text size.
Desktop materials and shared interaction timing are unchanged.

Verification: six focused synthetic-IPC surface checks passed in light/dark at
320, 390 and 768px, including rounded shell/header alignment, keyboard focus,
row separators and navigation. Health and plan workflows passed in both themes
at 320, 390, 768 and 1440px; the two initial 320px plan failures passed after the
row-spacing correction. Seven scroll/current-week checks passed at 320x640,
390x844, 844x390, 1440x844 and 1440x900 with normal/reduced motion cases.
Inspected the 320px light journal and 390px dark training screenshots. Physical
devices, the full palette matrix and the complete UI suite were not exercised.

## Adaptive Plan Roadmap, 2026-11-04

The plan roadmap can use additional height beside taller plan details on wide
layouts. Focused synthetic-IPC browser checks passed at 1440x844 for overflowing
lists and at 390x844 and 1440x844 for current-week centering with normal and
reduced motion. The full width/theme matrix, screenshots and native devices were
not verified for this change.

## Strava Sync Notifications, 2026-11-03

Configured Strava connections now report working and settled states alongside
the local database, calendar, Gist and COROS stages. The existing notification
shows Strava's queue summary and error details directly, including paused or
disconnected connections. New operations clear previous stage rows, so a removed
Strava configuration cannot leave a stale result in a later sync.

Queue checks refresh the stored summary even when every pending description is
on cooldown. Waiting, permission and review reasons are surfaced with a retry
delay where applicable. Matching safeguards, opt-in, request limits and retry
deadlines are unchanged; pending work does not hold the notification open.

Verification: production build, 13 focused Rust tests and 18 synthetic-IPC
Strava browser checks passed. The browser cases include light/dark 320px and
1440px notification wrapping, axe checks, failures and automatic dismissal.
The shared-toast regression run also encountered an unrelated existing test
that looks for Log pain on Health Overview before emitting any notifications;
that test was left unchanged. Live Keychain/OAuth and actual Strava activity
delivery were not exercised.

## Gist-Authoritative Plan Freshness, 2026-11-03

Plan sync checks run on launch, foreground/focus, reconnect and a one-minute
visible/online timer without requiring COROS activity downloads. Native plan
updates refresh evidence in place. A separate checked-status signal updates
Settings without reloading all workspace queries. Existing controls, dialogs,
tokens and notifications are reused; Settings adds Last checked and Check now.
Open session inspectors resolve refreshed evidence, retain date/matching drafts
and disable actions on removed sessions. Plan-wide dialogs carry the reviewed
plan identity so delayed actions cannot target a replacement plan.

Both devices can publish edits against their acknowledged Gist snapshot. A
changed Gist automatically replaces conflicting local plan changes, including
different plan IDs and equal/lower legacy revisions. SQLite checkpoints are
committed with remote imports/clears; explicit empty snapshots propagate plan
endings. Conditional reads, retry deadlines, no-op upload suppression and
read-back verification bound background traffic and stale overwrites.

### Verification

- 32 focused Rust sync tests passed, including real worker requests against a
  local fake Gist: stale mobile replacement, same-plan revision/clock conflicts,
  failed reads, local edit publication, empty-state propagation, restart recovery,
  conditional 304 reads, rate limits, credential invalidation and concurrent
  read-back adoption. No real credentials or user Gists were accessed.
- Production build and 23 focused browser checks passed with synthetic IPC:
  foreground/polling adoption at 390 and 1440px, reconnect, hidden/offline gating,
  event coalescing, failed-check recovery, imported endings, draft retention,
  inspector invalidation, guarded end-plan/benchmark requests, clear-plan history
  retention and existing native completion/manual-import guards.
- Settings geometry and axe checks passed at 320, 390, 768, 820 and 1440px in
  both light and dark themes; screenshots were captured by these checks.
- The iOS library target compiled with `cargo check --lib --target aarch64-apple-ios`.

### Limitations

Physical iPhone/macOS resume, real Keychain/network recovery and live two-device
convergence remain device acceptance items. Browser checks use synthetic IPC,
not native integration tests. Both apps need updating; older binaries still
perform unsafe unconditional uploads. Gist PATCH has no documented atomic
conditional-write guarantee, so simultaneous writes can still race. Offline
or suspended apps cannot remain continuously current.

The broader Rust run reported 218 passes, two ignored tests and the existing
calendar `quality_sessions_are_named_and_easy_steps_have_no_upper_bound`
formatting failure. The broader 331-case browser run reported 297 passes and
34 failures in motion/material, brand, legacy selectors and plan workflows.
Three plan-workflow failures were subsequently repaired and passed in the
23-case focused run: benchmark assertions now require the reviewed plan ID,
and the end-plan test opens the existing compact Plan details disclosure.
The full browser suite was not rerun after those test updates; unrelated
failures were left unchanged. The referenced foundations, components and
motion documents remain absent from this checkout.

## Local Strava Connection, 2026-11-03

Settings has a dedicated Strava task using the existing directory, form,
Button, Dialog, Presence and notification contracts. Personal API credentials
stay in Keychain. Connection, cancellation, reconnection and the description
automation opt-in are separate actions; failed saves retain drafts. Activation
requires confirmation that copied results and load/injury-risk lines inherit
the Strava activity's visibility. Existing notes are preserved, and local
receipts plus live remote checks prevent sequential duplicate copies.

### Verification

- Production build and TypeScript passed with `npm run build`.
- Eleven focused Rust tests passed, including real-worker requests against a
  local fake HTTP server: description-only writes, existing notes, cross-device
  marker and clipboard adoption, response-loss recovery, delayed/ambiguous
  activities, auth/rate limits, callback state/scopes, activation cutoff,
  calendar-copy parity, plan deletion and pending-text retention.
- Thirteen focused browser checks passed with synthetic IPC: credential save
  and blank-keeps-secret, privacy confirmation/cancellation, pending retry,
  disconnect focus return, failed draft retention, sign-in cancellation and
  light/dark geometry plus axe checks at 320, 390, 768, 820 and 1440px.
- Inspected focused 320px light and 1440px dark Settings screenshots. No shared
  type, palette, material or interaction primitives were changed.
- `cargo check --lib --target aarch64-apple-ios` passed. The unsigned debug
  iPhone build through `npx tauri ios build --debug --no-sign --target aarch64`
  passed and produced an IPA containing the native Strava authentication bridge.
- Callback plist validation and `git diff --check` passed.

### Limitations

Live OAuth, token rotation, Keychain persistence across device restarts and
actual Strava updates were not exercised with a user's API credentials. Native
browser sign-in on physical macOS/iPhone devices remains an acceptance item.
There is no server, closed-app automation or distributed lock; simultaneous
device/third-party description edits can race.

The broader Rust run reported 205 passes, two ignored tests and the existing
calendar `quality_sessions_are_named_and_easy_steps_have_no_upper_bound`
formatting failure. The broader 315-case browser run reported 281 passes and
34 failures in motion/material, brand alignment, training workflow and legacy
selectors; none were the new Strava cases. Those unrelated areas were left
unchanged. Direct Xcode GUI build initially failed because its shell could not
find npm; the configured Tauri CLI build succeeded instead. The local Xcode
installation also reports an outdated CoreSimulator runtime, although the
device-targeted IPA build completed.

## Compact Plan View, 2026-11-03

The plan uses a two-column metric grid on compact screens and shorter week date
labels. Week rows put mileage beside the week and a full-width progress track
below it; the current week has native current-date semantics and opens today's
itinerary. At 390px the measured current-week row decreased from about 108px to
75px. Centering the week no longer scrolls the plan heading out of view.

Race/test scheduling is beside the horizon switch on mobile. Supporting facts,
predictions, device sync and the guarded end-plan action remain available under
the keyboard-operable Plan details disclosure, initially closed on mobile and
open on desktop. At widths of at least 1200px, the details occupy a separate
column beside the roadmap rather than extending the single-column page.

### Verification

- Production build and TypeScript: `npm run build` and `npx tsc --noEmit` passed.
- 69 selected browser checks passed: Prism light/dark route-width matrix from
  320px through 2560px, intermediate/short viewports, the existing visual/axe
  matrix, mobile today-first layout, plan centering, scheduling, benchmark,
  reset, and plan-creation regressions.
- Eight focused plan workflow cases cover 320, 390, 768 and 1440px in both
  themes: compact geometry, 44px toolbar targets, keyboard disclosure, guarded
  cancellation, race/test dialog and navigation from the current week.
- Three enlarged-text checks passed at 200% and 320, 820 and 1440px, now
  including the Plan route.
- Inspected plan screenshots at 320px dark, 390px light and 1440px light.
- `git diff --check` passed.

### Limitations

Browser verification uses synthetic IPC, not native integration tests. Physical
iPhone safe areas, VoiceOver, momentum scrolling and provider operations remain
device acceptance items. The absent foundations, components and motion
documents noted below were not recreated.

## Mobile Training, 2026-11-03

The mobile week view prioritizes the selected day's workout or recovery state.
Today is selected on entry. The week range and date ribbon are compact, horizon
and date controls share a row at normal phone text sizes, and weekly progress,
race/test scheduling, and week reset follow the day's itinerary. Desktop
scheduling and confirmation contracts are retained.

The bottom navigation sits at `max(4px, env(safe-area-inset-bottom))`, with
palette-tinted glass. The mobile topbar hides on downward workspace scrolling
and returns on upward scrolling or workspace navigation. Only its controls and
background layer move; the fixed bottom navigation's ancestor is not transformed.
Reduced motion and opaque/high-contrast fallbacks retain visible navigation.

### Verification

- Production build: `npm run build`.
- Eleven mobile regressions in `tests/prism.spec.mjs`: today-first layout,
  aligned controls and 44px targets at 320, 375, 390, and 768px, light/dark,
  race/reset dialogs, wheel scrolling, fixed navigation, reduced motion, and
  opaque high-contrast topbar visibility.
- Prism route matrix in both themes: 320, 360, 375, 390, 414, 430, 600, 768,
  820, 1024, 1280, 1366, 1440, 1600, 1920, and 2560px.

## Activities Width, 2026-11-06

Desktop Activities reduces its outer inline gutter by 8px on each side, widening
the filter bar, activity list and right-hand detail area symmetrically. Existing
list hover clearance, column proportions and mobile padding are unchanged.

Verification: production build passed; five desktop activity morph checks passed
(820/1440px, light/dark, plus reduced motion); all 32 Prism route matrix checks
passed at the widths above in both themes. Inspected the 1440px dark activity
detail screenshot. An initial matrix attempt lost its preview server; the full
rerun used the persistent preview task. These are synthetic browser checks, not
physical iPhone or native integration verification.
- Existing visual/axe matrix: 320, 390, 768, and 1440px, light/dark.
- Enlarged text at 200%: 320, 820, and 1440px. Intermediate/short viewports:
  537x740, 913x480, and 667x375.
- Existing benchmark scheduling, reset/Undo, and mobile inspector workflows.
- Inspected screenshots at 320px light and 390px light/dark, including the
  hidden topbar state at 390x640.

### Limitations

These are Chromium browser checks with synthetic IPC, not native integration
tests. Real iPhone safe-area spacing, status-bar behavior, touch momentum,
VoiceOver, and provider operations remain device acceptance items.

The wider suite has unrelated stale checks: three mobile brand-centering
assertions conflict with the current left-aligned brand, and four Health
material tests try to open `Log pain` from Overview instead of Pain journal.
Those checks and their owning presentation were left unchanged.

The referenced foundations, components, and motion documents were absent from
this checkout during this work. This ledger records this change's evidence;
it does not recreate those missing design specifications.