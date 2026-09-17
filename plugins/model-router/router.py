#!/usr/bin/env python3
"""Four adapters: grok, gpt, deepseek, openai_compat. Empty vault keys degrade."""
import json
import os
import socket
import sys
import urllib.error
import urllib.request

SOCK = "/run/motherd/mother.sock"
ALIAS_FILE = "/var/lib/motherd/model-alias.json"
DEFAULTS = {
    "grok": "https://api.x.ai/v1",
    "gpt": "https://api.openai.com/v1",
    "deepseek": "https://api.deepseek.com/v1",
    "openai_compat": "",
}


def load_alias_file() -> dict:
    try:
        with open(ALIAS_FILE, encoding="utf-8") as f:
            j = json.load(f)
        return j if isinstance(j, dict) else {}
    except (OSError, json.JSONDecodeError):
        return {}


def aliases() -> dict:
    d = {
        "chat-mother": os.environ.get("AIOS_ALIAS_MOTHER", "grok"),
        "chat-code": os.environ.get("AIOS_ALIAS_CODE", "gpt"),
        "cheap": os.environ.get("AIOS_ALIAS_CHEAP", "deepseek"),
    }
    j = load_alias_file()
    for k in list(d):
        v = j.get(k)
        if v in DEFAULTS:
            d[k] = v
    return d


def model_id(adapter: str, alias: str = "") -> str:
    env = os.environ.get("AIOS_MODEL_" + adapter.upper())
    if env:
        return env
    ids = load_alias_file().get("model_ids") or {}
    if isinstance(ids, dict):
        if alias and ids.get(alias):
            return str(ids[alias])
        if ids.get(adapter):
            return str(ids[adapter])
    return "default"


def mother(req: dict) -> dict:
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect(SOCK)
    s.sendall((json.dumps(req) + "\n").encode())
    data = b""
    while True:
        chunk = s.recv(65536)
        if not chunk:
            break
        data += chunk
    s.close()
    return json.loads(data.decode())


def secret(sid: str) -> str | None:
    r = mother({"op": "secret_get", "secret_id": sid, "project_id": "", "room_id": "", "cap": "", "args": {}})
    if not r.get("ok"):
        return None
    v = r.get("body", {}).get("value")
    return v if v else None


def chat(adapter: str, messages: list, alias: str = "") -> dict:
    if adapter not in DEFAULTS:
        return {"ok": False, "error": f"unknown adapter {adapter}"}
    key = secret(f"model.{adapter}.api_key")
    base = DEFAULTS[adapter]
    if adapter == "openai_compat":
        base = secret("model.openai_compat.base_url") or ""
    if not key or not base:
        return {"ok": False, "error": "no_model", "degraded": True, "adapter": adapter}
    mid = model_id(adapter, alias)
    body = json.dumps({"model": mid, "messages": messages}).encode()
    req = urllib.request.Request(
        base.rstrip("/") + "/chat/completions",
        data=body,
        headers={"Authorization": f"Bearer {key}", "Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            payload = json.loads(resp.read().decode())
        mother({"op": "audit", "args": {"event": "model_visible", "adapter": adapter, "model": mid}, "project_id": "", "room_id": "", "cap": "", "secret_id": ""})
        return {"ok": True, "adapter": adapter, "model": mid, "payload": payload}
    except urllib.error.HTTPError as e:
        raw = e.read().decode("utf-8", "replace")
        msg = raw
        try:
            err = json.loads(raw).get("error")
            if isinstance(err, dict):
                msg = err.get("message") or raw
            elif isinstance(err, str):
                msg = err
        except json.JSONDecodeError:
            pass
        return {"ok": False, "error": f"{e.code} {msg}", "adapter": adapter, "model": mid}
    except urllib.error.URLError as e:
        return {"ok": False, "error": str(e), "adapter": adapter, "model": mid}


def main() -> int:
    alias = sys.argv[1] if len(sys.argv) > 1 else "chat-mother"
    table = aliases()
    adapter = table.get(alias, alias)
    raw = sys.stdin.read() or "ping"
    messages = None
    try:
        obj = json.loads(raw)
        if isinstance(obj, dict) and isinstance(obj.get("messages"), list) and obj["messages"]:
            messages = obj["messages"]
    except json.JSONDecodeError:
        pass
    if messages is None:
        messages = [{"role": "user", "content": raw}]
    out = chat(adapter, messages, alias=alias)
    json.dump(out, sys.stdout)
    sys.stdout.write("\n")
    return 0 if out.get("ok") or out.get("degraded") else 1


if __name__ == "__main__":
    raise SystemExit(main())
