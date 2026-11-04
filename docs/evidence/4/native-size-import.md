# Retained native-size image import

AssetQ #4, R1-R8. Owner answer [6624256251](https://github.com/taskqorg/assetq/issues/4#issuecomment-6624256251), verbatim: "Native-size method - + imports retained 1254 PNG unchanged, no new turn; - no <=1024 guarantee."

The contract changes before implementation. The consumed turn, original request and version 5 remain immutable. Version 6 imports the retained native PNG with its source provenance and no generation or downscale. The original out-of-bounds job must fail with the validation reason; the new import job uses the existing executable lifecycle. The accepted crate remains the 3D input.

Published prerequisite `62ad9989a59c8bc65769cb567b2222909d4b142c` supplies the accepted import-refusal model, trace and core/service checks. Its application preserves the branch's unknown-observation and cancellation handling. The native-adapter defect fixed here was that PNG validation failure discarded the completed Runtime report and became an unknown turn. Missing completion or whole-operation proof must still remain unknown, without release or replay.

## Verification and operator recovery

The new core regression failed before the native fix with `PNG dimensions exceed worker/request limits` returned as an error. It passes after a completed output refusal becomes a failed provider result, while missing completion proof still retains reservations. The existing forged-path check still prevents import; its observable result is now the required terminal refusal. Prerequisite fixture adoption first failed to compile because this branch's `Output` requires `evidence`; adding invented `null` evidence preserves its assertions.

A one-time offline probe copied only the retained native journal into a temporary directory. Initially it failed `app-server recovery scope differs`: ambient worker markers changed the reconstructed launch environment. Restoring the operator launch's explicit `AGENTQ_LAUNCHER=assetq` marker in the offline Launch fields passed the unchanged Runtime scope check. No role variables or native configuration changed; no native launch/input occurred. A persistent fixture protects the same marker/recovery boundary.

The actual checkpoint now restores `native_outcome: ok`, thread/turn and usage through pinned `Runtime::recover`/`report`, without parsing a second native report format. Full PNG decoding confirmed 1254 by 1254, 1,806,057 bytes and SHA-256 `880bdc25d7add790a766bad2123aa36b4157c6585e991c8d07743cf0be35e960`. Recovered usage is 30,520 uncached input + 120,960 cached input + 723 output = 152,203 tokens, model `gpt-6.1-sol`; charge remains unknown. The temporary probe was removed after this observation; the fixture remains. No second model turn was used.

Use the existing Mac operator channel; TaskQ worker context cannot reconcile external app servers. With the exact candidate binary, original private qualification directory and existing registered workspace, run from this worktree (replace the three private path placeholders):

```sh
python3 scripts/import_retained_image.py \
  --binary '<exact candidate assetq binary>' \
  --workspace '<existing registered AssetQ root>' \
  --directory '<original private qualification directory>' \
  --sha256 880bdc25d7add790a766bad2123aa36b4157c6585e991c8d07743cf0be35e960
```

The driver reopens only that retained isolated PostgreSQL cluster/store and starts an import-only local API. Its source job and import key come from the original acceptance receipt; reruns recover the same import identity rather than generate. It verifies the old job is failed with the original reason/request/version and one submission, submits version 6 through the durable API, verifies duplicate identity, downloads/hash-checks all manifest outputs and audits both jobs. A new local member token stays in memory. Native accounts/grants/settings remain untouched.

The CLI verifies the expected original hash before importing; the driver also compares authenticated original retrieval with that hash. Runtime checkpoint, source request, native original and all receipts stay retained. Any imported objects from a failed job remain outside a success manifest. Do not change the original request/version, edit database state by hand, regenerate, claim visual acceptance or report missing charge as zero. The import's provider charge is zero because it performs no generation; original charge remains unknown in linked source evidence.

## Native operator result

