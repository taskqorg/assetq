#!/bin/sh
# Encrypts the read lane's keys, config and retained state with SOPS and age. Run as root on the lane host.
# usage: read_lane_backup.sh <age recipient> <output file>; then copy the output off-host.
set -eu
recipient=$1 output=$2
ROOT=${ROOT:-}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
umask 077
state=""
for name in snapshot.good.json disabled-keys usage-baseline.json telemetry-lost; do
    [ -f "$ROOT/var/lib/pipe-read/$name" ] && state="$state var/lib/pipe-read/$name"
done
# shellcheck disable=SC2086
tar -C "${ROOT:-/}" -cf "$work/bundle.tar" etc/pipe-read/config.json etc/pipe-read/tavily-keys $state
sops encrypt --age "$recipient" --input-type binary --output-type json "$work/bundle.tar" > "$work/copy.json"
mv "$work/copy.json" "$output"
echo "encrypted copy: $output ($(sha256sum "$output" 2>/dev/null | cut -d' ' -f1 || shasum -a 256 "$output" | cut -d' ' -f1))"
