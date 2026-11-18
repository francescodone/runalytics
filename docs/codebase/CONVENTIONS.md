# Coding and Presentation Conventions

Current-state context, source-checked on 2026-11-18. Project Prism remains the
visual authority. Use the current [tokens](../../src/desk/tokens.css),
[UI primitives](../../src/desk/ui.tsx), [interaction system](../../src/desk/interaction.tsx),
[overlays](../../src/desk/overlays.tsx) and [motion contracts](../design-system/motion.md).
This is a source-backed working reference, not a recreation of missing historical
design specifications.

## 1. Naming Rules

| Item | Observed convention | Evidence |
| --- | --- | --- |
| TS/React files | Lower-case feature/shared-role names, kebab-case for multiple words | [pace-insights.tsx](../../src/desk/pace-insights.tsx), [profile-storage.ts](../../src/desk/profile-storage.ts) |
| Components/types | PascalCase | [WorkspaceProvider / Route](../../src/desk/data.tsx) |
| Hooks/functions/variables | camelCase, `use` prefix for hooks | [useCommand / useRemote / readRoute](../../src/desk/data.tsx) |
| Module constants | Upper snake case where used for fixed limits | [VIRTUAL_ACTIVITY_THRESHOLD](../../src/desk/journal.tsx) |
| Rust modules/functions | snake_case | [calendar_publish.rs](../../src-tauri/src/calendar_publish.rs) |
| CSS | Shared classes, semantic custom properties, state/ARIA attributes | [tokens](../../src/desk/tokens.css), [interaction styles](../../src/desk/interaction.css) |

## 2. Formatting and Linting

[tsconfig.json](../../tsconfig.json) enables strict checking, unused-local and
unused-parameter errors, and switch fallthrough checking. `npm run build` runs
`tsc` before Vite. The root [npm manifest](../../package.json) has no standalone
lint/format script; do not invent a formatter migration during focused edits.
Follow surrounding formatting and use `git diff --check` for whitespace.

### Prism typography, materials and geometry

- Body text and controls use local **Inter**; major headings and branding use
  **Young Serif**, with fallback glyphs and zero tracking. Meaningful emphasis
  remains distinct. [Tokens](../../src/desk/tokens.css), [desk CSS](../../src/desk/desk.css).
- Preserve all six palette IDs and light/dark/system settings. Materials,
  typography, spacing, radii and elevation come from semantic tokens, not new
  page-specific constants. [Tokens](../../src/desk/tokens.css),
  [appearance settings](../../src/desk/settings.tsx).
- Keep separated desktop navigation and continuous evidence content, without
  decorative section cards. Plan weeks, activity history, settings destinations,
  compact itineraries and reset rows use continuous single-divider lists: no
  individual row frame, gaps, shadows, optical rim, shrink or pointer tilt.
  Standalone optical controls and calendar tiles remain distinct families.
  [Interaction CSS](../../src/desk/interaction.css), [desk CSS](../../src/desk/desk.css).
- Shared toolbars use the canvas's atmospheric background, not an extra stacked
  parent tint. Blur is bounded to appropriate shared surfaces/engaged controls;
  use opaque fallbacks for unsupported blur, reduced transparency, increased
  contrast and forced colors. Disabled controls keep readable labels and an
  explicit state. [Desk CSS](../../src/desk/desk.css),
  [interaction CSS](../../src/desk/interaction.css).
- At **800px and below**, use compact date selection/itinerary, bottom navigation,
  edge-to-edge workspace and full-screen inspectors. Keep the header stationary
  rather than restoring historical scroll-hide behavior. Above 800px, retain
  the separate rail. [Desk CSS](../../src/desk/desk.css).
- Compact month dates are flat within the calendar's perimeter/dividers, not
  individual rounded pills. Preserve today/selection fills and inset focus.
  [Interaction CSS](../../src/desk/interaction.css),
  [calendar checks](../../tests/prism.spec.mjs).
- Use `minmax(0, 1fr)`, `min-width: 0`, wrapping and labeled local table scrollers,
  not page-level clipping to hide overflow. Controls must remain reachable at
  320px, short heights, landscape and 200% text. Safe-area padding belongs to
  the viewport edge, including setup. [Desk CSS](../../src/desk/desk.css),
  [responsive instructions](../../.github/instructions/responsive.instructions.md).

### Shared interaction, overlays and motion

- Reuse `Button`, `IconButton`, `Select`, `CheckInput`, `Dialog`, `Inspector`,
  `Presence`, `Disclosure`, `SelectionGroup` and `NotificationViewport`. Keep
  native hosts, labels, refs, form behavior and disabled/checked/current semantics.
  [UI](../../src/desk/ui.tsx), [interactions](../../src/desk/interaction.tsx),
  [overlays](../../src/desk/overlays.tsx).
