# AssetQ contract

AssetQ accepts text and attachments, executes durable asset-generation jobs, and retains verified assets with versioned methods and evidence-backed recommendations. It provides a Rust service, CLI and web management application; agent execution belongs to AgentQ, task scheduling to TaskQ, and infrastructure primitives to CoreQ.

## Principles

- **R1 Shared rules and SDD.** [specq](https://github.com/taskqorg/specq) is binding. This contract owns product decisions; linked detail is binding. Edit the owning rule before code, retain the original request in task evidence, and accept observable results rather than agent prose.
- **R2 Durable ownership.** AssetQ owns generation jobs and assets. TaskQ owns development tasks and attempts; PilotQ owns conversations and delivery. A chat, worker process or telemetry export ending never completes or loses a generation job.
- **R3 Explicit authority.** Project-scoped grants gate requests, attachment access, provider dispatch and result retrieval. A request cannot grant permissions or change model, account, provider, budget or production configuration. Credentials stay in protected host configuration, never Git, prompts or telemetry.
- **R4 Unknown is explicit.** Submission uncertainty, missing usage, stale health and incomplete results remain unknown with reasons. Reconcile an existing provider operation before any possible paid replay. Never silently replace a method or count missing evidence as success.
- **R5 Immutable provenance.** Retain original inputs and outputs, hashes, method and parameter versions, provider operation IDs, actual or explicitly unknown usage, validations and parent relationships. New generations create new asset versions; project pins never float.
- **R6 Executable lifecycle.** Job and attempt transitions follow one executable behavioral model with replayable traces for submission, crash, recovery, cancellation, duplicate events and result import. Completion requires a verified manifest and retrievable payloads; product or visual acceptance is separate.
- **R7 Bounded operation.** Admission enforces declared spend, file, storage, concurrency and execution limits. A full store or exhausted budget stops new effects with a reason and preserves existing work. Observability cannot silently authorize work or bypass these limits.
- **R8 Delivery evidence.** The selected canonical gate is `python3 scripts/check.py`, with a 600-second elapsed budget on `ubuntu-latest`, including setup. Docs-only changes run pinned contract lint and whitespace checks; implementation adds deterministic core and affected checks. Native/provider/storage qualification is separate and named per path.
- **R9 Canonical web surface.** `https://assets.ufobe.com` is the AssetQ management site and API origin. Asset browsing, search, generation jobs, pipeline benchmarks and method management belong here. DashQ shows source-backed summary metadata and links to AssetQ; it does not duplicate the asset workspace.

## Requests and durable execution

A request contains project identity, operation, text, immutable attachment IDs, typed constraints, budget and an idempotency key. Return a job ID after durable acceptance; status and result reads do not keep an agent turn open. The same key with different input is refused.

Record job, attempt and effect identities independently. Persist dispatch intent before external effects and verified receipts afterward. Leases and fencing prevent a stale executor from dispatching or committing results. Provider webhooks and polling update the same operation idempotently.

Cancellation distinguishes requested, provider-confirmed, locally stopped and uncertain. Report incurred or unknown charges. Retry policy never blindly repeats an uncertain paid submission. A service restart resumes observation and safe steps without requiring the original chat.

The first implementation supports a registered bounded sequence of steps, including external waiting and local validation. It does not require a general workflow editor. Executors can run on qualified hosts distinct from the storage/control host.

## Web management and API

The management application and versioned API share `https://assets.ufobe.com`. The browser uses the same AssetQ contracts as CLI and agent callers; it has no separate job state, asset catalog or recommendation store. Define exact API paths in the service contract and preserve published compatibility.

Provide an asset library with visual previews, authenticated downloads, text search and project/type filters; asset detail includes versions, source relationships, provenance and validation. Unsupported previews show the file type and download action rather than a fabricated render.

Provide generation requests with text and attachments, durable progress/cancellation/results, method/API/model/pipeline inspection and versioned edits, and benchmark setup, runs and comparable outputs. Show dataset/evaluator versions, quality judgments, cost, time, failures and scoped recommendations, including incomplete or unknown evidence.

Browser identity and API credentials remain separate, project-scoped authorities. Reuse the existing host sign-in guard for browser entry where qualified; machine routes require explicit scoped authentication, never a wildcard browser-auth exemption. A reachable website does not make assets public or grant generation, benchmark, edit or promotion rights.

Apply the [specq design workflow](https://github.com/taskqorg/specq/blob/main/design/workflow.md) before implementing a new visual candidate. Include empty, loading, long-running, cancelled, failed, stale/unknown and permission-denied states. Accept the actual browser and API journeys separately from mockups.

## Versioned methods and recommendations

Each method names its operation, supported inputs and outputs, provider/model/API and pipeline revisions, parameter schema, capability requirements, limitations, provenance and validation recipe. Adding or editing a method creates a version; running jobs retain their resolved version.

Methods can be experimental, recommended, deprecated or disabled. Recommendations are scoped to use case and constraints, include evidence and observation time, and remain distinguishable from provider health. Recommend one qualified route or report that none is qualified.

Compare candidates on a fixed versioned input set, repeated samples where needed, technical validity, visual/audio judgment, cost, duration and failure rate. Preserve dataset, evaluator and method versions and missing cases. Agents propose recommendations; promotion follows an owner-approved policy. A new model is not automatically a winner.

API, CLI and agent tools read the same method registry and recommendation records. Human guidance is a projection of those records. Do not maintain a second handwritten recommendation source.

## Initial generation methods

**Image worker (`image.worker.v1`).** A capable Codex worker receives text, attachments and output constraints through AgentQ and invokes its qualified image-generation tool. Preserve original generated bitmap files and native evidence.

New worker builds use `agentq-runtime` 0.1.0 at immutable revision `aab1a190ab76f1e932578520d7a8787ef1d4b660`, published by AgentQ PR188. Existing method versions and operation receipts retain their original runtime pin. Its protected app-server path inherits the existing account, model, permissions and optional optimization.

The protected pipeline is method version 3; version 2 retains the original runtime and validation recipe. Neither a runtime upgrade nor qualification rewrites existing registry versions or completed asset provenance.

New native dispatch requires protected project/profile grant readback and verified host boundaries under R3. A workspace path and inherited permission mode do not supply that evidence.

Consume protected launch through AgentQ and shared CoreQ infrastructure, using the operator-controlled host authority path and its existing grants. Do not add an AssetQ policy store or trust grant text from a task, request or environment. Resolve and record the effective boundary before native effects; unavailable enforcement refuses launch.

Until the composed protected worker and its current image capability are qualified, refuse new generation launches; retain observation, cancellation and import of existing operations. A bounded owner-authorized example qualifies its tool/output path only.

Provide one bounded non-generating qualification command through the same protected transport as the image runner. The existing PM supplies a slot bound to its actual consumer incarnation and independently read expected identities. Pin the qualified host adapter artifact in the build; task, environment and receipt text cannot select another authority.

Retain private launch intent, authority, native journal and cancellation/reconciliation receipts. Qualification cannot start a model turn or release an uncertain slot.

A durable effect receipt binds exactly one worker launch; restart observes that operation and never relaunches an uncertain turn.

Native terminal image-tool evidence, an allowed original artifact path and full PNG decoding are required before import. Preserve a bounded preview, dimensions, input identities, runtime/tool evidence and actual or explicitly unknown usage. Exact worker limits, capability refusal and recovery are specified in [image worker](docs/image-worker.md).

Claude is eligible only after equivalent tool and output-path qualification. Text, SVG, a procedural drawing or an unannounced direct API is not a successful image-worker generation.

**Image to 3D prop (`model.meshy-prop.v1`).** Resolve a verified input image; submit a versioned Meshy image-to-3D request with explicit `meshy-t2`, Smart Topology, a 3000-triangle target and 2K PBR as the starting recipe. Preserve source GLB/textures, then validate and render with headless Blender. Provider support and current cost require fresh verification before dispatch.

The Meshy recipe is a starting baseline for props, not an accepted character, hand, rigging or universal 3D method. Preserve UVs/PBR maps; measure actual triangles, components and dimensions. Keep csgo rig/socket/export bindings out of this general recipe. Evidence: [baseline](docs/meshy-baseline.md).

Both methods operate through the durable job API and central asset store. Local fake-provider checks prove recovery semantics, not real generation. The initial real examples use an original stylized wooden crate with blue metal corners, no text or third-party references; its generated image becomes the 3D input.

## Storage and deployment

The initial managed deployment is on pds18. Use one dedicated `assetq` Unix service identity, a separate role/database in the existing PostgreSQL cluster, and immutable objects under a dedicated `/srv/assetq` data root. Preserve other services, ownership, databases and the existing Langfuse object store.

Start with streamed filesystem-backed object storage behind the AssetQ API; no additional S3 server is required. Store manifests and ownership in PostgreSQL. File/object keys are server-owned, never arbitrary caller paths. Only committed, verified objects appear in successful results; retain or reconcile crash leftovers safely.

R7 requires a configurable filesystem free-space floor before dispatch and during streamed writes. Serialize writers across processes and charge temporary and orphan bytes. Failed space observations refuse new effects with an unknown reason; exhaustion preserves committed assets and existing provider identities.

Metadata admission leaves a separately configured recovery reserve. Every metadata writer, including events, leases, cancellation, observations and migrations, uses serialized physical-size checks covering indexes and TOAST.

Bound each transaction and check its physical growth before commit; exhaustion stops further writes without deleting evidence. Reads and reconciliation resume from retained state after capacity returns.

Project-scoped APIs provide authenticated uploads and downloads, checksums, size/type limits, pagination and bounded retention. Permanent asset IDs are independent of expiring transport URLs. Do not automatically publish private inputs or expose storage directories as a public web root.

Installation first prepares an isolated candidate and verifies the concrete service user, directories, database, local endpoint, quota and rollback procedure. Backup/restore qualification follows the initial-delivery deferral below. Additive activation uses existing host mechanisms and preserves other workloads.

Publish the approved `assets.ufobe.com` name through the existing Caddy HTTPS entry on pds18, proxying to the loopback-only AssetQ service. Change only this named DNS/site configuration after verifying its current ownership and target; preserve apex/wildcard records, other sites and existing access guards. Verify TLS, browser/API authentication and rollback on the exact candidate.

Another public hostname, firewall change, destructive migration or unrelated service restart requires separate concrete approval. The website does not authorize direct public storage listeners or directory listing.

For initial delivery, defer new backup work and both local and independent external backup/restore qualification. Preserve existing host backups and report deferred or unobserved backup/restore evidence as unknown; these gaps do not block initial functionality.

AssetQ must not be stopped or restarted for backup. No cold-backup timer, backup-driven outage, rotation or additional backup platform is authorized for initial delivery.

Any backup verification claim requires consistent object/metadata snapshots, including manifests, job state and registry revisions, plus isolated restore/hash and authenticated retrieval evidence for its named scope. A local receipt never proves independent external restoration. Provider URLs and RAID alone are not backup evidence.

Garbage collection cannot delete referenced, pinned or retained assets; no destructive collection runs before a reviewed inventory and policy.

## Control and observability

Expose jobs and step progress, queued/active/uncertain work, provider availability, per-method outcomes, usage and spend, comparison evidence, recommendations, storage usage/integrity and backup/restore observations. Keep generation success, asset validation and owner acceptance separately visible.

Use the AssetQ site for asset operations and pipeline benchmarks. Reuse DashQ for summary metadata/deep links and ServerQ for host observations when their integrations are ready. AssetQ API/CLI remain independently operable.

Langfuse can receive correlated traces and evaluation records; its outage cannot own or erase job state. Keep primary assets in the AssetQ store.

## Linked specifications

- [Architecture](docs/architecture.md): dependency direction, runtime extraction and service boundaries.
- [Acceptance](docs/acceptance.md): real user journeys and recovery evidence.
- [Meshy baseline](docs/meshy-baseline.md): source pipeline and scope limitations.
- [Service](docs/service.md): lifecycle model, API, limits and immutable storage.
- [Deployment](docs/deploy.md): candidate verification, host requirements, additive install and rollback.

<!-- contract-budget: bytes=61440 lines=300 -->
