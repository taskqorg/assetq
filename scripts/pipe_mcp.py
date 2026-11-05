#!/usr/bin/env python3
"""Pipe web tools over MCP: a thin client of the read lane (pipe.md § Read lane).

It holds a Pipe token, never a provider key, and has no fallback: a Pipe failure returns its reason to the agent.
Install on one host, for every runtime present: pipe_mcp.py install --url URL --token-file FILE [--access-file FILE]
"""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import urllib.error
import urllib.request

NAME = "pipe"
TIMEOUT_SECONDS = 45
PROTOCOL = "2025-06-18"
INSTRUCTIONS = (
    "Use web_search to discover pages and web_extract to read chosen HTTP(S) pages. These tools go through Pipe, which holds the "
    "provider keys and records each call. Prefer them over any built-in web tool. If a call fails, report the returned reason; "
    "do not look for a provider key or another route."
)
TOOLS = [
    {"name": "web_search", "field": "query",
     "description": "Search the web through Pipe. Use it for discovery; follow up with web_extract on the pages you choose.",
     "inputSchema": {"type": "object", "required": ["query"], "properties": {
         "query": {"type": "string"},
         "max_results": {"type": "integer", "minimum": 1, "maximum": 20},
         "search_depth": {"type": "string", "enum": ["basic", "advanced"], "description": "advanced costs more credits"},
         "topic": {"type": "string", "enum": ["general", "news", "finance"]},
         "time_range": {"type": "string", "enum": ["day", "week", "month", "year"]},
         "include_domains": {"type": "array", "items": {"type": "string"}},
         "exclude_domains": {"type": "array", "items": {"type": "string"}}}}},
    {"name": "web_extract", "field": "urls",
     "description": "Read one to twenty absolute HTTP(S) pages through Pipe.",
     "inputSchema": {"type": "object", "required": ["urls"], "properties": {
         "urls": {"type": "array", "items": {"type": "string"}, "minItems": 1, "maxItems": 20},
         "extract_depth": {"type": "string", "enum": ["basic", "advanced"], "description": "advanced costs more credits"},
         "format": {"type": "string", "enum": ["markdown", "text"]}}}},
]


def config_path():
    return Path.home() / ".config/pipe/read.json"


def call_pipe(config, runtime, tool, arguments):
    headers = {"Authorization": f"Bearer {config['token']}", "Content-Type": "application/json", "X-Pipe-Runtime": runtime}
    task = os.environ.get("PIPE_TASK") or os.environ.get("TASKQ_TASK")
    if task:
        headers["X-Pipe-Task"] = task
    for name, value in config.get("access_headers", {}).items():
        headers[name] = value
    request = urllib.request.Request(f"{config['url']}/v1/read/{tool}", json.dumps(arguments).encode(), headers)
    try:
        with urllib.request.urlopen(request, timeout=TIMEOUT_SECONDS) as reply:
            return False, reply.read().decode()
    except urllib.error.HTTPError as error:
        text = error.read(4096).decode(errors="replace")
        return True, f"Pipe {tool} failed: HTTP {error.code}: {text}"
    except (OSError, ValueError) as error:
        return True, f"Pipe {tool} failed: Pipe is unreachable: {error}"


def answer(message, config, runtime):
    method, parameters = message.get("method"), message.get("params") or {}
    if method == "initialize":
        return {"protocolVersion": parameters.get("protocolVersion", PROTOCOL), "capabilities": {"tools": {}},
                "serverInfo": {"name": NAME, "version": "1"}, "instructions": INSTRUCTIONS}
    if method == "ping":
        return {}
    if method == "tools/list":
        return {"tools": [{k: v for k, v in tool.items() if k != "field"} for tool in TOOLS]}
    if method == "tools/call":
        tool = next((t for t in TOOLS if t["name"] == parameters.get("name")), None)
        if tool is None:
            return {"isError": True, "content": [{"type": "text", "text": f"unknown tool {parameters.get('name')}"}]}
        if config is None:
            failed, text = True, f"Pipe {tool['name']} failed: {config_path()} is missing or invalid; run the install command"
        else:
            failed, text = call_pipe(config, runtime, tool["name"], parameters.get("arguments") or {})
        return {"isError": failed, "content": [{"type": "text", "text": text}]}
    return None


