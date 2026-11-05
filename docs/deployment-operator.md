# Deployment operator workflow

`scripts/deploy_assetq.py` moves one verified CI candidate onto the existing AssetQ service over the operator's existing SSH. It implements the [deployment](deploy.md) lifecycle for candidates only. It does not install a first service, and nothing starts it automatically.

**Code-ready is not deployed.** The merged tool and its fixtures prove tool logic. Whether a candidate runs on a host is shown only by a successful `apply` readback on that host. No run of this tool against a real host is recorded here.

## Inventory

| Area | Observed state | Consequence |
|---|---|---|
| CI | One workflow, `.github/workflows/tests.yml`, with jobs `tests` and `candidate` on GitHub-hosted `ubuntu-latest` | The tool reads this workflow's evidence |
| Merge result | The merge queue runs that workflow as a `merge_group` event at the exact commit that becomes `main` | The tool requires a green `merge_group` run, not a branch or pull-request run |
| Candidate | The run's `assetq-candidate` artifact holds `assetq-<version>-<source12>-x86_64-unknown-linux-gnu.tar.gz` and `SHA256SUMS` | The tool binds the artifact digest GitHub reports |
| Repository automation | No self-hosted runners, deployment environments, Actions secrets or webhooks | The tool adds none. It only reads through the operator's existing `gh` login |
| Host access | Existing operator SSH alias, optional explicit login user and `known_hosts` entry | The tool uses `BatchMode=yes` and `StrictHostKeyChecking=yes`, so it never prompts, learns a new key or creates a key. It refuses a non-root login |
| Service manager | A systemd **user** unit under the dedicated service account, not a system unit. Root reaches it with `runuser -u <account> -- env XDG_RUNTIME_DIR=/run/user/<uid> systemctl --user ...` | The target binds account, UID and unit. All lifecycle and transient commands use that shape; `systemctl restart <unit>` at system level would not reach this unit |
| Service command | The unit runs `<install_root>/current/<executable> serve --listen 127.0.0.1:<port>`, possibly behind a resource wrapper, with one protected environment file | The plan binds the whole command and refuses any other shape instead of adapting to it |
| Releases | `current` is an absolute symlink to `<install_root>/releases/<executable-sha256>` | New releases use the same name and pointer form |

The CI and repository rows are read from GitHub. The service manager and release rows come from a PM host readback on the task; the service command row comes from [deployment](deploy.md) and the [managed-delivery evidence](evidence/6/README.md). This task did not inspect a host, and the exact `ExecStart` and release contents were not read, so the first `plan` on the host is what verifies them.

### CoreQ lifecycle conventions

| CoreQ convention | This tool | Difference and reason |
|---|---|---|
| R12, R15: immutable `releases/<id>/`; prepare with verified bytes, then activate | Same. Identical bytes are reused and different bytes refuse and stay preserved | The release ID is the executable SHA-256, matching the host, because AssetQ candidates share one Cargo version |
| R12, R13: atomic `current` and `previous`; a running binary is never overwritten | Same, written with a temporary link and a rename | They are symlinks, not version files, because the unit's `ExecStart` and [deployment](deploy.md) use the `current` symlink |
| R25: `selftest` before install | Same, under the service's user manager, before the release gets its final name | None |
| R16: launcher rollback after three unhealthy starts; `healthy` marker | The tool verifies once and rolls back on failure. It writes `releases/<id>/healthy` after success | AssetQ has no CoreQ launcher. A later unit crash is not rolled back automatically |
| R12: self-update and pruning | Not provided | Automatic LIVE action is not authorized. No release is deleted |

## What the tool cannot do

It has no code path for DNS, Caddy, sign-in or access guards, firewalls, grants, tokens, keys, permissions on other accounts, unit edits, environment-file edits, quota changes, object or database deletion, backup, provider dispatch or `gh auth login`. It never prints or reads the values in a protected file. It only names those files, so the user manager can pass them to the service process, and reads their owner, mode, size and time.

## Prerequisites

- An operator terminal with `python3` 3.9 or newer, `gh` with the operator's existing login, and `ssh` that already reaches the host. No `CI`, `GITHUB_ACTIONS` or `TASKQ_TASK` variable may be set.
- On the host: a root SSH login, a running user manager for the service account, and the tools `plan` checks for: systemd, `runuser`, GNU coreutils, `tar`, `curl`, `sed`, `getconf` and `ssh-keygen`.
- A private target file outside Git. All values below are invented:

