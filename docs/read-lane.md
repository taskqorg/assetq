# Read lane

Detail of [pipe.md § Read lane](../pipe.md#read-lane): routes, host layout, operations and restore. Values below marked invented are examples.

## Interface

| Route | Authority | Behavior |
|---|---|---|
| `POST /v1/read/web_search` | Project bearer token | JSON object with `query`, forwarded to Tavily `/search` |
| `POST /v1/read/web_extract` | Project bearer token | JSON object with `urls`, forwarded to Tavily `/extract` |
| `GET /healthz` | Loopback, or the guard on public entry | Status, snapshot age, key counts, queue use, telemetry loss; no secret and no key identity |

`X-Pipe-Task` is an opaque label and `X-Pipe-Runtime` names the caller's runtime. Both hold at most 64 characters of `A-Za-z0-9._:-`; anything else refuses with `bad-label`, because telemetry stores codes, not free text. A caller-supplied `api_key` is dropped, and `include_usage` is always set.

| Reply | Meaning |
|---|---|
| Provider status and body | Success, quota replies (`432`, `433`) and `429`, unchanged, with `Retry-After` |
| `401 unauthenticated` | Unknown or revoked token |
| `400 bad-request`, `400 bad-label` | Invalid body or label |
| `503 queue-full` | The provider's bounded queue is full; refused at once |
| `503 no-usable-key` | Every key is disabled |
| `502 key-disabled-<status>` | This call's key was refused with 401, 402 or 403 and is now disabled |
| `502 provider-unreachable` | No key got an HTTP answer |
| `502 provider-body`, `504 provider-timeout` | The reply was cut off or too slow; no rotation, because the charge is unknown |

## Host layout

| Path | Owner and mode | Content |
|---|---|---|
| `/opt/pipe-read/bin/pipe-read` | root, 0755 | Release binary |
| `/etc/pipe-read/tavily-keys` | root, 0600 | One key per line; the unit reads it only through `LoadCredential` |
| `/etc/pipe-read/config.json` | root:pipe-read, 0640 | See `deploy/read-lane/config.example.json` |
| `/var/lib/pipe-read/` | pipe-read, 0750 | `telemetry.db`, `snapshot.good.json`, `disabled-keys`, `telemetry-lost`, `usage-baseline.json`, `coreq/` |
| `/var/lib/pipe-read/alerts/` | pipe-read, 2770 | One file per unsent alert; `pipe-read-alert.path` sends each as user `hermes` and keeps it until the send succeeds |
| `/var/lib/pipe-read-control/snapshot.json` | assetq:pipe-read, 0750 directory | Published by the control side, read by the lane |

`pipe-read.socket` owns `127.0.0.1:8741`. The lane refuses a non-loopback listener. Public entry is only the `deploy/read-lane/Caddyfile.snippet` route inside the existing `pipe.ufobe.com` site, which requires owner approval like any other Caddy or guard change ([deployment](deployment-operator.md)).

## Records

Each call is one CoreQ `coreq-observation/1` record of kind `operation`, stored in `telemetry.db` and sent by `Store::ship` over the enrolled connection.

| Acceptance name | Record field |
|---|---|
| pipeline, revision, step | `pipeline` (`web_search`, `web_extract`), `pipeline_revision`, `step` (`tavily.search`, `tavily.extract`) |
| project | `project`, from the grant |
| task | `source` holds the label as given; `task` also holds it when it is a whole number, because CoreQ's `task` is numeric; otherwise `task` is null with reason `non-numeric-label` |
| provider, key | `route` (`tavily`), `account` (first 12 hex characters of the key's SHA-256) |
| duration, units | `duration_us`; `units` with `unit_kind` `tavily_credits`, or null with a reason |
| result | `outcome`, `error`; a success after a rotation reads `retried` |
| caller | `runtime` (`claude`, `codex`) |

A key disabled by the provider also writes a `check` record `read.key_state` with outcome `failed`. `/healthz` shows `telemetry.lost`, a counter of records that could not be queued or stored; it persists in `telemetry-lost`. `telemetry.shipper` reads `off`, `ok`, `retrying` or `stopped: <reason>`. Off means no connection is enrolled, not that nothing is unsent.

## Operations

Grants, on the control side:

```sh
pipe read-grant create owner/repository   # prints the token once
pipe read-grant revoke owner/repository   # effective at the next publication
```

The lane reads the snapshot every second and adopts a new valid file. An invalid file increments `snapshot.rejected`, and the last valid one stays in service. On start it uses the control file when valid, else its retained copy.

Re-enable a disabled key after fixing it at the provider. This restart drains; callers see a delay.

```sh
sed -i '/^<key id>$/d' /var/lib/pipe-read/disabled-keys && systemctl restart pipe-read.service
```

Rotate keys: edit `tavily-keys`, restart, then take a new encrypted copy. Connect telemetry once, with owner approval in the browser: `pipe-read enroll /etc/pipe-read/config.json <dashq-url>` as user `pipe-read`.

Reconcile usage: `systemctl start pipe-read-reconcile.service`, then read its journal. The first run records the baseline per key; each later run prints `tavily_delta`, `recorded` and `difference` per key, and `calls_without_units`. Any other client sharing a key shows up as a positive difference, so the check needs keys that only the lane uses, or an accounted difference.

Probe, on another host, from cron or a timer:

```sh
read_lane_probe.py --url https://pipe.ufobe.com --state <file> --alert <existing alert command> --access-file <file>
```

The access file is a shell env file with `CF_ACCESS_CLIENT_ID` and `CF_ACCESS_CLIENT_SECRET`, mode 0600. Clients read it at use and store only its path.

## Agent install

One command per host, for every runtime found. Loopback on pds18; the guarded origin and access file elsewhere:

```sh
python3 pipe_mcp.py install --url http://127.0.0.1:8741 --token-file <file>
python3 pipe_mcp.py install --url https://pipe.ufobe.com --token-file <file> --access-file <file>
```

It writes `~/.config/pipe/read.json` (mode 0600), copies the client to `~/.local/share/pipe/mcp.py` and registers the server `pipe` with `claude mcp add` and `codex mcp add`. It does not touch `shared-search`.

## Backup

Prerequisites: `sops` and `age` on the lane host, an age identity kept off the host by the operator. No paid service is involved.

```sh
read_lane_backup.sh <age recipient> /root/pipe-read-copy.json
scp /root/pipe-read-copy.json <off-host target>
```

Take a new copy after any key, config or grant change. The copy holds keys, config, the retained snapshot (hashes only), the disabled-key list and the usage baseline. It holds no telemetry; records not yet shipped stay on the old host.

## Restore

Prerequisites on the clean host or container: root, `sops`, `tar`, `curl`, network; from elsewhere only the encrypted copy, the age identity and the `pipe-read` release from CI.

1. Unpack the release: `tar -xzf pipe-read-<...>.tar.gz`, after checking it against `SHA256SUMS`.
2. Restore: `SOPS_AGE_KEY_FILE=<identity> scripts/read_lane_restore.sh <copy> <release>`.
3. Without systemd, start by hand: `CREDENTIALS_DIRECTORY=/etc/pipe-read setpriv --reuid=pipe-read --regid=pipe-read --init-groups /opt/pipe-read/bin/pipe-read serve <config with "listen">` after making `tavily-keys` readable to that process only.
4. Accept: `curl -H "Authorization: Bearer <token>" -d '{"query":"..."}' http://127.0.0.1:8741/v1/read/web_search` returns real results.

The token works because the retained snapshot holds its hash. The control side is not needed to serve; it resumes publishing when restored. Verify by a run: a restore counts only with its exact steps recorded on a clean host or container.
