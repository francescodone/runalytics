# Architecture

Current-state context, source-checked on 2026-11-18. This is the entry point to
the context repository: [Stack](STACK.md), [Structure](STRUCTURE.md),
[Conventions](CONVENTIONS.md), [Integrations](INTEGRATIONS.md),
[Testing](TESTING.md) and [Concerns](CONCERNS.md).

Maintain these documents in place as contracts change. Keep exact test-run
results with the relevant change/release evidence rather than accumulating a
second chronological verification ledger. This consolidation did not execute
application tests; test presence is not a claim of a passing baseline.

## 1. Architectural Style

Runalytics is a local-first Tauri application: feature-oriented React workspaces
call a typed IPC layer; Rust commands coordinate profile-local SQLite, training
logic and provider workers. Evidence: [shell](../../src/desk/main.tsx),
[IPC](../../src/lib/ipc.ts), [native commands](../../src-tauri/src/commands.rs),
[store](../../src-tauri/src/store.rs).

Preserve entry URLs, IPC payloads, provider permissions, failed drafts,
confirmations and offline behavior. Presentation work is not authorization to
retune training models, merge accounts or change remote-plan authority.

## 2. System Flow

```text
HTML -> account bootstrap -> profile browser storage -> React workspace
     -> typed, profile-scoped IPC -> captured native profile
     -> SQLite / training engine / provider -> events and query refresh
```

1. [boot.ts](../../src/desk/boot.ts) awaits account initialization before loading
   the application and its module-level locale selection. Startup failures show
   an explicit retry surface instead of another account's workspace.
2. [profiles.ts](../../src/desk/profiles.ts) activates journaled local/session
   storage and the IPC scope. Account changes hide personal content and reload
   the Training route; [profile-storage.ts](../../src/desk/profile-storage.ts)
   archives outgoing preferences/drafts and recovers interrupted switches.
3. [main.tsx](../../src/desk/main.tsx) gates signed-out, legacy-import and setup
   states, then renders the selected workspace. Compact visited workspaces use
   React Activity so state survives navigation while hidden effects suspend.
4. [data.tsx](../../src/desk/data.tsx) supplies cancellable query consumption,
   duplicate-command guards, visible errors, refreshes and notifications.
   [ipc.ts](../../src/lib/ipc.ts) attaches `profileScope` and rejects obsolete
   replies after a profile change.
5. [Native profiles](../../src-tauri/src/profiles.rs) capture a scoped state/lease
   and reject mismatched scopes. [Commands](../../src-tauri/src/commands.rs)
   coordinate persistence, model calculations and external work.
6. Native events update workspace revision or plan-check status through
   [WorkspaceProvider](../../src/desk/data.tsx); refreshed evidence must not
   silently discard inspector drafts or act on a replacement plan.

## 3. Layer and Module Responsibilities

| Module | Owns / boundary | Evidence |
| --- | --- | --- |
| Account lifecycle | Stable COROS login ID selects a persistent profile; logout signs out without deleting local plans/history. Unknown-owner legacy import requires confirmation and an empty destination. | [profiles](../../src-tauri/src/profiles.rs), [profile screen](../../src/desk/main.tsx) |
| Workspace UI | Route state, evidence display, retained form drafts, action review and retry feedback; not native training models | [training](../../src/desk/training.tsx), [settings](../../src/desk/settings.tsx) |
| Shared presentation | Semantic tokens, native input hosts, focus/scroll ownership, overlays, icon playback and motion | [tokens](../../src/desk/tokens.css), [UI](../../src/desk/ui.tsx), [motion contracts](../design-system/motion.md) |
| Native domain/storage | SQLite history, plan generation, pacing, feedback and revision-guarded difficulty transactions | [store](../../src-tauri/src/store.rs), [plan modules](../../src-tauri/src/plan/) |
| External workers | COROS history/workouts, Gist plan authority, calendar publishing and optional Strava descriptions | [integrations inventory](INTEGRATIONS.md) |
| Native iPhone navigation | UIKit hosts tabs; the acknowledged bridge forwards existing React navigation and suspends for overlays/fields. Web navigation remains the fallback. | [native-navigation](../../src/desk/native-navigation.ts), [plugin](../../src-tauri/plugins/tauri-plugin-native-chrome/ios/Sources/NativeChromePlugin.swift) |

### Current feature contracts

