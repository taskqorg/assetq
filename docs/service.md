# Service, lifecycle and storage

One Rust binary (`assetq`) provides the HTTP API, embedded or separate executors, administration commands and an API client. It implements [assetq.md](../assetq.md) R2-R7 with a synthetic fixture provider, an optional [Codex image worker](image-worker.md) and the bounded [Meshy pipeline](meshy-pipeline.md). Native/provider and managed deployment qualification remain separate. Deployment and rollback: [deploy](deploy.md).

## Lifecycle model

Attempt IDs are persisted separately from job IDs and effect IDs; each attempt records its fence, owner and start/end. `src/model.rs` is the one executable job/attempt/effect model. Every job change is an event applied by `model::apply`. PostgreSQL stores the resulting state next to an append-only event log. `assetq audit <job>` replays the log and compares the result with the stored state.

- **Job phases:** `queued`, `active` (claimable when unleased), `waiting` (provider work outstanding, no lease), `succeeded`, `failed`, `cancelled`.
- **Attempt:** each claim advances the fence by one and records a lease owner. An event from an older fence is refused as stale. An expired lease becomes `lease_expired`, then a new claim. Executors renew live leases throughout blocking provider calls, streamed imports and hash verification, without holding a job row lock during filesystem work. Renewal never revives an expired lease. Database time rejects fenced writes after lease expiry even before another claim. A PostgreSQL session advisory lock serializes dispatch/reconciliation and poll/cancel calls for one job; loss of that session leaves submission uncertain, never authorizes replay while the previous call may still arrive.
- **Effect:** `none`, `intended` (intent persisted before the provider call), `submitted` (operation ID recorded), `uncertain` (reply lost), `done`, `failed`. A new dispatch requires `none`. An intended or uncertain effect is reconciled by provider lookup of its effect ID: a found operation is adopted, and only a confirmed sealed absence (the old effect can no longer be accepted) allows one new dispatch. A provider that cannot establish that barrier must return unknown; an empty eventually consistent lookup is not confirmed absence.
- **Cancellation:** `requested`, `provider_confirmed`, `locally_stopped`, `uncertain`. An unconfirmed provider cancel keeps the charge unknown with a reason, stays `waiting` without an executor lease, and retains project active-job and spend reservations. Observation of the same operation continues after restart until confirmed stop, completion or failure. Completion after requested or uncertain cancellation stops locally without importing results; only a terminal observation releases active-job capacity.
- **Completion:** `succeeded` requires every step done, every expected output imported and a manifest commit in the same transaction as the transition. Duplicate provider completions and late events after a terminal phase are no-ops.
- **Import refusal:** A completion report is a definitive result, so its output either imports or fails the job. A report with an impossible output count, an output above the configured upload size, an empty or undeclared output, an invalid PNG header, or a PNG beyond the request's `max_width` or `max_height` ends the job `failed` with the reason. No provider call repeats, and imported objects stay retained outside any manifest. The terminal phase releases the active-job slot and reduces spend to reported usage; unknown usage keeps its reservation under [limits](#limits). A provider that validates inside its own completion path reports that refusal as a failed poll result, never as a poll error.
- **Unknown observation:** A poll error means no completion report exists. The job stays `waiting` with its reservations and no replay until a completion report, the execution deadline or cancellation. Transient import conditions after completion (`store_full`, a source stream or fetch failure, unobserved space) pause and resume.

Replay traces in `tests/traces/` cover the happy path, crash after intent, lost reply (found and absent), duplicate input and events, stale executors, partial import, import refusal, full store and four cancellation paths. `assetq selftest` replays the same embedded traces.

## Execution

A method version declares a bounded step list (at most 16) from `synthetic.generate`, `provider.submit`, `provider.import` and `validate`. This first executor accepts exactly one generation followed by import (for a provider) and validation; unsupported sequences are refused rather than claiming general pipeline support. Every successful manifest is hash-verified again at commit. A job resolves its method version once at admission and keeps it.

Executors claim fenced work with `FOR UPDATE SKIP LOCKED`. A submission persists dispatch intent and the effect row, calls the provider, records the receipt, and releases the lease. Waiting jobs hold no executor capacity; pollers reconcile, poll and cancel provider operations through the same idempotent events that a webhook would use. On restart, expired leases are reclaimed and waiting operations continue from their stored IDs. No step needs the originating chat.

`synthetic.fake@1` keeps state in files under `ASSETQ_FAKE_PROVIDER_DIR`, uses atomic effect receipts and sealed-absence markers, and supports one-shot faults (`lose_reply`, `fail_fetch`, `cancel_unconfirmed`). Its outputs say `SYNTHETIC` and carry `synthetic: true`. `image.worker.v1@2` and `@3` retain their original specifications; experimental version 4 pins the corrected protected runtime. General generation dispatch refuses pending current tool qualification, while configured observers can retain/import existing operations with `synthetic: false`. Version 5 uses structured Runtime inputs. Operator-bound version 6 imports an existing completed native image without generation, using the same provider lifecycle and a new immutable source-linked request. The separately approved [single local qualification job](evidence/4/bounded-image-job.md) integrates runner-owned slot/preparation/reconciliation without changing this executable job model or enabling general dispatch. Versions 1 and 5 use provider `agentq.image` with capability `image_generation`; versions 2-4 keep their original runtime-specific specifications. Version 1 and `model.meshy-prop.v1@1` stay disabled; startup migration rewrites a built-in row from an earlier seed only when it is disabled and no job or recommendation references it. No recommendation is promoted automatically.

