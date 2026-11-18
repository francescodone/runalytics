# External Integrations

Current-state context, source-checked on 2026-11-18. These are implementation
contracts, not claims that live services or deployed native binaries were tested.

## 1. Integration Inventory

| System | Purpose and auth | Boundary / evidence |
| --- | --- | --- |
| COROS MCP | Primary activity/health reads and structured workout publishing; OAuth with PKCE | [COROS provider](../../src-tauri/src/providers/coros.rs), [provider trait](../../src-tauri/src/providers/mod.rs). A stable COROS login selects the local profile, not the watch model. |
| GitHub Gist | Optional encrypted editable-plan sync; user token with `gist` scope and linked COROS identity | [Plan sync](../../src-tauri/src/plan_sync.rs). Current wire version is 4; Gist is the remote plan authority, not Supabase. |
| Calendar hosting | Optional ICS upload and read-only subscription; configured upload credential | [Calendar publisher](../../src-tauri/src/calendar_publish.rs), [settings](../../src/desk/settings.tsx). Exact HTTPS upload/read URLs, not a generic hosting directory. |
| Strava | Optional description updates for newly recorded, plan-paired runs; own application credentials and OAuth `activity:read_all` / `activity:write` | [Strava](../../src-tauri/src/strava.rs), [consent/settings](../../src/desk/settings.tsx). No closed-app service or historical backfill. |
| Maps | OpenStreetMap raster tiles, or an existing local PMTiles path in Tauri | [Map](../../src/desk/map.tsx). Keep contributor attribution and failure handling; no current offline-map settings editor. |
| App updates | Desktop update/relaunch and release information | [Settings](../../src/desk/settings.tsx), [Tauri config](../../src-tauri/tauri.conf.json). Native updater acceptance is separate from browser controls. |
| Garmin | Feature-gated unsupported adapter | [Garmin stub](../../src-tauri/src/providers/garmin.rs), [Cargo features](../../src-tauri/Cargo.toml). The [parity plan](../ways-of-work/plan/wearable-integrations/garmin-parity/implementation-plan.md) is proposed work. |
| Supabase | Inactive hosted-backend foundation | [Backend status](../supabase-backend.md). Do not treat local migrations/plans as deployed application auth, APIs or sync. |

## 2. Data Stores

| Store | Role / access | Important boundary | Evidence |
| --- | --- | --- | --- |
| SQLite | Profile-local activity/health cache and plans | `profiles/<key>/runalytics.sqlite`; logout does not delete the profile | [profiles](../../src-tauri/src/profiles.rs), [store](../../src-tauri/src/store.rs) |
| OS credential store | Provider, GitHub, Strava and calendar secrets | Scoped by profile; real credential values do not belong in frontend storage or docs | [secure store](../../src-tauri/src/secure_store.rs), [profiles](../../src-tauri/src/profiles.rs) |
| Browser local/session storage | Appearance, language, onboarding and retained connection-transfer drafts | Journaled profile snapshots prevent another account inheriting live preferences | [profile storage](../../src/desk/profile-storage.ts) |
| Private Gist | AES-256-GCM encrypted plan snapshots derived from the COROS identity | Not a history backup or a transactionally locked database | [plan sync](../../src-tauri/src/plan_sync.rs) |
| Hosted ICS | Published history/schedule/notes | Anyone holding its read URL may read it; calendar clients control refresh | [settings consent](../../src/desk/settings.tsx), [README limitations](../../README.md) |

## 3. Secrets and Credentials Handling

- Account bootstrap and scoped IPC precede personal UI. Native legacy import
  checks expected ownership, an unclaimed source and an empty destination;
  never overwrite a populated profile to make login succeed.
  [Profiles](../../src-tauri/src/profiles.rs), [boot](../../src/desk/boot.ts).
- Desktop COROS auth uses a temporary loopback listener made nonblocking before
  Tokio conversion. Unrelated requests do not consume it; incomplete request
  reads are bounded, denial is retryable and abandoned login times out. Mobile
  uses the existing intercepted browser/WebView callback path instead.
  [COROS provider](../../src-tauri/src/providers/coros.rs).
