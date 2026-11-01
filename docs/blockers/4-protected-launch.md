# Protected app-server launch through AgentQ

One focused blocker draft for PM intake under AssetQ R3, R4 and R6. AssetQ #4 remains incomplete and new dispatch remains refused. This document creates no task, grant or executable authority.

Upstream resolution: AgentQ #186/PR188 is published as `6585656b5966f6f5912f504b6bf30f5bc576acaa` with green merge-result CI. Its original AssetQ source pin was `aab1a190ab76f1e932578520d7a8787ef1d4b660`; new builds adopt the [published receipt correction](4-agentq-receipt-wire.md). The API-gap analysis below is historical. The next boundary is the [composed AssetQ qualification](../evidence/4/protected-consumer.md), followed by durable slot integration and current tool/output qualification; the original task remains incomplete.

## Goal and owner

The API extraction above is published, but composed Mac qualification exposed an [AgentQ receipt wire mismatch](4-agentq-receipt-wire.md). That single focused follow-up owns the current PM intake; the original API-gap analysis below remains historical.

Publish a supported AgentQ app-server launch path that consumes the existing operator-controlled project grant, reuses the approved host boundary and CoreQ infrastructure, and returns authority readback bound to the launched native operation.

Proposed intake repository: **taskqorg/agentq**, which owns the runtime consumer API. CoreQ supplies generic process/transport primitives; TaskQ retains its operator grant store, resolver and qualified boundary adapter. AssetQ consumes the published API and retains its own jobs/effects. Do not copy TaskQ policy into AssetQ or make CoreQ depend upward on AgentQ.

