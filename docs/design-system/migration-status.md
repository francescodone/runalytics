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