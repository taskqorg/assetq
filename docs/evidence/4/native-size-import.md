# Retained native-size image import

AssetQ #4, R1-R8. Owner answer [6624256251](https://github.com/taskqorg/assetq/issues/4#issuecomment-6624256251), verbatim: "Native-size method - + imports retained 1254 PNG unchanged, no new turn; - no <=1024 guarantee."

The contract changes before implementation. The consumed turn, original request and version 5 remain immutable. Version 6 imports the retained native PNG with its source provenance and no generation or downscale. The original out-of-bounds job must fail with the validation reason; the new import job uses the existing executable lifecycle. The accepted crate remains the 3D input.

Published prerequisite `62ad9989a59c8bc65769cb567b2222909d4b142c` supplies the accepted import-refusal model, trace and core/service checks. Its application preserves the branch's unknown-observation and cancellation handling. The remaining native-adapter defect is that PNG validation failure discarded the completed Runtime report and became an unknown turn. Missing completion or whole-operation proof must still remain unknown, without release or replay.

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

Open: execution of this import-only operator recipe and its manifest/authenticated-retrieval receipts, exact installed adapter source/build CI mapping, general automatic dispatch qualification and merge-result CI. Native tool/path and the original PNG are observed; successful immutable import is not yet claimed.
