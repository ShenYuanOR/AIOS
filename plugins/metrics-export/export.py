#!/usr/bin/env python3
"""Export skin. PSI collection stays in core; this only exposes /metrics text."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


def scrape() -> bytes:
    lines = ["# aios metrics-export plugin", "# sense remains in motherd"]
    for name in ("cpu", "memory", "io"):
        p = Path(f"/proc/pressure/{name}")
        if p.exists():
            raw = p.read_text().replace(" ", ",").strip()
            lines.append(f'psi_{name}{{host="aios"}} 1')
            lines.append(f"# {name} {raw}")
    return ("\n".join(lines) + "\n").encode()


class H(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path != "/metrics":
            self.send_response(404)
            self.end_headers()
            return
        b = scrape()
        self.send_response(200)
        self.send_header("content-type", "text/plain; version=0.0.4")
        self.send_header("content-length", str(len(b)))
        self.end_headers()
        self.wfile.write(b)

    def log_message(self, *a):
        return


if __name__ == "__main__":
    ThreadingHTTPServer(("127.0.0.1", 9105), H).serve_forever()
