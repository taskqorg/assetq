# Corrected Mac protected transport: preflight

[Pipe #61](https://github.com/taskqorg/assetq/issues/61) supplies the native proof that [#52](https://github.com/taskqorg/assetq/issues/52) A4 needs for the adapter reviewed in [AgentQ #266](https://github.com/taskqorg/agentq/blob/a610cf7680b1ecc0d5b51847a0ea03eced077ded/docs/evidence/runtime-266.md). Pipe R3-R6/R10 and the #42 decision apply.

**Status: stopped before any effect.** This record covers A1 and A2 only. A3-A5 have no observation, so native transport stays unknown. The stop follows A2 and A3: the host does not trust this adapter yet, and no action-time authorization exists.

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

## A2: independent host readback

All reads ran on the Mac against the fixed host store, `.codex/agentq-host` under the account home. They changed nothing. Private values were compared locally and are not published.

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

Whether the host admits a second operation next to the retained one is unknown. Reading the published source showed no host capacity refusal, and only a native run proves it. Pipe capacity admission stays #47 work.

## Effects not performed

No trust edit, policy seal, permit, preparation, launch intent, launch, cancellation, reconciliation, model turn, input or image generation ran in this attempt. No file in the host store changed.

## Required operator action

The worker cannot supply these. A task note, the task start and historical revision 34 are not action-time authorization. The worker runs under seatbelt with `sandbox_apply: Operation not permitted`, so a nested sandbox is `sandbox-blocked` here; whether a launchd-started scope would work from this worker is unknown.

In the existing Mac operator session, with explicit authorization at that moment:

1. Copy the current `trust.json` beside the store, unchanged, and read its digest back as `e0a2e34d...`. Do not delete the old adapter.
2. Stage the exact candidate in a new directory under the store's `stage`, read back its size and digest, then pin the public manifest above in `trust.json`. Use no new key, grant, policy or re-signing.
3. Bind one new operation, op-id above, with one new `pipe.permits/1` permit from the existing issuer to the live consumer process. Persist preparation and one-shot intent in a new private directory, separate from the retained `assetq-52` one.
4. Verify the effective native boundary and initialize transport with no model turn, input replay or image generation. Report a TCC denial as Open.
5. Keep transport through cancellation readback. Recover the same checkpoint and reconcile without relaunch. Record native and descendant completion and the exact-generation cleanup acknowledgment. Missing proof retains occupancy.
6. Return safe outcomes only: identities, boundary readback, the final operation state and the journal size and digest. The journal grows by this one operation; the retained entry must stay byte-equal.

**Known effect of step 2.** `trust.json` holds one pin, and each adapter checks it against its own digest. While the new pin is active, the old adapter refuses the retained operation with `adapter not pinned by host operator`. The retained operation stays occupied and unknown either way. Recovering it through its original adapter needs the preserved copy put back, and that in turn stops the new adapter. Only one of the two can run at a time.

## Unknowns

- Native transport, TCC, boundary enforcement, cancellation, recovery and cleanup for this adapter: A3-A5 have not run.
- Completion of the retained failed operation stays unknown, and the new operation cannot release or replace it.
- Merge-result CI for this record has not run.
