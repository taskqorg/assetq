# Pipe contract

Any external service call by an agent is a pipeline, even with one step. Agents hold no service keys. Pipe is a standalone pipeline service that accepts text and attachments, executes durable jobs and retains verified outputs; assets are one kind of pipeline output. TaskQ may call Pipe. Agent execution belongs to AgentQ and infrastructure primitives to CoreQ.

## Principles

- **R1 Shared rules and SDD.** [specq](https://github.com/taskqorg/specq) is binding. This contract owns product decisions; linked detail is binding. Edit the owning rule before code, retain the original request in task evidence, and accept observable results rather than agent prose.
- **R2 Durable ownership.** Pipe owns generation jobs and assets. TaskQ owns development tasks and attempts; PilotQ owns conversations and delivery. A chat, worker process or telemetry export ending never completes or loses a generation job.
- **R3 Explicit authority.** Project-scoped grants gate requests, attachment access, provider dispatch and result retrieval. A request cannot grant permissions or change model, account, provider, budget or production configuration. Credentials stay in protected host configuration, never Git, prompts or telemetry. Agents receive scoped pipeline authority, never downstream service keys.
- **R4 Unknown is explicit.** Submission uncertainty, missing usage, stale health and incomplete results remain unknown with reasons. Reconcile an existing provider operation before any possible paid replay. Never silently replace a method or count missing evidence as success.
- **R5 Immutable provenance.** Retain original inputs and outputs, hashes, method and parameter versions, provider operation IDs, actual or explicitly unknown usage, validations and parent relationships. New generations create new asset versions; project pins never float.
- **R6 Executable lifecycle.** Job and attempt transitions follow one executable behavioral model with replayable traces for submission, crash, recovery, cancellation, duplicate events and result import. Completion requires a verified manifest and retrievable payloads; product or visual acceptance is separate.
- **R7 Bounded operation.** Admission enforces declared spend, file, storage, concurrency and execution limits. A full store or exhausted budget stops new effects with a reason and preserves existing work. Observability cannot silently authorize work or bypass these limits.
- **R8 Delivery evidence.** The selected canonical gate is `python3 scripts/check.py`, with a 600-second elapsed budget on `ubuntu-latest`, including setup. Docs-only changes run pinned contract lint and whitespace checks; implementation adds deterministic core and affected checks. Native/provider/storage qualification is separate and named per path.
- **R9 Canonical web surface.** `https://pipe.ufobe.com` is the Pipe management site and API origin. Asset browsing, search, generation jobs, pipeline benchmarks and method management belong here. DashQ shows source-backed summary metadata and links to Pipe; it does not duplicate the asset workspace. Pipe does not serve `assets.ufobe.com` or redirect it; all requests naming
  that host are refused before database access.

- **R10 Independent execution.** Pipe never uses TaskQ except temporarily for observation, cancellation, reconciliation, slot release and retained import of existing protected operations. Preserve their original signed adapter, identities and receipts under R3-R6 until migration to independent AgentQ/CoreQ authority. This exception permits no new TaskQ-backed launch or qualification.

## Requests and durable execution

A request contains project identity, operation, text, immutable attachment IDs, typed constraints, budget and an idempotency key. Return a job ID after durable acceptance; status and result reads do not keep an agent turn open. The same key with different input is refused.

Record job, attempt and effect identities independently. Persist dispatch intent before external effects and verified receipts afterward. Leases and fencing prevent a stale executor from dispatching or committing results. Provider webhooks and polling update the same operation idempotently.

Cancellation distinguishes requested, provider-confirmed, locally stopped and uncertain. Report incurred or unknown charges. Retry policy never blindly repeats an uncertain paid submission. A service restart resumes observation and safe steps without requiring the original chat.

A protected worker's supervisor exit proves only transport termination. Without verified original-operation native and descendant completion, retain turn observations as evidence, keep completion unknown and preserve capacity reservations.

Keep the Runtime transport alive until the original host cancellation readback finishes, including after work errors. Opening or cancellation failures remain unknown; cancellation acknowledgment alone never confirms descendant completion or releases capacity.

An operation that reports completion but whose output fails import validation ends the job `failed` with the validation reason, releases its reservations and is never resubmitted; any imported output stays retained outside a manifest. An outcome stays unknown only while no completion report exists.

The first implementation supports a registered bounded sequence of steps, including external waiting and local validation. It does not require a general workflow editor. Executors can run on qualified hosts distinct from the storage/control host.

## Web management and API

Human-facing times follow [specq Time](https://github.com/taskqorg/specq/blob/71fcb39ca9aafa868a57a6f6f5a13845cccec747/caveman/rules/indicators.md#time). Browser timestamps, including those nested in event, qualification and operations evidence, show relative time first and viewer-local exact time on hover. API and CLI JSON retain machine timestamps.

The management application and versioned API share `https://pipe.ufobe.com`. The browser uses the same Pipe contracts as CLI and agent callers; it has no separate job state, asset catalog or recommendation store. Define exact API paths in the service contract and preserve published compatibility.

Provide an asset library with visual previews, authenticated downloads, text search and project/type filters; asset detail includes versions, source relationships, provenance and validation. Unsupported previews show the file type and download action rather than a fabricated render.

Provide generation requests with text and attachments, durable progress/cancellation/results, method/API/model/pipeline inspection and versioned edits, and benchmark setup, runs and comparable outputs. Show dataset/evaluator versions, quality judgments, cost, time, failures and scoped recommendations, including incomplete or unknown evidence.

Browser identity and API credentials remain separate, project-scoped authorities. Reuse the existing host sign-in guard for browser entry where qualified; machine routes require explicit scoped authentication, never a wildcard browser-auth exemption. A reachable website does not make assets public or grant generation, benchmark, edit or promotion rights.

Apply the [specq design workflow](https://github.com/taskqorg/specq/blob/main/design/workflow.md) before implementing a new visual candidate. Include empty, loading, long-running, cancelled, failed, stale/unknown and permission-denied states. Accept the actual browser and API journeys separately from mockups.

The asset library uses a table with retained thumbnails and adjacent selected detail, with detail below the list on narrow screens. Browser routes, shared API extensions, session authority and the accepted visual reference are specified in [management design](docs/design/management-site.md) and [service](docs/service.md).

## Versioned methods and recommendations

Each method names its operation, supported inputs and outputs, provider/model/API and pipeline revisions, parameter schema, capability requirements, limitations, provenance and validation recipe. Adding or editing a method creates a version; running jobs retain their resolved version.

Methods can be experimental, recommended, deprecated or disabled. Recommendations are scoped to use case and constraints, include evidence and observation time, and remain distinguishable from provider health. Recommend one qualified route or report that none is qualified.

Compare candidates on a fixed versioned input set, repeated samples where needed, technical validity, visual/audio judgment, cost, duration and failure rate. Preserve dataset, evaluator and method versions and missing cases. Agents propose recommendations; promotion follows an owner-approved policy. A new model is not automatically a winner.

API, CLI and agent tools read the same method registry and recommendation records. Human guidance is a projection of those records. Do not maintain a second handwritten recommendation source.

The bounded comparison contract is [method evaluation](docs/method-evaluation.md). Dataset and evaluator revisions are immutable project-owned records.

A comparison pins their contents, exact method versions, operation, use case, constraints, repeat count, decision rule and maximum job/credit budget before admission.

Its samples use ordinary durable jobs and retain incomplete, failed, unknown-usage and unjudged cases. Resuming a comparison reuses sample identities and never replaces an uncertain job.

Read-only qualification records link real retained receipts to exact registry versions without rewriting their specifications or authorizing dispatch. Historical tool/output qualification and owner pipeline acceptance do not imply general generation eligibility or finished-asset quality.

A recommendation read includes its revision, evidence, observation time and an explicit scoped none result when no owner-promoted route is qualified.

Control summaries expose registry/comparison/recommendation revisions for downstream consumers, with private assets accessible only through scoped Pipe routes.

## Initial generation methods

**Image worker (`image.worker.v1`).** The method names the capability, not a runtime: provider `agentq.image` with capability `image_generation`. AgentQ selects and qualifies the worker runtime that holds that capability; it receives text, attachments and output constraints and invokes its qualified image-generation tool. Preserve original generated bitmap files and native evidence.

Infrastructure builds pin CoreQ `v0.19.2` at `b78ca64cac615500b57b4a04983e79333035dd65`; runtime builds retain their own resolved CoreQ dependency.

New worker builds use `agentq-runtime` 0.2.0, published as `runtime-v0.2.0` at immutable revision `70f85434546bd1c303ccdb7b0dd71866421627e2` by AgentQ PR218. Cargo.lock retains the resolved revision.

The runtime owns structured text/image forwarding and read-only account identity; Pipe verifies attachment ownership, hashes and PNG limits before `Runtime::input`, and requires subscription login through `Runtime::account` before any turn. No raw account request or credential refresh supplies an alternative path.

Existing operation receipts retain their original runtime pin and remain observable without replay. The protected app-server path inherits the existing account, model, permissions and optional optimization.

The structured generation pipeline is method version 5; versions 2, 3, 4 and 5 retain their original runtime and validation recipes. Neither a runtime upgrade nor qualification rewrites existing registry versions or completed asset provenance.

New native dispatch requires protected project/profile grant readback and verified host boundaries under R3. A workspace path and inherited permission mode do not supply that evidence.

Consume protected launch through AgentQ and shared CoreQ infrastructure, using the operator-controlled host authority path and its existing grants. Do not add a Pipe policy store or trust grant text from a task, request or environment. Resolve and record the effective boundary before native effects; unavailable enforcement refuses launch.

Until the composed protected worker and its current image capability are qualified, refuse new generation launches; retain observation, cancellation and import of existing operations. A bounded owner-authorized example qualifies its tool/output path only.

Keep the bounded non-generating qualification command unavailable until an independent protected transport replaces the legacy adapter under R10. Independent qualification must retain incarnation-bound capacity and independently read expected identities. Pin the qualified host adapter artifact in the build; task, environment and receipt text cannot select another authority.

A replacement adapter must first qualify an isolated build from reviewed immutable AgentQ/CoreQ source against the existing policy and host boundary. Record its final staged digest after signing, then review and pin it before composed qualification. Preserve prior adapters and receipts for recovery; no temporary trust override or grant edit is permitted.

For existing-operation recovery only under R10, the selected installed TaskQ `4.6.66-261103-26` artifact is an explicit exception to that compilation order: retain its verified signed bytes for original-operation recovery; new qualification and generation launches through it are refused. Its SHA-256 is `bd6dc0de5024ff7b6b0ae0a987f5467aa0c5196b005c7cdfd434d043709c7e42`.

Native qualification remains required; installation, signature and policy parsing alone do not qualify launch.

The prior qualified adapter source `e990d88943e95abd8d182d8157bfe84a94343a07` and digest `f8b04708db09c764ec14bbaa77d0dd40fd7bfc9a521104f93f20885ebb7d5df4` remain historical evidence. Preserve its original binaries for recovery. The refreshed build accepts only the new digest; it cannot use the prior artifact as a fallback.

Retain private launch intent, authority, native journal and cancellation/reconciliation receipts. The non-generating qualification command cannot start a model turn. No qualification may release an uncertain slot.

The runtime-trait qualification command exposes read-only account identity and catalogue progress; native probe methods unsupported by the selected runtime refuse explicitly. Historical boundary-probe receipts retain their exact original candidate.

Catalogue qualification retains the Runtime-assigned request ID and waits for its matching response for at most ten seconds within the session deadline. Notifications and an empty progress read do not complete it. A missing response stays unknown; never resend the original request to obtain evidence.

After independent host qualification under R10, one isolated local durable-API qualification job may generate an original stylized blue wooden wedge, with one verified invented blue-swatch attachment, one PNG requested within 1024 by 1024 pixels and zero paid-API credits. An operator-only command binds the exact already accepted job ID before dispatch; API requests and environment
strings cannot enable this path.

General dispatch remains refused. Preserve the accepted crate and its initial 3D-input identity; never replay it.

The qualification runner acquires its own existing-host slot for its actual incarnation, persists its independent AgentQ preparation before launch, and submits at most one Runtime input. It inherits subscription, model, effort and optional optimization without edits.

One concurrent job has at most 600 seconds generating plus 30 seconds observing interruption, a 64 MiB original PNG and 32 MiB native journal under existing resource guards. Report unreported usage/charge as unknown; no direct paid API, replacement submission, new grants, ordinary configuration change or deployment is authorized.

The runner retains Runtime through cancellation, then uses AgentQ original-operation reconciliation before terminal publication and slot release. A restart observes retained preparation/turn receipts without relaunch; missing terminal or descendant evidence preserves unknown and reservations. The executable job model remains the lifecycle owner.

Qualification requires duplicate-request, control-service interruption, original-tool/path, immutable import and authenticated retrieval evidence; a missing tool refuses precisely.

Native-size import is `image.worker.v1@6`, operation `image.import_native`: import one retained original from an explicitly operator-bound, completed generation without a new Runtime input. Preserve original PNG bytes and dimensions under the existing 64 MiB, 16-million-pixel and 4096-axis safety bounds. No <=1024 output guarantee or resampling applies.

The new import job retains the source job/effect, request, method, attachments, tool/path, Runtime checkpoint, hashes and usage. Version 5's 1024-bound request remains unchanged and fails precisely after verified original-operation completion. The accepted crate remains the 3D input. General generation stays refused; importing a retained image never authorizes replay.

A durable effect receipt binds exactly one worker launch; restart observes that operation and never relaunches an uncertain turn.

Native terminal image-tool evidence, an allowed original artifact path and full PNG decoding are required before import. Preserve a bounded preview, dimensions, input identities, runtime/tool evidence and actual or explicitly unknown usage. Exact worker limits, capability refusal and recovery are specified in [image worker](docs/image-worker.md).

A runtime is eligible only after AgentQ qualifies its tool and output path. Text, SVG, a procedural drawing or an unannounced direct API is not a successful image-worker generation.

**Image to 3D prop (`model.meshy-prop.v1`).** Resolve a verified input image; submit a versioned Meshy image-to-3D request with explicit `meshy-t2`, Smart Topology, a 3000-triangle target and 2K PBR as the starting recipe. Preserve source GLB/textures, then validate and render with headless Blender. Provider support and current cost require fresh verification before dispatch.

Meshy dispatch uses an operator-bound accepted job and a freshly reconciled account/workstream ledger, with no purchases or top-ups. Unknown prior spend, balance, parameter support or executor qualification refuses new effects.

Initial Meshy testing has no cumulative credit ceiling; retain actual and cumulative cost evidence without refusing on cumulative spend. Per-job limits and all other R7 bounds remain enforced.

Persist intent before the only POST; a missing reply without a verified original provider ID remains uncertain indefinitely, never authorizes another POST. Cancellation preserves remote results and unknown charges; do not delete a provider task to simulate cancellation.

Original downloads and headless validation use the existing counted store and resource guards. Exact implementation and operator prerequisites are specified in [Meshy pipeline](docs/meshy-pipeline.md).

The Meshy recipe is a starting baseline for props, not an accepted character, hand, rigging or universal 3D method. Preserve UVs/PBR maps; measure actual triangles, components and dimensions. Keep csgo rig/socket/export bindings out of this general recipe. Evidence: [baseline](docs/meshy-baseline.md).

Both methods operate through the durable job API and central asset store. Local fake-provider checks prove recovery semantics, not real generation. The initial real examples use an original stylized wooden crate with blue metal corners, no text or third-party references; its generated image becomes the 3D input.

## Storage and deployment

The product crate, binary and CLI are `pipe`. Keep the `assetq` command only at verified live host call sites. Keep `ASSETQ_*` configuration, proxy headers, cookies and pending request keys compatible.

The rename preserves database and role names, SQL schema, object paths, stored method/asset identifiers, immutable specifications and provenance.

The initial managed deployment is on pds18. Use one dedicated `assetq` Unix service identity, a separate role/database in the existing PostgreSQL cluster, and immutable objects under a dedicated `/srv/assetq` data root. Preserve other services, ownership, databases and the existing Langfuse object store.

Start with streamed filesystem-backed object storage behind the Pipe API; no additional S3 server is required. Store manifests and ownership in PostgreSQL. File/object keys are server-owned, never arbitrary caller paths. Only committed, verified objects appear in successful results; retain or reconcile crash leftovers safely.

R7 requires a configurable filesystem free-space floor before dispatch and during streamed writes. Serialize writers across processes and charge temporary and orphan bytes. Failed space observations refuse new effects with an unknown reason; exhaustion preserves committed assets and existing provider identities.

Metadata admission leaves a separately configured recovery reserve. Every metadata writer, including events, leases, cancellation, observations and migrations, uses serialized physical-size checks covering indexes and TOAST.

Bound each transaction and check its physical growth before commit; exhaustion stops further writes without deleting evidence. Reads and reconciliation resume from retained state after capacity returns.

Project-scoped APIs provide authenticated uploads and downloads, checksums, size/type limits, pagination and bounded retention. Permanent asset IDs are independent of expiring transport URLs. Do not automatically publish private inputs or expose storage directories as a public web root.

Installation first prepares an isolated candidate and verifies the concrete service user, directories, database, local endpoint, quota and rollback procedure. Backup/restore qualification follows the initial-delivery deferral below. Additive activation uses existing host mechanisms and preserves other workloads.

Publish the approved `pipe.ufobe.com` name through the existing Caddy HTTPS entry on pds18, proxying to the loopback-only Pipe service. Remove the prior Pipe site at `assets.ufobe.com` without a redirect. Change only these two named DNS/site configurations after verifying their current ownership and targets; preserve apex/wildcard records, other sites and existing access guards. Verify TLS,
browser/API authentication and rollback on the exact candidate.

Another public hostname, firewall change, destructive migration or unrelated service restart requires separate concrete approval. The website does not authorize direct public storage listeners or directory listing.

For initial delivery, defer new backup work and both local and independent external backup/restore qualification. Preserve existing host backups and report deferred or unobserved backup/restore evidence as unknown; these gaps do not block initial functionality.

Pipe must not be stopped or restarted for backup. No cold-backup timer, backup-driven outage, rotation or additional backup platform is authorized for initial delivery.

Any backup verification claim requires consistent object/metadata snapshots, including manifests, job state and registry revisions, plus isolated restore/hash and authenticated retrieval evidence for its named scope. A local receipt never proves independent external restoration. Provider URLs and RAID alone are not backup evidence.

Garbage collection cannot delete referenced, pinned or retained assets; no destructive collection runs before a reviewed inventory and policy.

## Control and observability

Expose jobs and step progress, queued/active/uncertain work, provider availability, per-method outcomes, usage and spend, comparison evidence, recommendations, storage usage/integrity and backup/restore observations. Keep generation success, asset validation and owner acceptance separately visible.

Use the Pipe site for asset operations and pipeline benchmarks. Reuse DashQ for summary metadata/deep links and ServerQ for host observations when their integrations are ready. Pipe API/CLI remain independently operable.

No Langfuse exporter is implemented. Job state and primary assets remain in Pipe; a telemetry outage cannot own or erase them.

## Linked specifications

- [Architecture](docs/architecture.md): dependency direction, runtime extraction and service boundaries.
- [Acceptance](docs/acceptance.md): real user journeys and recovery evidence.
- [Meshy baseline](docs/meshy-baseline.md): source pipeline and scope limitations.
- [Service](docs/service.md): lifecycle model, API, limits and immutable storage.
- [Deployment](docs/deploy.md): candidate verification, host requirements, additive install and rollback.
- [Management design](docs/design/management-site.md): browser scenarios, candidate references and acceptance gaps.

<!-- contract-budget: bytes=61440 lines=300 -->
