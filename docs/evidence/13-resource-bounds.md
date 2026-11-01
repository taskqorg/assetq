# Resource admission and recovery evidence

Owner request and acceptance: [task #13](https://github.com/taskqorg/assetq/issues/13), including the original request “Enforce storage free-space and metadata recovery bounds.” Applied rules: AssetQ R1, R4, R5, R6, R7 and R8; specq Testing development policy; TaskQ § 7 publication and evidence. Foundation source: `f1f382e9330554e9c87a844ece3fef61572d80a4` (PR12). This change owns resource guards only; provider integration remains with task #4.

## Failure before the fix

On the foundation implementation, `python3 scripts/check.py --test resource_bounds` ran the first three new regressions against disposable real PostgreSQL fixtures on Linux. All three failed:

- `resource_bounds_floor_refuses_native_upload`: setting the filesystem floor to `i64::MAX` still returned HTTP `201`; expected `507`.
- `resource_bounds_migration_refuses_exhaustion`: `assetq migrate` succeeded with metadata quota zero.
- `resource_bounds_recovery_refuses_exhaustion`: the executor returned `Ok(true)` and created an attempt at metadata quota zero.

These reproduce missing configuration enforcement, migration admission and recovery-write bounds without consuming actual disk capacity or dispatching a real provider.

## Pass after the fix

The same filtered command passes all eight resource checks: seven real PostgreSQL/native service checks and one store component check. The native tests preserve scoped authentication and execute the packaged CLI entrypoints. They cover exhausted fresh/repeat migrations; retained provider IDs during cancellation and recovery; recovery exhaustion without attempt creation; local persisted progress; and a native process refusing a store reservation held after the reserving database session ends.

The concurrent metadata fixture creates incompressible JSONB in PostgreSQL, verifies physical TOAST/index allocation, and holds the writer transaction while another session attempts admission. The second writer remeasures after commit and refuses admission at the ceiling. An oversized observation is rejected before insertion. The stream component injects changing free-space observations between 64 KiB chunks, verifies the second chunk never reaches disk, checks unknown observations and retained crash bytes, and rehashes the prior committed object.

One read-only review found a closed-admission claim filter that also excluded local jobs with saved progress. The correction permits saved steps/imports through recovery headroom; `resource_bounds_local_progress_resumes_at_admission_ceiling` replays the resulting job and verifies success without a provider submission.

Existing assertions for fencing, isolation, uncertain submissions, cancellation, restart, restore and manifests remain unchanged. Their fixture transactions now enter the same metadata guard as production writers. The canonical gate runs them on the frozen post-rebase candidate. Exact command outcomes, source SHA and candidate archive/binary digests belong to the TaskQ result and CI manifest, rather than a self-referential source file.

## Qualification boundary

No host activation, DNS change, paid generation, capacity-policy change, destructive collection or private deployment value is part of this change. Measured metadata admission limits include indexes, TOAST and retained aborted allocations; they do not establish a PostgreSQL filesystem hard quota or cover cluster WAL. Filesystem observations cannot reserve space against unrelated host writers. The managed-host task must qualify its actual mount, PostgreSQL/WAL capacity, service identity, rollback and the current [backup baseline](../../assetq.md#storage-and-deployment). Independent external restoration remains a separate Open/unknown claim.

Publication acceptance remains protected-main publication with green `merge_group` CI and immutable source/merge/run/artifact/digest receipts. A local gate or candidate selftest does not establish that publication, deployment, real generation or owner visual acceptance. TaskQ § 7 hands publication to review without worker CI polling.
