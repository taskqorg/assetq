# Independent protected host publication

[Pipe #52](https://github.com/taskqorg/assetq/issues/52) owns this prerequisite for [#47](https://github.com/taskqorg/assetq/issues/47). Its original goal is: “Publish the immutable AgentQ/CoreQ API and signed host adapter required by Pipe #47 without TaskQ runtime authority. Pipe permits, consumer integration and live image qualification stay in #47.”

The fresh #52 inbox resumed this task after [AgentQ #241](https://github.com/taskqorg/agentq/issues/241) closed. [PR #244](https://github.com/taskqorg/agentq/pull/244) published the independent API and signed adapters. This record reconciles that trusted publication; it changes no Pipe runtime, host trust or authority. Pipe R3–R6/R10 and TaskQ §7/R12/R13 apply. The [#42 legacy recovery decision](https://github.com/taskqorg/assetq/issues/42#issuecomment-6661217844) remains temporary and unchanged.

## Immutable identities

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
| A2 | Upstream preparation, every-input grant/profile and process-incarnation guards, one-shot launch, cancellation, native/descendant completion and durable journal checks are published. Missing or changed proof refuses effects and leaves occupancy unknown; transport exit or a stop acknowledgment cannot release it. Deterministic checks are evidence for the model/process boundaries; working-host native enforcement remains unqualified. |
| A3 | The host accepts authenticated `pipe.permits/1` caller permits. Pipe owns admission and final release acknowledgment; host cleanup cannot release its capacity. Authenticated retained archives preserve exact original checkpoints/receipts, operation identity and consumed stop intent without relaunch, receipt rewriting or TaskQ runtime fallback. Pipe's issuer, permit persistence and integration are still #47 work. |
| A4 | Deterministic adversarial recovery checks and green merge-result CI are published. The corrected signed macOS artifact has platform build/tests/offline-selftest evidence and detached-signature verification. Native non-generating transport qualification under the existing Mac operator authority is **unknown**, so the native portion of #52 A4 is not proved by #241's closure. No new grant, model/account/capacity change or image generation is established or authorized by this record. |

AgentQ #241 A4 did not include #52's sentence requiring existing Mac operator qualification. Its closure therefore does not establish that additional native requirement. This record preserves that acceptance gap; it does not amend A4 or treat portable signing checks as native transport proof. Pipe consumer integration and live image qualification remain in #47.

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

This handoff is prepared, not executed. No Mac operator endpoint or authenticated working-host authority was supplied to this Windows attempt. The next action is for the existing Mac operator channel to perform the bounded qualification and return its safe readback; A4 remains open until that happens. This continuation changes no test assertion, runtime, host trust, policy, permit, grant, account, model, capacity or retained operation. No native failure-before/pass-after claim is made. Before any first launch, abandoning the staged candidate leaves the old adapters intact; after launch intent, recovery must retain the new operation and journal rather than delete or reset them.