- **Training summaries:** Week/Month denominators count non-rest scheduled
  workouts and planned distance in the selected period. Numerators count
  recorded calendar activities and their actual distance/date, including
  unmatched runs. Month spillover days are excluded. The owner-prefixed plan
  label is shared across horizons; blank names keep the plan label alone.
  [Training implementation](../../src/desk/training.tsx).
- **Activity review:** histories above 18 results use measured virtualization
  with a three-row buffer. Compact history scrolls with the workspace. Review
  focus must return even if the originating row was unmounted; feedback drafts
  survive review-tab changes. [Journal](../../src/desk/journal.tsx).
- **Pace insights:** show the five most recent eligible sessions; outdated
  sessions remain inspectable but do not count toward current status. Every
  measured rep must meet its own target for a session to count. Proposals retain
  their review/approval flow. [Pace workspace](../../src/desk/pace-insights.tsx).
- **Race estimates:** Today is the rounded 50/50 mean of valid positive
  plan-heuristic and watch times, falling back to the valid source or Not
  available. The heuristic uses saved threshold plus accepted offset, not a new
  watch threshold. Eligible future race plans project all four distances;
  projections and Current/Proposed reviews remain plan-only. These are Riegel
  heuristics, not guaranteed race outcomes. [Race estimates](../../src/desk/race-estimates.tsx).
- **Setup:** connection transfer precedes personal/training questions. Answers
  and progress persist without provider secrets; only encrypted QR text survives
  authorization navigation, not the passphrase/decrypted bundle. Preview is
  separate from creation. Long-run, rest-day and quality-day constraints go to
  the native engine. [Onboarding](../../src/desk/onboarding.tsx),
  [transfer](../../src/desk/connection-transfer.tsx).
- **Reviewed changes:** easier/harder previews do not write; approval is
  revision-guarded and preserves session mileage. Optional eligible-session
  feedback uses the native transaction, not a UI-only prescription rewrite.
  [Difficulty engine](../../src-tauri/src/plan/difficulty.rs),
  [feedback](../../src/desk/feedback.tsx).
- **Sync feedback:** one ordered progress notice owns waiting/working/outcome
  rows and the result, without the old activity/health report summary. Dismissal
  suppresses the rest of that operation; setup/splash suppress competing notices.
  Notifications and milestone celebrations respect the opening handoff.
  [Workspace provider](../../src/desk/data.tsx), [splash](../../src/desk/splash.tsx).
- **Settings ownership:** connections stay in their own destinations; Language
  is separate from Appearance. No offline-map editor or injury assistant is
  mounted. Saved PMTiles paths still render; deprecated assistant transfer data
  is discarded. [Settings](../../src/desk/settings.tsx),
  [map](../../src/desk/map.tsx), [transfer decoder](../../src/desk/connection-transfer.tsx).

## 4. Reused Patterns

| Pattern | Where / purpose |
| --- | --- |
| Typed bridge and profile lease | [IPC](../../src/lib/ipc.ts) and [profiles](../../src-tauri/src/profiles.rs) reject cross-profile work |
| Query/command hooks | [data.tsx](../../src/desk/data.tsx) centralizes busy/error/refresh handling without replacing drafts |
| Shared lifecycle primitives | [interaction.tsx](../../src/desk/interaction.tsx), [ui.tsx](../../src/desk/ui.tsx) and [overlays.tsx](../../src/desk/overlays.tsx) own presence, focus and feedback |
| Catalog adapter | [animated-icons.tsx](../../src/desk/animated-icons.tsx) and [icons.tsx](../../src/desk/icons.tsx) preserve one playback/fallback path |
| Source-backed fixture boundary | [desk.fixture.mjs](../../tests/desk.fixture.mjs) supplies synthetic IPC without changing production storage/providers |

## 5. Known Architectural Risks

Gist read-back limits stale publication but is not atomic compare-and-swap.
Strava checks do not eliminate simultaneous device/third-party edits. Suspended
or offline clients cannot continuously converge. Native acceptance and the
current full-suite baseline require separate evidence. See
[Integrations](INTEGRATIONS.md), [Testing](TESTING.md) and [Concerns](CONCERNS.md).

## 6. Evidence

- [Boot](../../src/desk/boot.ts), [profiles](../../src/desk/profiles.ts), [shell](../../src/desk/main.tsx).
- [Workspace data](../../src/desk/data.tsx), [typed IPC](../../src/lib/ipc.ts).
- [Native commands](../../src-tauri/src/commands.rs), [profile state](../../src-tauri/src/profiles.rs), [SQLite store](../../src-tauri/src/store.rs).
