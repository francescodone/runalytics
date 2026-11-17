# Native tab icon playback

Calendar, chart and trophy retain their SF Symbol resting artwork and original
Runalytics lift/contraction feedback, not UseAnimations assets. The Activities
runner uses the same articulated artwork and gait as the shared web icon:
connected knees/elbows, two alternating steps, counter-swinging arms and a small
body bob. `NativeRunnerGlyph` draws that pose into fixed-size template frames,
including the resting/Reduce Motion image, instead of shifting an intact symbol.
Its linear 560ms clock preserves even step cadence; the joint curves supply the
acceleration. Other tab animations retain Prism's nonovershooting
`cubic-bezier(.22, 1, .36, 1)` response. Each activation restores its resting image
within 560ms. Navigation has no outcome animation: a tab activation is not a
success result or an earned achievement.

The app-owned render hosts publish small template images through the public
`UITabBarItem.image` and `selectedImage` APIs with fixed image metrics. UIKit owns
placement, tint, selection, touch, keyboard and VoiceOver throughout. No extra
views, accessibility elements, recognizers, private tab-view lookups or assumed
floating-bar coordinates are involved. The native items and their identifiers
survive snapshot updates. The bridge sends its existing navigation intent before
starting playback; selection still follows the acknowledged snapshot.

Only the activated icon runs, on a bounded display link in common run-loop modes.
Reactivation replaces that link rather than queuing work. Completion, hiding,
removal, window disconnection, resizing, backgrounding and live Reduce Motion
notifications invalidate playback and restore the same resting artwork. Static
preference/visibility changes never replay a consumed activation. Display links
have weak targets, and idle controls have no frame callbacks.

In-app close controls belong to the shared React UI inside the native WebView;
they already use its plain, unboxed X and brief 180ms pulse, including the static
fallback. There is no separate UIKit close glyph in this plugin. OS-owned
launcher, window and picker artwork remains unchanged.

## Native checks

From this directory, with an available iPhone simulator destination:

```sh
xcodebuild -scheme tauri-plugin-native-chrome \
  -destination 'platform=iOS Simulator,name=iPhone 18 Pro' test
```

`Tests/NativeTabBarTests.swift` covers connected alternating runner steps and
knee flexion, opposing arms, unclipped poses, actual rendered frame changes and
restoration, fixed frame/baseline/alignment metrics and template tint, rapid replay
and obsolete callbacks, natural completion, weak ownership, visibility/lifecycle
interruption, and retained selection/accessibility labels. A retained XCTest image
attachment shows native poses at enlarged and actual tab sizes in light/dark ink.
Lifecycle notifications are posted by the tests; these checks do not establish
full-app integration, physical-device performance or a manual
VoiceOver/keyboard/settings audit.

From the repository root, `npm run test:native-chrome` separately checks the
synthetic bridge protocol; it is not native end-to-end integration testing.
