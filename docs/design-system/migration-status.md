# Project Prism Verification Ledger

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