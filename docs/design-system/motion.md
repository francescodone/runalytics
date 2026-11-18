# Prism Motion

## Animated Icon Ownership

Production glyphs use the catalog-first adapter in `src/desk/animated-icons.tsx`.
`icon-catalog.ts` explicitly imports selected UseAnimations assets; `icons.tsx`
keeps existing component call sites on the shared playback path. Runner, plain
close, cycling, strength, flame, mountain, rest, palette, language, unlink, QR,
contrast, checkered finish and brand artwork uses app-authored named-part recipes
in `custom-icons.tsx`. Those recipes retain source attribution and the existing
brand geometry; they are not UseAnimations catalog exports. Prism still owns
control, selection, layout, disclosure and overlay physics. Icon playback never
delays an action/navigation, adds a focus target or changes a control's reserved
footprint.

The runner takes two alternating steps within the shared 1120ms replay, with
connected knee/elbow joints, a small body bob and opposing arm swing. Its clock
is linear so both steps have an even cadence; the joint curves supply their own
acceleration. Close is a plain, unboxed X throughout its brief 360ms feedback
pulse, without a spin, plus morph or disappearing stroke. Both settle to their
original SVG pose and use that same artwork when motion is disabled.

`Button` and `IconButton` accept a semantic `icon`, optional `iconSize` and
`iconPosition` (`start` by default). `ControlAnimation` composes the original
native host: call its handler first, then activation, honoring `preventDefault`.
It bridges the package's public render-prop callback to the real accepted event;
it never dispatches a synthetic click or uses a private player API. `AnimatedIcon`
also covers existing glyph children, decorative headings, activity rows and badge
artwork. One shared registry supplies fine-pointer hover, keyboard-focus and
accepted-activation previews without invoking the control's command. Icon error
boundaries and static-state changes never replace the focused native host.

`Select` and `CheckInput` keep actual native selects/inputs, labels, refs, keyboard
operation and form submission; only their artwork is replaced. Checkbox, radio
and toggle players follow checked state rather than maintaining their own toggle.
Disclosure orientation follows real `aria-expanded`/native details state through
shared styling, not route-specific transforms. Native picker popup chrome remains
OS-owned. Search retains the library's search artwork with a short original accent
instead of morphing an idle search field into an unrelated clear action. Up/down
action arrows likewise use their same-source library SVGs with a small directional
nudge: their catalog clips have blank initial/final frames and must not leave
plan difficulty controls empty at rest. These poster motions share the normal
replay timing, eligibility and cancellation; they never hide or redraw the strokes.

`StatusIcon` loops only for actual `pending` work. Success/error are static unless
`play` identifies a new command outcome; repeated outcomes have distinct sequence
keys. Historical done/error stage snapshots do not replay. Decorative/reward
artwork plays on first eligible visibility, not ordinary rerenders. Clips use
`speed` to fit action replay to `--motion-icon-replay` (1120ms) and outcomes to
`--motion-icon-result` (360ms), including shorter clips. These icon-specific tokens
are independent of surface arrivals and immediate press feedback. Original SVG
recipes share those timing/easing tokens and cancel rather than queue on repeat
input. Pending clips retain native playback speed: their multi-rotation timeline
is not compressed into a one-second loop. Header sync uses that same pending
asset, without additional CSS rotation. The normal 1000ms action-result hold is
unchanged.

`MotionSystem` shares visibility/preferences and delegated interaction listeners
rather than global observers/listeners per icon. Hidden documents, clipped or
offscreen slots, inert/exiting content, React Activity teardown, reduced motion
and forced colors replace players with same-size, same-source static SVGs.
`icon-posters.json` contains selected bundled library exports and captured library
frames only, with per-instance definition IDs and inherited theme colours; custom
players settle to their unchanged source SVG. Interrupted one-shots are consumed,
so returning does not replay stale outcomes/rewards. Still-pending work may resume.
Custom native animations restore all owned SVG styles on teardown. Actual QR
payload images, data charts, route geometry, progress rings and palette swatches
remain data, not decorative catalog replacements. Map icon portals retain the
shared React provider and map-owned geographic/control hosts.

