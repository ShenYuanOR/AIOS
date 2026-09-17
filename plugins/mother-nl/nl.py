#!/usr/bin/env python3
"""Mother dialogue-CLI host. Identity lives in *.md; this file only assembles."""
import json
import os
import re
import socket
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
PLUGINS = os.environ.get("AIOS_PLUGIN_DIR") or os.path.dirname(HERE)
SESSION_DIR = os.environ.get("AIOS_SESSION_DIR") or "/var/lib/motherd/sessions/aios/active"
ROUTER = os.path.join(PLUGINS, "model-router", "router.py")
SOCK = "/run/motherd/mother.sock"
BUDGET = 24000
CORE_CAP = 8000
LOOK_CAP = 256 * 1024
DO_BARE = {"/help", "/status"}
DO_PROJ = {"/spawn", "/attach", "/idle", "/reap", "/archive"}
PROJ_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_.-]{0,63}$")
ARG_RE = re.compile(r"^[A-Za-z0-9_./:=,+@%-]+$")


def sock_line(line: str) -> dict:
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect(SOCK)
    s.sendall((line.strip() + "\n").encode())
    data = b""
    while True:
        chunk = s.recv(65536)
        if not chunk:
            break
        data += chunk
    s.close()
    try:
        return json.loads(data.decode())
    except json.JSONDecodeError:
        return {"ok": False, "error": "bad core", "body": {}}


def mother(req: dict) -> dict:
    return sock_line(json.dumps(req, ensure_ascii=False))


def quota_body(kind: str) -> dict:
    try:
        r = mother(
            {
                "op": "quota",
                "project_id": "",
                "room_id": "",
                "cap": kind,
                "args": {"kind": kind},
                "secret_id": "",
            }
        )
    except (OSError, json.JSONDecodeError):
        return {}
    if not r.get("ok"):
        return {}
    body = r.get("body") or {}
    return body if isinstance(body, dict) else {}


def read_md(path: str) -> str:
    try:
        return open(path, encoding="utf-8").read().strip()
    except OSError:
        return ""


def ledger_path() -> str:
    return os.path.join(SESSION_DIR, "turns.jsonl")