PM executed the recipe once on 2026-11-04 with the exact binary below, exit 0 ([operator readback 6624912143](https://github.com/taskqorg/assetq/issues/4#issuecomment-6624912143)). The worker independently checked the retained API receipts and downloaded payloads. This is an import of the consumed native turn, with **zero new Runtime inputs**, no resampling and no crate replay (R4-R6).

- Source job `006a1ef7-a38f-43ad-af9f-a61f51794cd2` is now `failed`, reason `validation failed: PNG dimensions exceed worker/request limits`. Version 5, its original 1024-by-1024 constraints, text/attachment IDs, operation `86e7d75b-6d30-421a-a8d8-e52889e1c958` and one submission remain unchanged. Reserved credits are zero and no lease remains. Cancellation still reads `uncertain`; it is not relabeled confirmed.
- Import job `c6a8338d-034c-45af-a694-daf0482cf382`, version 6, succeeded through submit/import/validate with operation `9ff753ab-0fe0-4e13-b4ab-1baea5a8dc5c`. The duplicate returned this same job with `created: false`. Its one local provider submission is an import, not another native turn; provider credits are zero.
- Both event audits report `matches stored state` (13 source events, 12 import events). The retained AgentQ reconciliation reads `completed`; the original slot reads `released`, reason `consumer confirmed entire turn ended`.
- Authenticated downloads of all three manifest items match their object metadata, byte counts and SHA-256 hashes. Independent PNG chunk CRC and decompression checks confirm the original is 1254 by 1254 and the preview 512 by 512.

| Output | Immutable object ID | Bytes | SHA-256 |
|---|---|---:|---|
| Original PNG | `b8132d52-7255-48de-aeae-3a5f8cf08411` | 1806057 | `880bdc25d7add790a766bad2123aa36b4157c6585e991c8d07743cf0be35e960` |
| Preview PNG | `340be612-87b5-4c10-b1ad-883735859f0e` | 289790 | `95b65172f5d9b778e8c6ee789151a8b0e55edc4287ae6ebbede168b2ff3c7267` |
| Native evidence JSON | `6a3905d1-03d9-4755-a7b4-db8ff3014986` | 2411372 | `6fa7bfe119a36b4bc8282f07d2987bb23519fd8bf70d296eb835591428a47ba8` |

Each object's provenance links the original source job/effect/request, method/runtime revision, verified swatch attachment, native image item, thread/turn, dimensions/hash and source usage. The evidence download retains the completed `imageGeneration` item, original allowed saved path and Runtime report. Raw native paths, account/authority data and the large native image item remain private. Source usage is still 152,203 tokens; subscription charge and image backend model remain unknown. All three objects retain visual acceptance `pending`. The owner-accepted crate asset `2c887345-9f66-491f-8a0c-81d81cf0c4af` remains the initial 3D input.

The earlier missing manifest/retrieval result is now PASS for this exact local import. General automatic dispatch remains refused by the approved contract. Exact installed adapter source/build CI mapping and green PR merge-result CI remain unproved; this receipt does not qualify managed deployment, website or deferred backup/restore.

## Frozen candidate checks

Implementation `1f0de0fde12eb51f5b651319c658d8004c63f7b8`: `python3 scripts/check.py` PASS in 29.36 seconds on Mac including setup, with four checker checks, four driver checks, 20 core, 29 isolated PostgreSQL service and two trace checks, plus pinned lint/format/whitespace. The full gate ran once on this result candidate; no rebase or repeat because main moved. The merge of the already-adopted prerequisite retained the identical Git tree `04139f6bc1fa1f8f66885650453599c81e61cf0e`. Native/Ubuntu/merge-group evidence is separate.

Debug binary SHA-256: `dc74e844d4fa4e990a6e27645b75901717ec411df7952be51d32b50551765c9e`; the successful operator receipt matches it exactly. The earlier import-driver invocation in worker context refused before effects: `worker cannot reconcile external app servers`. That invocation remains **sandbox-blocked**, never PASS. No role variable was removed, no original job/slot was changed by that invocation, and no new Runtime input was sent. The separate operator execution above establishes native import, manifests and authenticated retrieval.
