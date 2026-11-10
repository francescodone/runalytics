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
| `entranceFor` | Vertical entrance with exact 60ms DOM-ordered staggering |

## Lifecycle

`MotionSystem` provides Motion configuration, one reveal observer and shared
interaction listeners. Startup reveals use Motion's native mini API with a
560ms Bezier landing response. Visible leaves begin 14px below their final
position at full size. Each visible sibling advances the cascade by 60ms in
DOM order, without jitter or a cap that makes later siblings start together.
Startup uses one document-wide sequence, so nested view groups cannot restart
ahead of navigation or the workspace heading. Later arrivals are scoped to their view.
Offscreen leaves do not consume visible stagger slots. Late arrivals subtract
elapsed startup time instead of restarting the sequence. Route changes end startup mode.
There is no animated blur on large surfaces or repeated rows.

Reveals register before paint and wait for viewport entry. Ordinary rerenders
do not replay them. Each view change gets the same fresh, scoped landing.
Observer callback entries are sorted by DOM order, independently of delivery order.
Nested reveal targets never animate their ancestor at the same time. Elements
containing dialogs, inspectors or notification viewports are not transformed.
Pointer-down and keyboard focus settle an entrance immediately. Preference
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
Layout shifts and row removals follow the same DOM-order cadence. Activity and
calendar reviews stay at full size with 14px vertical travel instead of stretching
a short row or tile into a full-height panel. Removed rows remain mounted until
their opacity exit completes. Disclosure children receive individual 60ms delays;
collapse waits for the final child's fade. `SelectionGroup` scopes shared selection indicators
without adding DOM wrappers or nested focus targets.

`CountUp` uses the 650ms `--motion-count` duration and the shared
`cubic-bezier(.22, 1, .36, 1)` curve. Playback waits until the counter enters
the viewport, including inside owned scroll containers and opened disclosures.
It runs once per view visit, not on ordinary data rerenders or scrolling away
and back. Returning to a retained view rearms its counters for viewport entry.
Observers and playback are disposed when hidden or unmounted.
Screen readers always receive the final value;
reduced motion displays it immediately. Press, hover and selection feedback
retain their immediate interaction feedback; selection-marker travel uses the
shared Bezier tween. Gesture tracking remains one-to-one with the pointer.

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