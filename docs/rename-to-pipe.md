# Pipe rename handoff

[Task #42](https://github.com/taskqorg/assetq/issues/42) retains the original owner goal and A1-A6. Its accepted answer [6661217844](https://github.com/taskqorg/assetq/issues/42#issuecomment-6661217844) permits temporary legacy TaskQ recovery. Note [6661196390](https://github.com/taskqorg/assetq/issues/42#issuecomment-6661196390) replaces A4: only `pipe.ufobe.com`, no redirect and no Pipe service on `assets.ufobe.com`. The owning rules are [Pipe R1-R10](../pipe.md).

## Compatibility and callers

The crate, command, help, logs, site titles and release candidate are `pipe`. The archive contains `bin/pipe`; it contains no `assetq` command. Existing `ASSETQ_*` configuration, guard headers, cookie, pending request keys and retained operator binding fields stay compatible. Database/role names, schema, objects, stored IDs and immutable method definitions remain unchanged, including historical `assetq.md` provenance strings and original receipts.

The existing deployment tool copies the verified `bin/pipe` bytes to the exact executable path already bound by a live unit when needed. It creates no global alias and changes no unit. The old command is retained only at that host path. An isolated fixture verifies both `bin/assetq` and top-level `assetq` host paths with unchanged account, environment, objects and rollback.

| Caller | Evidence and remaining observation |
|---|---|
| Existing pds18 service user unit | Prior PM inventory in [deployment operator](deployment-operator.md#inventory) establishes the unit shape and dedicated account. Task #25 must read back the exact live executable path before activation; it is not verified by this source change. |
| Host operator scripts or additional executors | No live inventory is available in this worktree. Task #25 must enumerate them before claiming that every remaining `assetq` caller is covered. |
| In-flight tasks #9 and #25 | Worktree paths remain unchanged. `assetq.md` remains a link to `pipe.md`; `scripts/deploy_assetq.py` keeps its path. Both tasks received a steer naming Pipe, the new origin and no-redirect requirement. |
| Retained protected image operations | Keep the original pinned adapter, operation/slot identities, credentials boundary and receipt paths. R10 allows observation, cancellation, reconciliation, release and retained import only. New protected launch and qualification refuse. |

## Host cutover and evidence

Task #25 owns private DNS/Caddy/guard readback and site-only rollback. Use the Pipe candidate's exact source, archive, executable and embedded UI digests. Preserve the dedicated `assetq` account/unit, database, environment and object root. Publish `pipe.ufobe.com` only; remove the prior Pipe site on `assets.ufobe.com` without a redirect. Prove TLS, authenticated browser/API entry, denied requests and retained payload hashes on `pipe.ufobe.com`.

This worker's existing `pds18` SSH alias did not resolve, and the HTTPS connection to `pipe.ufobe.com` timed out. Neither is a host qualification or proof of an application failure. Activation, live callers, DNS/TLS/sign-in and real browser journeys remain unverified until private host readback. Do not infer that source or fixture checks deployed the candidate.

## Repository move owned by PM

The repository is still `taskqorg/assetq`; this deliverable does not rename it or change board ownership. When the owner moves it to `taskqorg/pipe`, the PM updates these references together:

- Change each checkout and worktree's shared Git remote URL to `https://github.com/taskqorg/pipe` or its existing SSH equivalent. Preserve local checkout/worktree paths and active attempts.
- Change tracked `taskq.json` from `taskqorg/assetq` to `taskqorg/pipe`, and reconcile the corresponding host project registration/configuration on every registered host. Verify each host resolves the same board before admitting new work.
- Update `REPO` in `scripts/deploy_assetq.py`, CI/release consumers and artifact lookup references. The workflow artifact is already `pipe-candidate`.
- Update board links and cross-project dependencies, PM bookmarks and future task/commit references. Preserve historical issue, commit and evidence links through the repository redirect; do not rewrite immutable receipts or published history.
- Read back the moved board, open tasks #9/#25/#42, active attempt ownership, publication route and remote origin. A local Git rename never proves host adoption or board migration.

## Verification boundary

R9/A4 regression: `python3 scripts/check.py --test management` first failed with `retired host assets.ufobe.com/assets`, observed 403 versus required 421. After the common pre-database host refusal, the same command passed: CLI/refusal, browser/session and scoped HTTP checks, including all four retired-host routes and no redirect. The existing authentication, CSRF, immutable retrieval and durable request assertions remain required.

R10: `cargo test --locked --lib image_worker` passed 17 checks, including refusal at each native launch entry, original-operation reconciliation, cancellation ordering, immutable methods/import and uncertain reservations. `python3 scripts/test_qualify_image.py` passed the operator/worker refusals and retained duplicate/restart/import protections. The lifecycle model and replay traces are unchanged; the full R8 gate includes them.

The two selected deployment checks (`Layout` and `Apply.test_apply_verifies_preserves_and_rolls_back_on_operator_request`) passed for the new package and old host executable paths. Source comparison left SQL schemas, lifecycle model, trace data, stored method evidence and immutable method fixtures unchanged. The full canonical gate and exact packaged-candidate outcomes belong to this PR and task result; these local checks do not qualify Linux deployment or a live site.
