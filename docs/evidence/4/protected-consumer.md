# Protected AssetQ consumer qualification

Task [#4](https://github.com/taskqorg/assetq/issues/4), A1-A6; applied AssetQ R1-R8. This is the runnable handoff requested in steering note 6560541534, not native acceptance. No model turn, generation or replay of the accepted crate is authorized by this recipe.

## Immutable inputs and preservation

- AgentQ protected runtime: `aab1a190ab76f1e932578520d7a8787ef1d4b660`, package `agentq-runtime` 0.1.0. PR188 published it as `6585656b5966f6f5912f504b6bf30f5bc576acaa`; the issue's publication receipt reports green merge-result CI.
- TaskQ adapter: reviewed source `56b1a1eba274bf88520e4d668461372fe3048b00`, merge `e6555153df9515974d7b5ec3c93af7cb5a9c481f`. The build pins PM's staged signed Mac artifact SHA-256 `de4c406b9a8378623ee113d634f9f6808a5cc2ba8ab5ae23db77c0b5bd25fa0a`. A rebuilt artifact is a different candidate until qualified; no environment, input or local receipt can replace this anchor.
- The current source removes the direct `AppServer::start` path. Both the prepared image runner and qualification use `image_worker::protected::launch`, then the same protected `send`/`exchange` functions. AgentQ verifies the fresh host receipt and the host revalidates input delivery.
- Original asset `2c887345-9f66-491f-8a0c-81d81cf0c4af`, SHA-256 `fe2077d15b0c50617b84c6267277d46339c6a8cedbd6334e923cfa5475529baa`, remains unchanged. Original method version 2 and its runtime pin are guarded by an immutable specification fixture; version 3 declares the protected pipeline. Owner acceptance and the historical real native receipt remain [separate evidence](README.md).

## Existing PM channel and bounded command

Build the exact AssetQ candidate in an isolated checkout on the existing approved Mac host. Preserve ordinary installations, active sessions, grants, model/account/effort, capacity and optional optimization. No new host access or Windows Workspace enablement is part of this handoff.

Run the candidate from the registered canonical AssetQ project root through PM's existing private driver, keeping stdin/stdout open:

```sh
/absolute/path/to/exact-candidate/assetq image-qualify ./qualification-4-unique
```

The executable path and unique directory name above are operator placeholders. The directory must be new, inside the current project. It is created with Unix mode 0700. Raw control/authority/native receipts stay there and must never be pasted into a public task or PR.

The first output is a JSON line with `state: awaiting-existing-slot`, the real consumer `pid`/`started`, canonical `root`, runtime pin and pinned adapter digest. Through the existing host channel, reserve and observe one `taskq.slots/1` Codex slot bound to that exact live consumer and one new qualification operation. Read the current existing Workspace revision, account, approver, executable/hash, account home and generated profile hash independently. Historical values or a task/environment grant are insufficient.

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

Portable checks cover non-generating method/command bounds, refusal before native launch on unsupported platforms, unchanged legacy method specification, duplicate/uncertain effects, original PNG validation/retrieval and absent optimization. These are adapter/consumer checks, not Mac enforcement evidence. Record exact commands and results below before handoff.

Development evidence: `cargo test --locked --lib image_worker` passes all six affected checks. The existing receipt regression failed before the upgrade compatibility fix with `worker receipt mismatch; no replay` for the original runtime; it passes after accepting exactly the original/current pins. Unknown pins still refuse, and duplicate/uncertain effects remain unreplayable. The qualification regression failed against the prior saturated timeout because an expired wait returned a queued `receive` frame; it passes after explicit expiration checks. No guarding assertion was relaxed.

Read-only Astra review found both regressions and a post-launch cleanup gap. Before, failed journal/authority persistence returned before cancellation; after, all post-launch verification/persistence errors enter exact-host cancellation before returning. Correction review found no remaining blocker. Native evidence-write fault injection is unexecuted because this Windows worker lacks the approved Mac path; code inspection is not a native PASS.

A1 is partial: the published runtime is pinned and optional optimization remains inherited; actual host composition is unqualified. A2/A6 retain historical real API/recovery and deterministic evidence, with protected durable slot acquisition/release still to integrate after PM's native recipe is proved. A3 requires current selected tool/output qualification. A4 retains explicit refusal and now has a supported composed worker path to qualify. A5 preserves the exact owner-accepted crate, asset ID and original operation. No acceptance item is declared complete from this draft.

Open: the composed Mac authority/handshake/filesystem/network/child/control/outside/interruption/no-replay/reconciliation observations, fresh image-tool capability, and subsequent durable slot-backed generation integration. Managed-host generation, website acceptance, new backup work and backup/restore qualification are not claimed. New generation dispatch remains refused; PR15 must not be closed as completed #4 on this evidence.
