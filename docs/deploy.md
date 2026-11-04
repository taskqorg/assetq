# Deployment and rollback

Instructions for the separate host task that activates an AssetQ candidate. This page performs no host change. The target host, its inventory and receipts stay in the private host repository.

## Candidate

`python3 scripts/candidate.py` builds `dist/assetq-<version>-<source12>-<target>.tar.gz` with `bin/assetq`, this page, the contract and the service spec, plus `candidate.json`, binary checksums and archive `SHA256SUMS`. The manifest records the full source SHA, target, schema version and actual build libc. Packaging extracts the archive, verifies its binary digest and runs that extracted executable. CI publishes the Linux candidate as the `assetq-candidate` workflow artifact. Verify it before use:

```sh
sha256sum -c SHA256SUMS
tar -xzf assetq-<version>-<source12>-x86_64-unknown-linux-gnu.tar.gz
./assetq-<version>-<source12>-x86_64-unknown-linux-gnu/bin/assetq selftest
```

Match the full `candidate.json` source SHA to the reviewed PR/CI revision and obtain the archive checksum from that immutable CI run. Inside the extracted directory, also run `sha256sum -c SHA256SUMS`.

`selftest` without `--database` replays the lifecycle traces and checks store hashing, quota and free-space-floor refusal in a new temporary directory.

## Host requirements

- Linux x86_64 with glibc at least the version recorded in `candidate.json`; inspect the candidate target before activation.
- PostgreSQL 13 or newer reachable over a local socket, loopback or a separately protected tunnel. This build uses `NoTls`; it does not provide transport encryption. A dedicated role owns one dedicated database.
- Allow two PostgreSQL connections per active executor (work and lease renewal), plus API and administration connections. Provider waits release the renewal connection with the executor lease.
- A dedicated `assetq` Unix user that owns the data root, for example `/srv/assetq`. The data root needs room for the configured quota plus a safety margin.
- A protected environment file (mode 0600, owned by root or `assetq`) with the variables below. Credentials never go into Git, arguments or telemetry.
- The API binds to loopback. This task performs no DNS, HTTPS, firewall or live-host activation. The separate host task owns the approved canonical origin.

| Variable | Meaning |
|---|---|
| `ASSETQ_DATABASE_URL` | libpq URL or keyword string for the dedicated role and database |
| `ASSETQ_DATA_ROOT` | Data root owned by `assetq` |
| `ASSETQ_QUOTA_BYTES` | Store quota (default 50 GiB) |
| `ASSETQ_TEMP_QUOTA_BYTES` | Temporary-byte quota (default 1 GiB); crash leftovers count |
| `ASSETQ_FREE_SPACE_FLOOR_BYTES` | Minimum observed filesystem free space (default 1 GiB), plus 64 KiB write overhead; host-specific values stay in private configuration |
| `ASSETQ_METADATA_QUOTA_BYTES` | Measured dedicated database capacity ceiling (default 1 GiB), including indexes and TOAST |
| `ASSETQ_METADATA_ADMISSION_HEADROOM_BYTES` | Per-transaction allocation/payload allowance (default and minimum 4 MiB) |
| `ASSETQ_METADATA_RECOVERY_HEADROOM_BYTES` | Additional reserve unavailable to new admission and migration (default 16 MiB) |
| `ASSETQ_MAX_EXECUTION_MS` | Elapsed job deadline (default 86400000 ms); uncertainty remains reconciliable |
| `ASSETQ_MAX_UPLOAD_BYTES` | Per-object limit (default 512 MiB) |
| `ASSETQ_CONTENT_TYPES` | Comma-separated upload allowlist |
| `ASSETQ_EXECUTORS`, `ASSETQ_POLL_MS`, `ASSETQ_LEASE_MS` | Executor slots, provider poll interval, lease length |
| `ASSETQ_RETENTION_DAYS` | Recorded upload retention (no deletion in this build) |
| `ASSETQ_FAKE_PROVIDER_DIR` | Only for qualification of the synthetic method; leave unset in production |

## Additive install

Verify private values for all bounds before preparing activation. All service/executor/admin processes sharing a database/store must use the same protected configuration. Require the data root, objects and temp directories on one filesystem with working process file locks. Qualify the actual mount, service-user free-space observation, PostgreSQL disk capacity and WAL protection separately; source/CI fixtures do not prove these host properties. The metadata guard is a measured admission bound, not a filesystem hard quota. Account for allocations retained after aborted writes and unrelated cluster activity.

