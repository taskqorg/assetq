# TaskQ adapter qualification and anchor promotion

## Installed 4.6.66 qualification candidate

AssetQ R1/R3-R5/R8; continuation [note 6603148665](https://github.com/taskqorg/assetq/issues/4#issuecomment-6603148665) directs the current refresh. Original request: "PM ran the operator handoff on Mac: stopped before launch. Pinned adapter (TaskQ 4.6.58, f8b04708…5df4) refuses current host policy: 'host.json: claim must be any or labelled' (Mac claim is spillover since 4.6.6x). No slot, operation or native effect created. Host policy stays. Next: repin ADAPTER_SHA256 to the installed TaskQ 4.6.66 app-server adapter, qualify it, then ask PM again for the slot handoff; driver ready."

The selected installed artifact reports `4.6.66-261103-26`. Its 22,740,672 bytes were copied unchanged to a private isolated candidate, SHA-256 `bd6dc0de5024ff7b6b0ae0a987f5467aa0c5196b005c7cdfd434d043709c7e42`. Independent readback matches; `codesign --verify --strict` passes on those final bytes. No signing, installation, grant, model, account, capacity or host-policy write occurred. The copy must be retained: a subsequent installed dev build is a different candidate, even within version 4.6.66. Exact source/build mapping and corresponding upstream CI are unknown; the digest identifies observed bytes, not a guessed Git revision.

Failure before: the retained `f8b04708` adapter's read-only `service capacity` reports `Saved host policy: unknown (host.json: claim must be any or labelled)`. Although its process exits 0, its unknown policy result is a failed compatibility check. Pass after: the selected exact copy's same command parses the unchanged policy and reports `Claim: spillover`. This qualifies current policy parsing only, not capacity availability, protected launch or native enforcement. Both adapters' `service permissions` can read grant revision 34; that command alone does not catch the host-policy defect.

The refreshed contract explicitly permits compiling this selected installed digest for bounded non-generating qualification before native acceptance. The compiled anchor has no input/environment override or old-digest fallback. Original operations and prior adapter receipts require their original retained binaries, not a rewritten recovery pin. AgentQ runtime, method versions, prior output provenance and the accepted crate remain unchanged.

Native qualification remains blocked in this worker context: the selected adapter's `service app-server --action prepare --request '{}'` refuses `a worker cannot launch or control external app servers` before request parsing. The worker did not unset `TASKQ_ATTEMPT` or use another caller's slot. No slot, native operation, model turn or paid effect was created. This is an explicit host-authority boundary, not native qualification success. The existing operator-controlled handoff is needed for the next bounded session; the worker retains responsibility for recording its outcome.

The current host protocol requires a pre-launch preparation token. The updated [composed recipe](protected-consumer.md) retains that token in the existing nested request and preserves independent expected-identity comparison. No duplicate policy resolver or native-wire implementation was added. It still requires native authority, Runtime account/progress, cancellation and original-operation reconciliation evidence. New generation dispatch remains refused; current image-tool/output evidence remains unknown. Task #4 is incomplete.

Verification: `cargo test --locked --lib image_worker` passes all eight affected checks. The frozen implementation's `python3 scripts/check.py` passes on Mac in 24.86 seconds including setup: four checker checks, pinned contract lint/whitespace/formatting, 11 core, 26 isolated PostgreSQL service and two trace checks. No test or guarding assertion was changed. The rebuilt AssetQ debug executable is SHA-256 `6182ceb6320f19e0c57d616d501812cf80a4b65ae4227b2108e9e4a8e3fd9122`.

The bounded no-slot `image-qualify` smoke check initially fails the new-anchor comparison against the retained prior AssetQ executable (`f8b04708`). The rebuilt executable passes that comparison with full digest `bd6dc0de5024ff7b6b0ae0a987f5467aa0c5196b005c7cdfd434d043709c7e42` and reports `awaiting-existing-slot`. Both commands then exit 1 on closed input before any protected launch marker or host request exists. This verifies the compiled handoff identity only. No slot or native operation was created; no original operation was touched or released. Current account/progress, image-tool/output path and whole-operation completion remain unknown. New merge-result CI is not claimed.

## Historical qualified adapter

AssetQ #4 promotes one qualified Mac adapter under R1/R3-R5/R8. [PM native qualification note 6562927357](https://github.com/taskqorg/assetq/issues/4#issuecomment-6562927357) identifies TaskQ `e990d88943e95abd8d182d8157bfe84a94343a07` and final signed artifact SHA-256 `f8b04708db09c764ec14bbaa77d0dd40fd7bfc9a521104f93f20885ebb7d5df4`. This is adapter qualification; refreshed AssetQ composition and current image-tool/output capability remain unqualified.

Source direction: [PM note 6562586802](https://github.com/taskqorg/assetq/issues/4#issuecomment-6562586802): "select an appropriate reviewed/merged current TaskQ adapter source that can read the existing policy" and "update the compiled adapter anchor only in the same reviewed deliverable once qualified."

Promotion direction: note 6562927357 says, "Promote this qualified compiled adapter anchor in your existing reviewed deliverable; preserve historical anchor/evidence and current runtime193 fix."

## Failure and preserved state

PM built exact AssetQ `1111fe6b66fbdcfb2eaec1a96db65f97dd4be2e2` on Mac; binary SHA-256 `7f715332980c2b170e8a3c37a15b6a56cdedcfdcc86559f0d22686ab97c87554`. Build passed, but the old staged TaskQ adapter reported `policy unknown: unknown grant antigravity workspace` before durable launch intent or native creation. The installed current TaskQ read the same policy revision 33 and unchanged AssetQ Codex project Workspace grant. PM verified no launch intent/native identity and acknowledged original-slot release. No handshake, model/paid call or crate replay occurred.

The old artifact remains SHA-256 `de4c406b9a8378623ee113d634f9f6808a5cc2ba8ab5ae23db77c0b5bd25fa0a`, built from TaskQ `56b1a1eba274bf88520e4d668461372fe3048b00`. Preserve it, its receipts and its original AssetQ binary for original-operation recovery. The refreshed build compiles only the qualified digest above. `image-observe` also verifies that compiled digest: old qualification directories require their preserved original binaries, never a new-build fallback or replay. AgentQ's corrected runtime pin `8427bba6145aac0239cd16a8dae0caf8efa00695` is unchanged.

## Reported native qualification

PM's exact staged artifact passed the existing-current-policy read with no grant/settings changes. Its scoped receipt reports real launch with independently trusted effective authority and native executable identity, `initialize`/`initialized`, named kernel control denials, parent/child workspace file operations and loopback, and outside write/create/unlink/rename/symlink denial with unchanged canaries. Forged-policy refusal, consumed-operation no replay, standalone interruption with exit 137, native/descendant termination and acknowledged original-slot release passed. Model/paid turns: 0.

The Windows worker has not inspected the private artifact path or raw receipts; the linked PM record supplies the native evidence and explicit promotion direction. No staged path, key or raw authority payload is copied here. The adapter observation does not prove current image generation, an AssetQ handshake or durable slot integration. Use these exact retained signed bytes for the next composed run; a rebuild is another candidate until separately identified and qualified.

## Selected immutable source and CI

- Repository: `https://github.com/taskqorg/taskq`.
- Candidate: `e990d88943e95abd8d182d8157bfe84a94343a07`, current published main at inspection, merged [PR2146](https://github.com/taskqorg/taskq/pull/2146), including the Antigravity support from [PR2141](https://github.com/taskqorg/taskq/pull/2141).
- Git tree: `e1f834c5d0a301fc42b934cd5da5f7ab2308b7e1`.
- Exact merge-result [tests run 41338883682](https://github.com/taskqorg/taskq/actions/runs/41338883682): `merge_group`, `headSha` equals the candidate, conclusion `success`. Plan, shots, Windows Rust, Ubuntu Rust, Windows GNU, macOS Rust and aggregate tests all passed.
- Package version remains `4.6.58`; that version or the published `v4.6.58` download does not identify this candidate. Use the full source revision and final signed-artifact digest.
- No downloadable Mac binary artifact was exposed by that test run or its main-cache run `41339550214`. CI success proves the tested source, not a locally staged executable's identity or boundary. No Windows cross-build is claimed as a Mac build.

## Exact source comparison

Compare the old reviewed source directly with the selected merge, not a GitHub merge-base comparison. The whole binary changes more broadly: 68 tracked files, including ordinary runtime/UI work. It is not a one-line adapter rebuild.

| Path | Old to candidate observation |
|---|---|
| `src/runtime/protected_app_server.rs` | Identical Git blob `ba5257d10607afb4e68eb9f2025c2c6fe9d23a5c`; protocol, equality guards, launch intent, native ownership, observe/cancel and no-replay remain unchanged |
| `src/external_slots.rs` | Identical blob `ca1662be0cb932d63d57ce37fd2535db2c717dcb` |
| `src/runtime/accounts.rs` | Identical blob `1ab4c600c998121225fce41f048d7de4c8a40121` |
| `src/data.rs`, `src/platform.rs`, `build.rs`, `scripts/build-web.cjs`, `package.json`, `package-lock.json` | Identical bytes |
| `src/policy.rs` | `load` is identical and still rejects unknown grants; profile generation now protects all CLI package directories and Antigravity settings/hooks |
| `src/runtime.rs`, `src/service.rs` | Shared runtime list adds `antigravity`; `service` still exports that list. User-bin lookup can now resolve `agy`. Codex remains explicit in protected resolution |
| `src/resources.rs` | Replaces its fixed three-entry runtime-list binding with the shared runtime list; no protected-slot protocol change |
| `src/runtime/owner.rs` | Refuses transcript access for an unqualified non-Codex/non-remote runtime; Codex transcript branch remains |
| `Cargo.toml`, `Cargo.lock` | AgentQ source changes from `513193c8a96f3de1c71a86291a679230817ce0b2` to `9be735b5a82f5a526cd8b1757196387956c3b5d0`; CoreQ stays `v0.18.0`. This is TaskQ's dependency, not a change to AssetQ's runtime pin |

`policy::load` checks every saved grant against `service::RUNTIMES`. The old runtime list contains only Claude, Codex and Codex App; the candidate adds Antigravity. This identifies why the old binary refuses the observed mixed-runtime policy. No policy parser is copied or weakened. The later PM receipt above reports the successful actual-policy read for this exact staged artifact; source comparison alone did not prove it.

The changed profile also applies to Codex: it adds deny rules, not Antigravity writable homes. The expected profile must be independently derived from the candidate policy/profile append source, actual staged paths, native executable and current host environment. Reusing the old profile digest, or copying a digest from an operation receipt, is invalid.

Reproduce the comparison in a verified checkout containing both commits:

```sh
git diff --quiet 56b1a1eba274bf88520e4d668461372fe3048b00 e990d88943e95abd8d182d8157bfe84a94343a07 -- src/runtime/protected_app_server.rs src/external_slots.rs src/runtime/accounts.rs src/data.rs src/platform.rs build.rs scripts/build-web.cjs package.json package-lock.json
git diff 56b1a1eba274bf88520e4d668461372fe3048b00 e990d88943e95abd8d182d8157bfe84a94343a07 -- src/policy.rs src/runtime.rs src/service.rs src/resources.rs src/runtime/owner.rs Cargo.toml Cargo.lock
```

The first command passed locally. Read-only Astra review found no blocking source/API incompatibility; neither observation proves native execution.

## PM build, staging and digest

This reproduces the source/build preparation; PM has completed staging and adapter qualification for the recorded digest above. Do not rebuild or re-sign it merely to repeat composed qualification. A later rebuild must retain its own actual digest and qualification rather than borrowing this receipt.

Use the existing approved Mac Workspace channel and an isolated new build directory. Preserve installed TaskQ, the running host service, grants, accounts, models, capacity and all active operations. Do not run an installer or `dev-install.sh`. Keep private paths, keys and raw receipts in restricted PM evidence.

From a new directory inside the approved workspace, prepare the exact checkout:

```sh
set -eu
test ! -e taskq-adapter-e990d889
git init taskq-adapter-e990d889
cd taskq-adapter-e990d889
git remote add origin https://github.com/taskqorg/taskq
git fetch --depth=1 --no-tags origin e990d88943e95abd8d182d8157bfe84a94343a07
git checkout --detach e990d88943e95abd8d182d8157bfe84a94343a07
test "$(git rev-parse HEAD)" = e990d88943e95abd8d182d8157bfe84a94343a07
test -z "$(git status --porcelain)"
npm ci --ignore-scripts --no-audit --no-fund
npm run build
cargo build --release --locked --bin taskq
```

The source uses Node during `build.rs`; npm setup cannot be omitted. Its Mac Cargo runner is `scripts/macos-runner.sh`, which calls the existing `sign-local` mechanism before execution. Have PM sign and stage the built binary through that existing mechanism, as for the previous candidate. Record toolchain, architecture, lockfile hash, exact signing/staging action and resulting binary revision. No artifact identity is inferred from a version string or source equality alone.

Hash the **final staged bytes after all signing/copying** with `shasum -a 256` and independently read the same bytes back. Retain that receipt privately with the source/build identity. A hash of the unsigned build, source tree, archive or an operation receipt is not the adapter trust anchor. Do not publish a guessed Mac digest.

## Qualification and promotion order

PM's linked receipt completes steps 1 and 2 for the recorded artifact. This candidate performs step 3 by changing the compiled constant, without any runtime/input override. Step 4 remains open.

1. PM first qualifies the staged TaskQ adapter through the existing operator-controlled adapter driver and existing authority, without using an AssetQ digest override. Read-only `service permissions` from the registered AssetQ root must parse the current policy and report the existing Codex project Workspace grant; do not supply `--runtime`, `--mode` or `--scope`, which can write grants. Confirm the revision/account/approver/executable and independently derived profile match current host state.
2. Reuse the existing bounded non-generating TaskQ adapter qualification and owned slot protocol. Retain authority, handshake, filesystem/network/child and control/outside write/create/delete/rename/alias denial evidence, interruption, exact-incarnation reconciliation and acknowledged original-slot release. Unknown, refused or incomplete evidence leaves the adapter unqualified. Neither stage may start a model/image turn or replay an old operation.
3. Return the final signed-artifact digest and exact native receipt scope through PM's existing channel. Only after qualification, update AssetQ's compiled `ADAPTER_SHA256` in this same reviewed deliverable and preserve the old anchor as evidence. No environment variable, input field or temporary validator override can substitute for that reviewed change. Retain existing registry versions and historical operation provenance.
4. Build the exact refreshed AssetQ candidate and repeat the [composed consumer recipe](protected-consumer.md) with a fresh owned qualification operation and the exact retained signed adapter bytes. Confirm its initial `adapter_sha256` equals the full promoted digest above. Independently derive the current profile and identities again; prior-operation authority is not a match source. Retain current selected image-tool/output-path evidence separately; successful TaskQ adapter checks alone cannot qualify the AssetQ consumer or image generation.

Rollback retains the previous binaries, source pins and original operation receipts; it does not relaunch a turn or modify an operator grant. The failed PM operation above is already reconciled and released, not a reusable qualification slot.

Previous documentation handoff `9585a97bae34c0cb2811fd73ae86c3aa5802b6fa`: pinned contract lint, whitespace, 17 repository links and both shell syntax checks passed; read-only Astra review found no blocking source/runbook findings. It reused the unchanged implementation receipt from `1111fe6b66fbdcfb2eaec1a96db65f97dd4be2e2` (Windows canonical gate, 330.70 seconds). The present anchor promotion changes executable bytes and requires its own affected and canonical check receipt. That receipt belongs to the refreshed task/PR candidate, separately from PM's Mac adapter qualification.

Open: refreshed AssetQ Mac build and composed/current-image-tool qualification, then durable slot integration. New image dispatch remains refused. The original accepted crate, method versions and AssetQ runtime wire correction remain unchanged.
