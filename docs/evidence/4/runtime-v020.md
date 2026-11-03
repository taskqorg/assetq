# Structured runtime adoption

This candidate applies AssetQ R1-R8 to task [#4](https://github.com/taskqorg/assetq/issues/4). It implements the continuation request, retained verbatim: "Done: agentq tag runtime-v0.2.0 -> 70f85434546bd1c303ccdb7b0dd71866421627e2 (Test push run 41620673560 green). Pin agentq-runtime tag runtime-v0.2.0 and migrate A2/R3."

## Published input and consumer changes

Remote tag `runtime-v0.2.0` resolves to `70f85434546bd1c303ccdb7b0dd71866421627e2`. Independent readback of [run 41620673560](https://github.com/taskqorg/agentq/actions/runs/41620673560) returned `event: push`, that exact `headSha` and `conclusion: success`. Cargo.lock retains the full resolved revision. This is upstream publication evidence, not this candidate's merge-result or native qualification.

The prepared image runner now uses the public Runtime trait. Verified, project-owned PNG attachments become `Part::Image`; request text becomes `Part::Text`. `Runtime::account` must report subscription login, and inherited-provider progress must identify an approved subscription route before input. API key, absent/unknown identity and unsupported provider cannot start a turn. No account refresh, credential write, model override or authority fallback is introduced.

Method version 5 uses capability `image_generation` and provider `agentq.image`, preserving the current root contract's capability naming. Frozen fixtures retain versions 2, 3 and 4, including their original providers and runtime pins. Both provider identities remain registered for retained work; neither authorizes new dispatch.

Native progress and runtime checkpoints append to the existing bounded journal. Input and cancellation intent precede their runtime calls. Recovery is observation-only: the fixture proves a recovered run cannot read an account or send another input. Missing native image-tool evidence still cannot produce a bitmap result.

The non-generating [qualification recipe](protected-consumer.md) uses the same protected open and runtime interfaces. Account output is typed login/plan; progress output is an events array. The new runtime makes the old protocol helpers private and refuses config/command probes through `send`; the consumer now reports those refusals instead of copying the protocol. Historical composed boundary receipts retain their original candidate and are not repeated or relabeled as current proof.

## Checks and defect evidence

The focused `image_worker_runtime_inputs_account_and_recovery` check initially failed on Windows with `Cannot create a file when that file already exists. (os error 183)`: successive checkpoints attempted to replace an immutable receipt. After moving checkpoints into the bounded append journal, the same check passed. Receipt replacement was not enabled, and original-operation/no-replay protections remain intact.

The retained-runtime recovery check was extended with the version 4 pin `8427bba6`. It failed before with `worker receipt mismatch; no replay` because changing the current pin had removed that revision from the accepted historical set. The explicit historical allowlist now retains versions 2-4 while unknown pins and duplicate reservations still refuse. The same focused recovery check passed after the fix.

The Runtime fixture exercises subscription and API-key/absent/unknown identity, inherited provider refusal, exact text/image forwarding once, terminal-before-ack, interruption, observation-only recovery, absent optimizer and unchanged policy/model. It is an invented protocol process with no image tool, credentials or provider effects. Existing PNG/artifact/hash/import tests and protected supervisor-exit traces remain in the canonical gate.

The provider integration check submits versions 2-5 through real isolated PostgreSQL admission and the executor. It verifies both provider identities reach the authority refusal, duplicate requests retain the job, audit replay matches and no worker spool is created. Its first draft used the synthetic fixture's operation and correctly failed `operation_mismatch`; the fixture now explicitly requests `image.generate`. No admission guard changed.

Commands: `cargo test --locked --lib image_worker`, `python3 scripts/check.py --test image_worker`, and `python3 scripts/check.py --test image_worker_provider_names`. The final frozen-candidate canonical command is `python3 scripts/check.py`; exact candidate, outcomes and elapsed time are recorded in the task/PR. Local Windows tooling uses the installed Rust 1.99 compiler with Cargo/rustfmt 1.95 because the stable toolchain lacks cargo-fmt; PostgreSQL is the existing isolated 18.1 fixture. This does not establish Ubuntu timing or native host enforcement.

## Acceptance still open

A1/A2 have the published runtime and consumer migration, but runner-bound slot acquisition, protected preparation/reconciliation and current-source native qualification remain incomplete. A3/current image-tool availability and original artifact path remain unknown. A4 retains explicit refusal on both generation entry points; there is no direct paid API or alternate-worker fallback.

A5 preserves original asset `2c887345-9f66-491f-8a0c-81d81cf0c4af` and SHA-256 `fe2077d15b0c50617b84c6267277d46339c6a8cedbd6334e923cfa5475529baa`; the accepted crate was not accessed, regenerated or replayed in this continuation. Its prior native bitmap and managed-import receipts remain separate from the new runtime's qualification.

A6's deterministic consumer checks do not substitute for its required bounded real worker evidence. The previous real run remains historical. No native/model/paid turn was submitted by this continuation. New dispatch stays refused, supervisor/native completion and capacity release remain unknown, and backup work remains deferred. Task #4 is incomplete.
