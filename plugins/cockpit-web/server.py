#!/usr/bin/env python3
import json
import os
import socket
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

SOCK = "/run/motherd/mother.sock"
ALIAS_FILE = Path("/var/lib/motherd/model-alias.json")
VAULT_IN = Path("/var/lib/motherd/vault-in")
ADAPTERS = ("grok", "gpt", "deepseek", "openai_compat")
HTML = """<!doctype html><meta charset=utf-8><title>AIOS 机舱</title>
<style>
body{font:16px/1.45 system-ui,sans-serif;max-width:56rem;margin:2rem auto;padding:0 1rem;color:#111}
h1{font-size:1.4rem;margin:0 0 .4rem}
h2{font-size:1.05rem;margin:1.4rem 0 .5rem}
.muted{color:#555;margin:0 0 1rem}
button,input,select{font:inherit;margin:.15rem .2rem .15rem 0;padding:.35rem .6rem}
pre{background:#111;color:#eee;padding:1rem;min-height:7rem;white-space:pre-wrap}
label{display:inline-block;margin-right:.6rem}
hr{border:0;border-top:1px solid #ddd;margin:1.4rem 0}
</style>
<h1>AIOS 机舱</h1>
<p class=muted>按钮走核 slash。进房请新开终端 SSH，母不当 PTY。LAN only。</p>
<p>
<button onclick="go('/status')">/status</button>
<button onclick="go('/spawn '+id())">/spawn</button>
<button onclick="go('/attach '+id())">/attach</button>
<button onclick="go('/idle '+id())">/idle</button>
<button onclick="go('/reap '+id())">/reap</button>
<button onclick="go('/archive '+id())">/archive</button>
</p>
<p><label>项目 <input id=p value=hello></label></p>
<pre id=o></pre>
<hr>
<h2>模型</h2>
<p class=muted>不是 slash。别名进文件；钥进哑柜。</p>
<pre id=m>loading</pre>
<p>
<label>母闲聊
<select id=am>
  <option>grok</option><option>gpt</option><option>deepseek</option><option>openai_compat</option>
</select></label>
<button onclick="alias('chat-mother',am.value)">保存</button>
</p>
<p>
<label>房内代码
<select id=ac>
  <option>grok</option><option>gpt</option><option>deepseek</option><option>openai_compat</option>
</select></label>
<button onclick="alias('chat-code',ac.value)">保存</button>
</p>
<p>
<label>便宜通道
<select id=aq>
  <option>grok</option><option>gpt</option><option>deepseek</option><option>openai_compat</option>
</select></label>
<button onclick="alias('cheap',aq.value)">保存</button>
</p>
<p>
<label>适配器
<select id=ad>
  <option>grok</option><option>gpt</option><option>deepseek</option><option>openai_compat</option>
</select></label>
<label>钥 <input id=k type=password autocomplete=off></label>
<button onclick="putKey()">投钥</button>
</p>
<p>
<label>compat url <input id=u size=40 placeholder=https://api.example.com/v1></label>
<button onclick="putUrl()">保存</button>
</p>
<script>
function id(){return document.getElementById('p').value.trim()||'hello'}
function pretty(t){
  try{return JSON.stringify(JSON.parse(t),null,2)}catch(e){return t}
}
async function go(s){
  const r=await fetch('/slash',{method:'POST',body:s});
  o.textContent=s+'\\n'+pretty(await r.text());
}
async function refresh(){
  const r=await fetch('/model');
  const j=await r.json();
  const a=j.aliases||{};
  am.value=a['chat-mother']||'grok';
  ac.value=a['chat-code']||'gpt';
  aq.value=a['cheap']||'deepseek';
  const keys=j.keys||{};
  const lines=['别名'];
  lines.push('  母闲聊  '+(a['chat-mother']||'-'));
  lines.push('  房内代码  '+(a['chat-code']||'-'));
  lines.push('  便宜通道  '+(a['cheap']||'-'));
  lines.push('钥');
  for (const n of Object.keys(keys)) lines.push('  '+n+'  '+(keys[n]?'有':'无'));
  lines.push('compat url  '+(j.compat_url?'有':'无'));
  m.textContent=lines.join('\\n');
}
async function alias(name,adapter){
  await fetch('/model',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({alias:name,adapter})});
  refresh();
}
async function putKey(){
  const adapter=ad.value, key=k.value;
  k.value='';
  await fetch('/model',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({adapter,key})});
  refresh();
}
async function putUrl(){
  await fetch('/model',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({base_url:u.value.trim()})});
  refresh();
}
go('/status');
refresh();
</script>
""".encode()


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