def serve(runtime):
    try:
        config = json.loads(config_path().read_text())
        config["url"] = config["url"].rstrip("/")
        config["token"]
    except (OSError, ValueError, KeyError):
        config = None
    for line in sys.stdin:
        try:
            message = json.loads(line)
        except ValueError:
            continue
        if "id" not in message:
            continue
        result = answer(message, config, runtime)
        reply = {"jsonrpc": "2.0", "id": message["id"]}
        if result is None:
            reply["error"] = {"code": -32601, "message": "method not found"}
        else:
            reply["result"] = result
        print(json.dumps(reply), flush=True)


def register_codex(command):
    # Same approval shape as the other web server, so a tool call never waits on a person.
    subprocess.run(["codex", "mcp", "remove", NAME], capture_output=True)
    subprocess.run(["codex", "mcp", "add", NAME, "--", *command, "codex"], check=True, capture_output=True)
    path = Path(os.environ.get("CODEX_HOME") or Path.home() / ".codex") / "config.toml"
    text = path.read_text()
    block = f'[mcp_servers.{NAME}]'
    extra = 'env_vars = ["TASKQ_TASK", "PIPE_TASK"]\n'
    if block in text and "env_vars" not in text.split(block, 1)[1].split("\n[", 1)[0]:
        text = text.replace(block + "\n", block + "\n" + extra, 1)
    for tool in TOOLS:
        section = f'[mcp_servers.{NAME}.tools.{tool["name"]}]'
        if section not in text:
            text = text.rstrip("\n") + f'\n\n{section}\napproval_mode = "approve"\n'
    path.write_text(text)


def install(arguments):
    options = dict(zip(arguments[::2], arguments[1::2]))
    if len(arguments) % 2 or "--url" not in options or "--token-file" not in options or set(options) - {"--url", "--token-file", "--access-file"}:
        raise SystemExit("usage: pipe_mcp.py install --url URL --token-file FILE [--access-file FILE]")
    url = options["--url"].rstrip("/")
    if not re.fullmatch(r"https://[^/\s]+|http://(127\.0\.0\.1|localhost)(:\d+)?", url):
        raise SystemExit("--url must be https, or http on loopback")
    config = {"url": url, "token": Path(options["--token-file"]).read_text().strip()}
    if "--access-file" in options:
        config["access_headers"] = json.loads(Path(options["--access-file"]).read_text())
    target = Path.home() / ".local/share/pipe/mcp.py"
    target.parent.mkdir(parents=True, exist_ok=True)
    if Path(__file__).resolve() != target.resolve():
        shutil.copy2(__file__, target)
    path = config_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(descriptor, "w") as file:
        json.dump(config, file)
    os.chmod(path, 0o600)
    command = [shutil.which("python3") or "python3", str(target), "serve"]
    installed = []
    if shutil.which("claude"):
        subprocess.run(["claude", "mcp", "remove", "-s", "user", NAME], capture_output=True)
        subprocess.run(["claude", "mcp", "add", "-s", "user", NAME, "--", *command, "claude"], check=True, capture_output=True)
        installed.append("claude")
    if shutil.which("codex"):
        register_codex(command)
        installed.append("codex")
    print(f"installed {NAME} for: {', '.join(installed) or 'no runtime found'}")


def main():
    arguments = sys.argv[1:]
    if arguments[:1] == ["install"]:
        install(arguments[1:])
    elif len(arguments) == 2 and arguments[0] == "serve":
        serve(arguments[1])
    else:
        raise SystemExit("usage: pipe_mcp.py install ... | serve <runtime>")


if __name__ == "__main__":
    main()
