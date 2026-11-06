# Corrected Mac protected transport qualification

[Pipe #61](https://github.com/taskqorg/assetq/issues/61) supplies the native proof that [#52](https://github.com/taskqorg/assetq/issues/52) A4 needs for the adapter reviewed in [AgentQ #266](https://github.com/taskqorg/agentq/blob/a610cf7680b1ecc0d5b51847a0ea03eced077ded/docs/evidence/runtime-266.md). Pipe R3-R6/R10 and the #42 decision apply.

**Status: native non-generating transport and recovery qualified for this adapter on the Mac.** One new operation initialized, was cancelled and reconciled to `completed` without relaunch, with no model turn or image generation. This supplies #52 A4's native evidence only. Native workload enforcement, live account login and Pipe integration stay unproved; see [Unknowns](#unknowns).

Two kinds of observation appear below, and they are not interchangeable:

- **Operator run.** The owner's existing Mac operator session did the provisioning and the launch. Its [answer](https://github.com/taskqorg/assetq/issues/61) on this task is the action-time authorization record.
- **Worker readback.** This Mac worker then read the retained private receipts, journal and store directly, read-only. It ran no launch and changed nothing.

## Candidate

| Item | Value |
|---|---|
| Op-id | `pipe-52-corrected-mac-transport-a1cbb7b8` |
| Artifact | `13509232169` (`macos-aarch64`) of run [42231396039](https://github.com/taskqorg/agentq/actions/runs/42231396039) |
| Binary | `agentq-0.0.0-dev-macos-aarch64`, 7,675,376 bytes |
| SHA-256 | `a1cbb7b88959532f2be28ea8d696c4cc1ae17b7c97205d6549f7156164c82a58` |
| Manifest | [`adapter-macos-aarch64.json`](https://github.com/taskqorg/agentq/blob/a610cf7680b1ecc0d5b51847a0ea03eced077ded/docs/evidence/runtime-266/adapter-macos-aarch64.json), Git blob `c35460a4c25bee32ed77165469771fedde1f76a0` |
| Pinned key | `7f739abab8625718e2e5dbedf0deb5a98373260d3c22ecd20a1148f42bf6d584`, the key #52 recorded |

## A1: staged bytes and signature

This Mac worker downloaded the artifact into a new private temporary directory, outside the host store, and did not execute it or rebuild it.

```sh
gh run download 42231396039 --repo taskqorg/agentq --name macos-aarch64 --dir <new-private-dir>
stat -f '%z %N' <binary>
shasum -a 256 <binary>
git hash-object <manifest>
```

Size, digest and manifest blob match the table above. Node's built-in Ed25519 verified the manifest over `agentq.protected-adapter/1`, NUL and the lowercase digest text, using the key above from #52 rather than the key in the manifest.

| Check | Outcome |
|---|---|
| Published signature | valid |
| Zeroed signature | refused |
| Changed digest | refused |

These checks prove bytes and the detached signature. They do not prove macOS codesigning, a TCC grant, installed trust or native containment.

## A2: host readback before any effect

All reads ran before the operator run, on the Mac against the fixed host store, `.codex/agentq-host` under the account home. They changed nothing. Private values were compared locally and are not published.

| Authority | Observation | Effect |
|---|---|---|
| Host trust | `trust.json` (SHA-256 `e0a2e34de13e495badf75980ebd8eb0d2ae2053bc86d131836da22c6e4458803`) validly pins the **old** adapter `425c5924046338bf13167e5bf76eb827533435ed09e5f3cf336b6744d9bf2118` | **Blocks.** The host source refuses the new binary with `adapter not pinned by host operator` |
| Signed policy | The envelope verifies under the pinned key. Its authority is project Workspace, revision 34, approver `kirs`, effective `workspace`, nothing refused | Usable; it pins no adapter digest, so it needs no new seal |
| Current grant | `taskq service permissions`: policy revision 34, Codex Workspace by `kirs` for this project | Matches the signed policy. Read-only; no slot, adapter or CA use |
| Native identity | The executable digest and the profile digest match the signed policy | Usable |
| Account | The account home exists under the native home and has its configuration file. Credential files were not read | Usable. The live account login is unknown until a launch reads it |
| Caller permit | The issuer `assetq-52-qualification` is authorized by the signed policy. The old permit's signature is valid; it has expired and its caller process is gone | The old permit stays retained and cannot be reused. A new operation needs a new permit from this existing issuer, with no policy change |
| Journal | `operations.json` is 9,862 bytes, SHA-256 `ab4281c3aa8971a1ab965d68495364c8dcc5c9410de2dbe29f7884b2b77d96f6`, read again after all other reads. One operation, occupied | Matches the #266 baseline. Occupancy stays retained |
| Old adapter | The staged binary reads back as `425c5924...` | Preserved |

## A3: authorization and binding

The preflight stopped because the host pinned the old adapter and no action-time authorization existed. The [requested action](https://github.com/taskqorg/assetq/blob/7b0ddde3001600c2713f9b062df79fd7dc029540/docs/evidence/61/README.md#required-operator-action) went to the owner, who ran it through the existing operator session.

| Item | Worker readback |
|---|---|
| Host trust | `trust.json` now has SHA-256 `f47f400ca718c9ba7001dc76ee19804dc64b943ba339d930e97892e9fe785fe5` and equals the published manifest field for field |
| Old trust copy | Kept beside the store; its digest reads `e0a2e34d...` |
| Staged candidate | 7,675,376 bytes, SHA-256 `a1cbb7b8...`, in its own stage directory |
| Signing and grants | The policy file and its issuer are unchanged. Current grant readback is still revision 34 by `kirs` |
| Operation | `op-92dd349d1fd1a546c7d2b04fe04262d0aa12cb662cf3a7d1c75f738c036fcbce`, the only new journal entry |
| Permit | One new `pipe.permits/1` permit from the existing issuer, for the op-id above. Its binding equals the journal's binding, on the same boot |
| One-shot intent | Persisted with `one_shot: true` and preparation `08fe76706892613965d5cb94f84fbeffae63347e411f891ea9e47890d56e3d45` |

The operator's [harness source](https://github.com/taskqorg/assetq/raw/e495be7f35ee21dc86ea9db762fa71b97198987b/61/6008b334b3fd-assetq61-harness-main.rs) has SHA-256 `6008b334b3fd60510529909b1f9d1e71368eddeaaf210bf1942308bd20f225a6`. It uses `ProtectedHost::signed`, `prepare`, `launch_prepared`, `recover_preparation`, `observe` and `reconcile`, and calls no TaskQ slot, adapter or CA. Its exclusive-create checks refuse a second launch. Commands, from its `main`:

```sh
a61 qualify-one-shot
a61 recover
```

## A4: boundary and transport

| Check | Retained outcome |
|---|---|
| Identity match before launch | `independent_identities: match` for policy, profile, native digest and adapter digest |
| Effective boundary | `verified-effective`, both before and after initialize |
| Workspace write | Allowed |
| Protected paths | Write, create and unlink denied on all five named paths |
| Reads and network | `existing profile` |
| Native identity | Digest `4d210f7c5a18fd03...`, the same as the signed policy; native version `0.159.3` from the user agent |
| Initialize | Returned `userAgent`, `codexHome`, `platformFamily`, `platformOs` |
| Native messages | `native.jsonl` holds exactly one line, the `initialize` response, id 1 |
| Model turn, thread, input replay, image | None: the harness sends only `initialize` and `initialized` |
| TCC | No denial recorded |

This run passes where the failed `425c5924` launch stopped at `native stdout interrupted` before initialize.

## A5: cancellation, recovery and cleanup

The sequence below comes from the retained `run.jsonl` and `recover.jsonl`. The transport handle stayed open until the last step of the first process.

1. `cancel`: state `unknown`, reason `exact native termination requested; turn/descendant completion unproved; permit retained`.
2. Cancellation readback: `input_closed: true`, with a completion certificate of schema `agentq.native-completion/1`, counts `[20,19]`, for the exact supervisor, native process and scope.
3. `recover_preparation` from the persisted checkpoint: `matches: true`, no launch path.
4. `reconcile`: `completed` on attempt 1, with `bootout_ack: true` and `cleanup_verified: true`.
5. Final readback: `reconcile` and `observe` both `completed`; launchd job no longer loaded.
6. Frontend exit code 1.
7. Fresh process, 17 seconds later: `checkpoint_matches: true`; `observe` and `reconcile` both `completed`, with the same identities.

Worker readback: the launchd list holds no protected job, and the operation's scope directory is gone. The retained operation's scope directory is still there.

`native_ok: false`, frontend exit 1 and the reason `native app-server exited; turn outcome unproved, permit retained` are retained as recorded. The frontend log says `native stream ended without a successful root outcome`. This run started no turn, so no root outcome could exist. `completed` here rests on the certificate and the cleanup acknowledgment, and it says nothing about any turn.

## A6: preservation

| Item | Worker readback |
|---|---|
| Journal | 9,862 bytes `ab4281c3...` before; 19,999 bytes, SHA-256 `b1fbe9216f0bd9000ea56c74601e2294619bb162bd3b8f1feec398c9a2b80554` after |
| Retained entry | `op-7988f129136b...` appears byte-equal inside the new journal, compared with a baseline copy whose digest reads `ab4281c3...` |
| Retained state | Still occupied; its completion stays unknown |
| Retained evidence | Permit, checkpoint, receipts and logs in the `assetq-52` directory keep their earlier sizes and timestamps |
| Old adapter | Reads back as `425c5924...` |
| Accepted crate | Not read. The harness source touches no asset path |

**Known effect still in place.** `trust.json` holds one pin, now the new adapter. From the source, the old adapter refuses the retained operation with `adapter not pinned by host operator` until the kept copy returns, and returning it stops the new adapter. This refusal was not run.

## Unknowns

- Native workload enforcement: the receipt scope itself says `native workload qualification separate`.
- Live account login: no account read and no turn ran, and credential files were not read.
- Whether initialize used the network for model metadata.
- The harness binary: only its source is retained, so its build pin to `a610cf7` is the operator's statement. `ps` is `sandbox-blocked` for this worker, so process absence rests on the certificate and the launchd list.
- The new operation's journal record still reads `occupied: true`. The host cannot release capacity; the Pipe-owned final release is #47 work and did not run.
- Completion of the retained failed operation stays unknown, and the new operation neither released nor replaced it.
- Pipe permits, consumer integration and live image qualification stay #47. Merge-result CI gates publication of this record.
