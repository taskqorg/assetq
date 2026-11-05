#!/bin/sh
# Restores the read lane on a clean host from the encrypted copy alone. Run as root.
# usage: SOPS_AGE_KEY_FILE=<age identity> read_lane_restore.sh <encrypted copy> <unpacked pipe-read release>
# The control side (pipe binary, PostgreSQL) is not needed: the lane serves the retained snapshot.
set -eu
copy=$1 release=$2
ROOT=${ROOT:-}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
umask 077
sops decrypt --input-type json --output-type binary "$copy" > "$work/bundle.tar"
install -d -m 0750 "$ROOT/etc/pipe-read" "$ROOT/var/lib/pipe-read"
tar -C "${ROOT:-/}" -xf "$work/bundle.tar"
[ -s "$ROOT/etc/pipe-read/tavily-keys" ] || { echo "the copy holds no keys" >&2; exit 2; }
[ -s "$ROOT/var/lib/pipe-read/snapshot.good.json" ] || { echo "the copy holds no snapshot" >&2; exit 2; }
sh "$release/deploy/read-lane/install.sh" "$release" /nonexistent/pipe /nonexistent/pipe.env
chown -R pipe-read:pipe-read "$ROOT/var/lib/pipe-read"
echo "restored; run one real call to accept the restore"
