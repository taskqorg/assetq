# Meshy implementation and qualification evidence

Task [#5](https://github.com/taskqorg/assetq/issues/5) requests:

> Turn an immutable image asset into a retained textured GLB through a durable Meshy and headless Blender pipeline.

The owning rules were updated before implementation in `assetq.md` and [Meshy pipeline](../../meshy-pipeline.md), retaining the [original owner request](../../intake/2026-11-01-owner-request.md). This candidate implements the bounded initial route. **No live Meshy request, real GLB, native Blender qualification or 3D visual acceptance is claimed.**

## Applied rules and observed checks

- **R2/R6, A3:** reuse durable job/effect IDs, fences, event replay and atomic manifest publication. The adapter persists its shared-store selector before its only POST. Lost replies and failure to persist a returned task ID remain unknown across restart; no sealed absence or paid retry is invented. Existing lifecycle recovery traces remain required.
- **R3/R4, A1/A2:** the exact operator-bound job, compiled method version, unchanged source, artifact hashes, inherited memory readback and fresh protected account/ledger evidence gate dispatch. There is no DELETE cancellation, model fallback, grant edit or worker-role bypass. General dispatch remains refused.
- **R5, A4/A5:** retain original GLB/maps and provider identity; headless measurement/rendering never rewrites the GLB. The service imports originals before its final Meshy validation hook, preserving retrievable object records outside a successful manifest on invalid output. The isolated PostgreSQL check proves this failure path, recorded usage, released job lease, audit replay, duplicate identity and one submission after restart.
- **R4/R7, A6:** known prior spend plus the expected 15 credits must fit the cumulative 1000-credit ceiling; absent usage/balance, stale evidence, changed balance, inadequate capacity and an unqualified executor refuse effects. The initial selector prohibits a second paid run. Cross-store reconciliation/exclusivity is an operator qualification obligation, not an implemented distributed ledger.
- **R8:** focused adapter/registry checks passed on Windows with Rust 1.95.0. The final canonical gate passed as recorded below. Native/provider/managed-host qualification stays separate from deterministic checks.

The six adapter checks cover the exact request, credit boundary 985/986, unknown spend, stale qualification, unsafe URLs/GLB/texture inputs, lost reply, lost ID receipt, original-ID polling, cancellation without DELETE, retained completion without credentials, and matching confirmed-stop recovery. Two Python scene fixtures verify connected components, world dimensions, triangle reporting, UV/PBR presence, texture size and nonfinite geometry; their scene and Blender identity are explicitly invented.

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

The [operator procedure](operator.md) is the concrete next action. Read-only Windows preflight reported Meshy credential **absent**, Blender **not supplied**, inherited memory boundary **unqualified**, and balance/prior ledger **unknown**. Dispatch remains disabled. No paid request, purchase, top-up, host-policy change or deployment was attempted.

A4/A5/A7 require the one authorized live run, actual charge and before/after ledger reconciliation, native measurements/textured views, real result manifest and authenticated downloads. Final 3D visual acceptance belongs to the owner. Managed pds18 retrieval is coordinated separately with #6; local results will not qualify that host. Green merge-result CI is still required before publication/acceptance.

## Frozen candidate gate

`python3 scripts/check.py` **PASS** on Windows at `82dd1f1bb6ccdc68096359facf2b6b022279d16a`, Rust 1.95.0, 272.31 seconds including setup. Results: four checker, four image-driver, two measurement, 26 Rust core, 30 isolated PostgreSQL service and two trace checks; pinned contract lint, formatting, whitespace and doctest invocation passed. The first run stopped at contract lint before Rust execution (712-character paragraph); paragraph splitting passed the same pinned lint, and the rerun passed. Combined canonical elapsed time was 275.89 seconds. Ubuntu and merge-result CI remain separate.

The subsequent change records only this evidence; Rust, tests, dependencies, validation script and recipe remain unchanged from the tested candidate. Read-only worker preflight still reports no Meshy credential, no supplied Blender and no qualifying inherited memory boundary, with dispatch disabled. The exact tested debug binary SHA-256 is `a5ee9ddd043593ff68a88d1e5692b9fd9939fa75cef6ecc0928097fc2a701ae1`; this identifies the local artifact, not a qualified operator/Blender build. No native or paid run followed the gate.
