#!/usr/bin/env python3
"""Stingy ingest in front of official Letta on 127.0.0.1:8283. Plugin down => 503."""
import json
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

LETTA = "http://127.0.0.1:8283"
MIN = 24


def letta(method: str, path: str, body=None, timeout=8):
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(
        LETTA + path,
        data=data,
        method=method,
        headers={"Content-Type": "application/json"} if data else {},
    )
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        raw = resp.read().decode()
        return json.loads(raw) if raw else {}


def health() -> dict:
    for p in ("/v1/health", "/v1/health/", "/health"):
        try:
            letta("GET", p)
            return {"ok": True, "engine": "letta", "tag": "0.16.8", "sleep_time": True}
        except Exception:
            continue
    return {"ok": False, "error": "letta down", "engine": "letta"}


def ingest(text: str) -> dict:
    text = (text or "").strip()
    if len(text) < MIN:
        return {"ingested": False, "reason": "stingy"}
    h = health()
    if not h.get("ok"):
        return {"ingested": False, "error": "503", "reason": "letta down"}
    try:
        letta("POST", "/v1/sources", {"name": "aios-stingy", "embedding": "dummy"}, timeout=15)
    except Exception:
        pass
    try:
        letta("POST", "/v1/passages", {"text": text[:512]}, timeout=15)
        return {"ingested": True, "engine": "letta"}
    except Exception as e:
        return {"ingested": False, "error": str(e), "engine": "letta"}


class H(BaseHTTPRequestHandler):
    def _json(self, code, obj):
        b = json.dumps(obj).encode()
        self.send_response(code)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(b)))
        self.end_headers()
        self.wfile.write(b)

    def do_GET(self):
        if self.path in ("/health", "/v1/health"):
            h = health()
            self._json(200 if h.get("ok") else 503, h)
            return
        self._json(404, {"error": "not found"})

    def do_POST(self):
        n = int(self.headers.get("content-length") or 0)
        body = json.loads(self.rfile.read(n) or b"{}")
        if self.path == "/ingest":
            out = ingest(body.get("text") or "")
            code = 503 if out.get("error") == "503" else 200
            self._json(code, out)
            return
        self._json(404, {"error": "not found"})

    def log_message(self, *a):
        return


if __name__ == "__main__":
    ThreadingHTTPServer(("127.0.0.1", 8284), H).serve_forever()