- Strava's iPhone auth uses the native authentication session; preserve callback
  state/scope validation and cancellation. [Strava auth](../../src-tauri/src/strava.rs),
  [iOS bridge](../../src-tauri/src/strava/ios_auth.rs).
- Connection QR transfer is local AES-256-GCM with PBKDF2-SHA256 (600,000
  iterations), random salt/nonce and a minimum 12-character passphrase. It may
  contain calendar credentials/URLs, a GitHub token, Strava application
  credentials/athlete ID, a COROS comparison code (`identity.corosCode`) and an
  optional runner name. It excludes OAuth sessions/access/refresh tokens, plans
  and activity history. Deprecated assistant fields are discarded on decode.
  [Transfer encoder/decoder](../../src/desk/connection-transfer.tsx),
  [native export](../../src-tauri/src/calendar_publish.rs).
- Import requires account comparison, explicit acknowledgement when identity
  cannot be verified, and review before replacement; known mismatches block it.
  Setup can scan before sign-in, but applying settings still requires the watch
  flow. Passphrases/decrypted bundles do not persist across authorization;
  camera tracks stop on scan, close or source change. Native desktop supports
  image upload rather than camera capture.
  [Transfer flow](../../src/desk/connection-transfer.tsx).
- This documentation task did not audit secret history or access live Keychain
  data. Follow [hosted-services instructions](../../.github/instructions/hosted-services.instructions.md)
  before any dashboard/settings operation; dashboard context is not deployment
  authorization.

## 4. Reliability and Failure Behavior

- Gist freshness checks run separately from full activity downloads, including
  visible/online polling, focus/resume and reconnect. Changed remote snapshots
  replace conflicting unpublished local plan edits; explicit empty snapshots
  propagate plan endings. Conditional reads, no-op suppression and read-back
  bound traffic and stale publication, but cannot make Gist PATCH atomic.
  [Native sync](../../src-tauri/src/plan_sync.rs), [frontend scheduling](../../src/desk/data.tsx).
- Device-plan sync follows saved GitHub credentials and linked COROS identity;
  Strava automation follows authorization. Do not restore obsolete enable switches
  or remove privacy/replacement confirmations. Strava uses an activation cutoff,
  retains existing notes, checks previous copies, and exposes retry/review states.
  Simultaneous remote edits may still race.
  [Settings](../../src/desk/settings.tsx), [Strava worker](../../src-tauri/src/strava.rs).
- Calendar publishing preserves local data on upload failure and records retry
  status. Stopping publishing retains the hosted file and saved configuration;
  it does not revoke the public read URL or remove subscriptions.
  [Publisher](../../src-tauri/src/calendar_publish.rs), [settings](../../src/desk/settings.tsx).
- Ending/clearing scheduled training is not device-only: confirmation explains
  connected-device, COROS and calendar effects. Logging out is a different,
  non-destructive operation. [Training/settings](../../src/desk/settings.tsx),
  [profile disconnect](../../src-tauri/src/profiles.rs).
- An open profile supports local offline work; authorization and external
  convergence require connectivity. Do not equate an API response, a mock
  callback, or browser IPC with physical-watch delivery.
  [README](../../README.md), [Testing](TESTING.md).

## 5. Observability for Integrations

[WorkspaceProvider](../../src/desk/data.tsx) presents ordered stage snapshots,
retryable errors and one result notice. Settings expose native connection,
publishing and plan-check status through [typed IPC](../../src/lib/ipc.ts).
[TODO] Live-service monitoring/SLOs and operational acceptance need separate
verification; local status UI and native tests do not establish them.

## 6. Evidence

- [COROS](../../src-tauri/src/providers/coros.rs), [profiles](../../src-tauri/src/profiles.rs), [secure store](../../src-tauri/src/secure_store.rs).
- [Gist sync](../../src-tauri/src/plan_sync.rs), [calendar publisher](../../src-tauri/src/calendar_publish.rs), [Strava](../../src-tauri/src/strava.rs).
- [Connection transfer](../../src/desk/connection-transfer.tsx), [settings](../../src/desk/settings.tsx), [Supabase status](../supabase-backend.md).
