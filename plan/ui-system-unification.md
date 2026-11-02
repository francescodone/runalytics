# Runalytics Glass Design System

Maintained reference for designers and developers working on the production UI.
This replaces the previous redesign plans and UI-parity requirements. Training
algorithms, provider contracts and external product research are not design
authority and are outside this specification.

## Intent

A runner should recognize the same control, response and hierarchy everywhere:
planning a week, reading a run, recording pain, changing a preference or reviewing
a destructive action. Glass separates navigation and temporary surfaces from
training evidence. It does not turn every section into a card.

Light mode uses silver surfaces and cool neutral surroundings. Dark mode uses
graphite surfaces, pale text and restrained highlights. Both share the same
materials, spacing, typography, geometry and interactions. The six accent palettes
(Training Desk, Amethyst, Glacier, Evergreen, Sakura and Ember) change emphasis,
not layout or behavior. System appearance follows the operating system.

## Ownership

| File | Responsibility |
| --- | --- |
| [tokens.css](../src/desk/tokens.css) | Typography, spacing, geometry, themes, materials, shadows, state colors, layers and timing |
| [desk.css](../src/desk/desk.css) | Component appearance, screen composition, responsive layouts and material fallbacks |
| [interaction.css](../src/desk/interaction.css) | Shared hover, press, focus, validity, selection, entrance and preference rules |
| [interaction.tsx](../src/desk/interaction.tsx) | Semantic reveal registration, selection continuity, surface motion, shared timing and scroll-edge cues |
| [ui.tsx](../src/desk/ui.tsx) | Button, IconButton, Dialog, Inspector, Metric, Loading, Empty and ErrorNotice |
| [overlays.tsx](../src/desk/overlays.tsx) | Notification surfaces, viewport placement and shared scroll-lock ownership |
| [main.tsx](../src/desk/main.tsx) | Application shell and live theme/palette synchronization |
| [appearance.js](../public/appearance.js) | Cached appearance before the first React paint |

All production HTML entries are served through the React shell in
[vite.config.ts](../vite.config.ts). Older controllers and named harnesses remain
isolated regression fixtures, not a second production design system. The backend,
route contracts and native window configuration are unchanged by this redesign.

## Foundations

- **Type:** locally bundled Source Sans 3, exposed as `Desk Sans`. No remote font
  request, viewport-scaled type or letter tracking. Weights are 400, 500, 600 and
  700. Tabular figures are used for numeric comparisons.
- **Type roles:** caption 12px, detail 13px, label 14px, body 16px, subtitle 18px,
  title 20px, heading 24px, page 28px and display 32px. The 40px exceptional role
  is not for settings, cards or compact panels.
- **Spacing:** a 4px scale with a 2px optical allowance. Use semantic roles:
  control gap 8px, field gap 16px, section inset 24px, section gap 32px. Compact
  layouts use 16px insets and 24px section gaps.
- **Canvas:** content width is capped at 1600px; gutters are 32px on desktop and
  16px at or below 800px. Sparse screens keep the same alignment as dense ones.
- **Geometry:** controls have a 44px minimum target and 8px corners. Repeated
  items use 8px corners; toolbars use 16px; temporary surfaces use 24px. The
  compact navigation capsule is deliberately pill-shaped.
- **Layers:** use named content, raised, navigation, chrome, inspector,
  notification, tooltip, drag and skip-link tokens. Native dialogs and popovers
  use the browser top layer; arbitrary z-index values cannot replace it.

Authored font sizes, weights and positive stacking values must use tokens.
Chart dimensions, map coordinates, data-driven widths and gesture geometry are
not spacing tokens and may remain calculated values.

## Materials

| Role | Surface contribution | Use |
| --- | --- | --- |
| Navigation | 72% | Application chrome and compact navigation |
| Floating | 78% | Toolbars and contextual controls |
| Content | 60% | Quiet segmented-control grounds |
| Control | 76% | Secondary buttons, session tiles and settings items |
| Field | 84% | Text fields, selects and textareas |
| Dialog | 88% | Dialogs and inspectors |
| Notice | 96% ink | High-contrast action feedback |

The remaining contribution is transparent. Theme-specific edge highlights,
neutral shadows and a restrained sheen describe depth. Blur is 20px with 135%
saturation on floating chrome and overlays. The backdrop uses a 4px blur.
Buttons, repeated items, fields and charts do not each create a blur layer.
Primary actions retain a legible accent fill; danger is a semantic treatment,
not a new interaction style.

Do not nest decorative cards or blur chart evidence. Session kind uses a narrow
colored edge plus a written label. Completion and selection remain distinct.
Maps retain readable attribution and a contrasting route outline across themes.

Opaque surfaces are the baseline. The glass layer is enabled with feature
detection, including the WebKit prefix. Reduced transparency, increased contrast
and forced colors remove translucent fills, sheen and backdrop blur. Forced
colors restore native checkbox rendering. Never place blur or a transform on a
wrapper that must contain viewport-fixed navigation or an inspector; the appbar
uses its own pseudo-element for this reason.

## Interaction Contract

