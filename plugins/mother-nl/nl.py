#!/usr/bin/env python3
"""Mother smalltalk plugin. Slash stays in the core."""
import json
import subprocess
import sys

def main() -> int:
    text = sys.stdin.read().strip()
    if text.startswith("/"):
        print("slash is core; NL plugin ignores slash", file=sys.stderr)
        return 2
    p = subprocess.run(
        ["python3", "/var/lib/motherd/plugins/model-router/router.py", "chat-mother"],
        input=text.encode(),
        capture_output=True,
    )
    if p.returncode != 0 and not p.stdout:
        print(json.dumps({"ok": False, "error": "no_model", "degraded": True, "hint": "slash still works"}))
        return 0
    sys.stdout.buffer.write(p.stdout or p.stderr)
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