```json
{
  "ssh_alias": "invented-alias",
  "ssh_user": "invented-operator",
  "service_account": "invented-account",
  "service_uid": 12345,
  "executable": "bin/assetq",
  "install_root": "/invented/assetq",
  "unit": "invented.service",
  "port": 8740,
  "data_root": "/invented/data",
  "expected": {
    "hostname": "invented-host",
    "machine_id_sha256": "<sha256 of /etc/machine-id on the host>",
    "ssh_host_key_sha256": "SHA256:<fingerprint of the host ed25519 key>"
  },
  "status_env_files": ["/invented/protected/project-status.env"],
  "min_install_free_bytes": 1073741824,
  "min_data_free_bytes": 1073741824,
  "health_timeout_seconds": 60
}
```

`ssh_alias` remains a plain alias, never `user@host`. Optional `ssh_user` selects an existing operator login with separate SSH arguments `-l <user>`; omit it to preserve the alias's existing user selection.

`ssh_user` must be a nonempty string matching `[A-Za-z_][A-Za-z0-9_.-]*`. Null, whitespace, option syntax and shell expressions refuse before any SSH call.

This selects a username only: it changes no credentials, SSH configuration, trust pins or permissions. The actual remote UID must still be zero, and all expected host-identity and deploy/rollback checks remain required under R3.

Obtain the `expected` identity independently of this tool. `executable` is the path inside a release that the unit runs. A release always keeps `bin/assetq`; when the unit expects another path, the tool adds a verified copy there and does not edit the unit.

Each `status_env_files` entry is a protected host file, readable by the service account, with `ASSETQ_URL` and `ASSETQ_TOKEN` for one project. With an empty list, active work is unknown and the plan refuses unless you accept that explicitly. Status covers only the named projects.

Plans and target files name private infrastructure. Keep them in private storage. Do not attach them to public tasks or pull requests.

## Workflow

### 1. Create the plan

```sh
python3 scripts/deploy_assetq.py plan --target <private-target.json> --source <full-main-sha> --out <new-private-dir>
```

`plan` makes no host change. Its host reads include short-lived `systemd-run --user` processes for `assetq ops` and `assetq restore-check`, so a large store sees read load. It refuses, and writes nothing, unless all of these hold:

- **CI:** every `merge_group` run of the exact SHA is green, and jobs `tests` and `candidate` succeeded.
- **Source:** `main` contains the SHA, and packaged documents match the source tree blobs.
- **Digests:** the artifact, archive and executable digests agree across GitHub, `SHA256SUMS` and `candidate.json`, and the candidate names the same source, version and target.
- **UI:** `embedded_ui` matches the source `web/` blobs, and the executable contains those exact bytes. A source without `web/` needs `--allow-no-ui`, which the plan records.
- **Target:** hostname, machine ID and SSH host key match `expected`, the login is root, and the service account has the named UID.
- **Host:** the user unit's command shape, protected-file modes, free-space floors and glibc all pass.
- **Current and rollback:** the running process is healthy and runs the digest of `current`. `current` is the rollback target.
- **Retained state:** the current executable's `restore-check` verifies and reports the candidate's schema. This is a current-store observation, not a backup or restore qualification.
- **Active work:** named projects are idle. Otherwise `--accept-active-work` records your reviewed decision in the plan.

### 2. Review the plan

Read `plan.json` and compare it with the task you were approved for: source, CI run, candidate digests, UI state, target identity, rollback release, active work and the action list. The digest printed as `plan_sha256` stands for this review. The plan expires after one hour.

Re-verify without effect at any time:

```sh
python3 scripts/deploy_assetq.py apply --plan <dir>/plan.json --confirm-plan <plan_sha256> --dry-run
```

### 3. Apply the plan

**Warning:** `apply` is a LIVE action. It restarts the AssetQ service, which is offline until the new process is healthy. Run it only after approval for this exact candidate and host. Earlier approval of a restart does not carry over.

```sh
python3 scripts/deploy_assetq.py apply --plan <dir>/plan.json --confirm-plan <plan_sha256>
```

