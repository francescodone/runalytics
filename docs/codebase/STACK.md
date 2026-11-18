# Technology Stack

Current-state context, source-checked on 2026-11-18. Start with
[Architecture](ARCHITECTURE.md); this directory documents implementation, not a
chronological change log. Dependency versions below are manifest requirements,
not claims about the installed toolchain.

## 1. Runtime Summary

| Area | Current implementation | Evidence |
| --- | --- | --- |
| Presentation | TypeScript, React TSX, browser/WebView CSS | [TypeScript config](../../tsconfig.json), [desk entry](../../src/desk/boot.ts) |
| Native application | Rust, edition 2021, Tauri 2; Swift/UIKit iPhone plugin | [Cargo manifest](../../src-tauri/Cargo.toml), [native plugin](../../src-tauri/plugins/tauri-plugin-native-chrome/ios/Sources/NativeChromePlugin.swift) |
| Package management | npm with a committed lockfile; Cargo with its own lockfile | [npm manifest](../../package.json), [npm lock](../../package-lock.json), [Cargo lock](../../src-tauri/Cargo.lock) |
| Build/modules | ES modules, TypeScript checking followed by Vite's multi-entry build | [npm scripts](../../package.json), [Vite config](../../vite.config.ts) |

[TODO] Pin or document supported Node.js, Rust and Xcode versions; the root
manifests do not establish one exact development-toolchain version.

## 2. Production Frameworks and Dependencies

| Dependency | Requirement | Role | Evidence |
| --- | --- | --- | --- |
| React / React DOM | `^19.3.1` | Workspace rendering and retained React Activity views | [npm manifest](../../package.json), [app shell](../../src/desk/main.tsx) |
| Tauri API / opener | `^2` | Native commands, events and external URLs | [npm manifest](../../package.json), [IPC](../../src/lib/ipc.ts) |
| Tauri updater / process | `^2.13.1` / `^2.4.0` | Desktop update/relaunch controls | [npm manifest](../../package.json), [settings](../../src/desk/settings.tsx) |
| Motion | `^14.4.0` | Shared interaction, presence and reveal lifecycle | [npm manifest](../../package.json), [interaction system](../../src/desk/interaction.tsx) |
| react-useanimations | `3.1.1` | Selected, locally bundled icon clips | [npm manifest](../../package.json), [catalog](../../src/desk/icon-catalog.ts) |
| Tabler React icons | `^3.50.0` | Retained source artwork/legacy contracts, not an alternate production player | [npm manifest](../../package.json), [motion ownership](../design-system/motion.md) |
| i18next | `^27.0.1` | Bundled translation runtime | [npm manifest](../../package.json), [i18n](../../src/desk/i18n.ts) |
| MapLibre GL / PMTiles | `^5` / `^4` | Route maps, online raster tiles and saved local archives | [npm manifest](../../package.json), [map](../../src/desk/map.tsx) |
| qrcode / jsqr | `^1.5.4` / `^1.4.0` | Encrypted connection QR generation/decoding | [npm manifest](../../package.json), [transfer](../../src/desk/connection-transfer.tsx) |
| Tauri / Tokio | `2` / `1` | Native host and asynchronous work | [Cargo manifest](../../src-tauri/Cargo.toml) |
| rusqlite | `0.40`, bundled | Local SQLite activity and plan storage | [Cargo manifest](../../src-tauri/Cargo.toml), [store](../../src-tauri/src/store.rs) |
| reqwest / rmcp / rustyfit | `0.13` / `3.5` / `0.10` | HTTP, COROS MCP and FIT decoding | [Cargo manifest](../../src-tauri/Cargo.toml) |
| keyring / aws-lc-rs | `4.2` / `1` | OS credential storage and native cryptography | [Cargo manifest](../../src-tauri/Cargo.toml), [secure store](../../src-tauri/src/secure_store.rs) |

Inter and Young Serif are local font assets selected by
[Prism tokens](../../src/desk/tokens.css); no runtime font service is required.

## 3. Development Toolchain

| Tool | Requirement / purpose | Evidence |
| --- | --- | --- |
| TypeScript | `~6.0.3`; strict checking, no emit | [npm manifest](../../package.json), [compiler config](../../tsconfig.json) |
| Vite | `^8.0.16`; preview and packaging | [npm manifest](../../package.json), [Vite config](../../vite.config.ts) |
| Playwright / axe | `^1.64.1` / `^4.14.0`; synthetic-IPC browser and accessibility checks | [npm manifest](../../package.json), [test config](../../playwright.config.mjs) |
| Node test runner | Browser-storage and native-bridge unit checks | [storage tests](../../tests/profile-storage.unit.mjs), [bridge tests](../../tests/native-chrome.unit.mjs) |
| Cargo / XCTest | Native Rust and local UIKit package checks | [Cargo manifest](../../src-tauri/Cargo.toml), [tab tests](../../src-tauri/plugins/tauri-plugin-native-chrome/ios/Tests/NativeTabBarTests.swift) |

## 4. Key Commands

```sh
npm ci
npm run build
npm run test:ui
node --experimental-strip-types --test tests/profile-storage.unit.mjs
npm run test:native-chrome
cargo test --manifest-path src-tauri/Cargo.toml
node scripts/check-i18n.mjs
```

Use the **Redesign preview** [VS Code task](../../.vscode/tasks.json) on port
1500 for browser work. It starts Vite's fixture-aware server; it is distinct from
`npm run preview` (the built-asset preview). Avoid `npm run dev`: its `predev`
script forcibly clears port 1499. See [Testing](TESTING.md) for scoped commands.

## 5. Environment and Config

- [Vite](../../vite.config.ts) reads optional `TAURI_DEV_HOST`; native development
  uses port 1499 with strict port binding.
- [Playwright](../../playwright.config.mjs) uses port 1500 and Chrome by default.
  WebKit requires both `PLAYWRIGHT_WEBKIT=1` and `--browser=webkit`.
- [Tauri config](../../src-tauri/tauri.conf.json) and
  [capabilities](../../src-tauri/capabilities/) define native packaging and access.
  User provider credentials belong in the OS credential store, not frontend env
  variables; see [Integrations](INTEGRATIONS.md).
- [README](../../README.md) describes Apple build prerequisites, macOS ad-hoc
  signing and unsigned iPhone sideloading. These are not App Store distribution.

## 6. Evidence

- [npm manifest](../../package.json), [Cargo manifest](../../src-tauri/Cargo.toml).
- [Vite config](../../vite.config.ts), [TypeScript config](../../tsconfig.json).
- [Playwright config](../../playwright.config.mjs), [VS Code tasks](../../.vscode/tasks.json).
