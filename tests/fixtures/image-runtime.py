"""Invented native protocol fixture: no credentials, image tool or provider effects."""
import json
import sys
import time
from pathlib import Path

trace, login, provider, outcome = sys.argv[1:5]


def emit(message):
    print(json.dumps(message), flush=True)


def terminal(status):
    emit({"method": "turn/completed", "params": {
        "threadId": "invented-thread", "turn": {"id": "invented-turn", "status": status}}})


for line in sys.stdin:
    message = json.loads(line)
    with open(trace, "a", encoding="utf-8") as log:
        log.write(json.dumps(message) + "\n")
    method = message.get("method")
    if method == "initialize":
        result = {"userAgent": "invented-fixture"}
    elif method == "thread/start":
        result = {"thread": {"id": "invented-thread"}, "modelProvider": provider,
                  "model": "invented-inherited-model"}
    elif method == "account/read":
        result = {"account": None if login == "none" else {"type": login}}
    elif method == "model/list":
        emit({"id": message["id"] + 99, "result": {"data": ["invented-unrelated"]}})
        emit({"method": "fixture/catalogue-waiting", "params": {}})
        until = time.monotonic() + 5
        while not Path(trace + ".release").exists():
            if time.monotonic() >= until:
                sys.exit("invented catalogue release missing")
            time.sleep(.005)
        if outcome == "catalogue-missing":
            continue
        if outcome == "catalogue-error":
            emit({"id": message["id"], "error": {"code": -32603, "message": "invented catalogue refusal"}})
            continue
        result = {"data": [] if outcome == "catalogue-empty" else [{"id": "invented-inherited-model"}], "nextCursor": None}
    elif method == "turn/start":
        # Terminal-before-ack is deliberate: consumer must preserve runtime progress.
        if outcome == "complete":
            terminal("completed")
        result = {"turn": {"id": "invented-turn"}}
    elif method == "turn/interrupt":
        terminal("interrupted")
        result = {}
    else:
        continue
    emit({"id": message["id"], "result": result})

marker = Path(trace + ".eof.tmp")
marker.write_text("invented transport EOF", encoding="utf-8")
marker.rename(trace + ".eof")