`apply` repeats all evidence checks first. Any drift from the plan refuses. Then it asks for a typed phrase before each action. Declining stops there and keeps what is already done.

| Action | Effect | Service impact |
|---|---|---|
| `prepare` | Takes `deploy.lock`, uploads, verifies digests and `selftest`, and creates the immutable release | None |
| `migrate` | Records `restore-check` counts, then runs the candidate's `assetq migrate` under the service's user manager. Schema-1 migration is additive and stays in place after a rollback | The old process keeps running |
| `activate` | Re-reads the host, switches `previous` and `current`, restarts the user unit and verifies it | Outage until healthy |

Activation succeeds only if all of these hold:

- Two consecutive observations show one new process that is `active` and runs the candidate digest, and `/v1/health` reports the service up with database `ok`.
- The unit text, limits, protected-file identity and data-root identity are unchanged.
- `restore-check` verifies, with no retained count below its value before activation.

A failed verification rolls back automatically, as the `activate` phrase states. The unit restarts on the plan's release, which is verified the same way. A running provider operation is observed again after restart and is never resubmitted.

The tool writes a private journal beside the plan. A plan applies once, and a used plan refuses.

### 4. Read the result

| Exit | Result | Next step |
|---|---|---|
| `0` | Deployed and verified on this host | Record the release and digests. Separately run the remaining journeys below |
| `1` | Refused. A refusal before `activate` leaves the running service unchanged | Read the reason. Create a new plan if the old one was used |
| `2` | Rolled back and verified. The candidate release stays on disk for inspection | Treat the candidate as failed. Do not retry it without a new plan |
| `3` | Uncertain. The host outcome is unknown and `deploy.lock` stays in place | Inspect the host and the journal read-only. Do not re-run. Removing the lock is a separate operator decision after inspection |

### 5. Roll back later

**Warning:** `rollback` is a LIVE action. It restarts the service and needs its own approval and typed phrase.

```sh
python3 scripts/deploy_assetq.py rollback --plan <dir>/plan.json --confirm-plan <plan_sha256> [--dry-run]
```

It acts only while this plan's candidate is current, the preserved release still has its recorded digest and the service configuration is unchanged. The plan's one-hour expiry does not apply, because each of these is read fresh. It keeps data, jobs and the migrated additive schema. Any other case is outside this tool: use the reviewed procedure in [deployment rollback](deploy.md#rollback).

## Per-action confirmation

Each row needs its own confirmation for the exact target and candidate. One confirmation never implies another.

| Effect | Who confirms | Through this tool |
|---|---|---|
| `plan` or `--dry-run` | Operator, read-only | Yes |
| `prepare`, `migrate`, `activate`, `rollback` on a LIVE host | Operator, with the owner's approval of that candidate or action, then a typed phrase per action | Yes |
| `assets.ufobe.com` DNS change | Owner-approved concrete change | No |
| Caddy site, proxy or TLS change | Owner-approved concrete change | No |
| Access or sign-in guard, browser subject mapping, proxy secret | Owner-approved security change | No |
| Firewall, grant, token, key, runner, webhook or other security effect | Owner-approved security change | No |
| Environment, quota, limit or unit change | Separate reviewed host plan. The tool refuses to apply while these drift | No |
| Backup, restore, object or database deletion | Not authorized for initial delivery; see [deployment](deploy.md#backup) | No |

## What each state proves

| State | Evidence | Does not prove |
|---|---|---|
| Implemented | Source in this repository | Correct behavior |
| Tested | `python3 scripts/check.py`, with invented GitHub data and fake service tools in real POSIX sh. The fakes accept only the user-manager command shape | Real systemd, user-manager reachability, SSH trust, host layout or service behavior |
| Published | Green merge-result CI of the merged tool | That any host changed |
| Planned | A plan and its private digest | That anything was applied |
| Deployed | Exit `0` and its journal on the named host | DNS, Caddy, sign-in, browser journeys, backup or restore |
| Live-qualified | Separately recorded host, browser and API journeys under [acceptance](acceptance.md) | Owner visual or product acceptance |

The first real `plan` and `apply` are therefore also the first qualification of this tool on a host. Treat any difference between the host and the [inventory](#inventory) as a refusal to inspect, never as a reason to weaken a check.
