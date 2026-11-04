# Versioned comparison and guidance evidence

[Task #7](https://github.com/taskqorg/assetq/issues/7) implements R3-R5/R7-R9 through the existing service, registry, CLI and job model. The [evaluation contract](../../method-evaluation.md) was added to the owning contract before implementation. The [original owner request](../../intake/2026-11-01-owner-request.md) and full task acceptance remain the source of scope.

## Acceptance mapping

| Item | Observable implementation and protection |
|---|---|
| A1 | One PostgreSQL registry; CLI and agent JSON use the same authenticated API. No parallel recommendation source or new dashboard |
| A2 | Append-only method versions/statuses, operation/capability/typed-constraint filters, immutable dataset/evaluator revisions and attachment/hash snapshots. Ordinary jobs retain resolved methods and optional recommendation IDs |
| A3 | Fixed case/method/repeat matrix, bounded job/credit admission, resumable original jobs, measured terminal elapsed time/usage/failure, verified-manifest technical scores and immutable human scores. Missing jobs, judgments and usage remain explicit |
| A4 | Mixed-operation comparisons refuse; recommendation promotion and reads include operation/use case/constraints. A newer method/status alone produces no recommendation. Owner assessment remains required |
| A5 | Exact-spec-bound image and Meshy qualification metadata projects the retained receipts; no general route is promoted. Reads return scoped reasons, receipt evidence, observation time and revision |
| A6 | Authenticated status includes provider/job observations, reported/unknown cost, bounded outcomes, physical store/integrity/backup/restore observations and registry/evaluation revisions. Stable cursor reads preserve older comparison evidence. DashQ/ServerQ contracts preserve their ownership |
| A7 | Deterministic fixtures pin recommendation/method versions and fixed repeated inputs across promotion, partial admission and restart; rereads retain original sample IDs/manifests. Real-provider experiments refuse before job admission: this delivery grants no paid experiment budget |

The existing canonical runner discovers all registry checks; no new dependency or separate test runner was added. `python3 scripts/check.py --test registry` passed six isolated PostgreSQL fixtures, plus the existing four operator-driver and two validator checks. Rust 1.95.0 and PostgreSQL 18.1 were used on Windows; the installed default toolchain lacked `cargo-fmt`, so the existing complete 1.95.0 toolchain was selected for these commands. The final rebased candidate's canonical gate and source SHA are recorded in the task/PR result, separately from these development observations.

The first API fixture compile failed because it referenced a nonexistent `Service.addr`; using the existing `url` field passed. Its first successful native CLI run exposed safe stale-fence retries under the shared two-second fixture lease. This new projection fixture uses a 60-second lease because it tests registry/API journeys; the existing expiry/fencing assertions and production settings remain unchanged. The subsequent six-fixture run passed.

## Failure before and pass after

Review of `30d07e52afefa616cbbaa26460a448caf7473423` returned A2/A3/A7 and R5 for noncanonical method pins: `synthetic.fake@01` was accepted, then run/read returned `409 sample_conflict` after resolution to `synthetic.fake@1`. The admission regression added to `registry_cli_api_projection` failed before the fix: all five requests (`@01`, `@+1`, canonical-plus-each-alias and both aliases) returned 201 instead of 422. With the three-line shared comparison admission guard, the same `python3 scripts/check.py --test registry_cli_api_projection` passed on Windows. All five now refuse before retaining a comparison or admitting a job; the existing canonical CLI run, technical result, immutable score, authenticated read and foreign-project assertions still pass. The guarding assertions for pins, resumability and replay remain unchanged. The owning [evaluation rule](../../method-evaluation.md) was clarified before the implementation.

`registry_publication_limits_and_receipt_identity` was run against reconstructed pre-fix branches from this implementation. It failed with all five observations below. The fixed source was restored byte-for-byte, and the same check then passed in the six-fixture run:

- A project-owned different specification borrowed the real Meshy receipt when lookup used only `id@version`. Qualification now also requires the exact immutable recipe and exposes its SHA-256.
- 501 newer proposals hid a promoted route behind the history limit. Eligibility now has its own bounded promoted-candidate query, independent of history.
- Sample admission, job completion and human scoring each left the control comparison revision unchanged. The revision now includes original sample job state and immutable judgments.

The same fixture retrieves comparison 501 through a stable cursor and reads its historical missing-case record. Other fixtures protect cross-operation refusal, changed-revision conflicts, matrix job/credit ceilings, unknown usage, failed samples, paid-run refusal, foreign-project denial and original pins after promotion. The CLI fixture exercises dataset/evaluator registration, filters, comparison creation/run/result/scoring and control reads; anonymous reads return 401 and foreign comparison reads 404. No existing guarding assertion was deleted or weakened.

Independent read-only source review found the qualification, history-selection, revision and pagination gaps above and confirmed their corrections. The writer owns executable checks. A one-time receipt consistency check matched the unchanged crate PNG's SHA-256 and native runtime identity, six managed receipt identities/verdict values, and every local seed evidence link. PM-attributed managed observations were not independently repeated.

The first canonical invocation refused two new contract paragraphs at 478 and 600 characters against the existing 400-character line guard. Splitting the same rules into separate paragraphs preserves their meaning and the guard; the next canonical invocation verifies that correction. No Rust checks ran in the refused invocation.

The next invocation passed contract lint and 26 core checks, then exposed Windows sharing violation 32 in the existing catalogue fixture's directory removal. Closing its owned Runtime after cancellation and reusing the cancellation fixture's existing three-second sharing-violation cleanup preserves every catalogue identity, no-resend, no-turn and error assertion. This is fixture cleanup, not a new native completion claim; the focused fixture and subsequent canonical result record the after check.

## Qualification limits

The [image receipt](../4/native-receipt.json), [native-size recovery](../4/native-size-import.md) and [managed Meshy record](../6/README.md) remain unchanged. Image charge is unknown; version 5's bounds failure remains a failure. The Meshy observation retains its 15-credit charge, 60-credit historical workstream total and owner verdict: “accepted as a pipeline check, not as a finished game asset.” These are scoped historical observations, not current provider health, remaining spend authority or universal method rankings.

No native generation, paid API request, purchase, top-up, model/provider change, grant change, host activation or deployment occurred. Backup/restore remains deferred/unknown. General image dispatch, real-provider comparison execution, automatic promotion, website/browser acceptance and live DashQ/ServerQ integration remain outside this implementation's proof. GitHub publication still requires green merge-result CI; local Windows fixtures do not substitute for that gate.
