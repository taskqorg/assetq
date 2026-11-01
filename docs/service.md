# Service, lifecycle and storage

This page specifies the first AssetQ service build: one Rust binary (`assetq`) with an HTTP API, embedded or separate executors, administration commands and an API client. It implements [assetq.md](../assetq.md) R2-R7 with a synthetic provider only. The real image-worker and Meshy methods attach in later tasks. Deployment and rollback: [deploy](deploy.md).

## Lifecycle model

Attempt IDs are persisted separately from job IDs and effect IDs; each attempt records its fence, owner and start/end. `src/model.rs` is the one executable job/attempt/effect model. Every job change is an event applied by `model::apply`. PostgreSQL stores the resulting state next to an append-only event log. `assetq audit <job>` replays the log and compares the result with the stored state.

- **Job phases:** `queued`, `active` (claimable when unleased), `waiting` (provider work outstanding, no lease), `succeeded`, `failed`, `cancelled`.
- **Attempt:** each claim advances the fence by one and records a lease owner. An event from an older fence is refused as stale. An expired lease becomes `lease_expired`, then a new claim. Executors renew live leases throughout blocking provider calls, streamed imports and hash verification, without holding a job row lock during filesystem work. Renewal never revives an expired lease. Database time rejects fenced writes after lease expiry even before another claim. A PostgreSQL session advisory lock serializes dispatch/reconciliation and poll/cancel calls for one job; loss of that session leaves submission uncertain, never authorizes replay while the previous call may still arrive.
- **Effect:** `none`, `intended` (intent persisted before the provider call), `submitted` (operation ID recorded), `uncertain` (reply lost), `done`, `failed`. A new dispatch requires `none`. An intended or uncertain effect is reconciled by provider lookup of its effect ID: a found operation is adopted, and only a confirmed sealed absence (the old effect can no longer be accepted) allows one new dispatch. A provider that cannot establish that barrier must return unknown; an empty eventually consistent lookup is not confirmed absence.
- **Cancellation:** `requested`, `provider_confirmed`, `locally_stopped`, `uncertain`. An unconfirmed provider cancel keeps the charge unknown with a reason, stays `waiting` without an executor lease, and retains project active-job and spend reservations. Observation of the same operation continues after restart until confirmed stop, completion or failure. Completion after requested or uncertain cancellation stops locally without importing results; only a terminal observation releases active-job capacity.
- **Completion:** `succeeded` requires every step done, every expected output imported and a manifest commit in the same transaction as the transition. Duplicate provider completions and late events after a terminal phase are no-ops.

Replay traces in `tests/traces/` cover the happy path, crash after intent, lost reply (found and absent), duplicate input and events, stale executors, partial import, full store and four cancellation paths. `assetq selftest` replays the same embedded traces.

## Execution

A method version declares a bounded step list (at most 16) from `synthetic.generate`, `provider.submit`, `provider.import` and `validate`. This first executor accepts exactly one generation followed by import (for a provider) and validation; unsupported sequences are refused rather than claiming general pipeline support. Every successful manifest is hash-verified again at commit. A job resolves its method version once at admission and keeps it.

Executors claim fenced work with `FOR UPDATE SKIP LOCKED`. A submission persists dispatch intent and the effect row, calls the provider, records the receipt, and releases the lease. Waiting jobs hold no executor capacity; pollers reconcile, poll and cancel provider operations through the same idempotent events that a webhook would use. On restart, expired leases are reclaimed and waiting operations continue from their stored IDs. No step needs the originating chat.

`synthetic.fake@1` is the only enabled method. Its provider keeps state in files under `ASSETQ_FAKE_PROVIDER_DIR`, uses atomic effect receipts and sealed-absence markers to refuse delayed stale submissions, and supports one-shot faults (`lose_reply`, `fail_fetch`, `cancel_unconfirmed`). Its outputs say `SYNTHETIC` and carry `synthetic: true`. `image.worker.v1@1` and `model.meshy-prop.v1@1` are registered as disabled; a request for them is refused.

## API

All routes except `GET /v1/health` need `Authorization: Bearer <token>`. A token is a project-scoped grant with role `member` or `owner`; only its SHA-256 is stored. Methods, recommendations and comparisons are scoped to that project, including owner edits and promotions. Built-in method versions are seeded into each project. Machine routes never trust browser cookies or identity headers; the separate site task must use an explicitly scoped credential adapter without a wildcard auth exemption. The canonical HTTPS origin is `https://assets.ufobe.com`; this binary binds only to loopback. A foreign project's job or object reads as `404`. Errors are `{"error": code, "reason": text}`.

