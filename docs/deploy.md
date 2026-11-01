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

`selftest` without `--database` replays the lifecycle traces and checks store hashing and quota in a temporary directory. It uses only a new temporary directory.

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
| `ASSETQ_METADATA_QUOTA_BYTES` | Dedicated database admission quota (default 1 GiB), with 4 MiB reserved headroom per admitting operation |
| `ASSETQ_MAX_EXECUTION_MS` | Elapsed job deadline (default 86400000 ms); uncertainty remains reconciliable |
| `ASSETQ_MAX_UPLOAD_BYTES` | Per-object limit (default 512 MiB) |
| `ASSETQ_CONTENT_TYPES` | Comma-separated upload allowlist |
| `ASSETQ_EXECUTORS`, `ASSETQ_POLL_MS`, `ASSETQ_LEASE_MS` | Executor slots, provider poll interval, lease length |
| `ASSETQ_RETENTION_DAYS` | Recorded upload retention (no deletion in this build) |
| `ASSETQ_FAKE_PROVIDER_DIR` | Only for qualification of the synthetic method; leave unset in production |

## Additive install

1. Prepare an isolated candidate directory, for example `/opt/assetq/releases/<version>`, and verify `SHA256SUMS` and `selftest`.
2. Create the role, database, user and data root without touching other databases, services or the existing Langfuse object store.
3. Run `assetq migrate` with the environment file. It creates schema version 1 once; project creation seeds built-in methods. It never drops data.
4. Run `assetq selftest --database`.
5. Create the project and grants: `assetq project <id> --budget N --max-active N` and `assetq token <id> --role member|owner`. Store tokens in the consumers' protected configuration.
6. Point a `current` symlink at the release and start a service unit running `assetq serve --listen 127.0.0.1:<port>` as `assetq` with the environment file. Reuse the host's existing CoreQ lifecycle/install and service mechanism; this candidate ships no parallel launcher or service installer.
7. Verify `GET /v1/health`, `assetq ops`, one upload and a hash-verified download with a test grant.

## Rollback

- Stop the unit, point `current` at the previous release, start the unit. Jobs, events, effects and objects stay in PostgreSQL and the data root; executors resume from stored state.
- The first activation rolls back by stopping and disabling the unit. Keep the database and data root for inspection; remove them only through a separate reviewed decision.
- Schema version 1 is the only schema. A later schema change must ship with its own forward-compatible migration and rollback notes.

## Backup

Stop all service/executor writers for a consistent first backup, then back up the PostgreSQL database and the complete data root together outside the host. Resume the same candidate after the snapshot. Restore into a separate database and data root, never over production, and prove the restore: run `assetq restore-check` against restored configuration; it must replay every job and verify all blob/manifest hashes. Start a loopback-only restored service and use a restored project grant for `assetq status <job>`, `assetq result <job>`, `assetq methods`, and `assetq download <object> <new-file>`. Verify denied cross-project reads and unknown usage remain intact. Restore-check is read-only and refuses unknown schema versions or missing/corrupt payloads. RAID and provider URLs do not count. Until a restore observation exists, `GET /v1/status` reports backup as `unknown`.
