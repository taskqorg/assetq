#!/bin/sh
# Installs the read lane on one host as root. usage: install.sh <unpacked pipe-read release> <pipe binary> <pipe env file>
# Needs /etc/pipe-read/config.json and /etc/pipe-read/tavily-keys in place (first install or restore).
set -eu
release=$1 pipe_binary=$2 pipe_env=$3
ROOT=${ROOT:-}

id pipe-read >/dev/null 2>&1 || useradd --system --user-group --no-create-home --shell /usr/sbin/nologin pipe-read
install -d -m 0750 -o root -g pipe-read "$ROOT/etc/pipe-read"
chown root:root "$ROOT/etc/pipe-read/tavily-keys"
chmod 0600 "$ROOT/etc/pipe-read/tavily-keys"
chown root:pipe-read "$ROOT/etc/pipe-read/config.json"
chmod 0640 "$ROOT/etc/pipe-read/config.json"
install -d -m 0755 "$ROOT/opt/pipe-read/bin"
install -m 0755 "$release/bin/pipe-read" "$ROOT/opt/pipe-read/bin/pipe-read"
install -d -m 0750 -o pipe-read -g pipe-read "$ROOT/var/lib/pipe-read"
if id assetq >/dev/null 2>&1; then
    install -d -m 0750 -o assetq -g pipe-read "$ROOT/var/lib/pipe-read-control"
else
    install -d -m 0750 -o root -g pipe-read "$ROOT/var/lib/pipe-read-control"
fi
units="$ROOT/etc/systemd/system"
install -d "$units"
for unit in pipe-read.socket pipe-read.service pipe-read-reconcile.service pipe-read-snapshot.timer; do
    install -m 0644 "$release/deploy/read-lane/$unit" "$units/$unit"
done
sed -e "s|@PIPE_ENV_FILE@|$pipe_env|" -e "s|@PIPE_BINARY@|$pipe_binary|" "$release/deploy/read-lane/pipe-read-snapshot.service" > "$units/pipe-read-snapshot.service"
if [ -d /run/systemd/system ]; then
    systemctl daemon-reload
    systemctl enable --now pipe-read.socket pipe-read.service
    if id assetq >/dev/null 2>&1; then
        systemctl enable --now pipe-read-snapshot.timer
    fi
fi
/opt/pipe-read/bin/pipe-read selftest 2>/dev/null || "$ROOT/opt/pipe-read/bin/pipe-read" selftest
