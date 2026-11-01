# AgentQ protected receipt wire mismatch

One focused AgentQ follow-up draft for PM intake, applying AssetQ R3-R5/R8 and AgentQ M12.8. Task [#4](https://github.com/taskqorg/assetq/issues/4) remains incomplete. This document creates no task, grant, alternate resolver or native launch.

Resolution: [AgentQ #193](https://github.com/taskqorg/agentq/issues/193) is accepted and PR195 published the fix as `8427bba6145aac0239cd16a8dae0caf8efa00695`; [merge-group Test run 41336656270](https://github.com/taskqorg/agentq/actions/runs/41336656270) passed on that exact merge result. AssetQ adopts this immutable pin as method version 4, retaining versions 2 and 3 and all original receipts. The diagnosis below is historical; its local diagnostic fix was not used as a dependency override. The remaining gate is the [existing composed Mac qualification](../evidence/4/protected-consumer.md), current native tool/output capability and durable slot integration. Publication of this upstream fix is not native acceptance.

## Goal

Make AgentQ's protected app-server validator consume the published TaskQ authority receipt without weakening authority, process-identity or no-replay checks. Publish a corrected immutable runtime pin for AssetQ to consume.

## Native failure and exact cause

[PM note 6561661669](https://github.com/taskqorg/assetq/issues/4#issuecomment-6561661669) reports build PASS for AssetQ `0c257a82f4ddc14eabadbe234026e937ee701ab0`, but no composed handshake. The first operation had a stale expected profile. A distinct bounded diagnostic operation used an independently derived expected profile matching actual authority, yet AgentQ repeatedly returned `protected authority missing, unknown or mismatched; no native input`, then observe timed out. PM verified native absence and acknowledged original-slot release. Private receipts stay with PM. No image/model/paid call, grant/settings/account/model/capacity change or crate replay occurred.

The pinned source has a deterministic producer/consumer mismatch:

| Owner and immutable source | Wire behavior |
|---|---|
| TaskQ `56b1a1eba274bf88520e4d668461372fe3048b00`, [`policy::Effective::json`](https://github.com/taskqorg/taskq/blob/56b1a1eba274bf88520e4d668461372fe3048b00/src/policy.rs#L85) | Serializes the resolved mode under `effective`, not `mode` |
| Same TaskQ source, [`runtime::protected_app_server::resolve`](https://github.com/taskqorg/taskq/blob/56b1a1eba274bf88520e4d668461372fe3048b00/src/runtime/protected_app_server.rs#L338) | Sets `authority.resolved` to `access.json()` |
| AgentQ `aab1a190ab76f1e932578520d7a8787ef1d4b660`, [`agent::protected::validate_receipt`](https://github.com/taskqorg/agentq/blob/aab1a190ab76f1e932578520d7a8787ef1d4b660/runtime/src/agent/protected.rs#L185) | Requires `authority.resolved.mode == "workspace"`; genuine TaskQ receipts lack that field |
| Same AgentQ source, [protected process fixture](https://github.com/taskqorg/agentq/blob/aab1a190ab76f1e932578520d7a8787ef1d4b660/runtime/tests/support/protected.rs#L34) | Invents `resolved: {"mode":"workspace"}`, hiding the incompatible wire shape |

`serde_json::Value` indexing returns null for the absent `mode` field. The first authority-condition group therefore refuses before the consumer/supervisor/native-incarnation checks. This defect explains a refusal even with a matching independently expected profile. It does not establish that PM's other receipt fields or native bindings match; those remain unqualified. The later observe timeout has not been attributed to this mismatch.

Read-only main-source inspection still found the same lookup after #186 closed; that accepted task's body and writer are not changed here. The fix belongs to AgentQ's validator and producer-shaped fixture, not AssetQ or a second TaskQ resolver.

## Executable proof and proposed correction

Reuse the existing AgentQ test in a verified isolated checkout of `aab1a190ab76f1e932578520d7a8787ef1d4b660`:

```sh
cargo test --locked -p agentq-runtime --test agent -- protected_
```

Windows observations, with no native provider/host operation:

1. The unchanged published fixture passes its one selected process scenario.
2. Change only the fake receipt's `resolved` object to the actual TaskQ keys: `runtime`, `requested`, `grant`, `effective`, `enforcement`, `refused`, using explicitly invented fixture values. The same scenario fails at `server.verify` with PM's exact generic refusal. No native input reaches the fake process.
3. In that diagnostic checkout only, change the validator's lookup as below. The same scenario passes. Add refusal cases for `effective: "full"`, null, and a mode-only object lacking `effective`; the scenario still passes with all existing authority, identity, interruption, uncertainty, no-replay and one-usage-event assertions retained.

```diff
-        || !nonempty(&a["requested"]) || a["resolved"]["mode"] != "workspace"
+        || !nonempty(&a["requested"]) || a["resolved"]["effective"] != "workspace"
```

This is an isolated diagnostic correction, not an AssetQ dependency patch, published AgentQ fix or Mac PASS. AssetQ's Cargo pin and authority checks remain unchanged. Do not accept a `mode` alias, copy the runtime validator into AssetQ, skip identity checks or derive expected values from an operation receipt.

## Proposed upstream acceptance

1. Align the owning runtime detail with TaskQ's existing `taskq.app-server/1` receipt shape before the compatibility fix. Preserve M12.2-M12.8: optional caller-trusted adapter, CoreQ primitives, no mandatory TaskQ dependency or policy store.
2. Consume `authority.resolved.effective` at the existing validator. A producer-shaped Workspace receipt may proceed only after every existing authority/incarnation check passes. Missing, null, full, mismatched and mode-only evidence refuses before native input.
3. Keep the producer-shaped fixture and regression above. Preserve stale/tampered grant/profile/account/image/operation/host-generation/process checks, interruption, unknown readback, uncertain writes, no replay and one attributable usage event. Add no live/model turn or paid evaluation for this wire correction.
4. If diagnostic refinement is needed, emit fixed field/category names such as `authority.resolved.effective`, never raw values, paths, keys, profiles or receipts. Do not weaken a comparison to make native qualification proceed.
5. Publish the corrected immutable runtime pin with affected checks and green merge-result CI, retaining the existing consumer API. PM then resumes one bounded composed AssetQ qualification through the existing recipe and grant. Native identity/enforcement, current image-tool/output capability and remaining observations stay separate from the portable pass.

## PM intake and preservation

Create or coordinate one additive AgentQ defect task for this validator/fixture correction, then link AssetQ #4 to that concrete dependency. Do not rewrite closed #186 or start a competing writer. No TaskQ adapter/grant/model/account/capacity/configuration change is proposed.

Preserve PR15, the original crate asset `2c887345-9f66-491f-8a0c-81d81cf0c4af`, SHA-256 `fe2077d15b0c50617b84c6267277d46339c6a8cedbd6334e923cfa5475529baa`, original effect and both failed native receipts. Keep generation refused. After accepted publication, adopt its immutable pin without rewriting prior method/operation provenance, then resume qualification; do not replay the crate to diagnose authority.
