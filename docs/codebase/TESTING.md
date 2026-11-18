# Testing Patterns

Current-state context, source-checked on 2026-11-18. Commands and test scope below
are verified from configuration/source; this documentation consolidation did not
run the application suites or certify their current baseline.

## 1. Test Stack and Commands

[Playwright](../../playwright.config.mjs) is the browser runner, with Playwright
assertions and [axe checks](../../tests/prism.spec.mjs). Node's test runner covers
browser storage/bridge units; Cargo covers native Rust logic; the local iOS
plugin has [XCTest package checks](../../src-tauri/plugins/tauri-plugin-native-chrome/ios/Tests/NativeTabBarTests.swift).
Dependency requirements are in [Stack](STACK.md).

Start with the smallest relevant selection; build before checks that inspect
packaged assets. Existing commands include:

```sh
npm run build
npm run test:ui -- tests/profiles.spec.mjs tests/onboarding.spec.mjs
npm run test:ui -- tests/desk.spec.mjs --grep 'week summary|month summary'
npm run test:ui -- tests/prism.spec.mjs --grep 'Prism route matrix|Prism intermediate and short viewport|Prism detail views|Prism enlarged text'
node --experimental-strip-types --test tests/profile-storage.unit.mjs
npm run test:native-chrome
node scripts/check-i18n.mjs
cargo test --manifest-path src-tauri/Cargo.toml --lib plan_sync::tests
cargo test --manifest-path src-tauri/Cargo.toml --lib profiles::tests
```

For a focused alternate engine, use both the environment flag and browser option:

```sh
PLAYWRIGHT_WEBKIT=1 npm run test:ui -- tests/profiles.spec.mjs --browser=webkit
```

Escalate to `npm run test:ui` or the full Cargo suite when targeted results or
change scope require it. Do not treat a scoped run as a full-suite pass. The
existing [VS Code tasks](../../.vscode/tasks.json) include motion, native chrome,
plan freshness, Strava and Apple build checks; inspect a task before reusing it
because some historical tasks exclude tests or consume old temporary reports.

## 2. Test Layout

| Files | Purpose |
| --- | --- |
| [desk.spec.mjs](../../tests/desk.spec.mjs) | Shared controls, motion, notifications, training, activity review, connections and packaged-asset regressions |
| [prism.spec.mjs](../../tests/prism.spec.mjs) | Responsive geometry, materials, accessibility preferences, contrast and input behavior |
| [profiles.spec.mjs](../../tests/profiles.spec.mjs), [onboarding.spec.mjs](../../tests/onboarding.spec.mjs) | Account isolation and first-run/retained-draft workflows |
| [i18n.spec.mjs](../../tests/i18n.spec.mjs), [locale checker](../../scripts/check-i18n.mjs) | Bundled-language behavior and catalog parity/interpolation |
| [animated-icons.spec.mjs](../../tests/animated-icons.spec.mjs), [race-projections.spec.mjs](../../tests/race-projections.spec.mjs) | Focused behavior suites |
| [profile-storage.unit.mjs](../../tests/profile-storage.unit.mjs), [native-chrome.unit.mjs](../../tests/native-chrome.unit.mjs) | Node checks for storage transitions and synthetic bridge transport/lifecycle |
| [native Rust modules](../../src-tauri/src/), [profile tests](../../src-tauri/src/profiles/tests.rs) | Co-located native unit/worker/persistence tests |
| [native tab package tests](../../src-tauri/plugins/tauri-plugin-native-chrome/ios/Tests/NativeTabBarTests.swift) | UIKit tab/icon lifecycle and artwork, not whole-app acceptance |

## 3. Test Scope Matrix

