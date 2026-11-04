# Managed image-to-3D delivery evidence

Task [#6](https://github.com/taskqorg/assetq/issues/6), AssetQ R2-R8. This record preserves the [owner request](../../intake/2026-11-01-owner-request.md), the managed run reported in [PM note 6632591828](https://github.com/taskqorg/assetq/issues/6#issuecomment-6632591828), and the owner decision and isolated recovery repeat in [note 6633444528](https://github.com/taskqorg/assetq/issues/6#issuecomment-6633444528).

[PM readback 6634471512](https://github.com/taskqorg/assetq/issues/6#issuecomment-6634471512) supplies the retained model measurements, provider-wait observations and isolated host storage-fault results below. [Owner verdict 6634512869](https://github.com/taskqorg/assetq/issues/6#issuecomment-6634512869) records the scope of visual acceptance. These observations supplement the original receipts without changing them.

The host observations below are attributed to PM. The Windows worker verified the CI candidate bytes and existing source image locally; it did not access the private installation receipt, redownload the new host payloads or repeat a provider request. Public notes are the available host evidence; private native receipts and original results remain with the operator. This documentation publishes no credentials, private infrastructure locators or provider download URLs.

## Candidate and activation

| Identity | Value |
|---|---|
| Binary source | `ed30cd19aa5e83e726a99f59e06d705b3e5b0545` |
| Candidate | `assetq-0.1.0-ed30cd19aa5e-x86_64-unknown-linux-gnu.tar.gz` |
| CI artifact | [Run 41834320114](https://github.com/taskqorg/assetq/actions/runs/41834320114), `assetq-candidate`, artifact ID `13324262841` |
| GitHub artifact SHA-256 | `c9526f46854ad331a1cdc15f4b3104b2947a0c327439774d180c923f08e1b1fc` |
| Inner release tar SHA-256 | `2e08ee3de2631061ea5f1f2de547df2ba3928f58470f4e7e147ce13ec9443297` |
| Executable SHA-256 | `5aacde4b30e58a86d0e6d7c42b80532364f2ba5533cfb277081367ef2938c0d4` |
| Target / schema | `x86_64-unknown-linux-gnu` / `1` |
| Build / host glibc | `2.43` / `2.43` (host value PM-observed) |
| Preserved rollback executable SHA-256 | `f0b2d5be8958e94cb95f9cf85fd59692bbd69cb8f905c7ecc7baf3859b9ab462` |

The GitHub artifact digest identifies the enclosing artifact, not the inner release tar. The worker matched the tar to its `SHA256SUMS`, the packaged executable to both `candidate.json` and its inner checksum, and the packaged contract/service/deployment/Meshy specifications to the source Git blobs. The original crate PNG also retained its published hash. Linux execution was not attempted on Windows.

The source has green [merge-group CI 41834037186](https://github.com/taskqorg/assetq/actions/runs/41834037186) and push CI 41834320114. These checks qualify that source/candidate; publication of this documentation still requires its own merge-result CI. The candidate's `synthetic_provider_only: true` metadata is not general generation authority. Native image launches remain refused and Meshy dispatch retains its exclusive operator binding.

PM reports successful host `selftest`, a root-only private installation receipt, loopback-only listening, and preservation of PostgreSQL, the web proxy and other services. Activation restarted the AssetQ unit once. Qualification then stopped that unit from about 13:35Z to 13:57:12Z on 2026-11-04, approximately 22 minutes offline. `NRestarts=0` after activation does not erase those deliberate stops.

The owner explicitly accepted this historical A1 exception in note 6633444528. It is not a standing permission for downtime, ordinary restarts, configuration changes or future qualification effects. The repeat left the managed unit active and unchanged. Rollback schema compatibility was inspected: both releases use schema 1 with no schema statement change. No actual rollback execution is claimed.

## Original image and managed result

The [source manifest](../5/source-manifest.json) and [native image receipt](../4/native-receipt.json) retain the original generation job `0e1fdfd2-9f45-46a0-ae27-0b4ba6558968`, method `image.worker.v1@2`, original asset `2c887345-9f66-491f-8a0c-81d81cf0c4af`, runtime pin, tool identity and usage. The source is the unchanged 1254 by 1254, 1,936,663-byte PNG with SHA-256 `fe2077d15b0c50617b84c6267277d46339c6a8cedbd6334e923cfa5475529baa`. Image subscription charge remains unknown. Its owner content acceptance is recorded separately from the original receipt's immutable `pending` field.

PM used managed source object `3268ca7f-e3a8-4bd2-ac93-85c5d638ccc5` with that same hash. This is a managed copy of the existing image, not a new generation or a replacement original identity. The original chat had ended before these later 3D runs; the retained image remained usable without resuming it.

| Managed observation | PM-reported result |
|---|---|
| Job | `eb75039d-87bb-498e-9808-6b52e43463dd` |
| Method | `model.meshy-prop.v1@2` |
| Provider operation | `01a1a6c5-4d6f-719f-a60b-8f04dd24c344` |
| Recipe | Version 2: explicit `meshy-t2`, Smart Topology, target 3000 triangles, 2K PBR |
| Outcome | Provider `SUCCEEDED`; job succeeded with nine manifest items, one submission |
| Native validation | Blender 5.2.3 LTS, `valid: true`, 2815 triangles against target 3000, delta -185 |
| Geometry and UVs | One mesh object, 424 components, 2815 faces and 2815 UV faces |
| Dimensions | 1.0 x 1.0 x 0.9281949996948242, provider GLB units; no real-world scale inferred |
| Packed textures | Three, each 2048 x 2048 pixels; original maps retained separately below |
| Provider defaults | `texture_model: null`; unreported defaults are not inferred |
| Remaining validator identity gap | Exact Blender executable hash not supplied |
| Cost | Provider 15 credits; account 5470 to 5455, delta 15 |

The later readback identifies the retained report by the `output-8` hash below and confirms `meshy-t2`, `smart-topology`, triangle topology, target 3000, `should_texture`, `enable_pbr`, `2k` and `glb`. Its ledger retains prior spend 30, actual 15, cumulative 45, balance before 5470, `cap: null` and no purchases. The later repeat raises the workstream total to 60; it does not rewrite this report's historical total.

After recovery, PM downloaded the source and all nine outputs through the normal managed unit, matching each manifest hash:

| Import key | Retained payload | SHA-256 |
|---|---|---|
| `output-0` | Original GLB | `5e24db4bfe192b4cb294849fa0683a76d3cd2a7b46eaa5fbdec526da8f353c40` |
| `output-1` | Base color | `8afab6205bc998dfc300a50d97e14119f679f0c4008d8326f03efc451063ebe8` |
| `output-2` | Metallic | `8d9ce86bcf0f0a24973207df30a10a6a81b8497ec14a5267dd91b63c6e02fdf6` |
| `output-3` | Roughness | `82db3c4ddafab52026b6fa98d14b1f3c43a207d52110752cd14b4b8ccd0f3536` |
| `output-4` | Normal | `bad305c4e10dee6c9557c3da8e9b0b7ee8f9d46c4c01e2ee265fef08a5da92b0` |
| `output-5` | Front view | `acd3fa8b1560a5a36caab669d3724372a7682131cb0bb31eb6084a35a4c17341` |
| `output-6` | Back view | `7868f209c99369d3978a97c506fc0b4fbb8a543ab4316440d7469a10a9a23c98` |
| `output-7` | Side view | `7b256082a94ff483d06d857a58351a3119702919f687210a0b62f0344510215d` |
| `output-8` | Validation report | `1db28cbb5034c77000aed67cc73f5eac818c2e64969e65605da30c1a3211605c` |

For object metadata, content, job and result reads, the owning token returned 200, a foreign project token 404, and anonymous requests 401. Foreign download and submit-with-attachment attempts returned 404; the foreign project object list was empty. The zero-budget, job-free isolation probe project remains retained; its token was not kept.

PM's `restore-check` on the current managed database/store returned `verified=true`: schema 1, one job, ten blobs, 18,881,638 object bytes and 1,003,814 retained temporary bytes. This is a current-store integrity observation, not a backup or isolated restoration. `ops` reported zero uncertain submissions and a non-full store. Managed partial-import and full-store faults were not induced; the separate host qualification below supplements source regression coverage in `partial_import_resumes_and_full_store_preserves_work` without claiming a managed-store fault.

## Recovery: failed procedure and bounded repeat

The managed run's control process received TERM while the provider was `IN_PROGRESS`. Its immediate restart failed with `Address already in use` because the driver omitted the dedicated listener selection and used a port occupied by another isolated process. No AssetQ process serving that qualification run ran from about 13:35Z to 13:56:38Z; the remote task completed during that gap. Recovery preserved the job/provider IDs, returned `created=false`, retained one submission and produced an audit matching stored state.

That first recovery raised `ASSETQ_MAX_EXECUTION_MS` from 600000 to 86400000 after the original deadline elapsed. It proves retained-operation recovery after remote completion, not compliance with the original execution bound or a restart while the provider remained running. The later repeat supplies that separate evidence; it does not rewrite the first run.

PM repeated the qualification in a fresh isolated pds18 store using the exact candidate executable. The managed store's one-effect selector remained intact; no second managed POST or managed-unit downtime was introduced. This was an owner-authorized separate example, not a replay of either retained operation.

| Bounded isolated observation | PM-reported result |
|---|---|
| Job | `27f950ee-2665-42ed-a934-ffe0f422b4da` |
| Provider operation | `01a1a6fb-c07a-760e-8eee-130be20c36be` |
| Source / job budget | Same original crate hash / 15 credits |
| Deadline before and after restart | `ASSETQ_MAX_EXECUTION_MS=600000` |
| 14:34:41Z | Provider `IN_PROGRESS`; control received TERM |
| 14:34:43Z | Control listening again; provider still `IN_PROGRESS`, progress 67 |
| Repeat submit | `created=false`, same job/provider IDs, `submissions=1`; one POST |
| Remote outcome | `SUCCEEDED`, 15 consumed credits, about 44 seconds |
| 14:36:09Z / 14:36:11Z | Restarted control imported outputs / job succeeded, step 3, effect done |
| Total elapsed | 98 seconds from first control start, inside 600-second bound |
| Native bounds, both processes | Address space 2147483648 bytes; stack 2097152 bytes; CPUs 0,1 |
| Downloads / audit | Nine manifest items hash-matched; GLB 7,330,860 bytes; replay matches stored state |
| Geometry, comparison only | 313 components, 2539 triangles, dimensions 1.0 x 1.0 x 1.00006902217865 in provider GLB units |
| Cost | Balance 5455 to 5440; delta 15 equals provider usage |

The repeat corrects the procedure's restart/deadline failure on the isolated store. Its individual output hashes and raw native receipts were not supplied in the public note, so this record does not invent them or claim Windows independently verified them. It does not establish a bounded restart on the managed store.

## Provider-wait capacity

PM readback 6634471512 reports the pre-restart status of both real Meshy jobs: phase `waiting`, effect `submitted`, `leased: false`, `submissions: 1`, fence 1. The bounded repeat retained the address-space, stack and CPU bounds listed above on both control processes.

On pds18, the exact candidate binary also ran `synthetic.fake` with one executor. A job configured with `polls=25` remained waiting and unleased while a second job submitted during that wait reached `succeeded`. The waiter then completed with one submission. This shows that a provider wait held neither a job lease nor the sole service executor. It supplements `waiting_jobs_release_executor_capacity` with a shipped-binary host observation; it does not supply native TaskQ/agent-capacity telemetry, which remains open.

## Isolated host storage-fault qualification

PM used the exact candidate executable SHA-256 `5aacde4b30e58a86d0e6d7c42b80532364f2ba5533cfb277081367ef2938c0d4` on pds18 with a separate database and data root and provider `synthetic.fake`. This qualifies the shipped binary's host storage path with a synthetic provider. It is not a live Meshy stream fault or a fault on the managed store.

| Case | Failure observation | Recovery or preservation observation |
|---|---|---|
| Partial import | `fail_fetch` broke `output-1`; job remained `active`, reason `import of output-1 failed: source_failed: synthetic fault: stream reset`, result `incomplete`, one object committed | Resumed to `succeeded`, one submission, two manifest items, zero temporary files |
| Full store during import | Quota set to used + 10, 438 bytes; job remained `active`, reason `import of output-0 failed: store_full: quota or free-space floor exhausted`, one submission | Both previously committed objects downloaded and hash-matched; PM supplied hash prefixes `ea292c54600b` and `48f96ad23720`, not full digests |
| Admission at capacity | 428 of 428 bytes used; submission refused with `HTTP 507 store_full: store uses 428 of 428 bytes` | No job created; provider-operation count stayed four |
| Capacity restored | Isolated quota returned to 1 GiB | Paused job reached `succeeded` within seconds, one submission, provider-operation count stayed four |

The HTTP admission refusal occurs before a job exists, whereas the in-process regression reaches its no-effect assertion inside a job. Both observations preserve the no-new-provider-effect requirement; they are distinct entry paths. Finally, all four host-fixture jobs audited as `matches stored state`; `restore-check` returned `verified=true`, four jobs, five blobs, 535 object bytes and zero retained temporary bytes. This verifies that fixture's current state, not a backup restoration.

PM reports that the managed service remained active, its `initial-effect.json` barrier remained present, and its store, quotas and originals were untouched. The fault run used no Meshy key or paid request. Inducing faults on the managed store remains outside scope without a reviewed operator plan; these isolated results do not authorize one.

## Cumulative credit ledger

| Run | Provider operation | Credits | Balance movement | Cumulative |
|---|---|---:|---|---:|
| First reconciled #5 attempt, retained uncertain local effect | `01a1a625-c6c6-75c8-bcc0-4b738fb2603b` | 15 | 5500 to 5485 | 15 |
| Successful isolated #5 example | `01a1a68b-1231-7340-84d5-e22ba3cbedf0` | 15 | 5485 to 5470 | 30 |
| Managed #6 example | `01a1a6c5-4d6f-719f-a60b-8f04dd24c344` | 15 | 5470 to 5455 | 45 |
| Isolated bounded #6 repeat | `01a1a6fb-c07a-760e-8eee-130be20c36be` | 15 | 5455 to 5440 | 60 |

The [#5 ledger scope](../5/operator-run/ledger-receipt.json) and original jobs/results remain unchanged. This is observed workstream spend, not an independent whole-account audit. The former 1000-credit ceiling was withdrawn by [owner answer 6629641665](https://github.com/taskqorg/assetq/issues/5#issuecomment-6629641665); the cumulative ledger remains retained. No purchases/top-ups or further provider requests are authorized. Unknown spend still refuses new effects under R4/R7.

## Verdicts and remaining acceptance

| Item | Evidence and limit |
|---|---|
| A1 installation | PM-observed exact candidate installation and private receipt; owner-accepted historical outage exception, not uninterrupted activation |
| A2 technical/live journey | Existing native crate feeds the managed Meshy result; nine managed downloads and retained measurements PM-verified: 424 components, 2815 triangles, dimensions 1.0 x 1.0 x 0.9281949996948242. No new image generation or general method qualification |
| A3 recovery/capacity | PM-observed bounded recovery while provider running on the isolated store with the same binary; both real waits were unleased, and the one-executor host fixture served another job. First managed recovery exceeded its original bound; native agent-capacity telemetry remains open |
| A4 storage/access | PM-observed managed hashes, project isolation and current-store integrity; isolated exact-binary host faults prove partial-import recovery, quota refusal and retained-object preservation with a synthetic provider. Managed/live-Meshy faults and an executed rollback remain unproved |
| A5 restore | Deferred/unknown under owner note 6558008840; neither local nor independent external backup/restore qualification is claimed |
| A6 owner visuals | PM presented the source and three managed-result views in the owner chat. Owner verdict on 2026-11-04: "accepted as a pipeline check, not as a finished game asset." Source image accepted previously. PM's softer-corner and dark-stain observations remain separate from that verdict |
| A7 publication | This evidence and the linked deployment guidance identify source, binary, methods, checks and ledger. Documentation merge-result CI remains required; original inputs/results stay retained |

Task #6 remains incomplete where the table names open evidence. The published source's green checks, PM's live observations, restoration status and owner visual judgment are separate verdicts. No website, general Meshy recommendation, another executor, complete managed failure matrix or future generation authority is qualified by these examples.

## Read-only retrieval handoff

Use the installed candidate's existing protected project configuration and operator channel described in [deployment](../../deploy.md). `ASSETQ_URL` selects that already approved API endpoint; `ASSETQ_TOKEN` remains in protected configuration. Do not expose either private configuration or signed provider URLs in a public receipt.

```text
assetq status eb75039d-87bb-498e-9808-6b52e43463dd
assetq result eb75039d-87bb-498e-9808-6b52e43463dd
assetq audit eb75039d-87bb-498e-9808-6b52e43463dd
assetq download 3268ca7f-e3a8-4bd2-ac93-85c5d638ccc5 crate-original.png
```

For each returned manifest item, use its actual `object_id` in `assetq download <object-id> <new-local-file>`, then compare SHA-256 and byte count to that manifest. The table above supplies the managed output hash cross-check. Import keys are not object IDs. Keep the isolated repeat's job and outputs with its separate protected store configuration; it is not visible through the managed project's manifest.

These reads require no new generation, service restart, ledger reset or selector removal. Retain committed assets and temporary/orphan bytes. Do not induce a full managed store or partial import, run backup work, change quotas/deadlines, or execute rollback to fill an evidence gap without a separately reviewed operator plan.
