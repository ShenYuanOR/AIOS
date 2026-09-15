#!/usr/bin/env python3
"""In-room MCP mux. Tools change only on next turn via list_changed."""
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

STATE = {"tools": [], "epoch": 0}


class H(BaseHTTPRequestHandler):
    def _json(self, code, obj):
        b = json.dumps(obj).encode()
        self.send_response(code)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(b)))
        self.end_headers()
        self.wfile.write(b)

    def do_GET(self):
        if self.path == "/list":
            self._json(200, {"tools": STATE["tools"], "epoch": STATE["epoch"]})
            return
        if self.path == "/list_changed":
            self._json(200, {"epoch": STATE["epoch"]})
            return
        self._json(404, {"error": "not found"})

    def do_POST(self):
        n = int(self.headers.get("content-length") or 0)
        body = json.loads(self.rfile.read(n) or b"{}")
        if self.path == "/register":
            STATE["tools"] = body.get("tools", STATE["tools"])
            STATE["epoch"] += 1
            self._json(200, {"epoch": STATE["epoch"]})
            return
        self._json(404, {"error": "not found"})

    def log_message(self, fmt, *args):
        return


if __name__ == "__main__":
    ThreadingHTTPServer(("127.0.0.1", 7461), H).serve_forever()
