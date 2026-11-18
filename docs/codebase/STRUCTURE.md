# Codebase Structure

Current-state context, source-checked on 2026-11-18. See
[Architecture](ARCHITECTURE.md) for data flow and [Conventions](CONVENTIONS.md)
for Prism's presentation contracts.

## 1. Top-Level Map

| Path | Purpose / evidence |
| --- | --- |
| [src/desk](../../src/desk/) | Production React presentation, route workspaces, shared primitives, profiles and bundled locales |
| [src/lib](../../src/lib/) | Typed IPC, shared data types and reusable calculations; start with [ipc.ts](../../src/lib/ipc.ts) and [types.ts](../../src/lib/types.ts) |
| [src/styles](../../src/styles/) | Local font assets and legacy styling; production desk imports its own [desk.css](../../src/desk/desk.css) and [interaction.css](../../src/desk/interaction.css) |
| [src-tauri/src](../../src-tauri/src/) | Native commands, profiles, SQLite, providers, training engine and optional publishing/sync |
| [src-tauri/plugins](../../src-tauri/plugins/) | App-owned native iPhone chrome plugin, including Swift source and package tests |
| [public](../../public/) | Prepaint appearance script and distributed static assets/notices |
| [tests](../../tests/) | Playwright workflows, Node unit checks and [synthetic regression fixtures](../../tests/fixtures/) |
| [scripts](../../scripts/) | Version/release, Apple packaging, icon and locale-check utilities |
| [docs/design-system](../design-system/) | Maintained [motion contracts](../design-system/motion.md) |
| [docs/codebase](./) | Source-backed stack, structure, architecture, conventions, integrations, testing and concerns |
| [docs/ways-of-work](../ways-of-work/) | Planning material; the [Garmin parity plan](../ways-of-work/plan/wearable-integrations/garmin-parity/implementation-plan.md) is proposed, not implemented |
| [supabase](../../supabase/) | Inactive hosted-backend foundation, not the current plan-sync service; see [backend status](../supabase-backend.md) |
| [.github](../../.github/) / [.vscode](../../.vscode/) | Agent instructions, release workflow and existing preview/verification tasks |

## 2. Entry Points

[Vite configuration](../../vite.config.ts) builds eight preserved entry URLs:
`desk.html`, `index.html`, `plan.html`, `calendar.html`, `injury.html`,
`achievements.html`, `settings.html` and `support.html`. The seven compatibility
entries receive the [desk template](../../desk.html); their old controllers are
not production presentation.

- Frontend: [boot.ts](../../src/desk/boot.ts) initializes the account before
  importing [main.tsx](../../src/desk/main.tsx).
- Routing: [data.tsx](../../src/desk/data.tsx) selects hash workspaces and
  entry-file defaults. Current workspace IDs are `training`, `journal`, `pace`,
  `milestones`, `settings` and `help`. Old `health` hashes redirect to Pace
  insights, except `health/milestones`, which opens Milestones.
- Native: [main.rs](../../src-tauri/src/main.rs) and
  [lib.rs](../../src-tauri/src/lib.rs) host Tauri and register commands/plugins.
- Browser preview: Vite serves `/__desk_preview.html` with
  [desk.fixture.mjs](../../tests/desk.fixture.mjs). Legacy HTML is selected only
  through the regression path in [Vite](../../vite.config.ts).

Do not remove entry URLs or revive `__*`/legacy harnesses as production screens.

## 3. Module Boundaries

| Boundary | Belongs here | Keep out |
| --- | --- | --- |
| [Prism primitives](../../src/desk/ui.tsx), [interactions](../../src/desk/interaction.tsx), [overlays](../../src/desk/overlays.tsx) | Shared hosts, controls, focus, presence, notifications and motion | Page-specific copies of interaction physics |
| [Training](../../src/desk/training.tsx), [Journal](../../src/desk/journal.tsx), [Pace](../../src/desk/pace-insights.tsx), [Settings](../../src/desk/settings.tsx) | Presentation, drafts, action review and typed command calls | Provider credentials or reimplementations of native training algorithms |
| [IPC](../../src/lib/ipc.ts) / [types](../../src/lib/types.ts) | Frontend/native contract and profile-scope checks | Silent contract changes for cosmetic work |
| [commands](../../src-tauri/src/commands.rs) / [profiles](../../src-tauri/src/profiles.rs) | Native command orchestration and account isolation | UI layout/material decisions |
| [plan](../../src-tauri/src/plan/) / [store](../../src-tauri/src/store.rs) | Training rules, prescriptions and persistence | Browser-fixture approximations treated as training logic |
| [providers](../../src-tauri/src/providers/), [plan sync](../../src-tauri/src/plan_sync.rs), [Strava](../../src-tauri/src/strava.rs) | External-service adapters and workers | Claims that synthetic IPC validates live services |

## 4. Naming and Organization Rules

- Production React modules are feature/shared-role files, generally lower-case
  or kebab-case: [training.tsx](../../src/desk/training.tsx),
  [pace-insights.tsx](../../src/desk/pace-insights.tsx),
  [profile-storage.ts](../../src/desk/profile-storage.ts).
- Native modules use Rust snake_case, for example
  [calendar_publish.rs](../../src-tauri/src/calendar_publish.rs).
- TS imports use relative paths; [tsconfig.json](../../tsconfig.json) defines
  bundler resolution and no path aliases.
- Treat built assets, Cargo output and generated Apple projects as artifacts,
  not architectural examples. The source-owned native plugin lives under
  [plugins](../../src-tauri/plugins/), not generated Apple output.

## 5. Evidence

- [Vite entries](../../vite.config.ts), [desk template](../../desk.html).
- [Boot](../../src/desk/boot.ts), [shell](../../src/desk/main.tsx), [routes](../../src/desk/data.tsx).
- [Native entry](../../src-tauri/src/lib.rs), [fixture installer](../../tests/desk.fixture.mjs).
