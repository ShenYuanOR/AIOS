#!/usr/bin/env python3
"""Letta-facing shim on aios. Stingy ingest; decisions still belong in Gitea."""
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

MEM = []


class H(BaseHTTPRequestHandler):
    def _json(self, code, obj):
        b = json.dumps(obj).encode()
        self.send_response(code)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(b)))
        self.end_headers()
        self.wfile.write(b)

    def do_GET(self):
        if self.path == "/health":
            self._json(200, {"ok": True, "engine": "letta-shim", "sleep_time": True})
            return
        if self.path == "/memory":
            self._json(200, {"items": MEM[-32:]})
            return
        self._json(404, {"error": "not found"})

    def do_POST(self):
        n = int(self.headers.get("content-length") or 0)
        body = json.loads(self.rfile.read(n) or b"{}")
        if self.path == "/ingest":
            text = (body.get("text") or "").strip()
            if len(text) < 24:
                self._json(200, {"ingested": False, "reason": "stingy"})
                return
            MEM.append({"text": text[:512]})
            self._json(200, {"ingested": True})
            return
        self._json(404, {"error": "not found"})

    def log_message(self, *a):
        return


if __name__ == "__main__":
    ThreadingHTTPServer(("127.0.0.1", 8283), H).serve_forever()
