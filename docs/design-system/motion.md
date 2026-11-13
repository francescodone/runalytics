# Prism Motion

## Shared Presets

`src/desk/motion.ts` is the shared reference for animation targets and timing.
It reads the existing CSS tokens from `tokens.css`; exported times are seconds,
except the notification and action-result hold times, which are milliseconds.
The reader accepts both `ms` and `s`: production CSS minification can rewrite
`560ms` as `.56s`, so parsing must preserve the unit rather than assume milliseconds.
Do not add route-specific physics or duplicate these presets. Content, surfaces,
selection travel and layout projection use `cubic-bezier(.22, 1, .36, 1)`.
Press and direct-manipulation feedback remain immediate and interruptible.

The Apple-style response is restrained and non-overshooting: one continuous
Bezier landing, short vertical travel, immediate input, and no animated blur.
The following semantic tokens are the source of truth. Existing role tokens
such as `--motion-landing`, `--motion-surface`, `--motion-count`, and
`--motion-review` are aliases, not independent timings.

| Token | Value | Purpose |
| --- | --- | --- |
| `--motion-duration-arrive` | 560ms | Content and surface arrivals |
| `--motion-duration-depart` | 500ms | Content and surface departures |
| `--motion-duration-number` | 650ms | Numeric interpolation |
| `--motion-stagger` | 60ms | Adjacent newly visible units |
| `--motion-ease-standard` | `cubic-bezier(.22, 1, .36, 1)` | Shared non-overshooting response |
| `--motion-distance-enter` | 14px | Content entrance travel |
| `--motion-distance-exit` | -14px | Content departure travel |
| `--motion-distance-surface` | 16px | Modal preset travel |
| `--motion-viewport-inset` | 24px | Bottom viewport reveal inset |
| `--motion-viewport-threshold` | .08 | Minimum visible fraction, bounded by the scrollport |

| Export | Purpose |
| --- | --- |
| `springTransition` | Compatibility export for the shared 560ms Bezier tween |
| `fadeIn`, `fadeOut` | 560ms entrance and 500ms departure |
| `scaleIn` | Subtle 0.995-scale target; panels enter at full size |
| `slideUp`, `slideDown` | 14px vertical component entrance targets |
| `staggerContainer`, `staggerItem` | Reusable group and item variants |
| `pageTransition` | Vertical leaf-level view targets |
| `modalTransition` | Shared dialog surface targets |
| `activityTransition` | Full-size 560ms activity-panel arrival and 500ms exit |
| `entranceFor` | Vertical entrance with exact 60ms visible-batch staggering |
| `motionVisible`, `motionOrder` | Clipped viewport eligibility and top-to-bottom visual order |

## Lifecycle

`MotionSystem` provides Motion configuration, one reveal observer and shared
interaction listeners. Startup reveals use Motion's native mini API with a
560ms Bezier landing response. Visible leaves begin 14px below their final
position at full size. Each newly visible sibling advances the cascade by 60ms.
One animation-frame batch collects eligible units across observer callbacks and
sorts them by rendered top edge, then left edge, with DOM order as a tie-breaker.
This also handles responsive grids whose visual order differs from their DOM.
Offscreen, clipped, hidden, inert, settled and already running units do not consume
new stagger slots. Later scroll entries and asynchronously mounted content start
a fresh batch at zero; they do not inherit delays from previously revealed content.
Eligibility intersects the viewport with every clipping or scrolling ancestor.
Tall content uses the scrollport-sized visibility threshold rather than waiting
for an impossible percentage of the entire element. Route changes end startup mode.
There is no animated blur on large surfaces or repeated rows.

Reveals register before paint and wait for viewport entry. Ordinary rerenders
do not replay them. Each view change gets the same fresh, scoped landing.
Headings, metric groups, setup controls, lists, tables, forms and opened disclosures
use the shared registration path. Native details register their contents on opening.
Nested reveal targets animate as one outer visual unit, including its borders
and controls, instead of revealing only their innermost content. Elements
containing dialogs, inspectors or notification viewports are not transformed.
Pointer-down and keyboard focus on controls settle an entrance immediately.
Programmatic heading focus preserves the heading's reveal. Workspace headings
replay when their persistent host changes route; navigation chrome does not.
Preference
changes cancel native playback and restore the original inline styles.
Observers, listeners and detached-element animations are cleaned up on removal.

`Change` owns view presence. Desktop departures retain outgoing content for the
500ms exit plus its 60ms per-leaf cascade, mark it inert and move only revealed
leaves 14px upward while fading. The wrapper remains untransformed and opaque
until its final leaf finishes, so fixed inspectors remain viewport-relative and
no competing parent fade hides the cascade. Compact view changes update the
same host immediately and dispatch `desk:view-arrive` for its content. Retained
views replay their soft landing without remounting; nested requests are batched
so the same element never plays twice. Persistent navigation does not replay.
Compact views never wait for an exit. Changing the responsive breakpoint does
not replace the active workspace or discard its inspector state.

Native Tauri views use the same immediate, stable-host handoff as compact views,
including on macOS. They never wait for an outgoing cascade to empty the WebView.
Native appearance changes apply theme and palette together without a document
View Transition snapshot. Ordinary browsers retain the appearance crossfade.

