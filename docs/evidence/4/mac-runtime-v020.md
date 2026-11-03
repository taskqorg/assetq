# Runtime 0.2.0 Mac qualification preflight

Task [#4](https://github.com/taskqorg/assetq/issues/4), attempt 6602634968; AssetQ R3/R4/R8. This records a blocked qualification, not native success or task completion. The steering request is preserved verbatim: "Not a PM action: the worker produces qualification receipts. Moved to host-mac; run the a5e70c3 Runtime consumer qualification there yourself under the existing Mac authority and record account/progress, tool/path and slot reconciliation receipts."

## Exact candidate and observations

Source `a5e70c3011f622562cc775274a035461186ec724` builds on Mac with `cargo build --locked --bin assetq`: PASS in 55.67 seconds. The resulting debug executable has SHA-256 `a7ad2d1c9d41a12cdbc2bf846c8cc84263bb9bae8029f6921749532ceef5f806`. AgentQ remains `runtime-v0.2.0`, resolved to `70f85434546bd1c303ccdb7b0dd71866421627e2`. No implementation, dependency, adapter anchor or ordinary installation changed.

Fresh installed TaskQ `4.6.66-261103-25` permission readback reports revision 34, an existing project Codex Workspace grant by the owner, and effective TaskQ Seatbelt enforcement. This is preflight authority evidence only; it does not qualify a new native incarnation or supply a runtime slot.

The exact executable's `image-qualify` command ran from the registered project root with a new private receipt directory and closed stdin. It emitted `awaiting-existing-slot`, the real consumer identity and the expected compiled adapter digest `f8b04708db09c764ec14bbaa77d0dd40fd7bfc9a521104f93f20885ebb7d5df4`. It then exited 1 with `qualification input ended/deadline; retain original slot`. The create-once protected-launch marker and host request are absent. No external slot or native operation was created. Private consumer/preflight receipts remain outside tracked evidence.

The installed host command `taskq service slots --action reserve --request '{}' --json` exits 1 with `a worker cannot reserve or release external slots`. The request is intentionally empty: the worker guard runs before request parsing or store mutation. The pinned TaskQ source's `src/external_slots.rs::exchange` independently confirms that `TASKQ_ATTEMPT` rejects reserve, observe and release before parsing. This is an authority refusal, not a missing permission to remove that variable, impersonate an operator or reuse another operation's slot.

## Required host boundary

The existing [consumer recipe](protected-consumer.md#existing-pm-channel-and-bounded-command) requires an operator-controlled slot bound to the actual live consumer, plus independently read expected identities. The current AssetQ contract also explicitly assigns supplying that slot to the existing PM. The worker can build and drive qualification, but its current host channel cannot supply that prerequisite. Moving the same worker to Mac did not remove the host's worker prohibition.

The smallest next step is to make that existing operator-controlled slot channel available to a bounded live qualification consumer. Keep the current grant, account, model, capacity and adapter anchor. The operator supplies capacity authority; the worker retains responsibility for the resulting qualification receipts. Do not broaden the external-slot API for workers, unset role variables, copy a resolver or add another policy store as an implicit fix. If a service-owned handoff is required, its owning TaskQ boundary needs an explicit reviewed contract before implementation.

## Acceptance readback

- A1: the immutable runtime builds; current composed native qualification and durable runner/slot integration remain incomplete.
- A2: account/progress, structured native input and immutable import remain unqualified on this candidate. Historical and deterministic receipts retain their original scope.
- A3: current image-tool availability and its output path remain unknown; no admitted protected native session existed.
- A4: new dispatch remains refused under R3/R4. There was no direct API, alternate worker or authority fallback.
- A5: original asset `2c887345-9f66-491f-8a0c-81d81cf0c4af`, original operation and owner-accepted crate were not accessed, regenerated or replayed.
- A6: no new original slot exists to reconcile. Prior operations were untouched. Native/model/paid effects were zero; image usage remains unobserved. This preflight does not satisfy bounded native lifecycle acceptance.

No healthy historical host probes were repeated. There is no deployment, website, backup or restore claim. Task #4 remains incomplete pending the host channel and the remaining native/durable acceptance.

## Deterministic verification

`python3 scripts/check.py` passed on Mac in 22.82 seconds, including setup: four checker checks, pinned contract lint, whitespace/formatting, 11 core checks, 26 isolated PostgreSQL service checks and two trace checks. The candidate contains the unchanged `a5e70c3` implementation plus this Markdown evidence/link. No full-gate rebase or repeated historical native proof was performed. These checks protect authority refusal, structured inputs/account guards, original-operation recovery, interruption, absent optimization, PNG import/retrieval and resource limits; they do not supply the missing native receipts. Merge-result CI remains a separate publication gate.
