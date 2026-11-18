# Codebase Concerns

Current-state context, source-checked on 2026-11-18. This is not a vulnerability
audit or a current test-failure report. Historical test totals, temporary tool
failures and superseded UI decisions are intentionally not retained as current
facts. `[TODO]` marks verification that has not been established here.

## 1. Top Risks

| Priority | Concern / evidence | Impact and next action |
| --- | --- | --- |
| High | [Gist synchronization](../../src-tauri/src/plan_sync.rs) has no atomic remote compare-and-swap guarantee | Concurrent device writes can race despite fetch/read-back safeguards. Preserve conflict handling and test two-device convergence separately. |
| High | [Synthetic browser tests](../../tests/desk.fixture.mjs) do not exercise native integrations | [TODO] Establish release acceptance for live COROS/Strava, real Keychain, physical iPhone/macOS behavior, updater and offline archives. |
| Medium | The [current route parser](../../src/desk/data.tsx) redirects legacy Health hashes to Pace, while [README](../../README.md) still advertises a pain journal | Do not claim old Health journal tests or links establish reachable production UI. Resolve the intended product surface separately, not by reviving old controllers. |
| Medium | Native/full-browser baseline is not established by this documentation task | [TODO] Run the relevant suites for the next code change; do not carry old migration pass/failure counts into a current baseline. |

## 2. Technical Debt and Intent vs. Reality

| Area | Current evidence | Working rule |
| --- | --- | --- |
| Missing historical design specifications | This checkout has maintained [motion contracts](../design-system/motion.md) and implemented [tokens](../../src/desk/tokens.css), but no foundations/components specification files | Use [Conventions](CONVENTIONS.md) and current shared primitives. Do not invent/revive an absent specification from old change notes. |
| Garmin parity | [Adapter](../../src-tauri/src/providers/garmin.rs) is unsupported; [implementation plan](../ways-of-work/plan/wearable-integrations/garmin-parity/implementation-plan.md) is proposed | A plan or feature flag does not implement provider sign-in/sync. |
| Hosted backend | [Supabase status](../supabase-backend.md) is an inactive foundation | Do not document application accounts, server APIs or hosted sync as deployed capabilities. |
| Native chrome activation | [App shell](../../src/desk/main.tsx) starts navigation for Tauri iPhone/iPod after setup; [bridge](../../src/desk/native-navigation.ts) waits for acknowledged ownership and falls back | The old probe-only/production-off description is not current. Keep native and web verification scopes separate. |
| Locale quality | [Localization guidance](../i18n.md) identifies initial machine-generated catalogs | [TODO] Native-speaker review, especially coaching and medical terminology, plus native WebView locale acceptance. |

Typography, list geometry and motion now have one current description in
[Conventions](CONVENTIONS.md), not competing Outfit/Avenir/all-serif,
card-row/tilt, or earlier icon/counter timing proposals. Those superseded choices
are not compatibility requirements.

## 3. Security and Privacy Boundaries

These are documented exposure/concurrency boundaries, not newly confirmed
vulnerabilities or OWASP findings.

| Boundary | Existing mitigation / evidence | Remaining limitation |
| --- | --- | --- |
| Dependency advisory | [README](../../README.md) names a previously reported MapLibre sanitizer advisory; [lockfile](../../package-lock.json) pins resolved packages | [TODO] Revalidate applicability before distribution. No audit was run here; do not label the installed tree vulnerable based only on an old ledger. |
| Calendar read URL | Explicit disclosure in [settings](../../src/desk/settings.tsx) | Anyone holding the URL can read published data; stopping publication does not revoke/delete the file. |
| Strava visibility/writes | [Consent](../../src/desk/settings.tsx), activation cutoff and copy checks in [worker](../../src-tauri/src/strava.rs) | Descriptions inherit activity visibility. Concurrent device/third-party edits can race. |
| Connection QR | [Local encryption and reviewed import](../../src/desk/connection-transfer.tsx); OAuth sessions excluded | The encrypted payload still contains connection secrets. Keep image/passphrase private; browser decoding is not a physical-camera acceptance result. |
| Profile identity | [Native scope/ownership checks](../../src-tauri/src/profiles.rs), [journaled browser snapshots](../../src/desk/profile-storage.ts) | Login needs authorization; logout is not profile deletion. Never bypass ownership/import guards to recover a failed sign-in. |

## 4. Performance and Scaling Concerns

- [Activity virtualization](../../src/desk/journal.tsx) bounds mounted rows above
  18 results, with measured heights and a three-row buffer. Preserve scroll and
  unmounted-opener focus behavior when changing row geometry.
- [Shared motion](../design-system/motion.md) bounds observers, reveal work and
  aura animation; hidden/offscreen work suspends. Do not turn list rows into
  independently filtered or perpetually animated surfaces.
- [Gist sync](../../src-tauri/src/plan_sync.rs) and
  [Strava](../../src-tauri/src/strava.rs) are client-side workers, not closed-app
  services. Offline/suspended applications cannot continuously refresh external
  state. Physical-device frame-rate/energy behavior remains `[TODO]` acceptance.

## 5. Fragile / High-Churn Areas

A local `git log --since='90 days ago' --name-only --format=` inspection during
consolidation placed [desk styles](../../src/desk/desk.css),
[interaction styles](../../src/desk/interaction.css),
[app shell](../../src/desk/main.tsx), [desk regressions](../../tests/desk.spec.mjs),
[native commands](../../src-tauri/src/commands.rs) and
[store](../../src-tauri/src/store.rs) among frequently changed files, alongside
release/version manifests. This is a dated maintenance signal, not a defect
count or permanent ranking.

Shared styles affect multiple workspaces; native commands/storage affect
persistence and account isolation. Use scoped tests and the appropriate shared
matrix, and preserve draft/confirmation/offline contracts. Do not interpret
version-manifest churn as proof of architectural fragility.

## 6. Intent-Dependent Questions

No unresolved implementation decision is required for this documentation-only
consolidation. The context repository lives inside this Git repository; no new
remote, deployment, account linking or feature scope is implied. Product-surface
and release-readiness decisions above remain separate follow-up work.

## 7. Evidence

- [README limitations](../../README.md), [routing](../../src/desk/data.tsx), [app shell](../../src/desk/main.tsx).
- [Plan sync](../../src-tauri/src/plan_sync.rs), [Strava](../../src-tauri/src/strava.rs), [profiles](../../src-tauri/src/profiles.rs).
- [Prism tests](../../tests/prism.spec.mjs), [motion contract](../design-system/motion.md), [localization](../i18n.md).
- Churn inspection: local Git history, relative 90-day window at the source-check date.