All assets remain local/offline; MIT notices ship as
`public/useanimations-LICENSE.txt` and `public/tabler-LICENSE.txt`. Existing Tabler source geometry is retained only
where it supplies custom artwork or legacy regression contracts, not as an
unrelated catalog fallback. Do not import the entire animation/poster catalog.

Native iOS tab activation retains UIKit navigation, selection, layout, tint and
accessibility ownership. Public item-image updates supply bounded app-authored
1120ms motion after the existing intent is sent; no private UIKit view inspection
or web-navigation replacement is involved. Calendar/chart/trophy retain their SF
Symbols and shared easing. The native runner draws the same articulated two-step
gait and resting pose as the web glyph, using an even linear clock, connected
knees/elbows and opposing arms inside the existing symbol image metrics. Native
lifecycle and Reduce Motion interruptions restore that same resting artwork
without stale replay. In-app close controls are shared React/WebView controls and
already use the plain-X feedback; this plugin has no separate native close glyph.
These native custom animations are not React/library players; native
compilation/simulator checks are separate from synthetic-IPC browser tests.
OS-owned Dock/tray/window and picker artwork remains unchanged.

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
| `--motion-icon-replay` | 1120ms | Icon hover, focus, activation and first-visible replay |
| `--motion-icon-result` | 360ms | Icon outcomes and close feedback |
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
interaction listeners. Above 800px, startup reveals use Motion's native mini API
with a 560ms Bezier landing response. Visible leaves begin 14px below their final
position at full size. Each newly visible sibling advances the cascade by 60ms.
One animation-frame batch collects eligible units across observer callbacks and
sorts them by rendered top edge, then left edge, with DOM order as a tie-breaker.
This also handles responsive grids whose visual order differs from their DOM.
Offscreen, clipped, hidden, inert, settled and already running units do not consume
new stagger slots. Later scroll entries and asynchronously mounted content start
a fresh batch at zero; they do not inherit delays from previously revealed content.
At compact widths up to 800px, registered content settles immediately, including
later scroll entries, disclosures and asynchronously mounted content. Resizing
into compact mode settles pending and running reveals; widening does not replay
already settled content. Press, selection, surface and other interaction motion
is unchanged. Eligibility intersects the viewport with every clipping or
scrolling ancestor.
Tall content uses the scrollport-sized visibility threshold rather than waiting
for an impossible percentage of the entire element. Route changes end startup mode.
There is no animated blur on large surfaces or repeated rows.

Reveals register before paint; desktop reveals wait for viewport entry. Ordinary
rerenders do not replay them. Each desktop view change gets a fresh, scoped landing.
A view arrival that cancels a pending reveal frame releases its handle, so
rapid successive arrivals — native filter changes among them — never strand
reveal scheduling with invisible, permanently waiting content.
Headings, metric groups, setup controls, lists, tables, forms and opened disclosures
use the shared registration path. Native details register their contents on opening.
Nested reveal targets animate as one outer visual unit, including its borders
and controls, instead of revealing only their innermost content. Elements
containing dialogs, inspectors or notification viewports are not transformed.
Pointer-down and keyboard focus on controls settle an entrance immediately.
Workspace headings replay when their persistent host changes route; navigation
chrome does not. Route changes focus the workspace rather than the heading.
Preference
changes cancel native playback and restore the original inline styles.
Observers, listeners and detached-element animations are cleaned up on removal.