def load_turns() -> list:
    p = ledger_path()
    out = []
    try:
        with open(p, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    o = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if isinstance(o, dict) and o.get("user"):
                    out.append(o)
    except OSError:
        pass
    return out


def _fix_perm(path: str, mode: int) -> None:
    try:
        os.chmod(path, mode)
    except OSError:
        pass
    try:
        import grp

        os.chown(path, -1, grp.getgrnam("mother").gr_gid)
    except (OSError, KeyError, ImportError):
        pass


def append_turn(rec: dict) -> None:
    os.makedirs(SESSION_DIR, exist_ok=True)
    _fix_perm(SESSION_DIR, 0o770)
    rec = dict(rec)
    rec["ts"] = int(time.time())
    p = ledger_path()
    with open(p, "a", encoding="utf-8") as f:
        f.write(json.dumps(rec, ensure_ascii=False) + "\n")
    _fix_perm(p, 0o660)


def nbytes(s: str) -> int:
    return len((s or "").encode("utf-8"))


def catalog_text(help_b: dict, status_b: dict) -> str:
    lines = ["核 slash（do 可用）"]
    core = help_b.get("core") or []
    if isinstance(core, list) and core:
        for c in core:
            if not isinstance(c, dict):
                continue
            name = c.get("name") or ""
            summary = c.get("summary") or ""
            lines.append(f"  {name} {summary}".rstrip())
    else:
        lines.append("  /help /status /spawn /attach /idle /reap /archive")
    run = help_b.get("host_run")
    lines.append("家里 run（do 可用；PATH 下任意命令名，以 aios 跑 argv，不是 bash）")
    if isinstance(run, dict) and run.get("any_basename"):
        path = run.get("path") or "/run/current-system/sw/bin"
        user = run.get("user") or "aios"
        lines.append(f"  用户 {user}；命令名须在 {path}；不要路径、管道、sh -c")
        lines.append("  例  run date  /  run cat /etc/os-release  /  run ps aux")
    elif isinstance(run, list) and run:
        names = [x for x in run if isinstance(x, str) and x]
        lines.append("  " + " ".join(names) if names else "  （无）")
    else:
        lines.append("  命令名须在 /run/current-system/sw/bin；不要路径、管道、sh -c")
    lines.append("插件命令（人读；run 可进 do，其余不要放进 do）")
    plugs = help_b.get("plugins") or []
    if isinstance(plugs, list) and plugs:
        for c in plugs:
            if not isinstance(c, dict):
                continue
            name = c.get("name") or "-"
            src = c.get("source") or "-"
            summary = c.get("summary") or ""
            lines.append(f"  {name} ← {src} {summary}".rstrip())
    else:
        lines.append("  （无）")
    lines.append("已挂插件（目录；skill.md 仅按需装）")
    loaded = help_b.get("loaded") or []
    if isinstance(loaded, list) and loaded:
        for c in loaded:
            if not isinstance(c, dict):
                continue
            name = c.get("name") or "-"
            nl = (c.get("nl") or "").strip()
            lines.append(f"  {name}：{nl}" if nl else f"  {name}")
    else:
        lines.append("  （无）")
    lines.append("房间（活数据）")
    leases = status_b.get("leases") or []
    if isinstance(leases, list) and leases:
        for l in leases:
            if not isinstance(l, dict):
                continue
            lines.append(f"  {l.get('project_id') or '-'} {l.get('state') or '-'}")
    else:
        lines.append("  （无）")
    return "\n".join(lines)


def select_skills(user: str, help_b: dict) -> list:
    user_l = user.lower()
    loaded = help_b.get("loaded") or []
    commands = help_b.get("plugins") or []
    names = []
    for c in loaded:
        if not isinstance(c, dict):
            continue
        name = (c.get("name") or "").strip()
        if name:
            names.append(name)
    want = set()
    for name in names:
        if name.lower() in user_l:
            want.add(name)
    for c in commands:
        if not isinstance(c, dict):
            continue
        cmd = (c.get("name") or "").strip()
        src = (c.get("source") or "").strip()
        if cmd and cmd.lower() in user_l and src:
            want.add(src)
    out = []
    for name in names:
        if name not in want:
            continue
        body = read_md(os.path.join(PLUGINS, name, "skill.md"))
        if body:
            out.append((name, body))
    return out


def assemble_system(help_b: dict, status_b: dict, skills: list, omitted: int) -> str:
    ident = read_md(os.path.join(HERE, "identity.md"))
    op = read_md(os.path.join(HERE, "operator.md"))
    if not ident:
        return ""
    parts = [ident, "", op, "", catalog_text(help_b, status_b)]
    if skills:
        parts.append("")
        parts.append("本轮装入的 skill.md")
        for name, body in skills:
            parts.append(f"--- {name} ---")
            parts.append(body)
    if omitted:
        parts.append("")
        parts.append(f"更早 {omitted} 轮在会话账本，未进本轮工作集。点名可调。不要编造未装入的历史。")
    return "\n".join(parts).strip()


def fit_working_set(user: str, help_b: dict, status_b: dict, turns: list):
    skills = select_skills(user, help_b)
    ident = read_md(os.path.join(HERE, "identity.md"))
    if not ident:
        return None, "identity.md missing"

    def size(sk, included, om):
        sys_t = assemble_system(help_b, status_b, sk, om)
        n = nbytes(sys_t) + nbytes(user)
        for t in included:
            n += nbytes(t.get("user") or "") + nbytes(t.get("say") or "")
            d = t.get("do")
            if d:
                n += nbytes(str(d))
            n += nbytes(core_snippet(t.get("core")))
        return n, sys_t

    included = list(turns)
    omitted = 0
    sk = skills
    n, sys_t = size(sk, included, omitted)
    while n > BUDGET and sk:
        sk = sk[:-1]
        n, sys_t = size(sk, included, omitted)
    while n > BUDGET and included:
        included = included[1:]
        omitted = len(turns) - len(included)
        n, sys_t = size(sk, included, omitted)
    n, sys_t = size(sk, included, omitted)
    if n > BUDGET or not sys_t:
        return None, "工作集装不下 identity/目录，本轮失败（未暗截）。"
    return {"system": sys_t, "included": included}, None


def clip_complete_lines(text: str, cap: int) -> str:
    text = text or ""
    raw = text.encode("utf-8")
    if len(raw) <= cap:
        return text
    lines = text.splitlines(keepends=True)
    if len(lines) <= 1:
        cut = raw[:cap]
        sp = cut.rfind(b" ")
        if sp > cap // 2:
            cut = cut[:sp]
        return cut.decode("utf-8", "ignore") + "…（本行过长已截，未完；不要把半截当字段）"

    def enc(s: str) -> int:
        return len(s.encode("utf-8"))

    note_tmpl = "…（中间 {n} 行未装入；以下均为完整行，不要把半行当字段）\n"
    reserve = enc(note_tmpl.format(n=len(lines))) + 8
    budget = cap - reserve
    if budget < 64:
        budget = max(32, cap // 2)

    kept_head: list[str] = []
    body = list(lines)
    head_budget = min(768, max(64, budget // 4))
    used_head = 0
    while body and len(kept_head) < 4:
        n = enc(body[0])
        if used_head + n > head_budget:
            break
        kept_head.append(body.pop(0))
        used_head += n
    used = used_head
    kept_tail: list[str] = []
    for line in reversed(body):
        n = enc(line)
        if used + n > budget:
            break
        kept_tail.append(line)
        used += n
    kept_tail.reverse()
    omitted = len(lines) - len(kept_head) - len(kept_tail)
    if omitted <= 0:
        return "".join(kept_head + kept_tail)
    note = note_tmpl.format(n=omitted)
    return "".join(kept_head) + note + "".join(kept_tail)


def core_snippet(core, cap: int = CORE_CAP) -> str:
    if not isinstance(core, dict):
        return ""
    lines = []
    if core.get("ok") is False:
        lines.append(f"失败 {core.get('error') or 'unknown'}")
    body = core.get("body")
    if isinstance(body, dict):
        argv = body.get("argv")
        if isinstance(argv, list) and argv:
            names = " ".join(str(x) for x in argv)
            nline = 0
            stdout = body.get("stdout") or ""
            stderr = body.get("stderr") or ""
            if stdout:
                nline = stdout.count("\n") + (0 if stdout.endswith("\n") or not stdout else 1)
            lines.append(f"exit {body.get('exit', '')}  {names}  {nline}行".rstrip())
            if body.get("truncated"):
                lines.append("（核输出已截断，只含完整行）")
            if stdout:
                lines.append(stdout)
            if stderr:
                lines.append("stderr:\n" + stderr)
        else:
            try:
                lines.append(json.dumps(body, ensure_ascii=False))
            except (TypeError, ValueError):
                lines.append(str(body))
    text = "\n".join(lines).strip()
    return clip_complete_lines(text, cap)


def core_run_detail(do: str, core) -> str:
    if not isinstance(core, dict):
        return do
    body = core.get("body") if isinstance(core.get("body"), dict) else {}
    argv = body.get("argv") if isinstance(body, dict) else None
    if not (isinstance(argv, list) and argv):
        errn = core.get("error")
        return f"{do}  失败 {errn}" if errn else do
    stdout = body.get("stdout") or ""
    nline = stdout.count("\n") + (0 if stdout.endswith("\n") or not stdout else 1)
    bit = f"{do}  exit {body.get('exit', '')}  {nline}行"
    if body.get("truncated"):
        bit += "  核截断"
    return bit


def history_messages(included: list) -> list:
    msgs = []
    for t in included:
        u = (t.get("user") or "").strip()
        s = (t.get("say") or "").strip()
        if u:
            msgs.append({"role": "user", "content": u})
        d = t.get("do")
        snippet = core_snippet(t.get("core"))
        if s or d or snippet:
            content = s
            if d:
                content = f"{content}\n(do {d})".strip()
            if snippet:
                content = f"{content}\n(结果)\n{snippet}".strip()
            msgs.append({"role": "assistant", "content": content})
    return msgs


def extract_obj(raw: str):
    raw = (raw or "").strip()
    if raw.startswith("```"):
        raw = re.sub(r"^```(?:json)?\s*", "", raw)
        raw = re.sub(r"\s*```$", "", raw)
    i = raw.find("{")
    j = raw.rfind("}")
    if i < 0 or j <= i:
        return None
    try:
        o = json.loads(raw[i : j + 1])
    except json.JSONDecodeError:
        return None
    return o if isinstance(o, dict) else None


def parse_do(do) -> str | None:
    if not isinstance(do, str):
        return None
    line = " ".join(do.split())
    if not line or any(x in line for x in ";|&\n$`"):
        return None
    parts = line.split(" ")
    cmd = parts[0]
    if cmd in DO_BARE:
        return cmd if len(parts) == 1 else None
    if cmd in DO_PROJ and len(parts) == 2 and PROJ_RE.match(parts[1]):
        return f"{cmd} {parts[1]}"
    if cmd == "run":
        if len(parts) == 1:
            return "run"
        if len(parts) > 9:
            return None
        name = parts[1]
        if "/" in name or name.startswith("."):
            return None
        for a in parts[1:]:
            if not ARG_RE.match(a):
                return None
        return " ".join(parts)
    return None


def allow_do(user: str, do) -> str | None:
    parsed = parse_do(do)
    if not parsed:
        return None
    parts = parsed.split()
    if parts[0] == "run":
        return parsed
    if len(parts) == 2 and parts[1] not in user:
        return None
    return parsed


def exec_do(do: str) -> dict:
    parts = do.split()
    if parts and parts[0] == "run":
        return mother(
            {
                "op": "cap_call",
                "project_id": "",
                "room_id": "",
                "cap": "host.run",
                "args": {"argv": parts[1:]},
                "secret_id": "",
            }
        )
    return sock_line(do)


def progress(msg: str) -> None:
    print(f"#progress {msg}", file=sys.stderr, flush=True)


def emit_step(op: str, key: str, title: str = "", detail: str = "", ok: bool = True) -> None:
    rec = {"op": op, "key": key, "ok": ok}
    if title:
        rec["title"] = title
    if detail:
        rec["detail"] = detail
    print(f"#step {json.dumps(rec, ensure_ascii=False)}", file=sys.stderr, flush=True)
    if op == "start":
        msg = title if not detail else f"{title}  {detail}"
        if msg:
            progress(msg)


def chat_label() -> str:
    p = "/var/lib/motherd/model-alias.json"
    try:
        with open(p, encoding="utf-8") as f:
            a = json.load(f)
        name = a.get("chat-mother") if isinstance(a, dict) else None
        if isinstance(name, str) and name.strip():
            return name.strip()
    except (OSError, json.JSONDecodeError):
        pass
    return "模型"


def envelope(say: str, do, core):
    print(json.dumps({"say": say or "", "do": do, "core": core}, ensure_ascii=False), flush=True)


def chat_mother(messages: list) -> tuple[dict, str]:
    py = sys.executable or "python3"
    p = subprocess.run(
        [py, ROUTER, "chat-mother"],
        input=json.dumps({"messages": messages}, ensure_ascii=False).encode(),
        capture_output=True,
    )
    raw = (p.stdout or b"").decode("utf-8", "replace").strip()
    try:
        data = json.loads(raw) if raw else {}
    except json.JSONDecodeError:
        data = {}
    return data, raw


def choice_text(data: dict) -> str:
    payload = data.get("payload") or {}
    choices = payload.get("choices") or []
    if not choices:
        return ""
    return ((choices[0] or {}).get("message") or {}).get("content") or ""


def look_result(system: str, included: list, user: str, do: str, core: dict) -> str | None:
    snippet = core_snippet(core, cap=LOOK_CAP)
    if not snippet:
        return None
    messages = [{"role": "system", "content": system}]
    messages.extend(history_messages(included))
    messages.append({"role": "user", "content": user})
    messages.append({"role": "assistant", "content": f"(do {do})"})
    messages.append(
        {
            "role": "user",
            "content": (
                "结果原文如下。按行分隔，一行一条记录；行内空格才是字段。"
                "不要把同一条被显示折开的半行当成新记录或新字段。"
                "只根据完整行下结论。若文中标明有未装入的行，如实说还缺哪一段，不要猜被裁掉的内容。"
                "根据结果回答刚才那句。"
                '只输出 JSON：{"say":"中文结论","do":null}。'
                "不要再交 do。不要整段复述原文。\n\n"
                + snippet
            ),
        }
    )
    data, _raw = chat_mother(messages)
    if not data.get("ok"):
        return None
    obj = extract_obj(choice_text(data))
    if not obj:
        return None
    say = obj.get("say")
    if isinstance(say, str) and say.strip():
        return say.strip()
    return None


def main() -> int:
    text = sys.stdin.read().strip()
    if text.startswith("/"):
        print("slash is core; NL plugin ignores slash", file=sys.stderr)
        return 2
    if not os.path.isfile(ROUTER):
        print(json.dumps({"ok": False, "error": "no_model", "degraded": True, "hint": "slash still works"}))
        return 0
    help_b = quota_body("help")
    status_b = quota_body("status")
    turns = load_turns()
    ws, err = fit_working_set(text, help_b, status_b, turns)
    if not ws:
        envelope(err or "工作集失败", None, None)
        return 0
    messages = [{"role": "system", "content": ws["system"]}]
    messages.extend(history_messages(ws["included"]))
    messages.append({"role": "user", "content": text})
    label = chat_label()
    emit_step("start", "think", "推理", label)
    data, raw = chat_mother(messages)
    model = data.get("model") if isinstance(data, dict) else None
    if isinstance(model, str) and model.strip() and model.strip() != "default":
        emit_step("start", "think", "推理", f"{label} / {model.strip()}")
    emit_step("end", "think", ok=bool(data.get("ok")))
    if not data.get("ok"):
        errn = data.get("error") or "no_model"
        out = {
            "ok": False,
            "error": errn,
            "hint": "slash still works",
        }
        if data.get("degraded") or errn == "no_model":
            out["degraded"] = True
        print(json.dumps(out))
        return 0
    msg = choice_text(data)
    obj = extract_obj(msg)
    if not obj:
        say = msg.strip() or raw
        envelope(say, None, None)
        append_turn({"user": text, "say": say, "do": None, "core": None})
        return 0
    say = obj.get("say")
    if not isinstance(say, str) or not say.strip():
        say = msg.strip() or "哥哥，核这边没有可说的。"
    do = allow_do(text, obj.get("do"))
    core = None
    if do:
        emit_step("start", "run", "执行", do)
        try:
            core = exec_do(do)
        except OSError as e:
            core = {"ok": False, "error": str(e), "body": {}}
        run_ok = bool(core.get("ok")) if isinstance(core, dict) else False
        emit_step("start", "run", "执行", core_run_detail(do, core))
        emit_step("end", "run", ok=run_ok)
        emit_step("start", "look", "消化", f"回看 {label}")
        looked = look_result(ws["system"], ws["included"], text, do, core)
        emit_step("end", "look", ok=bool(looked))
        if looked:
            say = looked
    say = say.strip()
    envelope(say, do, core)
    append_turn({"user": text, "say": say, "do": do, "core": core})
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