def mother(req: dict) -> dict:
    raw = call(json.dumps(req)).decode()
    try:
        return json.loads(raw or "{}")
    except json.JSONDecodeError:
        return {"ok": False, "error": raw}


def secret_on(sid: str) -> bool:
    r = mother({"op": "secret_get", "secret_id": sid, "project_id": "", "room_id": "", "cap": "", "args": {}})
    if not r.get("ok"):
        return False
    v = (r.get("body") or {}).get("value")
    return bool(v)


def load_aliases() -> dict:
    d = {"chat-mother": "grok", "chat-code": "gpt", "cheap": "deepseek"}
    try:
        j = json.loads(ALIAS_FILE.read_text(encoding="utf-8"))
        for k in d:
            if j.get(k) in ADAPTERS:
                d[k] = j[k]
    except (OSError, json.JSONDecodeError):
        pass
    return d


def save_aliases(d: dict) -> None:
    cur = {}
    try:
        cur = json.loads(ALIAS_FILE.read_text(encoding="utf-8"))
        if not isinstance(cur, dict):
            cur = {}
    except (OSError, json.JSONDecodeError):
        pass
    cur.update(d)
    ALIAS_FILE.write_text(json.dumps(cur) + "\n", encoding="utf-8")
    try:
        os.chmod(ALIAS_FILE, 0o660)
    except OSError:
        pass


def put_vault(sid: str, value: str) -> None:
    VAULT_IN.mkdir(parents=True, exist_ok=True)
    p = VAULT_IN / sid
    p.write_text(value, encoding="utf-8")
    os.chmod(p, 0o660)
    call("/status")


def model_get() -> bytes:
    aliases = load_aliases()
    keys = {a: secret_on(f"model.{a}.api_key") for a in ADAPTERS}
    body = {
        "aliases": aliases,
        "keys": keys,
        "compat_url": secret_on("model.openai_compat.base_url"),
    }
    return json.dumps(body).encode()


def model_post(raw: str) -> bytes:
    try:
        j = json.loads(raw or "{}")
    except json.JSONDecodeError:
        return json.dumps({"ok": False, "error": "bad json"}).encode()
    if j.get("alias") and j.get("adapter"):
        if j["adapter"] not in ADAPTERS:
            return json.dumps({"ok": False, "error": "unknown adapter"}).encode()
        if j["alias"] not in ("chat-mother", "chat-code", "cheap"):
            return json.dumps({"ok": False, "error": "unknown alias"}).encode()
        d = load_aliases()
        d[j["alias"]] = j["adapter"]
        save_aliases(d)
    if j.get("adapter") and j.get("key"):
        if j["adapter"] not in ADAPTERS:
            return json.dumps({"ok": False, "error": "unknown adapter"}).encode()
        put_vault(f"model.{j['adapter']}.api_key", str(j["key"]).strip())
    if j.get("base_url"):
        put_vault("model.openai_compat.base_url", str(j["base_url"]).strip())
    return json.dumps({"ok": True}).encode()


class H(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/":
            self.send_response(200)
            self.send_header("content-type", "text/html; charset=utf-8")
            self.send_header("content-length", str(len(HTML)))
            self.end_headers()
            self.wfile.write(HTML)
            return
        if self.path == "/model":
            out = model_get()
            self.send_response(200)
            self.send_header("content-type", "application/json")
            self.send_header("content-length", str(len(out)))
            self.end_headers()
            self.wfile.write(out)
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
        if self.path == "/model":
            out = model_post(body)
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
