#!/usr/bin/env python3
"""Four adapters: grok, gpt, deepseek, openai_compat. Empty vault keys degrade."""
import json
import os
import socket
import sys
import urllib.error
import urllib.request

SOCK = "/run/motherd/mother.sock"
DEFAULTS = {
    "grok": "https://api.x.ai/v1",
    "gpt": "https://api.openai.com/v1",
    "deepseek": "https://api.deepseek.com/v1",
    "openai_compat": "",
}
ALIAS = {
    "chat-mother": os.environ.get("AIOS_ALIAS_MOTHER", "grok"),
    "chat-code": os.environ.get("AIOS_ALIAS_CODE", "gpt"),
    "cheap": os.environ.get("AIOS_ALIAS_CHEAP", "deepseek"),
}


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


def chat(adapter: str, messages: list) -> dict:
    if adapter not in DEFAULTS:
        return {"ok": False, "error": f"unknown adapter {adapter}"}
    key = secret(f"model.{adapter}.api_key")
    base = DEFAULTS[adapter]
    if adapter == "openai_compat":
        base = secret("model.openai_compat.base_url") or ""
    if not key or not base:
        return {"ok": False, "error": "no_model", "degraded": True, "adapter": adapter}
    body = json.dumps({"model": os.environ.get("AIOS_MODEL_" + adapter.upper(), "default"), "messages": messages}).encode()
    req = urllib.request.Request(
        base.rstrip("/") + "/chat/completions",
        data=body,
        headers={"Authorization": f"Bearer {key}", "Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            payload = json.loads(resp.read().decode())
        mother({"op": "audit", "args": {"event": "model_visible", "adapter": adapter}, "project_id": "", "room_id": "", "cap": "", "secret_id": ""})
        return {"ok": True, "adapter": adapter, "payload": payload}
    except urllib.error.URLError as e:
        return {"ok": False, "error": str(e), "adapter": adapter}


def main() -> int:
    alias = sys.argv[1] if len(sys.argv) > 1 else "chat-mother"
    adapter = ALIAS.get(alias, alias)
    msg = sys.stdin.read() or "ping"
    out = chat(adapter, [{"role": "user", "content": msg}])
    json.dump(out, sys.stdout)
    sys.stdout.write("\n")
    return 0 if out.get("ok") or out.get("degraded") else 1


if __name__ == "__main__":
    raise SystemExit(main())