| State | Shared behavior |
| --- | --- |
| Rest | Stable control geometry, readable label and semantic icon |
| Hover | 180ms tint/border/shadow response on fine pointers; no layout movement or label tilt |
| Press | 90ms response at scale 0.98; release or cancellation returns to rest |
| Keyboard focus | 2px visible outline, 3px offset; fields also receive an accent border and focus halo |
| Selected | Persistent fill or marker plus `aria-current`, `aria-selected`, checked or pressed semantics |
| Disabled | Native disabled state, reduced emphasis and no press response |
| Pending | Disabled action and centered progress indicator; preserve the label's layout and accessible name |
| Success/error | In-place result feedback without changing dimensions; visible status or alert carries the message |
| Invalid | Native validation or `aria-invalid`, danger border and halo; retain the entered draft |

Primary, secondary, danger and icon buttons share timing and geometry. Primary
states preserve text contrast instead of inheriting a pale generic hover fill.
Notification actions inherit their inverse surface's own state colors. Do not
introduce per-screen lift, tilt, press scale or alternate focus rules.

Use native labelled controls: segmented radios for a small exclusive mode set,
swatches for palettes, checkboxes/switches for binary preferences, numeric fields
for values and selects for larger option sets. A switch remains a real checkbox.
Use Tabler icons already installed in the app. Icon-only controls need an
accessible name and a hover/focus tooltip that Escape can dismiss.

## Motion Contract

| Role | Duration / behavior |
| --- | --- |
| Press | 90ms |
| Feedback | 180ms |
| Local change / disclosure | 240ms |
| Content arrival | 320ms, opacity plus 6px vertical travel |
| Exit | 160ms |
| Selection / surface spring | 320ms visual duration, zero bounce |
| Stagger | 32ms steps, at most three steps (96ms) |

CSS and Motion consume the same token source. The settling curve is
`cubic-bezier(.16, 1, .3, 1)`; exits use `cubic-bezier(.4, 0, .8, .4)`.

Content reveals once per mounted element when it intersects the viewport. A
child inside an arriving section settles without a second entrance. Focus,
hover or a press cancels the entrance so interaction is immediate. Route
wrappers do not retain transforms after arrival. New content surfaces use
`data-entrance` or the semantic registration in `MotionSystem`; do not add
individual component keyframes.

Reduced motion removes spatial animation, stagger and animated countdowns while
retaining focus, selection, feedback and keyboard operation. Changing the
preference during an entrance settles it immediately. Transparency/contrast
preferences separately restrict entrances to opacity.

Spatial overlays are an intentional exception to content travel: compact
dialogs rise from the bottom and inspectors enter from the side. Their gesture
can be interrupted or cancelled. Drag only from the dedicated handle/header;
typing, selecting a value and scrolling a form must not dismiss it. Map route
drawing and chart geometry have domain-specific timing, not decorative reveals.

## Screen And Overlay Coverage

| Area | Included surfaces |
| --- | --- |
| Training | Week, month, plan, session tiles and inspectors, roadmap, race/test scheduling, plan creation, reset preview, matching, move/skip/restore, end-plan confirmation |
| Activities | List, filters, empty/error states, full activity review, routes, execution, laps, splits and training context |
| Health | Overview, history, pace insights, pain journal, conversations, milestones and proposal review |
| Settings | Directory, watch, collapsed diagnostics, appearance, boundaries/reset, training-data deletion, calendar, device sync/import, assistant consent/removal, maps and updates |
| Help | Search, empty results, disclosures and navigation back to settings |
| Shared | Notifications, background progress, Undo, loading, errors, tooltips, selection and scroll-edge cues |

At or below 800px, navigation moves to a safe-area-aware bottom capsule,
training uses a date ribbon and itinerary, and inspectors become full-screen.
Desktop retains the week lanes and contextual review. Forms wrap without
reordering their reading order. Dense tables scroll within labelled regions;
page-level horizontal scrolling is not an acceptable fallback.

Native dialogs retain focus containment and Escape handling. Closing restores
the opener when it still exists. Scroll locks are shared, so closing one surface
cannot unlock another. Notifications stack in one viewport, attach above an
active dialog when needed, preserve queued Undo actions and pause expiry during
hover, focus or an action. Closing a notice must not close its host dialog or
erase a draft.

## Verification

Run the VS Code **Verify UI system unification** task, or:

```sh
npm run build
PLAYWRIGHT_JSON_OUTPUT_NAME=test-results/ui-system-report.json npm run test:ui -- --reporter=dot,json --output=test-results/ui-system
git diff --check
```

[The browser suite](../tests/desk.spec.mjs) covers light/dark layouts at
320/390/768/1440px, all settings editors in both themes, expanded diagnostics,
all six palettes' hover/press/dialog contrast, keyboard focus, touch cancellation,
reduced motion, opaque fallbacks, async control geometry, notifications, original
entry routes and mocked mutation workflows. Map tests sample rendered pixels
against the current accent token rather than assuming a specific green value.
Motion-enabled accessibility checks wait for a sustained quiet period before
sampling colors. Screenshots and traces are ignored outputs, not design sources.

Preview: `http://localhost:1500/__desk_preview.html` with synthetic data.
The preview is not a live COROS connection. Its inherited plan-date fixture can
show an invalid date; this redesign does not change training data calculations.

Browser checks do not certify native macOS/iOS composition, software keyboards,
VoiceOver, hardware frame rate, live OAuth or real offline archives. Those still
require platform acceptance. No Windows, Linux or Android runtime claim follows
from the CSS work.