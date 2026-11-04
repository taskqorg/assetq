# One operator-bound Meshy qualification

Task [#5](https://github.com/taskqorg/assetq/issues/5), A1-A7, AssetQ R2-R8. The [pipeline specification](../../meshy-pipeline.md) owns the recipe and recovery behavior. This is a prepared procedure, not a live receipt. The worker has made **zero Meshy submissions**. The [source manifest](source-manifest.json) identifies the unchanged accepted crate; [official verification](official-api-verification.json) records public documentation, not account qualification.

## Prerequisites and authority

Use the existing operator channel on a qualified executor. A TaskQ worker cannot run `meshy-serve`; never remove role variables or change grants to bypass that refusal. Do not install credentials, buy credits, change production settings or deploy this candidate through this procedure.

The operator must supply an existing protected Meshy API credential, scoped AssetQ credential, dedicated qualification PostgreSQL/database configuration and one authoritative data root for this entire initial workstream. Reconcile **all** AssetQ attempts and stores against the account ledger; prior spend is unknown until that inventory is complete. Ensure no other executor can dispatch workstream effects during this run. A clone-local directory cannot establish cross-host exclusivity. Preserve the original ledgers and store; never initialize unknown spend to zero.

Qualify the exact headless Blender artifact and inherited resource boundary before binding the job. The adapter reads an existing Windows job-object process/job memory limit or Unix address-space limit and requires 512 MiB through 2 GiB. It changes no limit. Children inherit that boundary and receive only PATH/system/temp environment values; API/database credentials are excluded. Blender has a 120-second local deadline, 512-by-512 views, two CPU render threads and bounded input counts/texture headers. The initial run needs at least 1280 MiB of available object/temporary capacity above existing floors for its conservative 640 MiB reservation and actual writes. Native enforcement and actual rendering still require operator evidence on the selected host.

Run the read-only command on the exact candidate:

```text
assetq meshy-preflight <qualified-blender-executable>
```

Retain its binary hashes, memory readback and credential presence; it never enables dispatch or prints a key. Missing memory boundary, Blender or credential is a qualification gap, not permission to create or change host policy. Keep protected account configuration and all private source/download locators outside Git and public TaskQ receipts.

Re-read the current official [request schema](https://docs.meshy.ai/en/api/image-to-3d), [pricing](https://docs.meshy.ai/en/api/pricing) and [balance schema](https://docs.meshy.ai/en/api/balance). Verify the exact recipe still costs 15 credits. Preserve fresh source hashes and a free authenticated balance response privately. Stop if any parameter, balance, prior usage or expected cost is unknown or changed. No model substitution is authorized.

## Admit without effects

Use an isolated, retained qualification database/store and existing project-scoped grant, never production configuration. Apply this candidate's additive migration if that dedicated database needs the new method. Set `ASSETQ_EXECUTORS` to `0`, leave the synthetic provider unset, then start:

```text
assetq meshy-admit
```

Use its printed loopback endpoint as `ASSETQ_URL`; `ASSETQ_TOKEN` stays in protected configuration. Import the verified repository original and retain the returned local object ID:

```text
assetq upload docs/evidence/4/crate-original.png --type image/png
assetq submit --method model.meshy-prop.v1@2 --operation model.image_to_3d --key assetq-20261101-meshy-prop --budget 15 --attach <local-object-id>
```

Repeat the identical submit once: it must return the same job with `created: false`. Retain both responses, source object metadata and a hash-verified authenticated source download. The new local object ID is a copy; the provenance links the original crate asset and unchanged hash. Do not relabel a copy as the original record.

Stop only this isolated admission process after recording its accepted job ID. Other AssetQ services, agents, provider operations and sessions remain untouched.

## Bind and dispatch once

Create a private JSON profile with the following fields. These are field descriptions, not invented qualification evidence; replace unknowns only from independently retained observations. Do not commit the profile.

| Field | Required observation |
|---|---|
| `job_id`, `source_object_id` | Exact durable admission and verified local object IDs |
| `source_sha256` | `fe2077d15b0c50617b84c6267277d46339c6a8cedbd6334e923cfa5475529baa` |
| `assetq_binary_sha256` | Exact candidate binary from preflight |
| `blender`, `blender_sha256` | Qualified absolute executable path and artifact hash |
| `memory_limit_bytes` | Actual inherited native limit from preflight |
| `prior_credits`, `balance` | Known cumulative workstream spend and current account balance; never a guessed zero |
| `all_attempts_reconciled`, `exclusive_workstream` | True only after complete ledger reconciliation and exclusive executor control |
| `verified_unix_seconds` | Actual observation time; dispatch must occur within 300 seconds |
| `executor_evidence_sha256` | Qualified executor/resource/Blender receipt hash |
| `ledger_evidence_sha256` | Complete prior-attempt/balance/exclusivity receipt hash |
| `official_api_evidence_sha256` | Fresh exact-parameter/15-credit official verification receipt hash |

Use the same retained database/data root, `ASSETQ_EXECUTORS=1`, `ASSETQ_MAX_EXECUTION_MS=600000`, a bounded upload size (at most 64 MiB for this example) and the existing resource guards. Keep `MESHY_API_KEY` in protected process configuration. Start once:

```text
assetq meshy-serve <accepted-job-id> <private-profile.json>
```

The adapter verifies the exact registry version, source bytes, binary hashes, memory boundary, profile freshness and fresh balance. It persists intent before its only POST. A missing POST reply or missing original ID receipt stays uncertain; never rerun in another directory, clear a selector or create a replacement request. A preflight refusal is retained evidence, not a paid-generation success.

## Observe, restart and retain results

Read status and events through the same scoped API. After the original provider ID is retained and the provider is still waiting, stop/restart **only the isolated control process** once with ordinary `assetq serve` under the same protected configuration, resource boundary and store. It discovers the original receipts without new-dispatch authority. Repeat the original API submit and verify unchanged job/provider IDs. Do not interrupt Blender merely to obtain restart evidence.

```text
assetq status <job-id>
assetq events <job-id>
assetq result <job-id>
assetq audit <job-id>
```

Poll at bounded intervals through the ten-minute job budget. On timeout or requested cancellation, call `assetq cancel <job-id>` once and continue original-operation observation. Cancellation may remain unconfirmed; never DELETE the Meshy task. Unknown usage preserves reservations and prohibits further spend. A local service ending is not proof of provider completion.

For a successful manifest, download every item with `assetq download <object-id> <private-output-file>` and independently compare SHA-256 and byte count with the manifest. Retain source GLB, every original map, validation/provenance JSON and three textured views. Capture before/after balance and the original task's `consumed_credits`; reconcile the final ledger independently. Missing or unattributable usage is unknown, never zero. The public result contains no signed asset URLs, credentials or private host paths.

Commit sanitized source/result manifests, measured triangle/component/dimension/UV/texture values, actual cost, payload hashes and review views through this task after readback. Failed validation retains already imported source objects outside a manifest; do not regenerate. Present the original crate and textured views for owner judgment. A local run does not qualify the managed pds18 journey, website, a different executor or a general Meshy method.

## Current worker evidence and open acceptance

Windows read-only preflight reports the Meshy key absent, Blender not supplied and no qualifying inherited memory boundary; ledger/balance remain unknown. No paid effect or host configuration change was attempted. The operator prerequisites above are the concrete remaining action for A4/A5/A7. The canonical gate and fake-provider checks qualify implementation/recovery only; native rendering, actual spend, downloadable real GLB and final owner visual acceptance remain open.
