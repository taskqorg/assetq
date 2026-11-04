# Meshy implementation and qualification evidence

Task [#5](https://github.com/taskqorg/assetq/issues/5) requests:

> Turn an immutable image asset into a retained textured GLB through a durable Meshy and headless Blender pipeline.

The owning rules were updated before implementation in `assetq.md` and [Meshy pipeline](../../meshy-pipeline.md), retaining the [original owner request](../../intake/2026-11-01-owner-request.md). The [corrected operator run](operator-run/README.md) on candidate `759fbd2` completed the durable Meshy/Blender path, including provider-wait restart and authenticated downloads. Its nine-item result costs 15 credits, with 30 observed cumulative workstream credits including the first attempt. **Owner 3D visual acceptance and the managed deployment journey remain separate and unproved.**

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

## Native outcome and remaining scope

The PM completed the [operator procedure](operator.md) on the exact corrected candidate in an isolated pds18 store. [Sanitized receipts, source/result identity, measurements, cost and textured views](operator-run/README.md) establish the named A4/A5/A7 technical example. The worker imported that evidence without a new dispatch, purchase, top-up, host-policy change or deployment.

Owner visual acceptance remains pending. Managed pds18 deployment/retrieval through the production service is coordinated separately with #6; an isolated run hosted there does not qualify that journey, the website, another executor or a general recommendation. Green merge-result CI is still required before publication/acceptance. The operator ledger explicitly retains its limited inventory scope; do not infer an independent whole-account or Windows-store audit.

## Previous frozen candidate gate

`python3 scripts/check.py` **PASS** on Windows at `82dd1f1bb6ccdc68096359facf2b6b022279d16a`, Rust 1.95.0, 272.31 seconds including setup. Results: four checker, four image-driver, two measurement, 26 Rust core, 30 isolated PostgreSQL service and two trace checks; pinned contract lint, formatting, whitespace and doctest invocation passed. The first run stopped at contract lint before Rust execution (712-character paragraph); paragraph splitting passed the same pinned lint, and the rerun passed. Combined canonical elapsed time was 275.89 seconds. Ubuntu and merge-result CI remain separate.

Commit `1df7046` recorded only that gate evidence, leaving Rust, tests, dependencies, validation script and recipe unchanged. Its tested debug binary SHA-256 was `a5ee9ddd043593ff68a88d1e5692b9fd9939fa75cef6ecc0928097fc2a701ae1`; this identifies the historical local artifact, not the corrected candidate or qualified operator/Blender build.

## First operator run and corrections

PM note [6628734645](https://github.com/taskqorg/assetq/issues/5#issuecomment-6628734645) reports one POST on `1df7046`, 15 credits spent (5500 to 5485), provider task `01a1a625-c6c6-75c8-bcc0-4b738fb2603b` terminal `SUCCEEDED`, and standalone `validate_meshy.py` passing. AssetQ stayed waiting/effect uncertain without a manifest. This does not establish adapter polling, downloads, validation, manifest, usage or provider-wait restart.

| Defect or approved behavior change | Before | Correction and observed check |
|---|---|---|
| HTTP 202 ID acceptance (A3, R4/R5) | `cargo test --locked --lib meshy_accepted_post_retains_original_id_after_restart` fails with `Meshy HTTP 202; response retained only by provider, outcome requires reconciliation` | Same check passes for 200/201/202, retains the original ID across observer restart and refuses a second POST |
| Former cumulative ceiling (amended A6, R1/R7) | `cargo test --locked --lib meshy_recipe_spend_and_input_boundaries` fails at 5000 prior credits | Same check passes; unknown/changed/insufficient balance, unknown spend and overflow refusals remain |
| Missing temporary-quota prerequisite (R7) | PM observed `store_full` before POST with the 1 GiB default | Procedure explicitly requires `ASSETQ_TEMP_QUOTA_BYTES=2147483648` and at least 1280 MiB measured room; the corrected operator run reaches its nine-item successful manifest |
| Blender allocation on 24-core Linux host (A4, R7) | PM observed `OIDN error: out of memory` under 2 GiB | Fixed allocator arenas/OIDN threads at 2, 2048 KiB inherited stack and two allowed CPUs now produce `valid: true` and three hash-verified views in the corrected adapter run |
| Windows cancellation fixture cleanup (R8) | Canonical gate at `ed08fae` failed after 28.14 seconds with sharing violation 32 at `protected.rs:583`, deleting the fixture directory after its EOF marker | Cleanup waits at most three seconds only for sharing violation 32; `cargo test --locked --lib image_worker_cancel_readback_precedes_native_eof` passes with all cancellation-order/error assertions retained |
| Lost timeout evidence (R8) | Canonical run at `094438e` hit the 480-second Rust deadline (503.45 seconds overall) without partial Cargo output; `python3 scripts/test_check.py GateTrust.test_timeout_retains_partial_command_evidence` reproduces missing output | The same regression passes after retaining partial output on timeout; diagnostics also retain the command/deadline. No timeout or selected check was relaxed |

`cargo test --locked --lib meshy` passes all seven checks after the corrections. Before/after logs are retained in ignored `target/taskq-5`. The owner-approved follow-up uses a fresh store and new job, with prior spend 15 preserved plus any other reconciled attempts. No repair path or replacement POST for the earlier operation was added; no paid effect was sent by this continuation.

`python3 scripts/check.py --test meshy` passes seven adapter checks and the isolated PostgreSQL validation-retention fixture, plus formatting, four image-driver and two measurement checks. The first corrected-candidate canonical run passed contract lint/whitespace and 26 Rust core checks, but failed the Windows cleanup above before service checks. Its log/receipt are preserved as `target/taskq-5/corrections-first-canonical-failure.*`; this failure is not reported as a passing gate.

The second canonical timeout is preserved in `target/taskq-5/corrections-second-canonical-timeout.*`. Its last running Rust test is unknown because the prior checker discarded that output; the diagnostic fix enables a meaningful next observation. Repeated Windows runs have exceeded 600 seconds cumulatively and do not establish the named `ubuntu-latest` budget or merge-result CI.

## Corrected candidate gate

`python3 scripts/check.py` **PASS** on Windows at `36737d62f092cbf6affddcfb0aa6928239761573`, Rust 1.95.0, 422.67 seconds including setup. Results: five checker, four image-driver, two measurement, 27 Rust core, 30 isolated PostgreSQL service and two trace checks; contract lint, formatting, whitespace and doctest invocation passed. The service suite took 380.76 seconds. Logs and receipt are `target/taskq-5/corrections-canonical.log` and `corrections-canonical-receipt.json`.

Combined corrected-candidate canonical elapsed time is 954.26 seconds across the cleanup failure, timeout and passing run. Read-only observation during the passing run showed its isolated service fixture active in `pg_database_size`, without a PostgreSQL lock wait; this does not identify the earlier timeout's cause. Windows timing variability, the `ubuntu-latest` budget and merge-result CI remain unqualified. No limit was raised or required check removed.

Commit `759fbd2` recorded only that gate result; implementation, tests, dependencies and recipe remain identical to the passing candidate. The subsequent [operator evidence import](operator-run/README.md) likewise changes only evidence/documents, so the exact implementation gate is retained without rerunning it. Pinned contract lint and whitespace checks apply to this import. The worker sent no paid effect; final visual acceptance and the managed journey remain open.
