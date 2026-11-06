# Independent protected host publication

[Pipe #52](https://github.com/taskqorg/assetq/issues/52) owns this prerequisite for [#47](https://github.com/taskqorg/assetq/issues/47). Its original goal is: “Publish the immutable AgentQ/CoreQ API and signed host adapter required by Pipe #47 without TaskQ runtime authority. Pipe permits, consumer integration and live image qualification stay in #47.”

The fresh #52 inbox resumed this task after [AgentQ #241](https://github.com/taskqorg/agentq/issues/241) closed. [PR #244](https://github.com/taskqorg/agentq/pull/244) published the independent API and signed adapters. This record reconciles that trusted publication; it changes no Pipe runtime, host trust or authority. Pipe R3–R6/R10 and TaskQ §7/R12/R13 apply. The [#42 legacy recovery decision](https://github.com/taskqorg/assetq/issues/42#issuecomment-6661217844) remains temporary and unchanged.

## Original publication identities

| Item | Pin |
|---|---|
| Published AgentQ API/merged source | `f87f308a90b34564cb8ab6f3686ca09ffdde638f` |
| Original API and Windows adapter source | `12507a213df2e6244e6163589bc92b5516ddcd8d` |
| macOS arm64 adapter source, PR build merge | `385dd54967d9c9456b80a11fefc2a5b45ad57e7c` |
| CoreQ linked by both signed adapters | `v0.19.2`, `b78ca64cac615500b57b4a04983e79333035dd65` |
| CoreQ used by the published merged library | `v0.21.0`, `18b177c445e92c5df58bd1dea1b473776dd0c6ae` |
| Final macOS arm64 adapter SHA-256, 7,146,016 bytes | `425c5924046338bf13167e5bf76eb827533435ed09e5f3cf336b6744d9bf2118` |
| Final Windows x86_64 adapter SHA-256, 11,123,352 bytes | `9ee905960a57dbe4649c42626a2a5c6b1ed871b440ecfbfa5aebe8cff71fb7ba` |
| Ed25519 publication public key | `7f739abab8625718e2e5dbedf0deb5a98373260d3c22ecd20a1148f42bf6d584` |
| Published signed package SHA-256 | `57f375ebbf4a3fee2c9642e377ed96527069740d30c0ff8020e5a0c14fdf4226` |

The immutable [upstream evidence](https://github.com/taskqorg/agentq/blob/f87f308a90b34564cb8ab6f3686ca09ffdde638f/docs/evidence/runtime-241.md) owns build provenance and the exact API/provisioning format. Public manifests are [macOS](https://github.com/taskqorg/agentq/blob/f87f308a90b34564cb8ab6f3686ca09ffdde638f/docs/evidence/runtime-241/adapter-macos-aarch64.json) and [Windows](https://github.com/taskqorg/agentq/blob/f87f308a90b34564cb8ab6f3686ca09ffdde638f/docs/evidence/runtime-241/adapter-windows-x86_64.json). The [signed package](https://github.com/taskqorg/agentq/raw/582f475b37809fd6df5b78e3e04ad7c315b291c4/241/57f375ebbf4a-agentq-241-signed-adapter.tar.gz) contains binaries, public manifests and evidence, with no private policy, key, permit or receipt.

The eight protected API/host/signing blobs match across all three AgentQ pins: `runtime/src/agent/protected.rs`, `runtime/src/agent/host.rs`, every blob under `runtime/src/agent/host/`, `runtime/examples/sign_adapter.rs` and `agentq/host.rs`. This was checked through immutable Git tree blob IDs. The whole trees and `agentq/main.rs` are not byte-identical. The adapters were not rebuilt after #243; a consumer requiring #243 inside the adapter needs another reviewed build and signature. A library upgrade does not change a signed artifact's source or CoreQ identity.

## Acceptance reconciliation

| Item | Published evidence and limit |
|---|---|
| A1 | The owning [AgentQ contract §12 M12.8](https://github.com/taskqorg/agentq/blob/f87f308a90b34564cb8ab6f3686ca09ffdde638f/agentq.md#12-agent-runtime) and its linked runtime detail define the independent signed route. The pins and final digests above are published. The route uses `agentq agent host` with no TaskQ binary, command, slot service or CA. Legacy support remains a separate upstream route, not an independent-route fallback. |
| A2 | Upstream preparation, every-input grant/profile and process-incarnation guards, one-shot launch, cancellation, native/descendant completion and durable journal checks are published. Missing or changed proof refuses effects and leaves occupancy unknown; transport exit or a stop acknowledgment cannot release it. Deterministic checks are evidence for the model/process boundaries. [#61](#native-qualification-through-61) adds native transport and recovery readback for the replacement adapter; native workload enforcement remains unqualified. |
| A3 | The host accepts authenticated `pipe.permits/1` caller permits. Pipe owns admission and final release acknowledgment; host cleanup cannot release its capacity. Authenticated retained archives preserve exact original checkpoints/receipts, operation identity and consumed stop intent without relaunch, receipt rewriting or TaskQ runtime fallback. Pipe's issuer, permit persistence and integration are still #47 work. |
| A4 | Deterministic adversarial recovery checks and green merge-result CI are published. The original Mac launch failed; the replacement publication and its failure-before/pass-after checks are recorded below. [#61](#native-qualification-through-61) qualifies the signed non-generating transport under existing Mac operator authority for replacement adapter `a1cbb7b8` only. Neither #241 nor #266 closure proves it; the #61 receipts do. No new grant, model/account/capacity change or image generation is authorized by this record. |

AgentQ #241 A4 did not include #52's sentence requiring existing Mac operator qualification. Its closure therefore does not establish that additional native requirement. That gap stayed open until #61 supplied native evidence for the replacement adapter. This record does not amend A4 or treat portable signing checks as native transport proof. Pipe consumer integration and live image qualification remain in #47.

## Checks and failure evidence

Fresh GitHub readback confirmed PR #244 merged as `f87f308a90b34564cb8ab6f3686ca09ffdde638f` and #241 is closed. [Test run 42103076240](https://github.com/taskqorg/agentq/actions/runs/42103076240) has `event: merge_group`, that exact `headSha`, and `success` for contract, Linux Rust, macOS Rust and Windows Rust. This is merge-result evidence, not a PR/head substitute (TaskQ §7/R12).

This attempt downloaded the immutable package without executing either binary. Python standard-library `tarfile`/`hashlib` checks matched both packaged sizes and SHA-256 values, and matched each packaged manifest byte-for-byte against its published manifest. Node's built-in `crypto` verified both Ed25519 signatures with the independently recorded public key over `agentq.protected-adapter/1`, NUL, and the lowercase SHA-256 text. Both zeroed signatures and changed digests were refused. These checks prove packaged bytes and signatures, not installed host trust, OS codesigning, TCC or native enforcement.

The upstream evidence retains deterministic before/after protection:

- `cargo test --locked -p agentq-runtime --test agent -- protected_preparation`: a changed well-formed preparation digest was accepted before the fix; digest recomputation now refuses it with the original assertion retained.
- `protected_retained_fences_bind_operation_and_incarnation_not_bytes`: formatting and signed-snapshot changes previously repeated stops, `(3, 3)` instead of `(1, 1)`; operation/incarnation fencing passes with one effect of each kind. Lost acknowledgment never permits replay.
- `cargo test --locked -p agentq-runtime --lib protected_retained_fences`: cross-action incarnation drift previously produced two second-action stops instead of zero. Both action orders now refuse with zero stops and byte-identical journal state, retaining the earlier no-replay guards.
- Windows journal sync failed with `protected journal sync unknown` in run `42086892150`. A writable sync handle fixed it without changing the assertion; the exact merge-result Windows job above also passed.

These are attributed upstream observations; this docs-only attempt did not rerun the Rust suite or native effects. The immutable upstream evidence also records signature/issuer/key substitution, authority drift, corrupt/lost/full journal, consumed launch intent, surviving descendants, completion counter regression and partial-cleanup checks. Portable fixtures do not qualify a working profile.

## Consumer handoff and preservation

#47 must integrate the independent API, authenticate Pipe-owned permits, pin the reviewed adapter/public key, reconcile original operation identities, and qualify its composed worker before enabling new dispatch. The Mac route must use only existing approved operator authority; missing enforcement or proof refuses effects. Windows native containment is unsupported. Ed25519 verification supplies neither macOS codesigning nor TCC grants. Private signing files do not isolate an uncontained process under the same OS user.

No adapter was installed or selected in Pipe by this change. New qualification and generation remain refused under R10. Preserve original adapters, binaries, grants, checkpoints, permits, journals, receipts and the accepted crate. Retained work must never be relaunched or have receipts rewritten to manufacture migration evidence. Removing this documentation would undo only the handoff record; it would neither roll back nor authorize any host effect.

## Native Mac operator handoff

The A4 review returned this task because native transport proof is missing. This continuation runs on Windows. The published host's non-macOS dispatch explicitly refuses with `protected native launch requires qualified macOS containment`; a Windows run cannot fill that gap. The upstream [operator provisioning procedure](https://github.com/taskqorg/agentq/blob/f87f308a90b34564cb8ab6f3686ca09ffdde638f/docs/evidence/runtime-241.md#operator-provisioning) records that no working host trust, policy, initial journal, permit or grant was provisioned. This is a separate operator action, not another request for the already accepted owning-repository scope.

Use the existing private Mac operator channel for this bounded action under R3–R6/R10:

1. Stage the published macOS binary separately from original adapters. Read back its 7,146,016-byte size, SHA-256 `425c5924046338bf13167e5bf76eb827533435ed09e5f3cf336b6744d9bf2118` and detached signature against the public key above. Do not rebuild, re-sign or replace the accepted bytes. Detached signing is not macOS codesigning or a TCC grant; a change to the bytes needs another reviewed final digest.
2. Independently resolve the existing approved project's Workspace grant, revision/approver, account home, native executable/hash, arguments and original profile. Through the trusted signing channel, authenticate only that existing authority in the fixed host store described upstream. Preserve any existing store and journal; `initial.json` is only for a never-used store. A missing signing capability or approved authority stops this action; do not transfer private keys or create another grant.
3. Authenticate one caller-owned `pipe.permits/1` qualification permit bound to the actual live consumer incarnation. This is a host-only qualification, not Pipe's #47 admission/integration. Keep the original permit and its private key locally. Do not call TaskQ slots, its adapter or its certificate authority.
4. Use the pinned API's `ProtectedHost::signed`, `prepare`, `ProtectedPreparation::checkpoint` and `launch_prepared` with inherited optimization. Persist preparation and a durable one-shot launch intent before launch. Compare expected identities from the independent pre-launch resolution. Verify the live transport and native boundary; initialize without a model turn or image generation. Keep the transport until original-operation cancellation readback completes. Errors retain the original operation and occupancy; never launch again to obtain a cleaner receipt.
5. Recover that same checkpoint with `recover_preparation`, then `observe`/`reconcile`. Record exact native, supervisor and frontend incarnations, input sealing, native/descendant completion certificate, exact-generation cleanup acknowledgment and final readback. Cancellation acknowledgment and process exit alone do not prove completion. Retain capacity whenever any proof is missing. Existing retained operations use only authenticated exact-byte archives and `reconcile_retained`; their original receipts, binaries and accepted crate stay unchanged.
6. Retain raw authority, permit, checkpoint, journal and native receipts in protected storage. Publish only the candidate/source identity, digest, check outcomes, safe receipt references and remaining unknowns. Report any TCC denial in Open, not as an owner ask. #58 still needs green `merge_group` CI; PR-head checks do not substitute for it.

At `0eafb71` this handoff was prepared, not executed; no Mac operator endpoint or authenticated working-host authority was supplied to that Windows continuation. Its documentation changed no test assertion, runtime, host trust, policy, permit, grant, account, model, capacity or retained operation. Before any first launch, abandoning the staged candidate leaves the old adapters intact; after launch intent, recovery must retain the new operation and journal rather than delete or reset them.

## Native failure and blocking prerequisite

The accepted [operator answer 6684045657](https://github.com/taskqorg/assetq/issues/52#issuecomment-6684045657) supersedes the pending operator action above. The operator reports staging the exact macOS artifact above, verifying its signature and provisioning the independent host store from the existing Codex Workspace grant, revision 34, approver `kirs`. The publication key sealed the policy on Windows. This is attributed operator readback, not a native observation by this Windows worker.

One non-generating launch reached host state `verified-effective`, then the frontend exited with `native stdout interrupted` before initialize. The operator reports no input reached Codex and no model turn ran. No completion certificate or cleanup acknowledgment exists; recovery reports `unknown` and occupancy remains retained. The original operation was not relaunched. This fails A4 native transport qualification; `verified-effective` alone does not establish it (R4/R6/R10).

[AgentQ #266](https://github.com/taskqorg/agentq/issues/266) took ownership of diagnosis and correction. At that checkpoint, accepted sockets inheriting non-blocking mode on macOS was a hypothesis; #266 required the actual errno, a macOS failure-before/pass-after check, unchanged fencing/no-replay protection and a reviewed adapter with a new final signed digest. Its signing key stays on its current host. The failed operation, original adapter, permit, journal and receipts must not be relaunched, rewritten, reset or deleted. No native pass-after or recovery-completion evidence was available at that checkpoint.

At `18fee94`, #52 parked on #266 per the accepted answer without rerunning qualification. Its closure requires reconciliation of immutable source, signed digest and merge-result CI before any separately authorized new-operation qualification; a new operation cannot release or replace the retained failed operation. A1–A3 publication evidence above remains historical for its exact pins, and A4 stays open. Pipe integration and live image qualification remain #47.

## Replacement publication after AgentQ #266

[AgentQ #266](https://github.com/taskqorg/agentq/issues/266) is now closed following [PR #267](https://github.com/taskqorg/agentq/pull/267). Its immutable [runtime-266 evidence](https://github.com/taskqorg/agentq/blob/a610cf7680b1ecc0d5b51847a0ea03eced077ded/docs/evidence/runtime-266.md) and [public manifest](https://github.com/taskqorg/agentq/blob/a610cf7680b1ecc0d5b51847a0ea03eced077ded/docs/evidence/runtime-266/adapter-macos-aarch64.json) supersede the old artifact for a future new-operation qualification. They do not change the retained failed operation or qualify native launch (R3–R6/R10).

| Item | Replacement pin |
|---|---|
| Published AgentQ API/merged source | `a610cf7680b1ecc0d5b51847a0ea03eced077ded` |
| Signed macOS arm64 build source | `1c46de5b12d7bcd3a1ce1ffbcbef962090778031` |
| Final adapter SHA-256, 7,675,376 bytes | `a1cbb7b88959532f2be28ea8d696c4cc1ae17b7c97205d6549f7156164c82a58` |
| Build run / macOS artifact | [42231396039](https://github.com/taskqorg/agentq/actions/runs/42231396039) / `13509232169` |
| Public manifest Git blob | `c35460a4c25bee32ed77165469771fedde1f76a0` |
| Existing publication public key | `7f739abab8625718e2e5dbedf0deb5a98373260d3c22ecd20a1148f42bf6d584` |

Fresh direct Git tree comparison found all 100 non-document blobs identical between the signed build source and published merge. GitHub's three-dot commit comparison uses their merge base and is not that tree-equality check. The replacement includes both stdout fixes and upstream #265/#270; it is not the superseded `711a6066` candidate. The prior Windows digest remains historical and supplies no native Mac proof.

Fresh readback of [run 42248842330](https://github.com/taskqorg/agentq/actions/runs/42248842330) returned `event: merge_group`, `headSha: a610cf7680b1ecc0d5b51847a0ea03eced077ded`, `conclusion: success`. Contract and Rust jobs on Linux, macOS and Windows all succeeded, including locked tests and offline selftest. This establishes upstream merge-result CI under TaskQ §7/R12; #58 still requires its own merge-result CI before publication.

The upstream macOS observations provide failure-before/pass-after evidence, not a successful rerun of the retained operation:

- `protected_scope_stream_waits_for_idle_native_output` failed before the socket fix with `Err(Some(35))` instead of `Ok(6)` on macOS 27.2 arm64. Darwin's accepted socket inherited `O_NONBLOCK`; clearing it made the same check pass. Errno 35 is from that reproduction; the old live run's errno remains unknown.
- `protected_scope_stdout_failure_exits_while_native_stderr_is_idle` timed out before the stderr-lock fix instead of returning `native stdout interrupted; retain scope`. Per-write stderr locking and a non-panicking error log made the same check pass. No-replay, fencing and retained-scope guards stayed unchanged.
- The upstream local authority-snapshot test was `sandbox-blocked` by EPERM, not PASS. The merged candidate's affected-platform CI above passed; neither observation proves working-host authority or TCC.

This Windows continuation downloaded artifact `13509232169` through `gh run download 42231396039 --repo taskqorg/agentq --name macos-aarch64` into a fresh temporary directory. `Get-FileHash -Algorithm SHA256` and file-size readback matched the replacement identity above; the binary was not executed. The connector returned a hosted artifact reference, but its local transfer stalled and was stopped; that transfer is not a passing check. Node's built-in Ed25519 verification accepted the published manifest using the independently retained publication key and refused both a zeroed signature and a changed digest. No signing key moved. These observations establish bytes and detached signature, not macOS codesigning, TCC, installed trust or native containment.

Upstream's latest read-only retained-journal observation remains 9,862 bytes, SHA-256 `ab4281c3aa8971a1ab965d68495364c8dcc5c9410de2dbe29f7884b2b77d96f6`. This is attributed historical Mac evidence, not a fresh journal observation by this worker. The failed operation remains unknown with occupancy retained. A new operation cannot complete, release, replace or rewrite it.

### Bounded replacement Mac handoff

#61 has since run this handoff; its result is in [the next section](#native-qualification-through-61). The text below records the handoff as written at `17ddcae` and authorizes nothing further. [Mac prerequisite #61](https://github.com/taskqorg/assetq/issues/61) owns the separately runnable native receipt, pinned to host `mac`; #52 waits for that evidence under TaskQ §7.3. Open-task readback found no duplicate native prerequisite. This Windows worker has no authenticated Mac operator transport and makes no native call. The new task does not grant runtime authority or weaken #52 acceptance.

The original numbered operator handoff above describes the old digest and must not be used to launch that artifact again. For the replacement, first stage and verify only the exact new bytes and manifest pinned here. Read back current authority independently: the previously observed Workspace grant revision 34 and approver `kirs` are historical evidence, not action-time authorization.

The existing operator must authenticate the replacement against its existing signed trust/policy and current grant/profile, account/native identity, durable journal and caller-owned permit. #266 provisioned none of these for the replacement. An unavailable signing/operator channel or a policy that does not authorize this digest stops before effects; request the concrete bounded existing-authority provisioning action. Do not edit a grant, transfer a private key, reset a journal or silently widen the trusted boundary.

Only after action-time authorization may the existing operator prepare one new incarnation-bound operation with a caller-owned `pipe.permits/1` permit, persist its checkpoint and one-shot launch intent, then initialize the non-generating transport. No model turn, image generation, model/account/capacity change, TaskQ slot/adapter/CA or fallback is allowed. Keep transport alive through cancellation readback and recover/reconcile that same checkpoint without relaunch. Missing native/descendant completion or exact-generation cleanup acknowledgment retains occupancy. Report TCC denial as Open, not an owner ask.

Preserve the failed operation and original journal entries, adapters, permits, receipts and accepted crate; do not mistake an expected new-operation journal append for permission to rewrite retained entries. Publish only safe candidate identities, check outcomes and receipt references. Pipe's permits implementation, consumer integration and live image qualification remain #47. A4 stays incomplete until the existing Mac operator supplies this native evidence.

## Native qualification through #61

[Mac prerequisite #61](https://github.com/taskqorg/assetq/issues/61) closed after [PR #62](https://github.com/taskqorg/assetq/pull/62) merged as `3b5134660a5dffb3535b59e33d01375852339a61`. [Run 42255613069](https://github.com/taskqorg/assetq/actions/runs/42255613069) returned `event: merge_group`, that exact `headSha` and `conclusion: success`. The immutable [#61 record](https://github.com/taskqorg/assetq/blob/3b5134660a5dffb3535b59e33d01375852339a61/docs/evidence/61/README.md) owns the commands, receipts and readback tables.

This Windows continuation made no native call and read no private receipt. The native results below are attributed to the [owner's operator run](https://github.com/taskqorg/assetq/issues/61#issuecomment-6690095841) and the #61 Mac worker's read-only readback, which the #61 review accepted. They qualify the replacement adapter only (R3–R6/R10).

| Check | Failed `425c5924` launch | Replacement `a1cbb7b8` operation |
|---|---|---|
| Effective boundary | `verified-effective` | `verified-effective`, before and after initialize |
| Initialize | Frontend exited with `native stdout interrupted` before initialize | Returned; `native.jsonl` holds exactly one line, the `initialize` response |
| Completion certificate | None | `agentq.native-completion/1`, counts `[20,19]`, for the exact supervisor, native process and scope |
| Cleanup acknowledgment | None | `bootout_ack: true`, `cleanup_verified: true` |
| Recovery | `unknown`, occupancy retained, no relaunch | Same checkpoint recovered with no relaunch; `observe` and `reconcile` read `completed`, also from a fresh process |
| Model turn, thread, input replay, image | None | None |

This is the native failure before and pass after for A4's transport sentence. The second column stays true of the retained operation: the new operation did not complete, release, replace or rewrite it. Its journal entry reads byte-equal inside the new 19,999-byte journal, SHA-256 `b1fbe9216f0bd9000ea56c74601e2294619bb162bd3b8f1feec398c9a2b80554`.

A4's authority limits held. Current grant readback stayed Codex Workspace revision 34 by `kirs`, and the policy file and permit issuer are unchanged. The operator repinned host trust to the reviewed manifest, keeping the old trust copy, and issued one new `pipe.permits/1` permit from the existing issuer; neither is a new grant. #61 records no model, account or capacity configuration change; this attempt did not read those values directly. `taskq service permissions` appears in #61 only as a read-only comparison of the current grant, not as a host dependency.

This attempt checked what it could reach from Windows. The published harness source reads back as 17,136 bytes, SHA-256 `6008b334b3fd60510529909b1f9d1e71368eddeaaf210bf1942308bd20f225a6`. It calls `ProtectedHost::signed`, `prepare`, `launch_prepared`, `recover_preparation`, `observe` and `reconcile`, and a search finds no `taskq`, `slot`, `thread/start`, `turn/start` or `image_generation` in it. This reads source only; the binary's build pin to `a610cf7` remains the operator's statement. AgentQ `main` is one commit past `a610cf7` and that commit changes no protected host path, so the pin below is still current.

### What #47 receives

#47 consumes source and API `a610cf7680b1ecc0d5b51847a0ea03eced077ded`, macOS arm64 adapter `a1cbb7b88959532f2be28ea8d696c4cc1ae17b7c97205d6549f7156164c82a58` and the publication key above. `425c5924` stays retained for evidence and must not be launched again. These items remain open and are not closed by #52 A4:

- Native workload enforcement, live account login and model-metadata network use are unproved. The receipt itself says `native workload qualification separate`.
- The new operation's journal record reads `occupied: true`. The host cannot release capacity, and the Pipe-owned final release is #47 work that has not run.
- The retained failed operation stays occupied with completion unknown. Its native reconciliation has not run; A3 rests on the published deterministic API and retained-archive checks.
- `trust.json` holds one pin, now the replacement. From source, not from a run, the old adapter refuses the retained operation with `adapter not pinned by host operator` until the kept trust copy returns, and returning it stops the replacement. #47 must plan this before it reconciles the retained operation.
- Windows native containment stays unsupported, and the Windows digest above has no native qualification.
- Pipe permits, consumer integration and live image qualification are unchanged and stay in #47.
