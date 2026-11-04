# Management site design package

First design checkpoint for [task #9](https://github.com/taskqorg/assetq/issues/9), not an implemented or approved website. The owning requirements are [AssetQ R1-R9](../../assetq.md), [website acceptance](../acceptance.md#website-and-api-origin) and the [service contract](../service.md). Apply the pinned specq design workflow, operator rules D1-D14 and fidelity rules F1-F7.

## Request and baseline

The [owner intake](../intake/2026-11-01-owner-request.md#website-direction) preserves the original request:

> assets.ufobe.com - сайт для api и для менеджмента наших ассетов - просмотр, поиск, бенчмарк пайплайнов и т.д.

Task note 6556018047 adds, verbatim: "это то что нам надо создать." The final outcome is the deployed, authenticated application/API and real browser journeys. This package cannot satisfy that outcome by itself. Note 6558731338 requires specification-led design and visual approval before UI implementation.

Inspected baseline: `47f2cbd18c94bdbc6070696f767e6c66d8601640`. There is no browser application or prior visual approval in this baseline. Existing API, CLI, durable jobs, registry, objects and evidence remain the foundation; no replacement store, removal or lifecycle change is proposed. Task #7's versioned evaluation API is already present.

The intake's Meshy testing amendment supersedes its historical 1000-credit ceiling. This site grants no extra spending: existing per-job/project limits, reconciliation, operator binding and refusal of real-provider comparison execution remain enforced. No generation, paid evaluation, promotion or host effect occurred during design.

## Scenarios and navigation proposal

These are proposals for owner review, not new decided behavior. All routes below are browser views under `https://assets.ufobe.com`; `/v1` retains its published machine contract. A project in a URL selects only an already authorized scope.

| ID / acceptance | Intent and action | Observable result and recovery | Proposed destination |
|---|---|---|---|
| W1 / A2,A5,A6 | Find the retained crate by text, authorized project and media type; page through results | Private previews and bounded server-side results; no foreign-project hits or counts. Empty results offer clear filters. Failed reads retain explicitly aged data; project changes clear old private data | `/assets?project=ID&q=crate&type=model&after=CURSOR&selected=OBJECT` |
| W2 / A2,A5,A6 | Open an object, inspect versions, parent/source job, method, hash, validation and acceptance; download original | Permanent object URL; retained render labeled as a render, never the original image. Unsupported types show MIME/download. Missing or foreign object is unavailable. Back restores filters, selected item, scroll and focus | `/assets/OBJECT?project=ID`; query state stays on the list history entry |
| W3 / A3,A5,A6 | Select a pinned method, enter text and existing/uploaded attachments, inspect constraints and maximum credits, submit once | Durable job ID after admission; unchanged retry uses its original key. Unqualified method explains refusal. Validation errors retain text/attachments; a request never changes grants, model or budget authority | `/jobs/new?project=ID&method=ID@VERSION`; drafts stay local to the open tab, never URL-encoded |
| W4 / A3,A6 | Reopen a long-running job, reload, inspect events/results or request cancellation | Same job/provider identity and original method; no replay on navigation. Requested cancellation differs from confirmed/stopped/uncertain. Missing usage remains unknown. Failed import never displays a successful manifest | `/jobs?project=ID&phase=PHASE`; `/jobs/JOB?project=ID` |
| W5 / A4,A5 | Inspect API/model/pipeline parameters and create a method version | Exact immutable versions and qualification limits; edit produces a new version, existing jobs keep old pins. Member sees owner requirement instead of an effective edit control | `/methods?project=ID&operation=OP`; `/methods/METHOD/versions/VERSION?project=ID` |
| W6 / A4,A5,A6 | Pin dataset/evaluator/method revisions, constraints, repeats and job/credit maxima; inspect comparison, resume and judge outputs | Fixed sample identities, ordinary durable jobs, previews and quality/cost/time/failure/missing evidence. Refused real-provider runs cannot be enabled by the form. Human judgments remain distinct from technical validity and verified identity | `/benchmarks?project=ID`; `/benchmarks/new?project=ID`; `/benchmarks/COMPARISON?project=ID` |
| W7 / A4,A5,A8 | Inspect scoped recommendation and service observations; enter through sign-in or a DashQ deep link | Exact operation/use-case/constraints and observed revision/time, including explicit none. Stale health and deferred backup/restore remain unknown. Sign-in never confers project or promotion authority | Benchmark detail recommendation section; `/operations?project=ID`; explicit sign-in/denied/unknown-path views |

Direct entry, Back/Forward, reload and a shared filtered URL must reproduce the view subject to authorization. Unknown paths/query values explain recovery rather than silently opening another tab. Private prompts, tokens, signed URLs and attachment contents never enter navigation URLs. Refresh preserves current selection and edits.

## Behavior and controls

One persistent navigation rail contains Assets, Jobs, Methods, Benchmarks and Operations. The authorized-project selector is global and explicit. Assets is the initial destination; New generation stays beside the Assets/Jobs heading. Service capacity is labeled as service capacity, not invented per-project storage.

Asset detail groups preview/download, validation, human acceptance, immutable identity, source relationships and version history. Versions are related immutable objects where provenance establishes that relationship; unrelated objects are never inferred to be versions. The initial 3D preview uses retained verified render objects; an interactive GLB viewer is unnecessary for the core journey.

Method detail exposes operation, provider/model/API/pipeline revisions, supported inputs/outputs, parameter schema, capabilities, limitations and validation recipe. Create version is separate from dispatch. Benchmark setup selects immutable datasets/evaluators or links to their owner-only version registration forms; it displays the computed sample matrix before admission.

Comparison detail keeps each case and repeat visible with output links, technical outcome, human judgment, credits, elapsed time, failures and missing reasons. Resume reuses sample identities. Project members can append permitted judgments; reviewer attribution is not claimed to be verified person identity. Owner-only proposal promotion displays exact scope/evidence and a confirmation of the affected scope. No automatic winner or promotion is added.

## Visual choice

The review set contains three static design compositions at 1440 by 1000 logical pixels, scale 1. They are not screenshots of a running browser or proof of interactions. Editable layout data is in [reference-scenes.json](management-site/reference-scenes.json); PNGs are the visual review artifacts.

1. **Visual library with adjacent detail — recommended.** [Library](management-site/library.png) groups retained renders and a file fallback while keeping identity and source beside the selection. Benefit: recognize assets and inspect provenance without losing search context. Cost: fewer metadata rows fit at once.
2. **Table library with adjacent detail.** [Table alternative](management-site/table.png) uses the same records, selection and detail. Benefit: scan file metadata faster. Cost: smaller thumbnails make visual recognition slower. This is an alternative default, not a second required interface or a new view toggle.
3. [Benchmark detail](management-site/benchmark.png) carries the recommended typography, navigation and evidence hierarchy into comparison work. Its sample plan/results are explicitly invented layout fixtures, not a real experiment, recommendation or new dispatch authority.

Proposed styling: neutral light surfaces, restrained blue selection/actions, 16 px body text, at least 14 px secondary text, 24 px section headings, visible keyboard focus and 4.5:1 text contrast. Colors never supply status meaning alone. Implement using shared CSS tokens after approval. Narrow layouts reflow detail beneath the list; ordinary text/forms do not require horizontal scrolling. Respect browser/system text scaling and theme preference.

The only actual imagery is the already published isolated #5 render set. `view-front.png` and `view-side.png` are retained Blender views, not the source PNG or managed #6 model. The selected GLB is `8d702403-0bf1-40de-baed-ad5c8ebbbbd7`, 6,424,280 bytes, 3022 triangles, 447 components, 15 reported credits and 313.20 seconds for that isolated job. Source: [operator evidence](../evidence/5/operator-run/README.md). The project display label and friendly object names are layout fixtures. Live acceptance must use the actual managed IDs from [#6](../evidence/6/README.md), not substitute these images or invent measurements.

## State fixtures and proof plan

| Fixture / paths | Required presentation and recovery | Check after implementation |
|---|---|---|
| Normal / W1-W7 | Real scoped records and exact revisions; sources/validation/acceptance separate | Real retained image-to-3D browse, provenance and SHA-matched retrieval; API readback |
| Empty / lists, search, version/recommendation history | Explain no matches/no history/no qualified route; contextual authorized next action | Empty project and unmatched search; no fabricated preview or recommendation |
| Loading / all reads, preview, upload | Named pending action, no zero counts or erased edits; cancellation of transport does not cancel a job | Delay reads/uploads and navigate/reload; preserve drafts and selected state |
| Long-running / W3,W4,W6 | Durable ID, last observation, step/event history, unknown remaining time; no false percentage | Reload and reopen retained job; duplicate submit remains one job |
| Cancelled / W4,W6 | Requested, confirmed, locally stopped and uncertain shown distinctly; actual/unknown charge retained | Shared model traces plus browser cancel/readback; no paid replay |
| Failed / all writes, download/import | Exact safe reason, retained inputs, no success toast; original-effect reconciliation before replay | Existing failure records and denied mutations; invalid input retains edits |
| Stale/unknown / W1,W4,W6,W7 | Observation time/reason; last-known values labeled stale, missing quality/usage never zero | Failed polling and null metrics; explicit incomplete comparison and none recommendation |
| Denied / every view, preview, download and mutation | No foreign object/name/count; no private flash before scope check; safe sign-in/project recovery | Anonymous, foreign-project, member-versus-owner and revoked/expired sessions; missing/wrong CSRF and origin |
| Dense/long/boundary / all views | Long names/IDs, 32 samples, limit errors and pagination remain readable; no hidden evidence | 1440, 1024 and 390 px widths; 100/200% text; light/dark; keyboard focus, scroll and error recovery |

All runtime/matrix cells above are **Not tested** at this checkpoint. The static compositions demonstrate only proposed information hierarchy and density in one light-theme size. Browser access is unavailable in the current tool session; this is not a product failure or a passing browser check.

## Foundation and security handoff

Read-only boundary review confirms the shared API already protects project grants, immutable objects, durable jobs and owner-only registry changes. Preserve `src/api.rs:142-146` Bearer authentication, `src/db.rs:392-405` same-project lookups and `src/api.rs:69-78` private download/checksum headers. Machine callers must not gain cookie or identity-header authorization.

Required work after approval:

- Extend the existing project-scoped `GET /v1/objects` query with bounded text/type filters; do not search only a downloaded first page or create another catalog. Specify searchable existing name, source prompt and method fields in `docs/service.md` before code. Project selection resolves an authorized grant before query/count/pagination.
- Add an authenticated preview contract without weakening original download behavior. Decode/validate supported raster bytes under bounds; use only verified retained render relationships for GLB. MIME supplied at upload is not evidence that bytes are safe to inline. Unsupported/active formats get file-type/download fallback, with private/no-store/nosniff responses.
- Qualify the existing host guard and verified identity-to-project/role mapping before defining the browser adapter's exact routes/session mechanism in `docs/service.md`. Keep credentials server-side. Unverified headers or caller project text cannot create authority. Session expiry/logout/revocation, secure cookies, session-bound CSRF and same-origin checks must cover every browser mutation, including upload/cancel/scoring/promotion.
- Render untrusted names, prompts and judgments as text, with a restrictive content-security policy. Do not cache private data in shared/service-worker caches; clear it on session/project change. Preserve recovery/cancellation writes under resource exhaustion.
- Reuse `/v1/jobs`, `/v1/methods`, `/v1/comparisons`, datasets/evaluators, recommendations and status. Preserve synthetic-only comparison execution and general native dispatch refusal. Method qualification, provider health, technical validation and owner judgment remain separate.

The exact host guard, authorized mappings and HTTPS configuration were not inspected. No guessed header names, credentials, DNS target, listener port or production action is approved by this document.

## Delivery and remaining acceptance

Before implementation, the owner chooses the library structure and approves the representative visual candidate. Record the answer on #9, then promote only the selected proposal into the owning contract/service/design specification before dependent code (R1; design workflow steps 5-8). Reuse the same task/worktree/PR; an ask does not complete A1.

After implementation, run affected browser/API checks against the frozen candidate, the R8 canonical gate and merge-result CI. Package the exact source commit, executable/UI digests, fixture identities, screenshots and interaction evidence. Mocks never qualify actual image/3D retrieval or the live host (A6/A7).

For A8, prepare a restricted handoff through the private host task: named DNS ownership/current target/TTL, exact Caddy site and existing guard, loopback upstream/service identity, signed candidate digest, origin/CSRF/session mapping, TLS checks, retained active jobs and site-specific rollback/readback commands. Concrete values and host receipts remain unknown until inspected there. Publish only `assets.ufobe.com`; preserve other records/sites/services and existing backups. A deployed authenticated journey remains required for final acceptance.

Current verdict: A1 **partial** (scenario/behavior/visual package prepared; approval pending); A2-A8 **Not implemented/Not tested by this change**. No website, binary, backend route, host configuration or provider state changed. New backup/restore qualification remains deferred/unknown under the contract.

## Checkpoint verification

The three static images were rendered and visually inspected. Their text stays inside the 1440 by 1000 canvas; this does not prove browser layout, control behavior or responsive states. Scene data, image dimensions, source-image references, local document links and text-token contrast are checked separately from runtime acceptance.

`python3 scripts/check.py` passed its contract/whitespace and Python fixture checks, then failed at `cargo fmt --check`: `cargo-fmt.exe` is not installed for `stable-x86_64-pc-windows-gnu`. Rust/database checks did not run. No test, assertion, dependency or gate was changed to bypass this host gap. The full canonical gate is **not passed**; merge-result CI and actual browser/API checks remain required for final delivery.