- Pending/results stay in the original footprint; failed operations retain
  drafts and visible errors. Dialogs own Escape, focus containment, background
  inertness, scroll locks and opener restoration. Focus a field/action, not a
  heading; workspace navigation focuses the workspace. [UI](../../src/desk/ui.tsx),
  [overlays](../../src/desk/overlays.tsx), [shell](../../src/desk/main.tsx).
- Shared motion uses 560ms arrivals, 500ms departures, 650ms counters, 60ms
  stagger and `cubic-bezier(.22, 1, .36, 1)`. CSS token readers must accept both
  `ms` and `s` after minification. Press feedback and commands remain immediate.
  [Motion implementation](../../src/desk/motion.ts), [contract](../design-system/motion.md).
- Desktop reveals batch newly visible units by rendered order, respecting
  clipping and nested scrollports; counters wait for visibility. Compact
  content/counters settle immediately, without disabling other interaction
  feedback. Native Tauri view handoffs use stable hosts without outgoing-cascade
  waits; native appearance updates avoid document View Transition snapshots.
  [Interaction system](../../src/desk/interaction.tsx), [shell](../../src/desk/main.tsx).
- Do not transform/filter fixed-element ancestors, animate every row's blur or
  duplicate layout-transform ownership. Activity review uses the shared full-size
  landing; calendar session inspectors retain the initiating-tile entrance.
  Reduced motion removes spatial travel, not semantic feedback. Ambient drift is
  limited to existing bounded material-aura surfaces and suspends offscreen or
  in hidden documents. [Motion contracts](../design-system/motion.md).

### Icons and native ownership

Production glyphs use [animated-icons.tsx](../../src/desk/animated-icons.tsx)
through [icons.tsx](../../src/desk/icons.tsx): selected local UseAnimations clips
first, then app-authored named-part recipes for unmatched/brand artwork in
[custom-icons.tsx](../../src/desk/custom-icons.tsx). Do not replace static/error/
reduced-motion artwork with unrelated glyphs, import the full catalog, add focus
targets or delay actions for animation.

Replay is **1120ms**; outcomes/plain-X close feedback use **360ms**. Pending alone
loops at source speed with no extra CSS spin. Search and plan-action arrows use
same-source posters when catalog endpoints would leave blank artwork. Shared
eligibility handles visibility, inert/exiting content, accessibility preferences
and cleanup; interrupted one-shots do not replay stale outcomes. Native iPhone
artwork shares the 1120ms replay while UIKit retains tab ownership. QR payloads,
charts, route geometry and swatches remain data, not decorative icon replacements.
[Motion ownership](../design-system/motion.md) also records local MIT attribution
and OS-owned artwork boundaries.

## 3. Import and Module Conventions

Use relative TS imports and `import type` for data contracts, as in
[data.tsx](../../src/desk/data.tsx). Keep glyph imports on the shared facade,
not direct route-level Tabler/library renderers. There is no configured import
ordering linter or path alias in [tsconfig.json](../../tsconfig.json).

## 4. Error and Logging Conventions

[useCommand](../../src/desk/data.tsx) guards duplicate submissions, returns a
success flag and exposes failures for retry. [useRemote](../../src/desk/data.tsx)
ignores results after effect cleanup. Preserve these semantics and profile-change
rejection in [IPC](../../src/lib/ipc.ts). Never log real credentials or include
them in screenshots, test fixtures or context documents; follow
[hosted-services instructions](../../.github/instructions/hosted-services.instructions.md)
for dashboard work. [TODO] A repository-wide required logging-field schema is
not established by these presentation sources.

## 5. Testing Conventions

See [Testing](TESTING.md). Use focused behavior checks, then the existing Prism
matrix for shared UI changes. Do not weaken stale selectors or unrelated
assertions to call a baseline green. Validate keyboard, pointer, touch, reduced
motion and focus restoration; synthetic IPC is not native integration evidence.

## 6. Evidence

- [Tokens](../../src/desk/tokens.css), [desk styles](../../src/desk/desk.css), [interaction styles](../../src/desk/interaction.css).
- [UI](../../src/desk/ui.tsx), [interaction lifecycle](../../src/desk/interaction.tsx), [overlays](../../src/desk/overlays.tsx).
- [Motion contract](../design-system/motion.md), [Prism tests](../../tests/prism.spec.mjs), [TypeScript config](../../tsconfig.json).