`Presence` retains conditional dialogs and inspectors until their existing
exit finishes. Keep dialog focus, Escape, inert background, scroll locks and
opener restoration owned by the shared surface components. Dialogs capture the
initiating action during render; its marker survives the event turn so WebKit
pointer activation returns focus to the clicked button rather than a prior input.
Dialogs and inspectors
share `useCascadeExit`; nested exits do not animate the same leaves twice.
Surfaces remain mounted through the cascade tail and 500ms departure. Notification
exits use the shared departure timing.

Activity rows own Motion layout projection and presence. Their entrances are
560ms staggered opacity changes rather than a second transform animation.
Offscreen rows remain waiting rather than playing index-based entrances before
they can be seen. Reveal cleanup restores only properties it owns, leaving
layout transforms to Motion. Layout shifts remain interruptible, and row
removals retain their opacity departure without an index-based delay. Activity
reviews stay at full size with 14px vertical travel. Modal calendar inspectors
instead capture their initiating tile's viewport bounds and use one 560ms native
transform animation to expand into their final surface. Measurement and playback
begin before paint; dismissal, unmount and live reduced-motion changes cancel
playback and restore inline transforms. Reduced motion opens directly at final
size. Native overlay surfaces use the opaque material fallback instead of moving
a backdrop-filtered layer; the stationary scrim retains blur. Dialogs enter the
top layer and establish focus and scroll locks before paint.
Removed rows remain mounted until
their opacity exit completes. Disclosure contents use the viewport scheduler;
collapse waits for the visible children's exit cascade. Desktop setup steps retain
their outgoing inert leaves through the same cascade before the incoming question
appears. Compact setup steps fade as whole units without depending on viewport
reveals, and waiting setup chrome remains visible. Setup headings and choices add
a small staggered blur during the handoff; reduced motion skips it. Step wrappers
stay untransformed.
`SelectionGroup` scopes shared selection indicators
without adding DOM wrappers or nested focus targets.

`CountUp` uses the 650ms `--motion-count` duration and the shared
`cubic-bezier(.22, 1, .36, 1)` curve. Playback waits until the counter enters
the clipped viewport, including inside owned scroll containers and opened disclosures,
and starts with its containing reveal's delay rather than counting behind it.
It runs once per view visit, not on ordinary data rerenders or scrolling away
and back. Returning to a retained view rearms its counters for viewport entry.
Observers and playback are disposed when hidden or unmounted.
Numeric metrics animate by default. Formatted training, pace, milestone and race
summaries pass their numeric source to the same counter. A hidden final-value
reserve and tabular numerals prevent width changes while counting. Dates, editable
values, identifiers and table indices are not interpolated.
Screen readers always receive the final value;
reduced motion displays it immediately. Press, hover and selection feedback
retain their immediate interaction feedback; selection-marker travel uses the
shared Bezier tween. Gesture tracking remains one-to-one with the pointer.

## Opening Brand Handoff

`BrandSplash` preserves the original logo and uses the local Outfit wordmark.
It runs once per document opening, outside route and setup-step presence. The
logo and staggered letters resolve using the shared 560ms arrival and 60ms
stagger, then move to the measured header logo and wordmark over 560ms. Compact
headers retain their icon-only layout; their splash text folds away during travel.

A stationary, full-viewport frosted layer keeps the workspace blurred during
travel. Only after landing does that layer fade out over another 560ms, with the
logo held sharp and still. The blur radius itself is not animated and no filter
or transform is added to the workspace or a fixed-element ancestor. Reduced
transparency, increased contrast and unsupported blur use an opaque fallback.
The real header branding stays hidden until the splash copy is synchronously
hidden, preventing a duplicate-logo frame during React unmount.

The shared `desk:reveal-settle` event settles the destination header before
measurement so viewport entrances cannot move the landing point. Reduced motion
skips the splash entirely. Input, resize, navigation, backgrounding and live motion
preference changes cancel it immediately without blocking the initiating action.
A bounded startup deadline exposes loading or errors if no header becomes ready;
fonts, delayed data and setup do not create an indefinite splash.

## Accessibility And Performance

`prefers-reduced-motion` removes spatial entrances, layout projection, shared
indicator travel and surface gestures. Semantic selection, color, outline and
action feedback remain available. Live preference changes settle running
reveals without replaying them when motion is enabled again.

Use transform and opacity for arrivals and departures. Do not add transforms
or filters to fixed-element ancestors, animate every row's blur, add permanent
`will-change`, or run ambient animation loops. Motion-owned transforms must not
also have CSS transform transitions. Native selects keep their platform popup
behavior; do not replace them just to add animation.

## Verification

The `Verify iOS-inspired motion` VS Code task builds production assets and runs
scoped browser checks. Lifecycle tests in `tests/desk.spec.mjs` measure actual
rendered frames, exact native keyframe easing/delays, one-shot navigation chrome,
live preference changes, retained exits, non-overshooting list shifts and cleanup.
Existing Prism checks cover
compact navigation latency, retained filters, full-size inspector landings, keyboard/touch
input and width/theme geometry. Browser checks use synthetic IPC, not native
provider integration. They do not establish physical-device 60 FPS performance.

The packaged-motion regressions in `tests/desk.spec.mjs` serve the actual built
HTML, JavaScript and CSS from `dist`; run `npm run build` before these checks.
They verify 560ms entrances, 500ms departures, intermediate counter/exit frames
and immediate reduced-motion values, catching production-only CSS time-unit changes.