## API

`model.meshy-prop.v1@2` preserves disabled version 1 and declares the explicit 15-credit prop recipe. `meshy-admit` exposes admission with zero executors. Only `meshy-serve` can bind an accepted initial job to protected operator preflight; general paid dispatch stays refused. After restart, the normal server discovers retained Meshy receipts and observes without dispatch authority. Fully retained completions recover without provider credentials or a live API. Original GLB/maps import before the Meshy validation hook can fail the job, so failed validation retains retrievable objects without a success manifest. [Operator qualification](evidence/5/operator.md) remains required; experimental registry presence is not native qualification.

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

Admission enforces project active-job and credit limits under a project row lock. Spend counts reported usage for finished jobs and the full reservation when usage is unknown. Execution has a configured elapsed deadline; uncertainty remains reconciliable and cancellation never authorizes a paid retry. Uploads enforce content type and per-object size limits.

One PostgreSQL session reservation serializes admission and dispatch across processes. An independent filesystem lock covers inventory, streaming, installation and metadata registration, including after loss of the database connection. All writers for one store use the same database and bounds. Physical inventory charges objects, orphan blobs and retained temporary files. The data root, `objects` and `tmp` must share one filesystem. Unix `statvfs` measures bytes available to the service identity; unsupported or failed measurements return `store_unknown`.

Before dispatch and each streamed chunk, remaining room is the minimum of object quota, temporary quota and observed free space above the configured floor, with 64 KiB left for filesystem allocation overhead. Recheck the floor after each write and before installation. `store_full` stops new effects and preserves committed objects; a partial write removes only its own uncommitted temporary file. Other host writers can change free space between observations; this guard does not replace a qualified host filesystem quota.

Let `Q` be metadata quota, `G` transaction headroom and `R` recovery reserve. Ordinary admission and migration require `used + G + R <= Q`; recovery requires `used + G <= Q`. `G` is at least 4 MiB. Every metadata transaction takes one common advisory transaction lock before row/table locks and measures `pg_database_size`, including indexes, TOAST and retained dead/aborted allocations. Events, attempts, lease renewal, poll scheduling, receipts, cancellation, imports and manifests use recovery headroom. New jobs, grants, registry edits, recommendations, comparisons and observations leave `R` untouched. Migration checks capacity before its first DDL and remains schema 1 without rewriting existing data.

Transactions charge cumulative serialized payload at 16 times its byte length plus 64 KiB per bounded mutation group; promotion also charges every withdrawn row. Before commit, remeasure physical growth and refuse growth beyond `G` or the applicable ceiling. These are conservative admission bounds, not a hard PostgreSQL filesystem quota: rollback cannot reclaim every physical allocation, and WAL, cluster-wide files and external SQL writers require separate host capacity protection. No metadata writer bypasses exhaustion for recovery; exhausted recovery retains durable state and resumes after capacity returns. New-effect claims stop at the admission ceiling, avoiding claim/pause churn. Provider receipts and lifecycle events commit atomically; an unrecorded receipt stays uncertain and is reconciled from its existing effect identity.

## Storage

Only payloads that pass hash verification can enter a successful manifest. Objects live at `<data root>/objects/<aa>/<bb>/<sha256>`, written through `tmp/`, fsynced, renamed and made read-only. Keys are server-owned content addresses; a caller name is metadata only. PostgreSQL holds blobs, project-scoped object rows, manifests and provenance: job, method version, provider and operation ID, parent attachments, request and import key. Crash leftovers in `tmp/` are retained and never appear in results.

Object IDs name immutable asset versions; new generations receive new IDs and retain attachment parents. No latest-version alias floats. Uploads record `retention_until`. Nothing deletes objects in this build; garbage collection needs a reviewed inventory and policy first.

## Status values

Unknown stays explicit. Provider health, store integrity and backup read `unknown` with a reason until an observation no older than five minutes is recorded. Recommendations match the exact constraints object (default `{}`); one scope never silently supplies a route for another scope. Promotion supersedes only the same project/use-case/constraints scope. Usage without a provider report is `null` with `unknown_reason`. Generation success (job phase), asset validation (manifest) and owner acceptance (object `acceptance`) are separate fields.

`GET /v1/status` adds observed filesystem free space/floor and separate metadata admission/recovery availability. Measurement failures read `unknown` with a reason. Existing routes, result fields and schema remain compatible; these status fields are additive.

## Not in this build

General Meshy dispatch, webhook ingestion endpoint, upload retention enforcement, backup and restore automation, and an integrity scan job. Each remains unknown or absent in status rather than reported as healthy. The bounded Meshy adapter requires separate native/provider qualification. The qualified native image route does not qualify another client, host or managed storage deployment.

## Recovery and restore checks

`assetq restore-check` is read-only: verify schema version, replay every job, verify every blob and manifest against the filesystem and project object records, and require successful jobs to have complete manifests. Run it against the isolated restored database/data root before API upload/download and job/status/result checks with restored project grants. No garbage collection or reservation cleanup is needed for correctness: physical inventory charges retained crash leftovers on the next admission. It never deletes objects or temp files.

## Development checks

Use the filtered commands in [testing](testing.md) while editing. The canonical gate remains `python3 scripts/check.py` with a 600-second elapsed CI budget including setup (R8).
