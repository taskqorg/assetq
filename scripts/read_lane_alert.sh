#!/bin/sh
# Delivers one read lane alert from the Mac probe. usage: read_lane_alert.sh <message>
# Sends through the existing Hermes path on pds18 and falls back to a local notification when pds18 is unreachable.
# ponytail: during a pds18 outage only the Mac screen shows the alert; add a sender independent of pds18 if that proves too weak.
set -u
message=$1
target=${READ_LANE_ALERT_HOST:-pds18}
if printf '%s\n' "$message" | ssh -o BatchMode=yes -o ConnectTimeout=10 "$target" \
    "runuser -u hermes -- /home/hermes/.local/bin/hermes send -t 'telegram:Tight Monitoring' -q -f -"; then
    exit 0
fi
exec osascript -e 'on run argv' -e 'display notification (item 1 of argv) with title "Pipe read lane"' -e 'end run' "$message"