| Scope | Evidence / what it establishes | What it does not establish |
| --- | --- | --- |
| Browser UI | [Synthetic fixture](../../tests/desk.fixture.mjs) drives real rendered controls, focus, layout and IPC requests | Live providers, real Keychain, native WebViews or device frame rate |
| Packaged browser assets | [Desk regressions](../../tests/desk.spec.mjs) load built HTML/JS/CSS and sample rendered motion | Installed native app contains those new assets |
| Native unit/worker | [Profiles](../../src-tauri/src/profiles/tests.rs), [COROS](../../src-tauri/src/providers/coros.rs), [sync](../../src-tauri/src/plan_sync.rs), [Strava](../../src-tauri/src/strava.rs) use temporary/synthetic inputs and local transports | Live authorization, real user credentials or physical-watch delivery |
| Bridge units | [Node bridge tests](../../tests/native-chrome.unit.mjs) check protocol and lifecycle | Actual UIKit presentation |
| UIKit package | [XCTest](../../src-tauri/plugins/tauri-plugin-native-chrome/ios/Tests/NativeTabBarTests.swift) checks native tab behavior | End-to-end application/provider flows or physical-device acceptance |

## 4. Mocking and Isolation Strategy

[installDeskFixture](../../tests/desk.fixture.mjs) injects synthetic Tauri commands
before navigation, loading persistent regression sources from
[tests/fixtures](../../tests/fixtures/). Fixtures approximate responses; they are
not the native training engine. Production pages require the real Tauri bridge
for actual data.

[Playwright config](../../playwright.config.mjs) uses two workers and reuses an
existing server at `http://127.0.0.1:1500`, otherwise starting a strict-port Vite
server. The **Redesign preview** [task](../../.vscode/tasks.json) also uses 1500;
`/__desk_preview.html` is the synthetic preview route. Do not run `npm run dev`
as a shortcut: its predev kills port-1499 listeners.

In shared environments, coordinate server ownership, avoid concurrent builds
of shared packaged assets, and give independent runs distinct `--output`
directories. A reused preview is not owned by the test run. Do not stop someone
else's listener or reinterpret connection/artifact failures as application bugs.

## 5. Coverage and Quality Signals

The existing [Prism matrix](../../tests/prism.spec.mjs) covers:

- Light/dark widths: **320, 360, 375, 390, 414, 430, 600, 768, 820, 1024,
  1280, 1366, 1440, 1600, 1920 and 2560px**.
- Short/intermediate viewports: **537x740, 913x480 and 667x375**.
- Detail/settings checks at **820px**, including scoped axe checks.
- **200% text at 320, 820 and 1440px**.
- Additional input/material tests, including compact month at 320/800px,
  contrast/fallbacks and settled-rendering budgets.

For shared presentation changes, retain the matrix plus relevant workflow tests,
keyboard/pointer/touch, live preference cancellation and focus-return checks.
Inspect representative light/dark compact/desktop screenshots. Record exact
commands, states, failures and native limitations with that change's evidence;
do not accumulate historical pass counts here.

The matrix includes compatibility hashes and retired settings destinations.
A passing geometry check on an alias/fallback does not prove the retired feature
exists. Confirm actual route/action ownership in
[data.tsx](../../src/desk/data.tsx) and [settings.tsx](../../src/desk/settings.tsx).

[TODO] Establish the current full-suite baseline and coverage metrics when
needed; [npm scripts](../../package.json) and [Playwright config](../../playwright.config.mjs)
do not define a coverage-percentage gate. Native safe areas, keyboard/VoiceOver,
physical-device performance, live provider flows, updater and offline archives
need separate acceptance evidence, not old browser-run totals.

## 6. Evidence

- [Playwright config](../../playwright.config.mjs), [npm scripts](../../package.json), [VS Code tasks](../../.vscode/tasks.json).
- [Fixture installer](../../tests/desk.fixture.mjs), [Prism matrix](../../tests/prism.spec.mjs), [desk regressions](../../tests/desk.spec.mjs).
- [Profile units](../../tests/profile-storage.unit.mjs), [bridge units](../../tests/native-chrome.unit.mjs), [native tab tests](../../src-tauri/plugins/tauri-plugin-native-chrome/ios/Tests/NativeTabBarTests.swift).