Source request: [AssetQ #4, PM answer 6557901594](https://github.com/taskqorg/assetq/issues/4#issuecomment-6557901594). It directs: "reuse a supported operator-controlled host authority/boundary path and its readback" and "If the published AgentQ pin or existing host launcher lacks a required primitive, identify its exact owning component/API and prepare one focused blocker draft with verifiable acceptance for PM intake, avoiding existing writers."

## Verified interface gap

| Checked component | Existing API and evidence | Missing connection |
|---|---|---|
| AgentQ runtime 0.1.0, `513193c8a96f3de1c71a86291a679230817ce0b2` | [`agentq_runtime::agent::AppServer::start`](https://github.com/taskqorg/agentq/blob/513193c8a96f3de1c71a86291a679230817ce0b2/runtime/src/agent/app.rs#L15) takes a caller-built `Command`, appends native arguments through `app_server_command`, then directly spawns. | No supported protected host-grant resolution/launch/readback API. A caller-selected command or serialized grant is not authority. |
| CoreQ 0.18.0, `e84761cdab43723b871a42b3021e9c39afcd9484` | [`process::identity` and `terminate`](https://github.com/taskqorg/coreq/blob/e84761cdab43723b871a42b3021e9c39afcd9484/src/process.rs#L19), bounded `output`/`exchange`, locks, durable writes and local sockets. Its [public modules](https://github.com/taskqorg/coreq/blob/e84761cdab43723b871a42b3021e9c39afcd9484/src/lib.rs#L3) have no boundary/grant resolver. | Process identity grants nothing. This published pin has no reusable Seatbelt/protected-launch primitive for AgentQ to consume. |
| TaskQ 4.6.58, checked main `9a1e2a818a3ed5bb9dc0bbbac275ebc2b42ab0c7` | [`policy::load`, `resolve`, `profile`](https://github.com/taskqorg/taskq/blob/9a1e2a818a3ed5bb9dc0bbbac275ebc2b42ab0c7/src/policy.rs#L176) resolve operator grants and build the Mac boundary. [`runtime::executable`](https://github.com/taskqorg/taskq/blob/9a1e2a818a3ed5bb9dc0bbbac275ebc2b42ab0c7/src/runtime.rs#L502) applies that wrapper; it is `pub(crate)` inside the TaskQ binary. | These are not exported through the AgentQ consumer runtime. The checked public CLI has no protected app-server launch entry point. |
| TaskQ permission readback | [`permissions_cmd`](https://github.com/taskqorg/taskq/blob/9a1e2a818a3ed5bb9dc0bbbac275ebc2b42ab0c7/src/main.rs#L2046) and [`Effective::json`](https://github.com/taskqorg/taskq/blob/9a1e2a818a3ed5bb9dc0bbbac275ebc2b42ab0c7/src/policy.rs#L85) report resolved policy. | Neither launches a child nor returns a receipt bound to that child's identity and applied boundary. Parsing a policy report cannot prove a new app server is contained. |

The same policy/runtime composition and CLI were inspected at the immutable `v4.6.58` source; current main still owns them inside TaskQ. AgentQ main remains the accepted runtime merge `b2887ec28f258ac31466024d1065dc41ad59c344`; CoreQ main equals the 0.18.0 pin. These are source observations, not qualification of the installed Mac binary.

The PM supplied read-only evidence of an existing AssetQ Codex project Workspace grant on Mac, revision 29, approved by kirs and enforced by TaskQ Seatbelt. Its scope is useful for a genuinely Mac-specific qualification through PM/TaskQ. The note is not trusted programmatic readback. Codex-app remains ungranted; Windows Workspace remains unqualified. No setting, account, model, capacity or grant change is part of this draft.

## Proposed acceptance

1. Update affected runtime/host authority contracts before code; reuse the existing operator grant store/channel and approved scope. No AssetQ policy store, new grant, Full fallback or Windows enablement.
2. Publish an immutable AgentQ protected app-server API/pin, reusing CoreQ process/transport primitives and the host's trusted boundary adapter. Preserve existing consumer APIs and frozen CoreQ runtime compatibility.
3. Bind resolution/launch to canonical project/profile, runtime/account/executable, grant approver/revision and operation. Record requested, saved and verified-effective boundaries; caller grant/profile text cannot authorize.
4. Refuse missing, unreadable, revoked, stale or mismatched authority and unknown enforcement before native input/effects. Prove task/environment/receipt tampering cannot widen scope; preserve existing operations without blind replay.
5. Under the existing Mac AssetQ Workspace grant through PM/TaskQ, verify approved reads, writes, network and child boundaries, including write/delete/rename/reparse denial outside the write set and protection of policy/launcher controls.
6. Preserve app-server stdio, thread/turn observation, account/model/permissions and optional optimization. A non-generating native handshake and bounded probes read back exact source/binary, child identity and authority receipt.
7. Deterministic refusal, stale/tampered identity, interruption and uncertainty checks prove no turn starts before verified authority. Keep job truth independent of usage/telemetry failures and retain explicit unknown reasons.
8. Publish immutable pins with green merge-result CI, named Mac boundary/native receipts and an AssetQ recipe. Portable/fake checks do not qualify Mac. Preserve the crate/effect; no generation or replay for this blocker.

## Coordination and PM intake

[TaskQ #2111](https://github.com/taskqorg/taskq/issues/2111) is already doing runtime adoption. Its `src/runtime.rs`/policy integration surface has an existing writer. Coordinate through that owner and an immutable handoff; do not start a competing TaskQ writer or rewrite its started acceptance. [CoreQ #112](https://github.com/taskqorg/coreq/issues/112) owns later legacy retirement and remains blocked on adoption; this blocker must preserve its compatibility prerequisites. The checked AgentQ open work has no existing protected-launch task to reuse.

Recommended technical route: additive AgentQ protected-launch support backed by existing CoreQ process/transport primitives and the TaskQ authority/boundary adapter. No new CoreQ policy or mandatory Seatbelt relocation is proposed. PM should triage this single draft against the live runtime writer, create or steer the owning upstream work, then add the concrete cross-repository dependency to AssetQ #4. Do not ask the owner to approve an unsupported mode. This draft is not a second AssetQ implementation task.

After the required pin is published, #4 continues at protected-launch integration and current-source native qualification. Its earlier real bitmap receipt, new reusable launch receipt, current-source qualification, owner visual judgment, managed deployment and website evidence remain distinct. The original crate asset ID is `2c887345-9f66-491f-8a0c-81d81cf0c4af`; its SHA-256 remains `fe2077d15b0c50617b84c6267277d46339c6a8cedbd6334e923cfa5475529baa`.