`Change` owns view presence. Desktop departures retain outgoing content for the
500ms exit plus its 60ms per-leaf cascade, mark it inert and move only revealed
leaves 14px upward while fading. The wrapper remains untransformed and opaque
until its final leaf finishes, so fixed inspectors remain viewport-relative and
no competing parent fade hides the cascade. Compact view changes update the
same host immediately and dispatch `desk:view-arrive` for its content; their
content does not replay an entrance. Retained desktop views replay their soft
landing without remounting; nested requests are batched so the same element
never plays twice. Persistent navigation does not replay. Compact views never
wait for an exit. Changing the responsive breakpoint does
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
For activity histories with more than 18 results, only the visible range and a
three-row buffer are mounted; scrolling out of that range removes rows immediately
rather than playing a departure. Shorter histories retain the existing row-presence
behavior, including opacity exits. The virtual list measures row heights, follows
both the desktop list scrollport and compact workspace scrollport, and restores
focus to a review's originating activity if it was unmounted while the review was
open. Disclosure contents use the viewport scheduler;
collapse waits for the visible children's exit cascade. Desktop setup steps retain
their outgoing inert leaves through the same cascade before the incoming question
appears. Compact setup steps fade as whole units without depending on viewport
reveals, and waiting setup chrome remains visible. Desktop setup headings and
choices use a small staggered blur during the handoff. Compact setup skips the
individual entrance blur while retaining the step fade and departure feedback;
reduced motion skips the blur. Step wrappers stay untransformed.
`SelectionGroup` scopes shared selection indicators
without adding DOM wrappers or nested focus targets.

On desktop, `CountUp` uses the 650ms `--motion-count` duration and the shared
`cubic-bezier(.22, 1, .36, 1)` curve. Playback waits until the counter enters
the clipped viewport, including inside owned scroll containers and opened disclosures,
and starts with its containing reveal's delay rather than counting behind it.
It runs once per view visit, not on ordinary data rerenders or scrolling away
and back. Returning to a retained view rearms its counters for viewport entry.
Observers and playback are disposed when hidden or unmounted.
Numeric metrics animate by default on desktop. Formatted training, pace, milestone
and race summaries pass their numeric source to the same counter. A hidden
final-value reserve and tabular numerals prevent width changes while counting.
Dates, editable values, identifiers and table indices are not interpolated.
Screen readers always receive the final value; compact layouts and reduced motion
display it immediately. Widening a compact layout keeps existing counters settled
until a subsequent view visit. Press, hover and selection feedback
retain their immediate interaction feedback; selection-marker travel uses the
shared Bezier tween. Gesture tracking remains one-to-one with the pointer.

## Opening Brand Handoff

`BrandSplash` preserves the original logo and uses the local Young Serif wordmark.
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
skips the splash entirely. Input, navigation, backgrounding and live motion
preference changes cancel it immediately without blocking the initiating action.
Viewport resizes during the intro are accommodated before measuring the destination;
resizes during docking cancel the handoff.
A bounded startup deadline exposes loading or errors if no header becomes ready;
fonts, delayed data and setup do not create an indefinite splash. Notifications
are hidden during the handoff, including when their viewport is in the browser
top layer. Their dismissal timers and countdown rings pause until the splash ends.
Achievement celebration dialogs and their haptic feedback are queued until the
handoff finishes or is interrupted; the input that interrupts it finishes before
the first dialog opens.

## Accessibility And Performance

The header sync control uses the shared pending asset at its source playback
speed while sync is active, with no additional CSS rotation. Reduced motion
uses the same-source static artwork.

`prefers-reduced-motion` removes spatial entrances, layout projection, shared
indicator travel and surface gestures. Semantic selection, color, outline and
action feedback remain available. Live preference changes settle running
reveals without replaying them when motion is enabled again.

Use transform and opacity for arrivals and departures. Do not add transforms
or filters to fixed-element ancestors, animate every row's blur, add permanent
`will-change`, or run ambient animation outside the bounded material aura below.
Motion-owned transforms must not
also have CSS transform transitions. Native selects keep their platform popup
behavior; do not replace them just to add animation.

Existing aura-backed toolbars, route controls, choice tracks, pace modes, setup
bands, proposal prompts and notices may drift their broad color washes. The
shared `MotionSystem` changes only background position along independent random
paths, easing each segment over an 18-30 second sequence. Each new sequence
starts where the last finished. Controls, content and page backgrounds do not
move. No animated blur, permanent layer hints or per-frame JavaScript is added.
Offscreen surfaces and hidden documents pause; removed surfaces cancel playback.
Reduced motion keeps a static wash. Reduced transparency, increased contrast and
forced colors cancel playback and retain the existing opaque fallback.

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