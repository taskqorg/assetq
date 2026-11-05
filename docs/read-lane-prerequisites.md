# Read lane prerequisites

Named inputs for the live stages of the [read lane](https://github.com/taskqorg/assetq/issues/49). This page holds no secret values.

## Access and keys

- Root route: SSH alias `pds18` on the Mac host, with a `known_hosts` entry.
- Keys: `/etc/pipe-read/tavily-keys` on pds18, root 0600. The file holds the same key pool as the existing `web_search` service, so the lane does not use these keys alone. Usage reconciliation therefore shows a difference that `web_search` calls explain.

## Encrypted copy

- Age recipient: `age1ln5muz0yrl8h7zzrkfanpzufupq4t928y780tpznvt7vf7dheghq464g3d`.
- Age identity: Mac login Keychain, service `pipe-read-age-identity`, account `pipe-read`. Read it only for a restore, and pass it to `SOPS_AGE_KEY` without printing it:

  ```sh
  SOPS_AGE_KEY="$(security find-generic-password -a pipe-read -s pipe-read-age-identity -w)"
  ```

- Off-host target: `~/.local/share/pipe/pipe-read-copy.json` on the Mac, mode 0600. The Mac is the final destination and pulls the copy from `/root/pipe-read-copy.json` on pds18 over the `pds18` route.
- Limitation: the identity and the copy share one machine, so this protects against loss of pds18, not loss of the Mac.
- Missing: `sops` and `age` are not installed on pds18. Install them with the lane.

## Caddy route

The route in `deploy/read-lane/Caddyfile.snippet` is approved for the existing `pipe.ufobe.com` site. It uses the live guard `127.0.0.1:9122` with `uri /verify` and goes first inside the site's `route` block, because that block sends `/v1/*` to the service without `forward_auth`.

## Second host and alerts

- Second host for live calls and the probe: the Mac.
- Existing alert path: `hermes send` on pds18 to `telegram:Tight Monitoring`, the host health channel. The Mac's Hermes has no configured platform.
- Probe on the Mac: launchd job `online.ufobe.pipe-read-probe`, every 60 seconds, from `~/.local/share/pipe/`. It is loaded with `launchctl bootstrap` from there, so a logout or reboot unloads it until the plist is linked into `~/Library/LaunchAgents`.
- Probe alert command on the Mac: `scripts/read_lane_alert.sh`. It sends through pds18 and shows a local notification when pds18 is unreachable, so during a pds18 outage the alert reaches only the Mac screen.
- Lane alert command on pds18: `/opt/pipe-read/bin/pipe-read-alert spool`. `pipe-read.service` sets `NoNewPrivileges` and `ProtectHome`, so the lane spools a file and `pipe-read-alert.path` sends it as user `hermes`.

## Enrollment

The owner approves the CoreQ enrollment of `pipe-read` to DashQ in the browser when it starts. This approval happens at that moment and cannot be recorded in advance.