| Route | Purpose |
|---|---|
| `POST /v1/uploads` | Streamed upload; `Content-Type` must be allowed; returns object ID, SHA-256 and size |
| `GET /v1/objects`, `GET /v1/objects/{id}` | Paged list and metadata with provenance, synthetic flag and owner acceptance |
| `GET /v1/objects/{id}/content` | Streamed download with `X-Content-Sha256` |
| `POST /v1/objects/{id}/acceptance` | Owner sets `pending`, `accepted` or `rejected` |
| `POST /v1/jobs` | Durable admission; `201` new, `200` same key and payload, `409` changed payload |
| `GET /v1/jobs`, `GET /v1/jobs/{id}`, `/events`, `/result` | Status, event log, and the verified manifest only after success |
| `POST /v1/jobs/{id}/cancel` | Request cancellation |
| `GET/POST /v1/methods` | Registry; a POST (owner) creates the next version |
| `GET /v1/recommendations?use_case=&constraints={}`, `POST`, `POST /{id}/promote` | One promoted route for exact use-case/constraints scope, or an explicit none reason; agents propose, owners promote |
| `GET/POST /v1/comparisons` | Dataset, evaluator and method versions, results and missing cases |
| `GET /v1/status` | Jobs by phase, uncertain submissions, per-method outcomes and usage, provider health, store use, integrity and backup |

A job request has `operation`, `method` (`id` or `id@version`), `text` (64 KiB at most), `attachments` (16 object IDs at most, same project), typed `constraints` declared by the method, `budget.max_credits` and `idempotency_key`. Unknown fields, including permissions or configuration, are refused.

## Limits

Admission locks the project row and enforces the active-job limit, the project credit budget and a free store. Configurable object, temporary-byte and metadata-byte limits are enforced under one PostgreSQL advisory admission lock across service processes. This first build permits one admitting writer/dispatch at a time; streaming holds that reservation until metadata commits. Each write inventories actual object/temp bytes, so crash leftovers and orphan blobs remain charged. Metadata mutation reserves conservative headroom against measured database size; an exhausted metadata limit stops new effects while preserving reads and recovery events. Execution has a configured elapsed deadline; uncertain submissions remain reconciliable and provider cancellation never becomes a paid retry. Spend counts reported usage for finished jobs and the full reservation when usage is unknown. Uploads enforce the content-type allowlist and the per-object size limit. Uploads and imports stop at the store quota with `store_full`; existing objects and jobs stay intact. A full store pauses a job before any new provider effect.

## Storage

Only payloads that pass hash verification can enter a successful manifest. Objects live at `<data root>/objects/<aa>/<bb>/<sha256>`, written through `tmp/`, fsynced, renamed and made read-only. Keys are server-owned content addresses; a caller name is metadata only. PostgreSQL holds blobs, project-scoped object rows, manifests and provenance: job, method version, provider and operation ID, parent attachments, request and import key. Crash leftovers in `tmp/` are retained and never appear in results.

Object IDs name immutable asset versions; new generations receive new IDs and retain attachment parents. No latest-version alias floats. Uploads record `retention_until`. Nothing deletes objects in this build; garbage collection needs a reviewed inventory and policy first.

## Status values

Unknown stays explicit. Provider health, store integrity and backup read `unknown` with a reason until an observation no older than five minutes is recorded. Recommendations match the exact constraints object (default `{}`); one scope never silently supplies a route for another scope. Promotion supersedes only the same project/use-case/constraints scope. Usage without a provider report is `null` with `unknown_reason`. Generation success (job phase), asset validation (manifest) and owner acceptance (object `acceptance`) are separate fields.

## Not in this build

Real providers, webhook ingestion endpoint, upload retention enforcement, backup and restore automation, and an integrity scan job. Each remains unknown or absent in status rather than reported as healthy.

## Recovery and restore checks

`assetq restore-check` is read-only: verify schema version, replay every job, verify every blob and manifest against the filesystem and project object records, and require successful jobs to have complete manifests. Run it against the isolated restored database/data root before API upload/download and job/status/result checks with restored project grants. No garbage collection or reservation cleanup is needed for correctness: physical inventory charges retained crash leftovers on the next admission. It never deletes objects or temp files.

## Development checks

Use the filtered commands in [testing](testing.md) while editing. The canonical gate remains `python3 scripts/check.py` with a 600-second elapsed CI budget including setup (R8).
