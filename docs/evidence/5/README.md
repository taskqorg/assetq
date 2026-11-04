# Meshy implementation and qualification evidence

Task [#5](https://github.com/taskqorg/assetq/issues/5) requests:

> Turn an immutable image asset into a retained textured GLB through a durable Meshy and headless Blender pipeline.

The owning rules were updated before implementation in `assetq.md` and [Meshy pipeline](../../meshy-pipeline.md), retaining the [original owner request](../../intake/2026-11-01-owner-request.md). This candidate implements the bounded initial route. The PM's first operator run spent 15 credits and reached provider success, but the adapter rejected HTTP 202 and produced no manifest. **Adapter live completion and 3D visual acceptance remain unproved.**

## Applied rules and observed checks

- **R2/R6, A3:** reuse durable job/effect IDs, fences, event replay and atomic manifest publication. The adapter persists its shared-store selector before its only POST. Lost replies and failure to persist a returned task ID remain unknown across restart; no sealed absence or paid retry is invented. Existing lifecycle recovery traces remain required.
- **R3/R4, A1/A2:** the exact operator-bound job, compiled method version, unchanged source, artifact hashes, inherited memory readback and fresh protected account/ledger evidence gate dispatch. There is no DELETE cancellation, model fallback, grant edit or worker-role bypass. General dispatch remains refused.
- **R5, A4/A5:** retain original GLB/maps and provider identity; headless measurement/rendering never rewrites the GLB. The service imports originals before its final Meshy validation hook, preserving retrievable object records outside a successful manifest on invalid output. The isolated PostgreSQL check proves this failure path, recorded usage, released job lease, audit replay, duplicate identity and one submission after restart.
- **R1/R4/R7, amended A6:** owner answer [6629641665](https://github.com/taskqorg/assetq/issues/5#issuecomment-6629641665) removes the cumulative testing ceiling; known actual/cumulative costs are still retained. Unknown usage/balance, stale evidence, changed or insufficient balance, ledger overflow, inadequate capacity and an unqualified executor refuse effects. Per-job limits remain. Each run's selector prohibits a second POST. Cross-store reconciliation/exclusivity is an operator qualification obligation, not an implemented distributed ledger.
- **R8:** focused adapter/registry checks passed on Windows with Rust 1.95.0. The final canonical gate passed as recorded below. Native/provider/managed-host qualification stays separate from deterministic checks.

The seven adapter checks cover the exact request, admission at 5000 prior credits, unknown spend/insufficient or changed balance/overflow refusal, stale qualification, unsafe URLs/GLB/texture inputs, HTTP 200/201/202 ID persistence across restart, lost reply, lost ID receipt, original-ID polling, cancellation without DELETE, retained completion without credentials, and matching confirmed-stop recovery. Two Python scene fixtures verify connected components, world dimensions, triangle reporting, UV/PBR presence, texture size and nonfinite geometry; their scene and Blender identity are explicitly invented.

## Failure before, pass after

The review found three recovery defects in the first adapter draft. The same regression checks were run with each prior branch reconstructed, then with the correction; no expected outcome was weakened:

| Failure | Before | After |
|---|---|---|
| Retained completion still required live credentials | `meshy_retained_completion_does_not_need_provider` fails with `protected Meshy credentials unavailable; observation unknown` | Pass; verifies local hashes and retained 15-credit observation before any HTTP read |
| Oversize completed output was treated as transient | `meshy_download_refusals_and_native_allocation_bounds` fails the definitive-refusal assertion | Pass; terminal validation report replaces an endless observation retry, and short original files remain retained |
| Confirmed local stop still prevented safe validation recovery | `meshy_validation_restart_requires_original_stop_readback` rejects the confirmed-stop case | Pass; matching stop permits recovery, while absent, mismatched or unconfirmed stop stays unknown |

The new R5 method version intentionally expands the built-in catalog from seven to eight entries under A1; old version pins and project-scope protections remain. The two registry fixtures initially failed `left: 8, right: 7`; after updating only the expected catalog count and adding old-catalog migration/disabled-v1 preservation checks, both pass. The CLI catalog assertion also expects eight. Additive migration inserts version 2 for existing projects without rewriting any prior method or changing grants, budgets or recommendations.

Full development logs are retained in the ignored `target/taskq-5` directory. The initial default-toolchain formatting command reported missing `cargo-fmt`; subsequent checks select the already installed Rust 1.95.0 toolchain consistently for compiler, rustdoc and formatting. No toolchain installation or host setting change was made.

## Immutable input and fresh public documentation

[Source manifest](source-manifest.json): original asset `2c887345-9f66-491f-8a0c-81d81cf0c4af`, PNG 1254 by 1254, 1,936,663 bytes, SHA-256 `fe2077d15b0c50617b84c6267277d46339c6a8cedbd6334e923cfa5475529baa`. The repository bytes were independently hash-verified again; no image regeneration, resampling or source acceptance rewrite occurred.

[Official API receipt](official-api-verification.json) records unaltered HTTP-body hashes and fresh public source URLs. The official schema supports the explicit Smart Topology/meshy-t2/3000/2K PBR request, documents 15 credits and `consumed_credits`, and describes DELETE as permanent deletion rather than safe cancellation. Public docs do not establish account credit, prior workstream spend or provider/native qualification.

## Remaining operator action

The [operator procedure](operator.md) is the concrete next action. The worker's original Windows preflight reported Meshy credential **absent**, Blender **not supplied**, inherited memory boundary **unqualified**, and balance/prior ledger **unknown**. The PM subsequently ran the first paid example as recorded below; no worker dispatch, purchase, top-up, host-policy change or deployment is claimed.

A4/A5/A7 still require a successful authorized adapter run, actual charge and before/after ledger reconciliation, native measurements/textured views, real result manifest and authenticated downloads. Final 3D visual acceptance belongs to the owner. Managed pds18 retrieval is coordinated separately with #6; local results will not qualify that host. Green merge-result CI is still required before publication/acceptance.

## Previous frozen candidate gate

`python3 scripts/check.py` **PASS** on Windows at `82dd1f1bb6ccdc68096359facf2b6b022279d16a`, Rust 1.95.0, 272.31 seconds including setup. Results: four checker, four image-driver, two measurement, 26 Rust core, 30 isolated PostgreSQL service and two trace checks; pinned contract lint, formatting, whitespace and doctest invocation passed. The first run stopped at contract lint before Rust execution (712-character paragraph); paragraph splitting passed the same pinned lint, and the rerun passed. Combined canonical elapsed time was 275.89 seconds. Ubuntu and merge-result CI remain separate.

Commit `1df7046` recorded only that gate evidence, leaving Rust, tests, dependencies, validation script and recipe unchanged. Its tested debug binary SHA-256 was `a5ee9ddd043593ff68a88d1e5692b9fd9939fa75cef6ecc0928097fc2a701ae1`; this identifies the historical local artifact, not the corrected candidate or qualified operator/Blender build.

## First operator run and corrections

PM note [6628734645](https://github.com/taskqorg/assetq/issues/5#issuecomment-6628734645) reports one POST on `1df7046`, 15 credits spent (5500 to 5485), provider task `01a1a625-c6c6-75c8-bcc0-4b738fb2603b` terminal `SUCCEEDED`, and standalone `validate_meshy.py` passing. AssetQ stayed waiting/effect uncertain without a manifest. This does not establish adapter polling, downloads, validation, manifest, usage or provider-wait restart.

| Defect or approved behavior change | Before | Correction and observed check |
|---|---|---|
| HTTP 202 ID acceptance (A3, R4/R5) | `cargo test --locked --lib meshy_accepted_post_retains_original_id_after_restart` fails with `Meshy HTTP 202; response retained only by provider, outcome requires reconciliation` | Same check passes for 200/201/202, retains the original ID across observer restart and refuses a second POST |
| Former cumulative ceiling (amended A6, R1/R7) | `cargo test --locked --lib meshy_recipe_spend_and_input_boundaries` fails at 5000 prior credits | Same check passes; unknown/changed/insufficient balance, unknown spend and overflow refusals remain |
| Missing temporary-quota prerequisite (R7) | PM observed `store_full` before POST with the 1 GiB default | Procedure now explicitly requires `ASSETQ_TEMP_QUOTA_BYTES=2147483648` and at least 1280 MiB measured room; native follow-up remains pending |
| Blender allocation on 24-core Linux host (A4, R7) | PM observed `OIDN error: out of memory` under 2 GiB | Child now sets allocator arenas/OIDN threads to 2; procedure supplies 2048 KiB inherited stack and two allowed CPUs. PM reports those settings passed standalone validation; corrected adapter run remains pending |
| Windows cancellation fixture cleanup (R8) | Canonical gate at `ed08fae` failed after 28.14 seconds with sharing violation 32 at `protected.rs:583`, deleting the fixture directory after its EOF marker | Cleanup waits at most three seconds only for sharing violation 32; `cargo test --locked --lib image_worker_cancel_readback_precedes_native_eof` passes with all cancellation-order/error assertions retained |

`cargo test --locked --lib meshy` passes all seven checks after the corrections. Before/after logs are retained in ignored `target/taskq-5`. The owner-approved follow-up uses a fresh store and new job, with prior spend 15 preserved plus any other reconciled attempts. No repair path or replacement POST for the earlier operation was added; no paid effect was sent by this continuation.

`python3 scripts/check.py --test meshy` passes seven adapter checks and the isolated PostgreSQL validation-retention fixture, plus formatting, four image-driver and two measurement checks. The first corrected-candidate canonical run passed contract lint/whitespace and 26 Rust core checks, but failed the Windows cleanup above before service checks. Its log/receipt are preserved as `target/taskq-5/corrections-first-canonical-failure.*`; this failure is not reported as a passing gate.
