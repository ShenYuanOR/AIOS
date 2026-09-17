#!/usr/bin/env python3
"""Map vault ids to Letta container env. Never prints secret values."""
import json
import os
import socket
from pathlib import Path

SOCK = "/run/motherd/mother.sock"
OUT = Path("/var/lib/motherd/letta.env")


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
    return json.loads(data.decode() or "{}")


def secret(sid: str) -> str:
    r = mother({"op": "secret_get", "secret_id": sid, "project_id": "", "room_id": "", "cap": "", "args": {}})
    if not r.get("ok"):
        return ""
    return (r.get("body") or {}).get("value") or ""


def main() -> int:
    gpt = secret("model.gpt.api_key")
    grok = secret("model.grok.api_key")
    ds = secret("model.deepseek.api_key")
    oc = secret("model.openai_compat.api_key")
    base = secret("model.openai_compat.base_url")
    key = gpt or oc or grok or ds
    lines = []
    if key:
        lines.append(f"OPENAI_API_KEY={key}")
    if base:
        lines.append(f"OPENAI_API_BASE={base.rstrip('/')}")
    elif grok and not gpt:
        lines.append("OPENAI_API_BASE=https://api.x.ai/v1")
    elif ds and not gpt and not grok:
        lines.append("OPENAI_API_BASE=https://api.deepseek.com/v1")
    OUT.parent.mkdir(parents=True, exist_ok=True)
    tmp = OUT.with_suffix(".tmp")
    tmp.write_text("\n".join(lines) + ("\n" if lines else ""), encoding="utf-8")
    os.chmod(tmp, 0o600)
    tmp.replace(OUT)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
