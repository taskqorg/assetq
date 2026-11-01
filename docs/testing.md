# Service verification

The canonical gate is `python3 scripts/check.py` (R8): pinned contract lint, whitespace and the selected deterministic Rust checks within 600 seconds on `ubuntu-latest`, including setup. Changes containing only Markdown run the document checks; unknown impact runs the Rust core and all isolated service fixtures. Real provider and managed-host qualification remain separate.

## Development

Run only the affected check while editing:

```sh
cargo test --locked --lib store::tests
cargo test --locked --lib provider::tests
cargo test --locked --test traces
python3 scripts/check.py --test admission_idempotency_scope_and_typed_limits
```

The last command starts a disposable PostgreSQL cluster and filters Rust tests by name. Set `ASSETQ_PG_BIN` to a directory containing `initdb`, `pg_ctl`, `pg_dump` and `psql` when discovery does not find them. Run as an ordinary unprivileged user. Fixtures create unique databases and temporary directories; they never use production configuration or credentials. A sandbox refusal is `sandbox-blocked`, never a passing database or service check.

## Protected risks

- R3/R5: project-scoped jobs, attachment/download access, method versions, comparisons and recommendation promotion; caller names never become storage paths.
- R5/R6: manifest/state transaction atomicity, expired and stale fences, event replay, original provider operation adoption and sealed absence before a new dispatch.
- R7: typed constraints, idempotency under concurrency, credit reservations, executor release during waits, metadata admission reservation, full object/temp stores and preservation of existing payloads.
- A7/A8: real PostgreSQL transactions, a killed/restarted service through the CLI, streamed SHA-verified retrieval, genuine SQL dump/restore followed by API checks, and refusal of corrupt/missing restored payloads.

`synthetic.fake` proves service semantics only. It never qualifies real image/3D generation, provider cost, native worker output, public browser authentication or the managed store.