1. Prepare an isolated candidate directory, for example `/opt/assetq/releases/<version>`, and verify `SHA256SUMS` and `selftest`.
2. Create the role, database, user and data root without touching other databases, services or the existing Langfuse object store.
3. Run `assetq migrate` with the environment file. It creates schema version 1 once; project creation seeds built-in methods. It never drops data.
4. Run `assetq selftest --database`.
5. Create the project and grants: `assetq project <id> --budget N --max-active N` and `assetq token <id> --role member|owner`. Store tokens in the consumers' protected configuration.
6. Point a `current` symlink at the release and start a service unit running `assetq serve --listen 127.0.0.1:<port>` as `assetq` with the environment file. Reuse the host's existing CoreQ lifecycle/install and service mechanism; this candidate ships no parallel launcher or service installer.
7. Verify `GET /v1/health`, `assetq ops`, one upload and a hash-verified download with a test grant.

## Qualification readback

The [managed-delivery evidence](evidence/6/README.md) records candidate `ed30cd1`, exact archive/binary identities, the owner-accepted historical activation outage, managed authenticated retrieval and a separate bounded isolated restart. It does not grant another restart or paid run. Its current-store `restore-check` observation is not backup/restore qualification.

Before a separately approved control-process restart, retain the exact listener, protected database/data-root identity, candidate digest, resource limits, job/provider IDs and execution deadline. Restart on the same explicit `--listen 127.0.0.1:<port>` selection; default-port availability does not establish the intended service identity. Read back that the same provider operation is observed and the execution deadline is unchanged. A bind failure is a failed restart, not a reason to extend the deadline or stop another listener.

The Meshy one-effect selector deliberately refuses another POST in a previously used store. Preserve it and the original receipts. The completed operator examples authorize only original-operation observation and authenticated retrieval; removing the selector, changing stores or repeating `submit` with a fresh key does not grant a new paid effect. The separately authorized isolated A3 repeat is retained as a distinct example, not a managed-store recovery claim.

For retained results, use `assetq result <job-id>` to obtain actual manifest object IDs and `assetq download <object-id> <new-local-file>` through the existing scoped API. Compare each payload's SHA-256 and byte count with its manifest, including the source image, GLB and original texture maps. Keep payloads private and report technical, managed-live, restore and owner-visual verdicts separately.

## Rollback

- Stop the unit, point `current` at the previous release, start the unit. Jobs, events, effects and objects stay in PostgreSQL and the data root; executors resume from stored state.
- The first activation rolls back by stopping and disabling the unit. Keep the database and data root for inspection; remove them only through a separate reviewed decision.
- Schema version 1 is the only schema. A later schema change must ship with its own forward-compatible migration and rollback notes.
- This candidate adds no schema version or destructive migration. Rollback to a foundation executable removes the new resource protections; keep the service stopped until its capacity policy is reviewed. Do not reduce quotas, delete retained evidence or replay an uncertain provider effect to clear exhaustion. Restore capacity through the separately authorized host procedure, then reconcile retained jobs on the same candidate.

## Backup

For initial delivery, defer new backup work and both local and independent external backup/restore qualification. Preserve existing host backups. Do not stop or restart AssetQ for backup; no cold-backup timer, backup-driven outage, rotation or additional platform is authorized. These deferred gaps do not block initial functional installation and testing.

The later restoration recipe uses a consistent paired snapshot of metadata and the complete data root. Restore into a separate database and data root, never over production, and run `assetq restore-check` against restored configuration; it must replay every job and verify all blob/manifest hashes. A loopback-only restored service and restored project grant then verify `assetq status <job>`, `assetq result <job>`, `assetq methods`, and `assetq download <object> <new-file>`. Denied cross-project reads and unknown usage remain intact. Restore-check is read-only and refuses unknown schema versions or missing/corrupt payloads. This recipe does not authorize running new backup or restoration work during initial delivery.

Report deferred/unobserved backup and restoration as unknown with their scope and reason. RAID and provider URLs alone are not backup evidence. Until a restore observation exists, `GET /v1/status` reports backup as `unknown`; a local receipt never proves independent external restoration. Data integrity, authentication, resource bounds and durable job recovery remain required.
