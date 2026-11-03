# Project Prism Verification Ledger

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