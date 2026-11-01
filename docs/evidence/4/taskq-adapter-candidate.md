# TaskQ adapter refresh candidate

Prepare one isolated Mac adapter candidate for AssetQ #4 under R1/R3-R5/R8. This is a source/build handoff, not native qualification or a replacement trust anchor. The signed staged Mac binary and its SHA-256 remain **unknown pending PM build and qualification**.

Source direction: [PM note 6562586802](https://github.com/taskqorg/assetq/issues/4#issuecomment-6562586802): "select an appropriate reviewed/merged current TaskQ adapter source that can read the existing policy" and "update the compiled adapter anchor only in the same reviewed deliverable once qualified."

## Failure and preserved state

PM built exact AssetQ `1111fe6b66fbdcfb2eaec1a96db65f97dd4be2e2` on Mac; binary SHA-256 `7f715332980c2b170e8a3c37a15b6a56cdedcfdcc86559f0d22686ab97c87554`. Build passed, but the old staged TaskQ adapter reported `policy unknown: unknown grant antigravity workspace` before durable launch intent or native creation. The installed current TaskQ read the same policy revision 33 and unchanged AssetQ Codex project Workspace grant. PM verified no launch intent/native identity and acknowledged original-slot release. No handshake, model/paid call or crate replay occurred.

The old artifact remains SHA-256 `de4c406b9a8378623ee113d634f9f6808a5cc2ba8ab5ae23db77c0b5bd25fa0a`, built from TaskQ `56b1a1eba274bf88520e4d668461372fe3048b00`. Preserve it and its receipts for original-operation recovery. AssetQ still compiles this digest; a newly built adapter cannot pass it. AgentQ's corrected runtime pin `8427bba6145aac0239cd16a8dae0caf8efa00695` is unchanged.

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

`policy::load` checks every saved grant against `service::RUNTIMES`. The old runtime list contains only Claude, Codex and Codex App; the candidate adds Antigravity. This identifies why the old binary refuses the observed mixed-runtime policy. No policy parser is copied or weakened. Source compatibility is established; a successful read of the actual host policy by this exact staged candidate is still required.

The changed profile also applies to Codex: it adds deny rules, not Antigravity writable homes. The expected profile must be independently derived from the candidate policy/profile append source, actual staged paths, native executable and current host environment. Reusing the old profile digest, or copying a digest from an operation receipt, is invalid.

Reproduce the comparison in a verified checkout containing both commits:

```sh
git diff --quiet 56b1a1eba274bf88520e4d668461372fe3048b00 e990d88943e95abd8d182d8157bfe84a94343a07 -- src/runtime/protected_app_server.rs src/external_slots.rs src/runtime/accounts.rs src/data.rs src/platform.rs build.rs scripts/build-web.cjs package.json package-lock.json
git diff 56b1a1eba274bf88520e4d668461372fe3048b00 e990d88943e95abd8d182d8157bfe84a94343a07 -- src/policy.rs src/runtime.rs src/service.rs src/resources.rs src/runtime/owner.rs Cargo.toml Cargo.lock
```

The first command passed locally. Read-only Astra review found no blocking source/API incompatibility; neither observation proves native execution.

## PM build, staging and digest

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

1. PM first qualifies the staged TaskQ adapter through the existing operator-controlled adapter driver and existing authority, without using an AssetQ digest override. Read-only `service permissions` from the registered AssetQ root must parse the current policy and report the existing Codex project Workspace grant; do not supply `--runtime`, `--mode` or `--scope`, which can write grants. Confirm the revision/account/approver/executable and independently derived profile match current host state.
2. Reuse the existing bounded non-generating TaskQ adapter qualification and owned slot protocol. Retain authority, handshake, filesystem/network/child and control/outside write/create/delete/rename/alias denial evidence, interruption, exact-incarnation reconciliation and acknowledged original-slot release. Unknown, refused or incomplete evidence leaves the adapter unqualified. Neither stage may start a model/image turn or replay an old operation.
3. Return the final signed-artifact digest and exact native receipt scope through PM's existing channel. Only after qualification, update AssetQ's compiled `ADAPTER_SHA256` in this same reviewed deliverable and preserve the old anchor as evidence. No environment variable, input field or temporary validator override can substitute for that reviewed change. Retain existing registry versions and historical operation provenance.
4. Build that exact AssetQ candidate and repeat the [composed consumer recipe](protected-consumer.md) with a fresh owned qualification operation. The current compiled old digest must refuse the replacement until step 3. Retain current selected image-tool/output-path evidence separately; successful TaskQ adapter checks alone cannot qualify the AssetQ consumer or image generation.

Rollback retains the previous binaries, source pins and original operation receipts; it does not relaunch a turn or modify an operator grant. The failed PM operation above is already reconciled and released, not a reusable qualification slot.

Local verification: pinned contract lint and whitespace checks pass; all 17 repository links in the changed documents resolve; both shell blocks pass `bash -n`. Read-only Astra review found no blocking source or runbook findings. This continuation changes documentation only: implementation, tests, scripts and dependency locks match `1111fe6b66fbdcfb2eaec1a96db65f97dd4be2e2`, whose Windows canonical gate passed in 330.70 seconds. That unchanged-code receipt is retained, not reported as a new Mac build or native pass. No full implementation suite was repeated for this documentation handoff.

Open: actual Mac build/sign/staged digest, exact-candidate current-policy read and native adapter qualification; then reviewed anchor promotion, composed/current-image-tool qualification and durable slot integration. New image dispatch remains refused. The original accepted crate and AssetQ's runtime wire correction remain unchanged.
