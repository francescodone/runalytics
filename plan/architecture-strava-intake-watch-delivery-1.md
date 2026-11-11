---
goal: Focus exclusively on training plans, with Strava workout intake and output-only watch calendar connections
version: 1.1
date_created: 2026-11-11
last_updated: 2026-11-11
owner: Runalytics
status: 'On Hold'
tags: [architecture, migration, strava, calendar, providers, privacy]
---

# Introduction

![Status: On Hold](https://img.shields.io/badge/status-On%20Hold-orange)

This is an implementation plan, not authorization to execute the migration or change production data. It records the requested training-only architecture: import completed workouts through Strava, retain optional watch connections only to publish and maintain structured scheduled workouts, and completely remove health collection, storage, processing and presentation. The pain/injury journal, injury assistant, physiological assessments, readiness, recovery and injury-risk features are removed, not hidden or retained as unavailable panels. Workout recovery steps and scheduled easier weeks remain: they are training prescriptions, not health assessments.

Implementation is technically feasible, but production rollout is on hold pending resolution of the Strava policy gate below. Mock-only development of the provider split and watch delivery can proceed without collecting Strava data. No watch expansion, hosted service, deployment, credential operation, or destructive migration is authorized by this document.

### Execution Progress: 2026-11-11

Implementation branch: `feature/strava-as-primary-source`. The user requested implementation on this branch; the live-data permission and confirmed-erasure gates are not waived by that request.

- Completed TASK-004 as an isolated, synthetic-tested preparatory change: `ProviderConnection`, `ActivitySource` and `WorkoutDestination` are separate traits; COROS and the feature-gated Garmin placeholder implement them independently. `ProviderRegistry` stores intake sources and output destinations separately, allowing a destination with no activity/health implementation. The temporary combined `Provider` remains only for legacy health/fitness sync until confirmed migration.
- Watch authentication, diagnostics, push, cancel, adaptive re-push, schedule refresh and proposal delivery now resolve `Arc<dyn WorkoutDestination>`. Startup and legacy recorded-activity downloads resolve the activity-source registry. This refactor does not change existing runners' active intake mode, collect Strava history, delete health data, or complete GOAL-002.
- Verification: VS Code task `Verify provider capability separation` passed 14 tests with `--features garmin`, including calendar-only registration/push/cancel and replacement without implicit intake. Task `Verify difficulty persistence` passed 12 difficulty, 1 feedback/device-sync, 18 lap and 12 COROS tests after the shared-state registry change. Two existing warnings remain: unused `Path` import in `plan/adaptive.rs` tests and unused `calendar::workout_purpose`.
- Synthetic-only preparation partially implements TASK-005/006 and TASK-008 through TASK-014: Strava provenance, training-only summary/stream mapping, read-only OAuth scopes, transactional checkpoints, bounded bootstrap and incremental scans, exact verified legacy pairing, a review queue for possible duplicates, and transactional review resolution. These import and review functions are not registered as a live source or exposed to the UI; retention expiry and deletion are not enforced end to end. The writer uses locally persisted app/read quota observations, but a future importer must explicitly use that quota store and cross-device quota coordination remains unresolved.
- Exact Strava-backed description jobs use the verified remote activity ID. Legacy aliases preserve completion lookup and existing description ledger identity after an explicitly verified pairing. The synthetic ambiguity workflow does not yet provide migration preview, account isolation, or a user-facing review flow.
- Imported description jobs require unexpired athlete-scoped evidence both when queued and immediately before processing, including retained legacy aliases. Older page replays cannot overwrite newer evidence, and subsequent imports prune expired pending-review payloads. These checks do not implement expiry before display, prevent every in-flight race, purge imported activities or derivatives, or satisfy deletion and revocation requirements.
- Delivery preparation adds an isolated SQLite ledger keyed by workspace, destination, account and plan. Monotonic plan revisions reject stale or conflicting enqueues; identical later snapshots retain their existing delivery state. Obsolete push results can record remote IDs for reconciliation, invalidating any later confirmation until every recorded side effect is explicitly resolved. Tests cover restart, old-schema repair, cancellation races and account isolation. No production mutation enqueues into it yet: serialization, capture of in-flight results, retries, remote verification, cross-device publishing ownership and integration remain outstanding under TASK-022 through TASK-025. A remote ID in the reconciliation ledger is not authorization to delete a shared or active remote plan.
- The isolated ledger can list pending and due retry work by workspace, destination and account, excluding confirmed entries and retries before their deadline. This query does not claim or lock work and is not a delivery worker; concurrent dispatch and cross-device ownership remain unresolved.
- Difficulty-feedback and pacing-proposal paths no longer offer harder adjustments based on physiological signals. Previously saved faster pacing proposals are also rejected at application and storage boundaries. Slower pacing proposals and difficulty easing remain available. This does not remove the health subsystem or retire every health-derived proposal.
- Verification: `Verify Strava source contracts` passed TypeScript checking and 44 Strava tests, including synthetic HTTP, identity resolution, evidence freshness and rate-limit behavior. `Verify provider capability separation` passed 23 provider tests with the Garmin feature. `Verify difficulty contracts` passed localization checks, TypeScript and 24 pacing tests; `Verify difficulty persistence` passed 12 difficulty, 1 feedback/device-sync, 18 lap and 12 COROS tests. The two existing Rust warnings noted above remain.
- TASK-001 is unresolved. TASK-003 has focused regression evidence only; its full synthetic migration fixture is not complete. The production intake switch, confirmed health removal, durable delivery, device pairing and training-only UI are not implemented. No native/watch acceptance, full UI suite, commit, push or deployment has been performed. All task rows below remain incomplete except TASK-004.

## 1. Requirements & Constraints

### Product Contract

- **REQ-001**: Strava is the sole source of new completed activities after a runner explicitly switches intake. Do not automatically fall back to COROS or another watch for activity, FIT, track, health, fitness assessment, or training-load downloads.
- **REQ-002**: Watch connections are optional output destinations for structured training-calendar delivery: warm-up, work steps, repeats, recoveries, cool-down, pace targets, dates, updates, and cancellations. An iCalendar event or GPX route is not a substitute for a guided workout.
- **REQ-003**: Keep the existing COROS delivery implementation and fixed remote plan identity. Permit account/authentication and remote plan/course reads needed to update our schedule safely. Do not permit activity, health, fitness, or recorded-FIT reads through the delivery interface.
- **REQ-004**: Plan creation, recorded-run review, matching, workout feedback, and local calendar editing must work without a connected watch or any health evidence. Require sufficient imported workout evidence or explicit manual baseline and threshold inputs, not a watch-history sync stamp.
- **REQ-005**: Remove health collection, storage, models, calculations, commands, settings, charts, exports and UI. This includes heart rate, sleep, HRV, stress, steps, calories, physiological fitness assessments, VO2max, vendor training-load/status, readiness/recovery scores, injury risk, pain records and injury-assistant conversations. Strava intake retains only workout identity, date, sport, distance, timing, pace, elevation, laps and optional GPS. Unwanted fields present in provider responses are discarded before persistence or logging. Power and cadence are also outside this training-plan version.
- **REQ-006**: Preserve existing plan/workout IDs, completed training prescriptions, moves, skips, non-clinical workout feedback, training-only proposals and notes, match links, reset tombstones, calendar UIDs, offline edits, and cancellation confirmations. Do not regenerate an existing plan as a migration side effect. Health-derived fields and proposals are explicitly excluded from this preservation rule; TASK-017 defines their removal.
- **REQ-007**: Separate intake success from watch delivery success. A watch authentication or network failure must not fail a successful Strava import, undo a local edit, or mark a stale remote calendar as current.
- **REQ-008**: Preserve the existing opt-in Strava description writer without creating/uploading duplicate activities. New Strava-backed rows target their own verified Strava activity ID instead of fuzzy matching themselves back to Strava. Reading activities must not require write permission.
- **REQ-009**: Preserve iCalendar publishing as a separate optional destination. Watch delivery, phone calendar subscriptions, and encrypted cross-device plan sync have independent state and failure reporting.
- **REQ-010**: Preserve historical watch workout recordings needed for training continuity, but do not retain a health archive inside the target app. A confirmed, restart-safe migration removes health tables/columns, scores, pain/conversation records, health-specific settings and derived payloads from the app's controlled stores and remote copies. Explain the removal before confirmation; this planning edit does not itself authorize deletion of live data. Historical workout preservation is not permission to retain Strava data beyond its approved policy.
- **REQ-011**: Import and synchronize only the authenticated runner's records. Switching Strava athletes must not merge one person's data, completion links, sync channel, or watch delivery configuration with another person's workspace.
- **REQ-012**: Cross-device plan sync must not require COROS login for new runners. Existing COROS-derived encrypted channels must remain readable until an explicit, verified channel migration completes.
- **REQ-013**: First release supports the existing COROS workout destination only. Watches that upload activities to Strava can supply completed runs, but automatic workout delivery is advertised only for implemented and hardware-verified destinations. Garmin remains an unimplemented placeholder, not an enabled destination.
- **REQ-014**: The product contains plan creation, periodization, calendar editing, structured sessions, workout delivery, completion matching, pace/repetition execution review, benchmarks and non-clinical session feedback. Remove the Health destination, injury routes, body map, pain journal and assistant configuration/actions; redirect old entry URLs to Training without restoring removed data or making network calls.
- **REQ-015**: Training decisions use explicit plan inputs and workout-execution evidence only. Remove physiological eligibility gates instead of leaving every runner permanently blocked by missing health. Preserve ordinary planned progression, taper/easier weeks and explicit user edits. Automatic or offered increases in intensity/volume remain off until deterministic activity-only eligibility rules are specified and tested; easing and within-prescription customization remain available. Do not introduce a replacement health, injury-risk or readiness score.

### Release-Blocking Policy Gate

The Strava API Agreement and API Policy fetched on 2026-11-11 state an effective date of 2026-06-01. They materially qualify the earlier technical feasibility discussion:

| Published provision | Effect on this architecture | Required resolution |
| --- | --- | --- |
| API Policy 6.2: seven-day cache limit; 5.5: persistent-index/storage restriction | Current SQLite history, tracks, completion receipts, and derived caches cannot simply become indefinite Strava storage. | Written permission covering operational storage, retention, derivatives, and offline use, or a revised product architecture approved by the user. |
| API Policy 5.4: analytics and combination restrictions; 5.2: competing functionality | Runalytics reviews workout execution and relates recordings to plans and session feedback, even without health features. | Confirm permission for this specific training-plan workflow; removing health does not remove these restrictions. |
| API Policy 5.3: AI operation restrictions, including derived data | No AI feature is required by the training-only product. | Remove the injury assistant and its egress paths; do not send Strava data or derivatives to an LLM. Do not treat official personal-use MCP access as authorization for this app's API workflow. |
| API Policy 6.3 and 7.4: deletion propagation and revocation | Deleted activities and revoked accounts require removal, including derivatives and controlled remote copies. | Approved deletion propagation, bounded refresh, expiry, and offline-device behavior. |

- **SEC-001**: Before live intake or release, record written permission or a qualified review establishing the permitted use, retention window, analytics scope, derivative handling, cross-device storage, and watch/calendar disclosure. A user checkbox, own API key, small athlete count, encryption, or local-only execution is not a substitute for permission. If unresolved, keep live intake disabled and stop the rollout; do not silently implement a different data source.
- **SEC-002**: Keep tokens and client secrets in the existing platform secure store; never in frontend storage, SQLite payloads, logs, URLs, screenshots, test fixtures, or source. Reuse existing macOS and iOS OAuth callback protections, state validation, cancellation, and refresh handling.
- **SEC-003**: Explicitly disclose activity import and optional location collection. Use read permission for import; request `activity:read_all` only with explicit private-activity consent and `activity:write` only when description writing is enabled. A token with read access alone is a supported configuration.
- **SEC-004**: Add fetch/expiry/provenance metadata to imported records and derivatives. Until a permission grants otherwise, cache age must not exceed seven days; cache refresh does not justify an indefinite historical archive or prohibited analytics. Do not ship the proposed historical import under these default terms.
- **SEC-005**: On explicit disconnect, provider revocation, account deletion, or a verified unavailable/deleted resource, hide affected Strava content immediately and execute the applicable purge. Reflect source activity deletion within 48 hours, expire unverified cached resources sooner when required, and complete revocation erasure within 30 days at the latest. Purge derivatives, queued descriptions, controlled calendar/Gist copies, and completion receipts according to the permission; do not use preserved watch aliases to launder Strava-derived fields.
- **SEC-006**: Remove `src-tauri/src/assistant.rs` from the production module/command surface, remove assistant settings and secure-store credentials after confirmed migration, and remove conversation/health data from controlled QR/Gist/calendar/description payloads. Retained structured prescription text is limited to training instructions and execution results. Do not send workout data to external AI endpoints.
- **CON-001**: Preserve established periodization, prescription generation, pace calculations, taper and volume bounds where independent of health. Remove physiological branches and their data dependencies explicitly. No replacement training-load, medical-risk or readiness algorithm is in scope. Activity-only escalation requires its own reviewed rule specification and absolute-value tests before enabling it; missing health is not an input or an eligibility state.
- **CON-002**: Keep Tauri entry URLs and training commands/events compatible through wrappers or additive DTO fields. Retired health/assistant commands return an explicit unsupported response without reading or writing data; do not retain functional health APIs for compatibility. Update Rust and TypeScript together. A legacy `sync_now(coros)` call must route to the selected activity source after migration, never trigger hidden watch downloads.
- **CON-003**: Preserve Project Prism and the current React production UI. Before later UI implementation read the four documents under `docs/design-system/` named in repository instructions and the applicable design-system, interaction, and responsive instruction files. Reuse shared controls, dialogs, overlays, tokens, and all eight localization catalogs; do not create a new visual system or Figma deliverable for this migration.
- **CON-004**: No hosted webhook receiver, Supabase schema, distributed OAuth broker, new watch vendor, subscription change, or deployment in the initial implementation. If permission requires webhooks to satisfy deletion guarantees, that is a separate approved scope gate, not an omitted requirement.
- **PAT-001**: Reuse `strava.rs` OAuth, HTTP/error handling and description queue; `calendar::reconcile`; `plan_sync` revision/conflict handling; and COROS's update-in-place plan builder. Introduce separate intake and delivery interfaces rather than another monolithic provider.

### Verified Starting Points

| Current owner | Verified behavior | Required change |
| --- | --- | --- |
| `src-tauri/src/providers/mod.rs`, `Provider` | One trait includes activity/health/fitness/track reads and plan push/cancel. | Separate intake and workout-delivery capabilities. |
| `src-tauri/src/commands.rs`, `run_full_sync_inner` | Imports watch activities, health, fitness, tracks; reviews adaptation; may re-push through the same provider. | Make orchestration source-independent and delivery nonblocking. |
| `src-tauri/src/strava.rs`, `Client::request`, `process_job`, `sync` | Reads remote activities to match and append descriptions; OAuth currently requires both read-all and write. | Add genuine intake and read-only scopes; retain opt-in descriptions. |
| `src-tauri/src/types.rs`, `ProviderKind` | Source enum currently has COROS and Garmin only. | Represent Strava activity provenance without making Strava a watch destination. |
| `src-tauri/src/store.rs` | Activity identity is `(provider, provider_record_id)`; completion links refer to local integer IDs. | Add athlete-scoped source aliases and deterministic canonical ownership without breaking local IDs. |
| `src-tauri/src/plan_sync.rs`, `account_id` | COROS subject drives channel discovery/encryption; completion wire carries provider identity; wire includes `pushed_id`. | Preserve legacy decryptability and add explicit watch-independent channel pairing and versioned provenance. |
| `src/desk/training.tsx` | Plan creation requires connected/synced provider; auto-push targets COROS and checks `plan.status`. | Gate on baseline evidence and use independent destination delivery state. |
| `src/desk/settings.tsx` | Watch panel imports history; Strava panel controls descriptions; device sync requires COROS. | Separate recorded-activity intake, watch calendar delivery, and device pairing. |

The starting-point references to health describe code to remove, not target capabilities. Only minimal migration readers may recognize retired fields to delete or reject them; they cannot expose a legacy health archive or recreate its schema.

### Target Flow And Ownership

```text
Watch -> vendor companion/cloud -> Strava -> activity intake -> local canonical activities
                                                           -> reconcile -> execution reviews

Runalytics plan + explicit edits -> local committed schedule -> COROS delivery -> watch
                                                         -> iCalendar publishing (optional)
                                                         -> encrypted device sync (optional)
```

Strava owns the remote recording; Runalytics owns prescriptions, user edits and explicit match decisions; COROS owns execution of the delivered workout. A recording arriving late must not rewrite the original prescription. Delivery success means the remote service accepted and verified the expected schedule, not that hardware has downloaded it. The vendor companion app remains responsible for the last hop to the watch.

### Training-Only Evidence Contract

| Feature | Behavior after switch |
| --- | --- |
| Running history, distance, duration, pace, elevation | Use canonical Strava activity fields within the approved storage/use permission. Persist elapsed and moving time distinctly; never reinterpret one as the other. |
| Manual/treadmill run | Import without inventing GPS or repetitions. Show summary; abstain from track-dependent grading. |
| Maps and detailed repetition grading | Fetch required streams only with explicit GPS opt-in; missing, privacy-trimmed, or mismatched streams mean insufficient evidence, not a failed session. |
| Baseline weekly mileage | Use verified, deduplicated run history when authorized; otherwise require explicit runner input. An incomplete import is not a measured zero-mileage history. |
| Threshold pace and race estimates | Require a confirmed manual threshold or eligible benchmark supported by sufficient timing/distance evidence. Do not silently take a stale COROS assessment or label a fallback as measured fitness. Existing plans retain their stored threshold. |
| Health, pain, injury, physiological fitness/load and assistant features | Removed end to end, including stored history, routes, settings, calculations, commands and exported derivatives after confirmed migration. No archive or unavailable placeholder remains. |
| Session feedback | Retain structured non-clinical workout difficulty/completion feedback and training notes. Remove pain questions, body locations and health-derived fields; do not analyze free-text notes as health evidence. Existing training-only approval, cooldown, race/taper, maximum-offset and completed-session safeguards remain. |
| Planned progression and explicit edits | Work from the authored plan and runner-confirmed inputs; do not require a health connection, recovery score or vendor-load measurement. |
| Automatic/offered harder training | Off until deterministic workout-only eligibility rules are separately specified and validated. Easing and customization within the existing prescription remain available without physiological gates. |
| Training progress | Show completion, adherence, planned/completed distance and duration, and eligible pace/benchmark results only when policy permits. No health-risk, readiness, recovery, VO2max or replacement load score. |

## 2. Implementation Steps

Tasks are ordered within each phase unless their descriptions explicitly allow independent execution. A phase starts only after its listed dependencies pass, except the isolated synthetic-only TASK-004 preparation recorded above. Task rows reflect implementation status; planning research does not count as implementation. A completed preparatory task does not mark its phase or the migration complete.

### Implementation Phase 0

- **GOAL-001**: Resolve live-data policy and pin the release boundary. Dependency: none. Exit: written decision with evidence and permitted retention/use values; otherwise subsequent live-data tasks remain blocked.

| Task | Description | Completed | Date |
| --- | --- | --- | --- |
| TASK-001 | Record the SEC-001 decision and exact permission reference in this document. Cover historical activity storage, derivatives, adaptive coaching, manual-data combination, GPS, same-user Gist sync, calendar/watch transfer, optional description writes and deletion. If permission is not granted, leave status On Hold; only synthetic-data work may continue. | No | Not started |
| TASK-002 | Record allowed OAuth scopes, app athlete capacity, applicable subscription requirements and header-reported rate limits without printing credentials. Verify real representative activity/detail/lap/stream responses only after TASK-001 authorizes them; sanitize fixtures and map each retained field to a purpose. Do not promise free or unlimited Strava access. | No | Not started |
| TASK-003 | Capture current focused Rust/UI test baselines and a disposable synthetic SQLite fixture containing an active pushed plan, saved workout feedback, completed prescription, duplicate candidate, legacy health/pain/conversation records and legacy encrypted channel. The removed-data fixture exists only to verify erasure and non-resurrection. Record pre-existing failures; never use the runner's live DB as a fixture. | No | Not started |

### Implementation Phase 1

- **GOAL-002**: Separate intake, delivery and their state without switching existing runners. Dependencies: TASK-003; synthetic implementation can proceed while TASK-001 is pending. Exit: compile-time and spy-test proof that migrated watch paths cannot read recorded data.

| Task | Description | Completed | Date |
| --- | --- | --- | --- |
| TASK-004 | In `providers/mod.rs`, introduce separate intake and delivery capability contracts with shared connection/authentication; adapt COROS and the Garmin placeholder without duplicating the existing builders. Store sources and destinations independently; narrow all output callers to `WorkoutDestination`. Verify a destination can register/push/cancel without implementing intake, and replacement does not duplicate it. Preserve legacy read behavior until the later explicit mode switch. | Yes | 2026-11-11 |
| TASK-005 | In `types.rs`, `src/lib/types.ts` and `src/lib/ipc.ts`, separate activity source (`Strava` added to source provenance) from destination kind (COROS now; Garmin unavailable). Add additive intake status and delivery status DTOs. Do not dispatch Strava through watch-provider construction or deserialize unknown legacy providers as COROS. | No | Not started |
| TASK-006 | In `store.rs`, add idempotent schema migration/repair for source account, external aliases, intake checkpoints, data provenance/expiry, a workspace intake-mode setting, and delivery state. Delivery is keyed by workspace, destination account and plan, with remote ID, desired revision/hash, confirmed revision/hash, state, attempt time and sanitized failure. Never store tokens or raw response bodies in these tables. | No | Not started |
| TASK-007 | Preserve current command names via wrappers in `commands.rs` and register additive commands in `lib.rs`. Define `sync_activity_source`, `workout_delivery_status` and explicit delivery retry. Leave the old intake mode unchanged until the migration confirmation; after switch, every old sync entry point must resolve to Strava. | No | Not started |

### Implementation Phase 2

- **GOAL-003**: Import authenticated Strava recordings reliably and resumably. Dependencies: GOAL-002 and TASK-001 for real data. Exit: fake HTTP tests pass for read-only auth, incremental import, interrupted backfill, quotas and malformed responses.

| Task | Description | Completed | Date |
| --- | --- | --- | --- |
| TASK-008 | Extend `strava.rs` credentials/status and OAuth scope validation to distinguish read authorization from optional write authorization. Reuse the operation lock and platform OAuth implementation, serialize refresh across importer/writer, persist rotated refresh tokens before reuse, and require a separate workspace migration on athlete mismatch. Do not share refresh tokens through QR/Gist; each device authorizes independently. | No | Not started |
| TASK-009 | Add `src-tauri/src/strava/intake.rs` for summary/detail DTO mapping and bounded import. Preserve UTC start, local date/offset, sport type, elapsed time, moving time, distance, elevation, source device attribution and exact external ID. Accept finite nonnegative measurements; reject invalid identities; discard prohibited/unneeded health fields. Map running, trail and virtual runs explicitly and preserve non-running sport identity without including it in running mileage. | No | Not started |
| TASK-010 | Implement resumable initial history import capped to 180 days, subject to TASK-001 retention permission; never imply this cap overrides policy. Use fixed `before` boundaries, `per_page=100`, a maximum of 10 list pages and 40 total HTTP calls per foreground sync, then persist pending progress. After bootstrap, re-read a seven-day overlap and maintain a separate older-history cursor. Report incomplete coverage; do not advance the verified watermark across a failed page. | No | Not started |
| TASK-011 | Share application/read quota state between intake and description writes in `strava.rs`. Respect both rate-limit header families, Retry-After and UTC reset boundaries; stop on exhausted quota, refresh authentication once on 401, distinguish 403 scope/access failure from revocation, and retry transient failures with bounded backoff. Prioritize new activity summaries, then required detail, then optional streams, then description writes. | No | Not started |
| TASK-012 | Add lazy stream/lap enrichment in `strava/intake.rs`, `store.rs` and `laps.rs`. Validate aligned stream lengths, finite coordinates and increasing elapsed timestamps; retain accumulated distance/moving flags so pauses and privacy-trimmed sections are represented correctly. Extend track evidence additively or abstain where the existing GPS-only track model cannot represent it. Keep moving time for summary pace separate from elapsed-time interval boundaries. Missing GPS never becomes fabricated coordinates or an automatic negative grade. | No | Not started |

### Implementation Phase 3

- **GOAL-004**: Migrate without duplicate runs, lost completions or destructive history changes. Dependencies: GOAL-003. Exit: disposable migration fixtures prove local activity IDs and completed prescriptions survive; the switch is atomic and restart-safe.

| Task | Description | Completed | Date |
| --- | --- | --- | --- |
| TASK-013 | In `store.rs` and `strava/intake.rs`, bind exact athlete/Strava IDs first, then exact previously verified `strava_exports.target_id` links. For remaining legacy rows, propose candidates only when sport agrees, start differs by at most 120 seconds, and duration/distance satisfy the existing `matches_activity` predicate. Ambiguous candidates remain pending review and are excluded from totals until resolved; do not silently count both or destructively merge by timestamp alone. Preserve deliberate manual match decisions. | No | Not started |
| TASK-014 | Canonicalize each confirmed pair in a single `store.rs` transaction while retaining the existing local activity ID and adding the Strava alias. Preserve `calendar_completions`, original notes, saved prescriptions and user feedback. Keep independent legacy provenance rather than relabeling all historical rows Strava; invalidate derived caches only when their inputs change. Treat the Strava athlete as part of identity, not merely a global `provider_record_id`. | No | Not started |
| TASK-015 | Add an explicit migration preview/status in `commands.rs`: athlete identity, imported date coverage, counts of new/paired/ambiguous records, missing optional streams, health/pain/conversation data and settings to be permanently removed, and remaining delivery connection. Confirm removal separately from connecting Strava; support confirmed cleanup for manual-plan users without Strava authorization. Commit the product-mode switch only after local cleanup and transaction success. Persist resume state and never mark remote cleanup or first import complete while work remains pending. | No | Not started |
| TASK-016 | Update `calendar.rs` reconciliation and the completion wire in `plan_sync.rs` to resolve canonical external aliases across devices while retaining stable workout/calendar IDs. Revalidate auto-matches after source corrections; preserve explicit matches unless source identity is deleted. A matched source deletion removes its recording/receipt and marks verification unavailable without destroying the authored prescription or silently scheduling a past session for repetition. | No | Not started |

### Implementation Phase 4

- **GOAL-005**: Remove health end to end and make training planning independent of it. Dependencies: GOAL-004 for migrated workout identity; cleanup also supports existing/manual-only plans without live Strava intake. Exit: confirmed cleanup erases legacy health data, no production health surface remains, and training creation/edits/easing work with zero health records or gates.

| Task | Description | Completed | Date |
| --- | --- | --- | --- |
| TASK-017 | In `store.rs`, `metrics.rs`, `commands.rs`, `types.rs`, Rust module registration and TypeScript DTOs, remove health-specific models, scores, queries and payload fields. Implement confirmed transactional schema cleanup for `health_daily`, `fitness_assessment`, `training_load`, pain/message records, health-only scores/proposals and activity/track physiological columns. Sanitize mixed completed receipts, feedback, proposals and caches while retaining the original training prescription and workout identity. Inspect serialized payloads as well as dedicated columns. Remove old schema-repair paths that would recreate retired tables; reject/sanitize legacy imports on every device. Clear app-controlled cached files and secure-store assistant credentials; checkpoint remote erasure separately and never claim it complete early. No health archive remains. | No | Not started |
| TASK-018 | In `commands.rs` plan creation and `plan/engine.rs`, require verified history coverage or explicitly confirmed weekly-mileage input, and an explicit valid threshold source. Preserve existing plans' threshold and baseline. Remove connected-watch/last-watch-sync requirements, but do not manufacture evidence from empty history, short backfills or the absence of an error. | No | Not started |
| TASK-019 | In `plan/adaptive.rs`, `plan/pacing.rs`, `plan/difficulty.rs` and shared evidence callers, remove pain/recovery/health/vendor-load branches and their eligibility predicates. Preserve ordinary plan progression, explicit runner edits, easing from workout execution/difficulty feedback, cooldowns, immutable historical prescriptions, review approvals, revision checks and race/taper/volume constraints. Disable automatic/offered intensity or volume increases until a separate deterministic workout-only rule specification is approved; do not leave references to missing health as permanent blockers. Retire health-derived pending proposals rather than applying them. | No | Not started |
| TASK-020 | In `calendar.rs`, `plan/receipt.rs`, `commands.rs`, `assistant.rs`, `lib.rs` and Strava description generation, remove health/risk narrative and assistant invocation paths. Rebuild app-owned published descriptions/calendars/Gist snapshots as training-only payloads, respecting revision history and preserving third-party/user-authored text; require review where ownership of a legacy text block is ambiguous. Remove health/conversation transfer fields and reject old imports. Apply SEC-006 and record inaccessible remote cleanup as pending. | No | Not started |

### Implementation Phase 5

- **GOAL-006**: Deliver the committed schedule independently of activity intake and foreground navigation. Dependencies: GOAL-002; integration requires GOAL-005. Exit: failed/offline delivery is retryable and never creates duplicate remote plans or claims hardware completion.

| Task | Description | Completed | Date |
| --- | --- | --- | --- |
| TASK-021 | In `commands.rs`, replace provider-owned `run_full_sync_inner` with stages for device-plan refresh, Strava import/enrichment, reconciliation, evidence review, local publication and independent watch delivery. Run reviews once for the committed canonical revision, never against partially imported pages. Set health days to 0 and fitness-updated false in compatibility reports; emit distinct activity-import, description-write and watch-calendar stages. | No | Not started |
| TASK-022 | Add `src-tauri/src/providers/delivery.rs` as the persistent delivery coordinator over TASK-006 state. Enqueue on create, move, skip, benchmark, feedback/customization, accepted proposal, imported plan, completion reconciliation, abort and clear. Serialize sends per destination account, snapshot the desired hash, and mark only that exact snapshot confirmed. A later local edit or tombstone supersedes an older in-flight push; replaying the same hash is a no-op. | No | Not started |
| TASK-023 | Route all direct push/cancel paths in `commands.rs` through the coordinator, including `push_plan_to_provider`, schedule-change refresh and proposal application. Make `training.tsx` auto-push an optional status refresh/retry trigger, not the sole delivery mechanism. Represent authored-plan lifecycle separately from delivery; retain `PlanStatus::Pushed` only as a legacy compatibility view and never as proof that the current revision is delivered. | No | Not started |
| TASK-024 | Keep `providers/coros.rs` single fixed-name editable plan, remote-ID adoption, upward-only supported week sizing and blank-before-retire cancellation behavior. Read back the accepted schedule or verifiable provider revision before confirming service delivery. Test cancel during pending push, cancel with no remote ID, abort offline and retry after authentication recovery; never delete recorded watch activities. | No | Not started |
| TASK-025 | Make the delivery coordinator safe across desktop/phone: use existing encrypted-channel conflict checks before publishing, compare current local revision after every await, and adopt the same remote plan ID. Do not treat a local mutex as a cross-device lock. If cross-device concurrency cannot be proven safe, designate one explicitly paired publishing device per destination and reject writes from others until ownership is transferred. | No | Not started |

### Implementation Phase 6

- **GOAL-007**: Remove watch dependency from device identity and implement provider-data lifecycle. Dependencies: GOAL-004 and GOAL-006; real data remains subject to TASK-001. Exit: paired no-watch devices sync correctly, legacy channels survive, and revocation cannot resurrect deleted source data.

| Task | Description | Completed | Date |
| --- | --- | --- | --- |
| TASK-026 | In `plan_sync.rs`, add a versioned random workspace identifier and cryptographically random pairing secret for new channels, held in secure storage and explicitly transferred between devices. Verify the same authorized Strava athlete before attaching Strava-backed state. Never derive a new encryption secret solely from a public Strava athlete ID or COROS subject, and do not share OAuth refresh tokens. | No | Not started |
| TASK-027 | Implement legacy channel migration in `plan_sync.rs` and connection-transfer handling in `calendar_publish.rs`: decrypt using the existing COROS path, create a versioned new channel, verify a successful round trip on the paired device, then switch pointers. Do not overwrite legacy ciphertext in place or make watch disconnect destroy the last decryption route. Migrate clear/reset tombstones and completion provenance; retain compatibility or require both devices upgraded before switching. | No | Not started |
| TASK-028 | In `store.rs`, `strava.rs`, `calendar.rs` and `plan_sync.rs`, implement provenance-specific expiry/deletion and account-revocation tombstones with idempotent cascading cleanup. Run expiry before displaying cache, before sync and on foreground resume. Re-fetch imported resource identity on a bounded rotation that meets the 48-hour deletion requirement; if verification expires while offline, hide it and exclude its derivatives. Pending remote erasure must remain visible and retryable, not reported as complete. | No | Not started |
| TASK-029 | Model account/source switch, explicit disconnect, write-scope loss and verified remote deauthorization separately. Confirm destructive effects with accurate copy; do not delete user-authored training plans or training notes by accidental scope error. Quarantine conflicting account data; prevent QR/Gist or older app versions from restoring tombstoned source records or removed health data. Verify third-party deletion requirements against TASK-001 before preserving any derivative. | No | Not started |

### Implementation Phase 7

- **GOAL-008**: Present a training-only product with separated intake and delivery connections. Dependencies: GOAL-005 through GOAL-007. Exit: no health/journal/assistant destinations remain; desktop/mobile synthetic IPC tests pass, with all catalogs and established Prism contracts preserved.

| Task | Description | Completed | Date |
| --- | --- | --- | --- |
| TASK-030 | In `src/desk/settings.tsx`, promote Strava to recorded-activity connection with read scope, athlete, history coverage, last successful import, retry and missing-private-permission states. Keep description writes a separate opt-in. Change watch connection to training-calendar delivery with remote account, desired/confirmed revision, retry/cancel state and service-confirmed timestamp; remove history import actions and health copy. | No | Not started |
| TASK-031 | In `src/desk/training.tsx`, `data.tsx`, `settings.tsx` and the production router/navigation owners, replace watch gates with workout/manual-input gates. Show optional watch destination independently; support no-watch/manual-plan users, incomplete imports and offline authorized data. Remove Health, injury journal/body map, assistant, physiology settings, risk/recovery cards and unused assets/components. Place eligible pace/benchmark reviews under Training; retain workout difficulty feedback only. Redirect retired URLs to Training and remove dead actions, subscriptions and fetches. Reuse Prism sections/controls rather than redesigning the visual system. | No | Not started |
| TASK-032 | Update `src/lib/ipc.ts`, `src/lib/types.ts`, `tests/desk.fixture.mjs`, relevant `tests/desk.spec.mjs`, `tests/prism.spec.mjs`, `tests/i18n.spec.mjs`, all `src/desk/locales/*.json`, `README.md` and `docs/i18n.md`. Correct troubleshooting to vendor-to-Strava recording flow and direct watch delivery; explain that service acceptance is not hardware receipt. Update compatibility fixture call sites without reviving legacy controllers as production. | No | Not started |

### Implementation Phase 8

- **GOAL-009**: Ship only after synthetic, native, migration and privacy gates pass. Dependencies: all previous goals and explicit live-data permission. Exit: every release criterion below passes or rollout remains blocked with specific evidence.

| Task | Description | Completed | Date |
| --- | --- | --- | --- |
| TASK-033 | Execute TEST-001 through TEST-012, record exact commands/results and distinguish new failures from TASK-003 baseline. Run `npm run build`, `npm run test:i18n`, `npm run test:ui` and `cargo test --manifest-path src-tauri/Cargo.toml --lib --offline`. Report pre-existing failures explicitly; do not silently skip required regressions or describe synthetic IPC as native testing. | No | Not started |
| TASK-034 | Perform authorized native macOS/iOS acceptance with one COROS watch and one independently recorded Strava source: import, duplicate reconciliation, offline local edit, failed delivery/retry, guided interval execution and completed-run return. Inspect actual watch step order/pace/date and confirm no health or watch-activity API calls. Cross-watch intake alone does not prove cross-watch workout delivery. | No | Not started |
| TASK-035 | Release after all gates with explicit existing-user confirmation for the training-only migration and separate Strava intake consent. Verify one workspace and two paired devices before defaulting new installs. Do not create an app-managed backup retaining deleted health; any separately requested user export precedes erasure, is outside live stores/sync, follows retention policy and is not automatically restored. Rollback preserves the training-only schema and disables intake/delivery as needed; it must not re-enable health, old watch downloads or revoked data. | No | Not started |
| TASK-036 | After migration acceptance, remove unreachable watch-intake and health/assistant code, obsolete models, dependencies, commands, settings, routes and actions; retain only isolated synthetic fixtures and minimal migration recognizers needed to reject/sanitize retired fields. Record supported devices, training-only limits, release evidence and policy permission. Preserve historical workout records and legacy channel access only within their explicit migration/retention criteria; do not preserve health history. | No | Not started |

### Release Acceptance Checklist

- A runner with Strava and no COROS account can supply a valid baseline, create/edit a plan and review eligible imported runs.
- Connected COROS is used only for authentication/account and scheduled-workout operations; tests fail on any watch activity/health/fitness/FIT call.
- Exactly one canonical run appears in totals and calendar after migrating a watch recording already present on Strava.
- Imported records and saved completions are scoped to the correct athlete, respect retention/deletion, and never enter an AI request.
- A local schedule change commits offline; COROS shows pending/error independently; retry updates the same remote plan without duplication.
- Completed training prescriptions remain immutable; missing workout streams mean insufficient execution evidence. Health-derived annotations are removed without altering prescribed steps.
- No production health models, collection, storage, calculations, APIs, panels, journal, assistant, export fields or legacy archive remain after confirmed migration; stale device imports cannot restore them.
- Plan creation, planned progression, calendar edits and workout-based easing work with zero health records. No eligibility gate waits for removed health data, and no automatic/offered escalation occurs without approved workout-only rules.
- Device sync works without a watch; legacy channel migration, reset tombstones, stale-device rejection and destination ownership are verified.
- At least one actual watch executes the delivered structured workout correctly. UI tests and remote API acceptance alone do not satisfy this gate.
- Current Strava use/storage permission is recorded. An unresolved permission, deletion guarantee or required subscription is a release blocker.

## 3. Alternatives

- **ALT-001**: Strava for both intake and workout delivery. Rejected: the public API reference does not provide arbitrary structured workout scheduling onto watches.
- **ALT-002**: Keep watch activity/health import as an automatic fallback. Rejected: violates the requested output-only watch role and reintroduces inconsistent sources and duplicate runs. A future change requires explicit approval.
- **ALT-003**: Use iCalendar or GPX as universal guided-workout delivery. Rejected: calendar events carry appointments and GPX carries routes, not a universally supported structured workout schedule. Keep iCalendar as a separate supported convenience.
- **ALT-004**: Add FIT workout export, Garmin, Apple WorkoutKit, Suunto or a third-party delivery hub now. Deferred: separate vendor/API authorization, compatibility and device-testing work; not required to preserve current COROS delivery.
- **ALT-005**: Keep a health archive, pain assistant, unavailable readiness cards or a replacement wellness score. Rejected: the requested product is training-only. Remove those surfaces and dependencies; retain prescription/execution safeguards, not physiological gates.
- **ALT-006**: Use public athlete IDs as new device-sync encryption keys. Rejected: IDs establish labels, not secrets; use random pairing material and explicit legacy migration.
- **ALT-007**: Circumvent Strava restrictions with own keys, scraping, repeated cache refresh, anonymization, user consent or derived summaries. Rejected: these do not establish permission for the requested processing/storage model.
- **ALT-008**: Add a hosted webhook/OAuth platform as part of this migration. Deferred: changes hosting, credentials, privacy and operations. Reassess separately if deletion guarantees or multi-user scale cannot be met without it.

## 4. Dependencies

- **DEP-001**: TASK-001 policy decision and any required written Strava permission. No provider-data storage/analysis assumption supersedes this dependency.
- **DEP-002**: Strava app authentication, read scope, approved athlete capacity and applicable subscription tier. Read quota is shared across devices using the same app; per-device budgets alone cannot ensure fleet-wide capacity.
- **DEP-003**: Vendor-to-Strava upload configured by the runner. Runalytics cannot ingest a recording that has not arrived in Strava; account/device upload delays must be reported rather than blamed on watch delivery.
- **DEP-004**: Existing COROS OAuth/MCP access with plan/course capability, compatible watch and companion-app sync. The provider may expose broader scopes than the app needs; enforce output-only operations in code even if granular authorization is unavailable.
- **DEP-005**: Existing Rust/Tauri, rusqlite, reqwest, chrono, secure-store and plan-sync cryptographic facilities; no new runtime library is presumed necessary. Any schema/crypto dependency must follow the existing lockfiles and audited APIs.
- **DEP-006**: Existing Prism components, eight-locale checks, synthetic IPC fixtures and macOS/iOS native toolchains. Real watch access is required for release evidence, not for synthetic implementation.
- **DEP-007**: Before publishing Strava-derived content through iCalendar/Gist/watch delivery, TASK-001 must authorize that disclosure and define remote erasure. If it does not, those payloads must exclude such derivatives and their related features remain disabled.

## 5. Files

- **FILE-001**: `src-tauri/src/providers/mod.rs`, `providers/coros.rs`, `providers/garmin.rs`: split capability contracts, preserve COROS delivery and leave Garmin unavailable.
- **FILE-002**: Proposed `src-tauri/src/providers/delivery.rs`: durable delivery coordinator; not implemented by this planning change.
- **FILE-003**: `src-tauri/src/strava.rs`, `src-tauri/src/strava/ios_auth.rs`, proposed `src-tauri/src/strava/intake.rs`: shared OAuth/client, bounded importer and existing description writer. Keep platform auth in place unless scope handling requires a focused edit.
- **FILE-004**: `src-tauri/src/store.rs`, `types.rs`: additive source identity, aliases, expiry/provenance, checkpoints, evidence DTOs and delivery state; embedded SQLite migration/repair, not a Supabase migration.
- **FILE-005**: `src-tauri/src/commands.rs`, `lib.rs`: command registration, sync orchestration, compatibility wrappers and all schedule mutation/delivery paths.
- **FILE-006**: `src-tauri/src/calendar.rs`, `calendar_publish.rs`, `plan_sync.rs`: reconciliation, subscription publishing, connection transfer, encrypted channel migration and deletion/tombstone propagation.
- **FILE-007**: `src-tauri/src/metrics.rs`, `laps.rs`, `plan/receipt.rs`, `assistant.rs`, `plan/engine.rs`, `plan/adaptive.rs`, `plan/pacing.rs`, `plan/difficulty.rs`: remove health/assistant logic and physiological gates; retain training-only calculations, immutable execution review and prescription safeguards. Remove obsolete module registration and dependencies after caller cleanup.
- **FILE-008**: `src/lib/types.ts`, `src/lib/ipc.ts`, `src/desk/settings.tsx`, `training.tsx`, `data.tsx`, `src/desk/locales/*.json`, production router/navigation and health/journal/assistant view owners: remove retired destinations/data fields, preserve Training and its pace reviews, and localize the migration/delivery states. Resolve exact view-owner paths before their implementation edits.
- **FILE-009**: Existing inline Rust test modules and `tests/desk.fixture.mjs`, `tests/desk.spec.mjs`, `tests/prism.spec.mjs`, `tests/i18n.spec.mjs`: focused regressions and synthetic workflows, reusing current test files where practical.
- **FILE-010**: `README.md`, `docs/i18n.md` and this plan: setup, privacy/retention, scope, device limitations and execution evidence. Do not edit credentials, deployment workflows or hosted infrastructure as part of this plan.

## 6. Testing

- **TEST-001**: Provider-boundary spies: every migrated manual/background/legacy-command sync and every delivery retry performs zero watch activity/health/fitness/FIT calls. Destination push/cancel/auth/plan verification remain allowed. Run focused Rust module tests for providers and commands.
- **TEST-002**: Strava fake HTTP: read-only OAuth works; private scope is optional/explicit; description write is separately gated; wrong athlete/state is rejected; refresh rotation is serialized; 401/403/404/429/network failures are distinct; both quota families stop calls; retries never loop without a budget.
- **TEST-003**: Import mapping: elapsed/moving seconds, metres and seconds-per-km magnitudes are pinned; running/trail/virtual/manual/treadmill/multisport, DST, date offsets, zero distance, NaN/negative values, elevation, Garmin device attribution, missing/trimmed/misaligned streams and long pauses are covered. Assert health fields are not persisted.
- **TEST-004**: Backfill/checkpoints: more than one page, exactly full last page, page failure/resume, restart, delayed upload in overlap, old activity update, duplicate repeated page, quota pause and incomplete coverage. Listing absence on a failed/partial page must never delete an existing row.
- **TEST-005**: Migration: fresh DB, existing DB, replayed repair, paired IDs, ambiguous similar races, two recordings at the same start, explicit manual matches, changed source values, account switch and transaction rollback. Assert stable local IDs, exactly-once totals, notes, completed prescriptions and feedback.
- **TEST-006**: Training-only lifecycle: confirmed migration erases health tables/columns, mixed payload annotations, pain/conversation data, settings and assistant keys without losing prescribed steps, workout IDs or training feedback. Restart/repair, stale QR/Gist imports and rollback cannot recreate removed data. With zero health records, manual baseline/threshold creation, ordinary planned progression, explicit edits, easing, cooldowns, benchmarks and revision guards work. Missing streams abstain from grading; automatic/offered harder changes stay off pending approved workout-only rules.
- **TEST-007**: Delivery state machine: unchanged payload, move/skip/benchmark, completed run, active edit during push, offline retry, scope loss, cancellation racing push, abort/clear, unknown remote ID, legacy Pushed status and remote verification mismatch. Confirm one remote COROS plan, correct dates/steps and no stale acknowledgement.
- **TEST-008**: Two-device encrypted sync: no-watch new workspace, independent Strava authorization, legacy COROS channel round trip, failed re-encryption, older client compatibility, conflicting athlete, concurrent edits, single publishing-owner fallback and reset/revocation tombstones. Assert no OAuth token transfer and no public-ID-only new secret.
- **TEST-009**: Privacy lifecycle: TTL before display, 48-hour source deletion deadline, missing access vs verified deletion, immediate hidden state, disconnect/revocation purge, stale offline device, remote erasure failure/retry and pending writer cleanup. Instrument calendar/Gist/watch/description payloads to assert no health fields, physiological scores or secrets; assert zero assistant/LLM requests and no restoration of health data from previous revisions. Ambiguous user-authored text is not silently rewritten.
- **TEST-010**: Existing Playwright suites with synthetic IPC: training-only navigation; retired health/injury/assistant URLs redirect without data calls; no health placeholders/settings remain; pace reviews remain reachable under Training; Strava/manual onboarding, explicit private/GPS consent, no-watch plan creation, removal confirmation, import retry, separate delivery failure, scope loss, migration ambiguity, retained draft, keyboard focus, translations and compact/desktop viewport matrix. Use artifacts outside the workspace and existing preview conventions.
- **TEST-011**: Final regression commands: `npm run build`; `npm run test:i18n`; `npm run test:ui`; `cargo test --manifest-path src-tauri/Cargo.toml --lib --offline`; `git diff --check`. First use narrower Rust filters and Playwright grep selections for each touched slice. Document baseline failures rather than expanding scope to fix unrelated tests.
- **TEST-012**: Authorized native/hardware acceptance on macOS and iOS: OAuth callbacks, secure-store refresh, same-user device migration, actual COROS guided-workout receipt/execution/cancel, vendor-to-Strava result return and import. Record service-side acceptance and hardware-side receipt separately; no claim of native coverage from browser fixtures.

## 7. Risks & Assumptions

- **RISK-001**: Published Strava restrictions may make the requested long-lived analytics architecture unavailable without written permission. This is an actual release blocker, not a generic legal disclaimer. On Hold remains correct until resolved.
- **RISK-002**: Strava is an intermediary: missing source uploads, privacy trimming, absent streams/laps and elapsed/moving differences reduce review fidelity. Show evidence limits and abstain rather than inventing raw FIT parity.
- **RISK-003**: Health fields are embedded in schema repair, activity/track rows, completed receipts, proposals, settings, legacy routes and encrypted remote snapshots, not just dedicated tables. GOAL-005 must remove those dependencies and sanitize mixed payloads without altering training prescriptions; hiding cards or filtering queries is insufficient.
- **RISK-004**: Incorrect deduplication can double mileage or break completions. Transactional aliasing, stable local IDs, ambiguity review and provenance-specific purge are mandatory.
- **RISK-005**: Existing channels derive identity/encryption inputs from COROS. An automatic identity change can strand encrypted plans or create parallel channels; explicit verified migration and pairing are mandatory.
- **RISK-006**: COROS has limited remote plan lifecycle controls. Cancellation may blank workouts while retaining a provider plan entry. Do not promise deletion that its API cannot perform.
- **RISK-007**: Background iOS execution and a closed desktop cannot guarantee prompt polling or watch sync. Enforce cache freshness before use and do not promise real-time import. If publication-side deletion cannot meet its deadline, block that feature or request separate hosted-webhook scope.
- **RISK-008**: Multi-device delivery can overwrite a newer calendar despite local locks. Prove serialized ownership or use the explicit single-publisher fallback; encrypted plan sync alone is not a remote compare-and-swap guarantee.
- **RISK-009**: A read-only Strava token and current mandatory-write OAuth implementation are incompatible until GOAL-003 lands. Do not make the new intake switch available prematurely.
- **RISK-010**: Revocation and retention can invalidate completion evidence and plan-derived values. Preserve independently authored prescriptions and user-entered records, but permission governs which derived fields must disappear. Encryption does not remove deletion obligations.
- **ASSUMPTION-001**: "Push the calendar" means native, executable structured workouts as currently delivered to COROS, not only phone-calendar appointments.
- **ASSUMPTION-002**: Existing COROS users keep their connection for output only; no additional watch brand is required in the first release. Broad Strava intake support must not be described as broad watch-delivery support.
- **ASSUMPTION-003**: "Completely removing health data" includes the pain/injury journal, assistant, physiological history, scores, APIs, settings and exports, not only watch sensors. Non-clinical workout feedback and authored training notes remain; free text is not automatically classified or mined for health. Workout recovery intervals/easier weeks remain ordinary plan instructions.
- **ASSUMPTION-004**: Existing personal client-ID/secret setup may remain for the initial approved personal deployment. A consumer onboarding broker and fleet-scale quota coordination are separate work, not presumed solved by this plan.
- **ASSUMPTION-005**: New or migrated channel secrets can be transferred through an explicit secure pairing flow. No new hosted identity system is necessary for the requested local-first scope.

## 8. Related Specifications / Further Reading

- [Repository guidance](../.github/copilot-instructions.md)
- [Norwegian threshold model](../docs/norwegian-threshold-model.md)
- [Runna-like plan model](../docs/runna-like-plan-model.md)
- [Localization](../docs/i18n.md)
- [Strava API reference](https://developers.strava.com/docs/reference/)
- [Strava OAuth](https://developers.strava.com/docs/authentication/)
- [Strava rate limits and athlete capacity](https://developers.strava.com/docs/rate-limits/)
- [Strava webhook semantics](https://developers.strava.com/docs/webhooks/)
- [Strava API Agreement](https://www.strava.com/legal/api)
- [Strava API Policy, especially 5.2-5.5, 6.2-6.3 and 7.4](https://www.strava.com/legal/api_policy)

Policy references were fetched on 2026-11-11. Recheck their current text before executing TASK-001. This plan records engineering constraints and is not legal advice or a claim of provider approval.