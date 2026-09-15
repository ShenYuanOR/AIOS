#!/usr/bin/env python3
import json
import socket
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

SOCK = "/run/motherd/mother.sock"
HTML = b"""<!doctype html><meta charset=utf-8><title>AIOS</title>
<body>
<h1>AIOS cockpit</h1>
<p>buttons talk to motherd. slash is core.</p>
<button onclick="go('/status')">/status</button>
<pre id=o></pre>
<script>
async function go(s){
  const r=await fetch('/slash',{method:'POST',body:s});
  o.textContent=await r.text();
}
</script>
</body>"""


def call(line: str) -> bytes:
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect(SOCK)
    s.sendall((line.strip() + "\n").encode())
    data = b""
    while True:
        c = s.recv(65536)
        if not c:
            break
        data += c
    s.close()
    return data


class H(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/":
            self.send_response(200)
            self.send_header("content-type", "text/html; charset=utf-8")
            self.send_header("content-length", str(len(HTML)))
            self.end_headers()
            self.wfile.write(HTML)
            return
        self.send_response(404)
        self.end_headers()

    def do_POST(self):
        n = int(self.headers.get("content-length") or 0)
        body = self.rfile.read(n).decode()
        if self.path == "/slash":
            try:
                out = call(body)
            except OSError:
                out = json.dumps({"ok": False, "error": "motherd down"}).encode()
            self.send_response(200)
            self.send_header("content-type", "application/json")
            self.send_header("content-length", str(len(out)))
            self.end_headers()
            self.wfile.write(out)
            return
        self.send_response(404)
        self.end_headers()

    def log_message(self, *a):
        return


if __name__ == "__main__":
    ThreadingHTTPServer(("0.0.0.0", 7460), H).serve_forever()
