# Protected AssetQ consumer qualification

Task [#4](https://github.com/taskqorg/assetq/issues/4), A1-A6; applied AssetQ R1-R8. This is the runnable handoff requested in steering note 6560541534. The scoped native receipts below do not complete image acceptance. No model turn, generation or replay of the accepted crate is authorized by this recipe.

## Immutable inputs and preservation

- AgentQ protected runtime: `8427bba6145aac0239cd16a8dae0caf8efa00695`, package `agentq-runtime` 0.1.0, published by PR195. [Merge-group Test run 41336656270](https://github.com/taskqorg/agentq/actions/runs/41336656270) passed on this exact merge result; AgentQ #193 is closed. The producer-shaped receipt regression and prior authority/identity/no-replay guards passed upstream. Exact source-tree review also identifies intervening Antigravity path/profile fixes; dependencies remain unchanged. This pin is not a Mac or image-tool qualification receipt.
- TaskQ adapter: published source `e990d88943e95abd8d182d8157bfe84a94343a07`, exact merge-result Test run `41338883682` green. The refreshed build pins PM's native-qualified signed Mac artifact SHA-256 `f8b04708db09c764ec14bbaa77d0dd40fd7bfc9a521104f93f20885ebb7d5df4` from [note 6562927357](https://github.com/taskqorg/assetq/issues/4#issuecomment-6562927357). A rebuilt or re-signed artifact is a different candidate until qualified; no environment, input or local receipt can replace this anchor.
- The current source removes the direct `AppServer::start` path. Both the prepared image runner and qualification use `image_worker::protected::launch`, then the same protected `send`/`exchange` functions. AgentQ verifies the fresh host receipt and the host revalidates input delivery.
- Original asset `2c887345-9f66-491f-8a0c-81d81cf0c4af`, SHA-256 `fe2077d15b0c50617b84c6267277d46339c6a8cedbd6334e923cfa5475529baa`, remains unchanged. Immutable specification fixtures guard original versions 2 and 3; version 4 declares the corrected protected pipeline. Old runtime receipts remain observable, never permission to relaunch them. Owner acceptance and the historical real native receipt remain [separate evidence](README.md).

The [adapter qualification record](taskq-adapter-candidate.md) retains the old digest/source and its mixed-runtime policy refusal, plus PM's successful replacement qualification. This build promotes that exact replacement digest; it does not accept the old artifact as a fallback. Old qualification recovery requires its preserved original AssetQ/TaskQ binaries. The later composed receipt below qualifies the unchanged transport, separately from image-tool and durable generation acceptance. Do not repeat the healthy host probes in this historical recipe.

## Existing PM channel and bounded command

Build the exact AssetQ candidate in an isolated checkout on the existing approved Mac host. Preserve ordinary installations, active sessions, grants, model/account/effort, capacity and optional optimization. No new host access or Windows Workspace enablement is part of this handoff.

Run the candidate from the registered canonical AssetQ project root through PM's existing private driver, keeping stdin/stdout open:

```sh
/absolute/path/to/exact-candidate/assetq image-qualify ./qualification-4-unique
```

The executable path and unique directory name above are operator placeholders. The directory must be new, inside the current project. It is created with Unix mode 0700. Raw control/authority/native receipts stay there and must never be pasted into a public task or PR.

Use the exact refreshed AssetQ source identified by the task/PR receipt and PM's retained final signed adapter bytes above. Its first JSON line must report that full `adapter_sha256`; a different value means the wrong AssetQ build. Keep the adapter's staged executable path private. Rebuild only AssetQ for this composed check, not the already qualified adapter.

The first output is a JSON line with `state: awaiting-existing-slot`, the real consumer `pid`/`started`, canonical `root`, runtime pin and pinned adapter digest. Through the existing host channel, reserve and observe one `taskq.slots/1` Codex slot bound to that exact live consumer and one new qualification operation. Read the current existing Workspace revision, account, approver, executable/hash, account home and generated profile hash independently. Historical values or a task/environment grant are insufficient.

Derive the expected profile from the selected TaskQ policy/profile append source and the actual staged paths/PATH. Its added CLI-package and protected-settings denials change that profile even with the same Codex grant. Neither the old profile hash nor one copied from a prior operation receipt is an acceptable match source.

Serialize this shape as one JSON line on private stdin, substituting independently verified values. `<original-slot-request>` means the exact object with `binding` and private `key`, not a string:

```json
{
  "adapter": "<canonical path to the pinned staged TaskQ artifact>",
  "request": {
    "schema": "taskq.app-server/1",
    "slot": "<original-slot-request>",
    "policy_revision": 0,
    "account": "<current existing account>"
  },
  "profile_sha256": "<independently expected profile hash>",
  "executable": "<canonical native executable>",
  "executable_sha256": "<independently expected native hash>",
  "account_home": "<canonical existing account home>",
  "approver": "<existing grant approver>"
}
```

This is a shape example; revision 0 and placeholder strings deliberately refuse. Unknown top-level fields refuse, including adapter digest, grant or permission overrides. The compiled adapter digest is the trust anchor; the remaining values are match requirements. The host independently authenticates the original slot and resolves the saved grant. There is no AssetQ grant or policy store.

Before launch the consumer retains the request and a create-once marker. Launch errors are uncertain; use `image-observe`, never repeat `image-qualify` for that operation. After fresh authority verification it performs only `initialize`/`initialized`. The next output reports `initialized` with native handshake data and explicit unknown image-tool capability.

## Native probe frames and evidence

Each following stdin frame is one compact JSON line. The interaction window is 120 seconds from its first control wait, input frames are limited to 32 KiB, and its native journal to 32 MiB. Initial authority readiness has a separate maximum 10-second observation budget; final cancellation/exit/recovery can add up to 15 seconds. Expired interaction refuses queued input too. Native commands require `timeoutMs` in 1..10000 and `outputBytesCap` in 1..65536. Use the pinned client's supported `command/exec` schema from PM's existing probe recipe; do not guess missing native fields.

- `{"action":"verify"}` obtains fresh composed authority evidence.
- `{"action":"request","method":"account/read","params":{"refreshToken":false}}` reads existing authentication without refreshing it.
- `{"action":"request","method":"config/read","params":{}}` and `model/list` can inspect current native configuration/catalogue. These observations alone never qualify an image tool.
- `{"action":"request","method":"command/exec","params":{...}}` runs only PM's bounded non-generating boundary probes. Include a unique `processId`; dispatch returns `probe-submitted` and the RPC ID, then `receive` collects native events and completion. Use an owned workspace fixture and loopback HTTP endpoint. Check parent and child reads/writes/rename/delete and network. Verify outside write/create/delete/rename and symlink-alias denial against disposable canaries; production policy/launcher/account controls receive nonmutating kernel queries only.
- Use a caller-selected owned `processId` supported by the native schema for a bounded standalone probe, then `command/exec/terminate` with that same identity to prove interruption. `{"action":"receive"}` reads one queued native event; absence means no event observed.
- Check fresh denial/unknown readbacks and recovery through the same operation, retaining no-replay evidence. Never tamper with the actual grant, policy or production controls. Protocol methods for model turns, thread creation and configuration writes are refused by qualification.
- `{"action":"cancel"}` requests exact host cancellation, observes supervisor exit and records same-operation host recovery. This neither proves every descendant ended nor releases the slot.

Native responses and events are journaled before interpretation. Any protocol error, EOF, deadline or failed readback remains unknown with a reason. The consumer attempts exact host cancellation after a connected-session error and retains all receipts. It never automatically approves native requests or releases capacity.

For recovery after interruption, use the same candidate and retained directory:

```sh
/absolute/path/to/exact-candidate/assetq image-observe ./qualification-4-unique
/absolute/path/to/exact-candidate/assetq image-observe ./qualification-4-unique --cancel
```

The second command is used only when the owned operation still requires cancellation. Both use AgentQ's protected host API with the original request; neither launches or substitutes another operation. PM must reconcile exact native/supervisor incarnations and every probe child, then attest whole-operation completion through the original host slot release and read back acknowledgment. Missing observations preserve occupied capacity.

Check the actually resolved native image tool/catalogue and original artifact path without generation. Record whether the current client exposes the required tool and how that was observed. An image-generation item in JSON schema, an installed skill or the historical crate alone is insufficient proof of current tool availability; missing evidence remains a precise capability gap.

## Acceptance and remaining work

PM executed exact source `0c257a82f4ddc14eabadbe234026e937ee701ab0` on Mac: build PASS, composed qualification FAIL before handshake ([note 6561661669](https://github.com/taskqorg/assetq/issues/4#issuecomment-6561661669)). The independently corrected profile matched actual authority, but the old runtime still refused. Both operations were reconciled/released by PM without model calls or crate replay. The [AgentQ wire-mismatch evidence](../../blockers/4-agentq-receipt-wire.md) records the defect, isolated failing-before/passing-after proof and now-accepted upstream publication. The corrected published pin is adopted; current consumer/process bindings and the later observe timeout still require native qualification. Do not reuse either reconciled operation or infer expected identities from its receipt.

PM subsequently built corrected source `1111fe6b66fbdcfb2eaec1a96db65f97dd4be2e2` on Mac: build PASS, old adapter policy-read FAIL before durable intent/native creation ([note 6562586802](https://github.com/taskqorg/assetq/issues/4#issuecomment-6562586802)). The original slot was released after verifying absence. This is a stale adapter's `unknown grant antigravity workspace` refusal, not a grant change request or another runtime wire failure. Its [replacement candidate](taskq-adapter-candidate.md) preserves the existing policy and separates native adapter qualification from later composed qualification.

PM reports that exact replacement adapter's native qualification PASS in [note 6562927357](https://github.com/taskqorg/assetq/issues/4#issuecomment-6562927357): current-policy read, trusted independent authority/native executable identity, handshake, kernel control/outside denials, parent/child workspace/loopback operations, forged-policy refusal, consumed-operation no replay, interruption and exact termination/slot release. Model/paid turns were zero. These are TaskQ adapter receipts; composed qualification followed separately.

PM then qualified exact AssetQ `05476e88c53f5232cc17f37bd664aad0ba103a79` with the pinned signed adapter and corrected runtime ([note 6563501391](https://github.com/taskqorg/assetq/issues/4#issuecomment-6563501391)): initialize/initialized, independently matched effective authority, account/config read, parent/child workspace operations and loopback HTTP, outside write/create/unlink/rename/symlink denial with unchanged canaries, cancellation exit 0, actual native disappearance and original slot release all PASS. Expected profile derivation included the exact selected policy's agy package protection and Gemini settings deny regex. Equality was preserved; no expected value came from a receipt. Earlier refusals and released operations are retained. Model turns and paid effects were zero; the accepted crate was untouched. This proves composed transport/boundaries, not current image-tool/output or unattended durable generation. Do not repeat these healthy proofs.

Portable checks cover non-generating method/command bounds, refusal before native launch on unsupported platforms, unchanged legacy method specification, duplicate/uncertain effects, original PNG validation/retrieval and absent optimization. These are adapter/consumer checks, not Mac enforcement evidence. Record exact commands and results below before handoff.

Development evidence: `cargo test --locked --lib image_worker` passes all six affected checks. The existing receipt regression failed before the upgrade compatibility fix with `worker receipt mismatch; no replay` for the original runtime; it passes after accepting exactly the original/current pins. Unknown pins still refuse, and duplicate/uncertain effects remain unreplayable. The qualification regression failed against the prior saturated timeout because an expired wait returned a queued `receive` frame; it passes after explicit expiration checks. No guarding assertion was relaxed.

Read-only Astra review found both regressions and a post-launch cleanup gap. Before, failed journal/authority persistence returned before cancellation; after, all post-launch verification/persistence errors enter exact-host cancellation before returning. Correction review found no remaining blocker. Native evidence-write fault injection is unexecuted because this Windows worker lacks the approved Mac path; code inspection is not a native PASS.

The canonical gate on source `65a1a5c24b2958ec1128eead19290584c3a21257` failed one pre-existing R7 fixture after 327.61 seconds: whole-database size did not increase after a small TOAST insert. All nine core checks and the other 24 service checks passed. Diagnostic readback measured 122,880 bytes of net allocation versus a 160,944-byte volatile catalogue-cache file. Cache removal can offset that growth, but the exact cache event in the failed run was not captured. The fixture now uses 192 KiB of input and reports before/after sizes; all original serialization, physical-growth, TOAST/index, quota-refusal and row-count assertions remain. The same focused check passes after calibration; production limits/admission code are unchanged.

Frozen implementation `89f518758337749497e88ce5d98fade239cdda68`: `python3 scripts/check.py` passed in 314.44 seconds on Windows with Rust 1.99.0 GNU and isolated PostgreSQL 18.1. Three checker checks, pinned contract lint, formatting/whitespace, nine Rust core checks, 25 isolated service checks and both trace checks passed. All 19 local documentation links resolve. Later evidence-only edits retain identical source/tests/dependencies; these receipts do not prove Ubuntu timing, green merge-result CI or native Mac qualification.

Corrected-pin adoption: the existing receipt regression, extended to the prior protected pin, failed after the pin upgrade with `worker receipt mismatch; no replay` (exit 101). Accepting that exact prior pin for lookup makes `cargo test --locked --lib image_worker` pass all six affected checks. Unknown pins still refuse and every retained effect still rejects a second reservation. Native execution still requires the current pin and both new-dispatch refusal guards are unchanged. Method 4 is additive; the original version 2 golden and a captured version 3 golden retain both specifications exactly. The registry, scoped-list and CLI fixtures now expect the additional built-in version without dropping their access checks. This extends recovery compatibility under R4/R5; no authority comparison or prior protection changes. The frozen candidate's canonical outcome belongs to the task/PR check receipt, separately from this focused observation.

R8 gate correction: an initial canonical attempt rejected an overlong contract paragraph; splitting it preserved all requirements. Its next attempt reported docs-only because selection inspected only the last commit, hiding the runtime adoption. That outcome is not implementation qualification. A temporary Git repository reproduces implementation followed by documentation: the original selector fails with `False is not true`; merge-base selection passes. The regression also retains docs-only selection, published-main comparison, staged/untracked impact and unknown-base refusal to skip Rust. `python3 scripts/test_check.py` passes all four checker checks after correction. The canonical gate now selects the whole branch candidate under [service verification](../../testing.md).

The prepared generation runner had a distinct R2/R4/R6/R7 defect: `ProtectedAppServer::poll` observes the supervisor, but its result was assigned to `exit_confirmed`. The new focused regression failed before with `unwrap_err()` on a completed result carrying `exit_confirmed: true` (exit 101). After correction, all seven image-worker checks pass: completed/cancelled turn observations, known/unknown usage, supervisor exit/timeout and cleanup/turn errors remain retained in `native-observation.json`, with whole-operation completion unconfirmed. No result is published or reservation truncated; effect lookup and duplicate refusal remain intact. The unchanged historical bitmap/import/method checks still pass. Both trace checks pass, including `protected-supervisor-exit.json`, which retains the waiting operation after repeated unknown observations and rejects replacement claim/dispatch. No guarding assertion or accepted behavior was weakened. Actual Mac execution of this generation cleanup path is untested; composed qualification remains scoped to its unchanged transport.

The [focused lifecycle blocker](../../blockers/4-protected-lifecycle.md) identifies missing non-launching authority preparation and original-operation native/descendant completion through the supported host/runtime interface. AssetQ retains its own slot/control orchestration work; it must not copy the private resolver, accept receipt-selected trust, infer descendant completion or bypass a slot refusal. Current image-tool discovery remains unknown; no extra generation is authorized.

A1 is partial: the published runtime is pinned, optional optimization remains inherited and composed host transport/boundaries passed. A2/A6 retain historical real API/recovery and deterministic evidence; protected durable slot/control/completion integration is incomplete. A3 requires current selected tool/output qualification. A4 retains precise refusal and a supported composed transport, with its remaining upstream lifecycle primitives identified. A5 preserves the exact owner-accepted crate, asset ID and original operation. #4 remains incomplete.

Open: current image-tool/output qualification, supported authority preparation and whole-operation completion, then AssetQ durable slot/control integration. Managed-host generation, website acceptance, new backup work and backup/restore qualification are not claimed. New generation dispatch remains refused; PR15 must not be closed as completed #4 on this evidence.
