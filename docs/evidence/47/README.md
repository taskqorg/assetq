# Remove TaskQ from the protected image worker

[Pipe #47](https://github.com/taskqorg/assetq/issues/47) owns this record. Its goal is: “Pipe launches and recovers protected workers without TaskQ: the signed adapter and taskq.slots link leave src/image_worker/protected.rs, replaced by a shared AgentQ or CoreQ layer, with recovery guarantees kept.” Pipe R3, R4, R6, R7 and R10 apply.

**Status: implemented and tested with fixtures. Not deployed. Not live-qualified.** A1, A2 and A3 are met by this change. A4 is partly met: capacity comes from Pipe permits, but no live image job has run, so the composed consumer is unproved. See [Open](#open).

## Acceptance

| Item | State | Evidence |
|---|---|---|
| A1 | Met | `pipe.md` R10 states the rule; the temporary adapter exception and its digest are gone |
| A2 | Met | `scripts/check.py` fails on a call or pin in tracked files other than Markdown; `scripts/test_check.py` plants each kind |
| A3 | Met with fixtures | `tests/traces/capacity` replays launch, cancellation and crash recovery; job traces under `tests/traces` are unchanged and still replay |
| A4 | Partly met | Permits are implemented and verified against the host's own signature check. The live job needs [the operator](#operator-action-for-a4) |

## What counts as a TaskQ link

The check catches slot and app-server schemas, the `service slots` and `service app-server` commands, `ProtectedHost::new`, a `taskq` subprocess, the `taskqorg/taskq` repository and the old adapter digests.

These stay, and none of them calls or pins TaskQ:

- `TASKQ_ATTEMPT` refusals in operator commands, which keep a worker from running them.
- `TASKQ_TASK` in `scripts/pipe_mcp.py`, an opaque label that Pipe records and never checks against a board.
- `taskq.json`, `.gitignore` and the CI branch pattern, which are the board's registration of this repository.
- Spool file names such as `slot-request.json`, read only to recognize operations started through TaskQ.

## Runtime pin differs from the #52 handoff

[#52](../52/README.md#what-47-receives) names `a610cf7680b1ecc0d5b51847a0ea03eced077ded` as the source and API. This change pins the library at `f87f308a90b34564cb8ab6f3686ca09ffdde638f`, the earlier AgentQ #244 publication, and still pins the adapter binary from #52.

At `a610cf7`, `Runtime::progress` returns typed events and `Runtime::send` is gone. Those events carry no native `imageGeneration` item or `savedPath`, which R5 and the method 5 validation recipe require. `native_image` could no longer find the original bitmap, and server requests could no longer be refused through the runtime. Bumping to `a610cf7` failed `cargo check` with `no method named send` and `expected &Value, found &Event`.

`f87f308` has `ProtectedHost::signed` and keeps those messages, on the CoreQ `v0.21.0` Pipe already uses. Between the two revisions, `protected.rs` differs by one comment and two added functions Pipe does not call. So the library's host requests have the same form. This is read from source: #61 proved the adapter with a harness built on `a610cf7`, so the pairing of this library with that adapter is unproved until the live run.

A later runtime upgrade needs AgentQ to expose native image evidence on the protected path. This change opens no upstream task for it.

## Checks

On macOS 27.2 arm64 at this branch:

| Check | Outcome |
|---|---|
| `python3 scripts/check.py` | Passed locally; CI runs the same gate on `ubuntu-latest` |
| `cargo test --locked --lib image_worker` | 22 passed |
| `cargo test --locked --test traces` | 4 passed, including 8 capacity traces and a wrong-expectation check |
| Permit signature | Accepted by `agentq_runtime::agent::host::verify_signature`; refused for a changed binding and a foreign key |

The signed adapter was not executed. Permits used invented keys. No `.codex/agentq-host` file, grant, policy, journal, account or model setting was read or changed, and pds18 was not touched.

## Open

Each item needs host or owner authority, or an upstream change. None is done here.

- **Live image job (A4).** Native run, restart and recovery on the Mac have not happened.
- **Native workload enforcement and live account login.** Still unproved, as #61 records.
- **Operation `op-7988f129136b…`, the failed `425c5924` launch.** It stays occupied with completion unknown in the host journal. It is outside Pipe's spools. Its preparation names the old adapter and the independent schema, so neither the pinned adapter's `reconcile` nor the retained route (which accepts only TaskQ-era checkpoints) can complete it. It needs an AgentQ decision, not a Pipe one.
- **Operation `pipe-52-corrected-mac-transport-a1cbb7b8`.** Its record reads `occupied: true` and has no spool here. The host cannot release it, and it does not hold Pipe capacity, because Pipe's permit file starts free on the host.
- **TaskQ-era spools with an unacknowledged slot.** They stay unknown and charged by design. Whether to retire any of them is an owner decision.

## Operator action for A4

Nothing below has run. It needs action-time authorization from the owner, on the Mac, in the existing operator session.

1. Build Pipe from the merged source. Stage the adapter already verified in #61 and point `--adapter` at it.
2. Set `ASSETQ_PERMIT_ISSUER_KEY` to the existing issuer key for caller `assetq-52-qualification`. The policy and grant need no edit.
3. Run `python3 scripts/qualify_image.py --binary <pipe> --adapter <adapter> --workspace <project> --directory <new-dir>` once. It creates an isolated store, submits the one blue-wedge job, restarts the service and retrieves the outputs. Never reuse the directory.
4. For an operation started through TaskQ that needs recovery, set `ASSETQ_PROTECTED_ADAPTER` and seal its exact `preparation.json` and receipt bytes as a retained archive first.

Keep raw permits, checkpoints and journals in protected storage. Publish identities, outcomes and safe references only.
