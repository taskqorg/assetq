"""Read lane agent interface and probe (A5, A7). Risks: a stored-key fallback, a swallowed Pipe reason, a host's
other MCP servers touched by the install, and a probe that alerts repeatedly or never."""
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import threading
import unittest

SCRIPTS = Path(__file__).resolve().parent
MCP = SCRIPTS / "pipe_mcp.py"
PROBE = SCRIPTS / "read_lane_probe.py"
ALERT = SCRIPTS.parent / "deploy/read-lane/pipe-read-alert"


class Pipe(BaseHTTPRequestHandler):
    calls = []
    status = 200
    body = {"results": [{"title": "invented"}]}

    def respond(self, payload):
        data = json.dumps(payload).encode()
        self.send_response(Pipe.status)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        Pipe.calls.append((self.path, dict(self.headers), body))
        self.respond(Pipe.body)

    def do_GET(self):
        self.respond(Pipe.body)

    def log_message(self, *arguments):
        pass


class Base(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.home = Path(self.directory.name)
        self.server = HTTPServer(("127.0.0.1", 0), Pipe)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        self.url = f"http://127.0.0.1:{self.server.server_port}"
        Pipe.calls, Pipe.status, Pipe.body = [], 200, {"results": [{"title": "invented"}]}

    def tearDown(self):
        self.server.shutdown()
        self.server.server_close()
        self.directory.cleanup()


class Mcp(Base):
    def install(self):
        (self.home / "bin").mkdir(exist_ok=True)
        for runtime in ["claude", "codex"]:
            stub = self.home / "bin" / runtime
            stub.write_text(f'#!/bin/sh\necho "{runtime} $*" >> "$HOME/registered"\n'
                            'if [ "$0" != "${0%codex}" ] && [ "$2" = add ]; then printf "\\n[mcp_servers.pipe]\\ncommand = \\"x\\"\\n" >> "$CODEX_HOME/config.toml"; fi\n')
            stub.chmod(stub.stat().st_mode | stat.S_IEXEC)
        (self.home / "codex-home").mkdir(exist_ok=True)
        self.original = '[mcp_servers.shared-search]\ncommand = "/invented/mcp"\n'
        (self.home / "codex-home/config.toml").write_text(self.original)
        (self.home / "token").write_text("invented-token\n")
        env = dict(os.environ, HOME=str(self.home), CODEX_HOME=str(self.home / "codex-home"), PATH=f"{self.home}/bin:{os.environ['PATH']}")
        subprocess.run([sys.executable, str(MCP), "install", "--url", self.url, "--token-file", str(self.home / "token")], check=True, env=env, capture_output=True)
        return env

    def session(self, env, *messages):
        text = "".join(json.dumps(m) + "\n" for m in messages)
        target = self.home / ".local/share/pipe/mcp.py"
        run = subprocess.run([sys.executable, str(target), "serve", "claude"], input=text, env=env, capture_output=True, text=True, timeout=30, check=True)
        return [json.loads(line) for line in run.stdout.splitlines()]

    def call(self, env, name="web_search", arguments=None):
        message = {"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": name, "arguments": arguments or {"query": "q"}}}
        return self.session(env, message)[0]["result"]

    def test_access_values_stay_in_their_file(self):
        # Risk: the guard secret copied into the client's config, or a stale copy after rotation.
        env = self.install()
        access = self.home / "access.env"
        access.write_text("CF_ACCESS_CLIENT_ID=invented-id\nCF_ACCESS_CLIENT_SECRET=invented-secret\n")
        subprocess.run([sys.executable, str(MCP), "install", "--url", self.url, "--token-file", str(self.home / "token"), "--access-file", str(access)], check=True, env=env, capture_output=True)
        self.assertNotIn("invented-secret", (self.home / ".config/pipe/read.json").read_text())
        self.assertFalse(self.call(env)["isError"])
        self.assertEqual({k.lower(): v for k, v in Pipe.calls[0][1].items()}["cf-access-client-secret"], "invented-secret")
        access.unlink()
        result = self.call(env)
        self.assertTrue(result["isError"])
        self.assertIn("unusable", result["content"][0]["text"])

    def test_install_registers_each_runtime_and_leaves_others_alone(self):
        self.install()
        registered = (self.home / "registered").read_text()
        self.assertIn("claude mcp add -s user pipe", registered)
        self.assertIn("codex mcp add pipe", registered)
        self.assertNotIn("shared-search", registered)
        config = (self.home / "codex-home/config.toml").read_text()
        self.assertTrue(config.startswith(self.original), "the existing server stays unchanged")
        self.assertIn('[mcp_servers.pipe.tools.web_search]\napproval_mode = "approve"', config)
        mode = stat.S_IMODE((self.home / ".config/pipe/read.json").stat().st_mode)
        self.assertEqual(mode, 0o600)

    def test_handshake_offers_tools_with_instructions(self):
        env = self.install()
        replies = self.session(env, {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05"}},
                               {"jsonrpc": "2.0", "method": "notifications/initialized"},
                               {"jsonrpc": "2.0", "id": 2, "method": "tools/list"})
        self.assertEqual(len(replies), 2, "a notification gets no reply")
        self.assertIn("Prefer them over any built-in", replies[0]["result"]["instructions"])
        self.assertEqual([t["name"] for t in replies[1]["result"]["tools"]], ["web_search", "web_extract"])

    def test_call_carries_token_runtime_and_opaque_task(self):
        env = self.install()
        env["TASKQ_TASK"] = "49"
        result = self.call(env)
        self.assertFalse(result["isError"])
        self.assertIn("invented", result["content"][0]["text"])
        path, headers, body = Pipe.calls[0]
        self.assertEqual(path, "/v1/read/web_search")
        self.assertEqual((headers["Authorization"], headers["X-Pipe-Runtime"], headers["X-Pipe-Task"]), ("Bearer invented-token", "claude", "49"))
        self.assertEqual(body, {"query": "q"})

    def test_pipe_failure_returns_its_reason_and_has_no_fallback(self):
        env = self.install()
        Pipe.status, Pipe.body = 503, {"error": "queue-full", "reason": "the provider queue is full; retry later"}
        result = self.call(env)
        self.assertTrue(result["isError"])
        self.assertIn("queue-full", result["content"][0]["text"])
        self.server.shutdown()
        self.server.server_close()
        self.server = HTTPServer(("127.0.0.1", 0), Pipe)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        (self.home / ".config/pipe/read.json").unlink()
        env["TAVILY_API_KEY"] = "invented-key"
        calls = len(Pipe.calls)
        result = self.call(env)
        self.assertTrue(result["isError"], "no configuration never falls back to a stored key")
        self.assertIn("run the install command", result["content"][0]["text"])
        self.assertEqual(len(Pipe.calls), calls)

    def test_client_holds_no_provider_route(self):
        self.assertNotIn("tavily", MCP.read_text().lower())


class Probe(Base):
    def run_probe(self):
        log, state = self.home / "alerts", self.home / "state"
        script = self.home / "alert"
        script.write_text(f'#!/bin/sh\nprintf "%s\\n" "$1" >> {log}\n')
        script.chmod(0o755)
        run = subprocess.run([sys.executable, str(PROBE), "--url", self.url, "--state", str(state), "--alert", str(script)], capture_output=True, text=True)
        return run.returncode, (log.read_text().splitlines() if log.exists() else [])

    def test_alerts_once_per_transition(self):
        Pipe.body = {"status": "ok"}
        self.assertEqual(self.run_probe(), (0, []))
        Pipe.body = {"status": "no-usable-key"}
        code, alerts = self.run_probe()
        self.assertEqual((code, len(alerts)), (1, 1))
        self.assertIn("no-usable-key", alerts[0])
        self.assertEqual(len(self.run_probe()[1]), 1, "a continuing failure does not repeat")
        Pipe.body = {"status": "ok"}
        code, alerts = self.run_probe()
        self.assertEqual((code, len(alerts)), (0, 2))
        self.assertIn("Alert cleared", alerts[1])

    def test_unreachable_is_reported_as_seen_by_the_probe(self):
        self.server.shutdown()
        self.server.server_close()
        code, alerts = self.run_probe()
        self.server = HTTPServer(("127.0.0.1", 0), Pipe)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        self.assertEqual(code, 1)
        self.assertIn("unreachable from this probe", alerts[0])


class AlertSpool(Base):
    # Risk: a lane alert lost because the sender failed, or sent from a half-written file.
    def alert(self, *arguments, sender="true"):
        (self.home / "spool").mkdir(exist_ok=True)
        stub = self.home / "sender"
        stub.write_text(f'#!/bin/sh\ncat >> {self.home}/sent\nexec {sender}\n')
        stub.chmod(0o755)
        env = dict(os.environ, PIPE_READ_ALERT_SPOOL=str(self.home / "spool"), PIPE_READ_ALERT_SENDER=str(stub), PIPE_READ_ALERT_RETRY_SECONDS="0")
        return subprocess.run(["sh", str(ALERT), *arguments], env=env, capture_output=True, text=True).returncode

    def test_a_failed_send_keeps_the_alert_and_a_good_one_clears_it(self):
        self.assertEqual(self.alert("spool", "key disabled"), 0)
        self.assertEqual(len(list((self.home / "spool").iterdir())), 1)
        (self.home / "spool/.half-written").write_text("half")
        self.assertEqual(self.alert("send", sender="false"), 1)
        self.assertEqual(len(list((self.home / "spool").glob("[!.]*"))), 1, "the failed alert stays")
        self.assertEqual(self.alert("send"), 0)
        self.assertEqual(list((self.home / "spool").glob("[!.]*")), [])
        self.assertEqual((self.home / "sent").read_text(), "key disabled\nkey disabled\n")

    def test_an_empty_spool_sends_nothing(self):
        self.assertEqual(self.alert("send"), 0)
        self.assertFalse((self.home / "sent").exists())


if __name__ == "__main__":
    unittest.main()
