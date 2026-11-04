# Service verification

The canonical gate is `python3 scripts/check.py` (R8): pinned contract lint, whitespace and the selected deterministic Rust checks within 600 seconds on `ubuntu-latest`, including setup. Changes containing only Markdown run the document checks; unknown impact runs the Rust core and all isolated service fixtures. Real provider and managed-host qualification remain separate.

Select changes across the candidate branch from its merge base with `origin/main`, including staged, unstaged and untracked files. On the published main revision, compare its first parent. A missing comparison base has unknown impact and runs implementation checks. A final documentation commit cannot hide earlier implementation or dependency changes in the same candidate.

## Development

Run only the affected check while editing:

```sh
cargo test --locked --lib store::tests
cargo test --locked --lib provider::tests
cargo test --locked --test traces
python3 scripts/check.py --test meshy
python3 scripts/check.py --test admission_idempotency_scope_and_typed_limits
python3 scripts/check.py --test resource_bounds
```

The last command starts a disposable PostgreSQL cluster and filters Rust tests by name. Set `ASSETQ_PG_BIN` to a directory containing `initdb`, `pg_ctl`, `pg_dump` and `psql` when discovery does not find them. Run as an ordinary unprivileged user. Fixtures create unique databases and temporary directories; they never use production configuration or credentials. A sandbox refusal is `sandbox-blocked`, never a passing database or service check.

Windows fixtures accept the same binaries with `.exe` suffixes and use a loopback-only temporary port instead of a Unix socket. Image worker development uses `cargo test --locked --lib image_worker` and `python3 scripts/check.py --test image_worker`; the filtered gate includes `python3 scripts/test_qualify_image.py` for the bounded operator driver.

Windows runs unrelated Rust fixtures sequentially; concurrency cases retain their explicit competing callers. The slow-import fixture measures a real guarded metadata transaction, bounds its calibrated lease to ten seconds, and keeps both fetch and stream slower than that lease. Its forced-expiry, no-commit, attempts, submission and replay assertions remain unchanged. This calibrates the check; it does not qualify 250 ms production leases on Windows.

The cancellation-order fixture's EOF marker precedes Python process exit. After asserting EOF and cancellation order, directory cleanup waits at most three seconds only for Windows sharing violation 32 to clear. This does not repeat the test or relax its success/work-error/cleanup-error assertions.

## Protected risks

- R3/R5: project-scoped jobs, attachment/download access, method versions, comparisons and recommendation promotion; caller names never become storage paths.
- R5/R6: manifest/state transaction atomicity, expired and stale fences (including refusal to revive an expired lease during a slow import), renewal through blocking fetch/stream work, event replay, original provider operation adoption and sealed absence before a new dispatch.
- R4/R6 import refusal: a completed operation with an oversize PNG, an invalid PNG, an output above the size limit or an impossible output count ends `failed` with one submission, a retained reason and released reservations; a poll error without a completion report stays `waiting` with its reservations.
- R7: typed constraints, idempotency under concurrency, credit reservations, executor release during waits, uncertain-cancel admission reservations until terminal provider observation, metadata admission reservation, full object/temp stores and preservation of existing payloads.
- R7 resource bounds: native configured floor refusal, changing per-chunk space/unknown observations, process file-lock exclusion, real PostgreSQL TOAST/index growth under concurrent admission, reserved cancellation/reconciliation capacity, exhaustion/restart recovery and bounded migrations. Simulated free-space changes test the loop; actual managed filesystem qualification remains separate.
- A7/A8: real PostgreSQL transactions, a killed/restarted service through the CLI, streamed SHA-verified retrieval, genuine SQL dump/restore followed by API checks, and refusal of corrupt/missing restored payloads.

`synthetic.fake` proves service semantics only. It never qualifies real image/3D generation, provider cost, native worker output, public browser authentication or the managed store.

Meshy checks use a local HTTP fixture, counted-store receipts and isolated PostgreSQL; they protect HTTP 200/201/202 ID persistence, the single-POST barrier, lost reply/receipt recovery, retained completion without credentials, unknown spend/changed or insufficient balance refusal, non-destructive cancellation and retained originals after failed validation. Reconciled testing beyond the former cumulative ceiling is allowed. The canonical gate also runs `scripts/test_validate_meshy.py` for connectivity/world dimensions, UV/PBR presence, texture bounds and nonfinite geometry. Its invented scene does not qualify native Blender. The live operator run, memory-boundary readback, actual Meshy charge and textured views remain separate.
