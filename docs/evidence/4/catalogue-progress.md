# Bounded catalogue progress during protected qualification

Task [#4](https://github.com/taskqorg/assetq/issues/4), AssetQ R3/R4/R6/R8. The continuation request is retained verbatim: "Ran once, not qualified: op qualification-4-pm-20261103T121434Z prepared, initialize + verify ok (verified-effective), model/list response not received, image-observe unknown; reconcile completed, slot released. Fix model/list under the protected launch, then ask PM for a new slot."

## Observed failure and consumer correction

PM's retained operation reports successful preparation, initialize and verified-effective authority. Its private consumer output also reports typed subscription identity. After `model/list`, the consumer returned `read-submitted`, then a progress batch of 18 notifications and an empty batch. Neither batch contains the catalogue response. AssetQ cancellation/readback remained unknown; PM subsequently observed host reconciliation `completed` and acknowledged original-slot release. These scopes remain separate; the operation is not reused.

The pinned Runtime's `send` allocates a native request ID and submits asynchronously; `progress` drains messages currently available. AssetQ's qualification command previously returned those instantaneous snapshots without waiting for the outstanding reply. This exposes a consumer timing defect. The old receipt does not prove why the native response was absent or that it would eventually arrive.

The consumer now journals read intent before dispatch and retains the Runtime checkpoint afterward. It reads the allocated pending ID from that pinned Runtime checkpoint, never allocates or overrides a native ID, and waits on `Runtime::progress` for that exact response. The wait is at most ten seconds and cannot exceed the original session deadline. Intervening notifications and foreign replies stay in the bounded journal and returned events. A native error stays a refusal, an empty catalogue stays empty, and a missing reply becomes explicit unknown without another submission. The driver frame shapes remain unchanged.

## Regression and retained protection

The invented existing protocol fixture now withholds catalogue responses until the consumer has journaled its notification. It emits a foreign response ID before release. The focused check failed before correction because the immediate progress read returned without even the expected notification; no matching response was collected. After correction, the same check passes for delayed, empty, refused and absent catalogue responses. It verifies one `model/list` dispatch and zero `turn/start` calls for each case.

Command: `cargo test --locked --lib image_worker_qualification_waits_for_catalogue_response`. The fixture has no credentials, image tool or provider effects. It tests the consumer's actual pinned Runtime send/progress/checkpoint path, not native Mac authority. The existing qualification allowlist, no-approval replies, original-operation/no-replay protection, deadline, journal limit, adapter anchor and generation refusal remain unchanged. No decided assertion was weakened.

`cargo test --locked --lib image_worker` passes all nine affected checks. The frozen implementation's `python3 scripts/check.py` passes on Mac in 32.03 seconds including setup: four checker checks, pinned contract lint/whitespace/formatting, 12 core, 26 isolated PostgreSQL service and two trace checks. Rebuilt debug executable SHA-256: `7d9186725dd5158e633116c7255549016e96482b12cbd31d4c497f739d5a2be6`. The adapter remains the retained `bd6dc0de` artifact and AgentQ remains `runtime-v0.2.0`; no source/runtime repin or native launch occurred in this continuation. Merge-result CI and refreshed native qualification are not claimed.

## Native follow-up

The subsequent `1f0f2e7` native attempt's first stdout event batch does contain the matching catalogue response: eight models, followed by an empty later batch. The [fresh readback and cancellation correction](cancel-readback.md) distinguish that delivered response from the initial PM summary. The catalogue wait was not changed again on the basis of that summary.

PM must reserve a new slot for the refreshed exact consumer and use the retained adapter bytes and independent pre-launch preparation in the [existing recipe](protected-consumer.md). No old operation, receipt-derived authority, grant change, role bypass, paid API or crate replay is authorized. The worker cannot reserve or control an external app server through its current host role.

The previous initialize/verify/account observations do not qualify the corrected candidate. Native catalogue delivery, current image-tool/output path and durable runner-bound slot/completion integration remain open. The owner-accepted crate and its original provenance are untouched; task #4 is incomplete.
