mod tui;

use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use serde_json::{json, Value};
use std::fs;
use std::io::{self, BufRead, IsTerminal, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;

const ALIAS_FILE: &str = "/var/lib/motherd/model-alias.json";
const VAULT_IN: &str = "/var/lib/motherd/vault-in";
const ADAPTERS: &[&str] = &["grok", "gpt", "deepseek", "openai_compat"];
const ROOM_PORT: u16 = 2222;

fn call(line: &str) -> Result<String, String> {
    let mut s = UnixStream::connect("/run/motherd/mother.sock")
        .map_err(|e| format!("motherd down: {e}"))?;
    s.write_all(format!("{}\n", line.trim()).as_bytes())
        .map_err(|e| e.to_string())?;
    let mut out = String::new();
    s.read_to_string(&mut out).map_err(|e| e.to_string())?;
    Ok(out)
}

fn call_json(v: &Value) -> Result<Value, String> {
    let raw = call(&v.to_string())?;
    serde_json::from_str(raw.trim()).map_err(|e| e.to_string())
}

fn python() -> &'static str {
    if Path::new("/run/current-system/sw/bin/python3").exists() {
        "/run/current-system/sw/bin/python3"
    } else {
        "python3"
    }
}

fn session_dir() -> PathBuf {
    let root = PathBuf::from("/var/lib/motherd/sessions/aios");
    let _ = fs::create_dir_all(&root);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&root, fs::Permissions::from_mode(0o770));
    }
    let cur = root.join("current");
    let sid = fs::read_to_string(&cur)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            let s = "active".to_string();
            let _ = fs::write(&cur, &s);
            s
        });
    let dir = root.join(sid);
    let _ = fs::create_dir_all(&dir);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o770));
    }
    dir
}

fn talk_nl(text: &str) -> Result<String, String> {
    talk_nl_with(text, |_| {})
}

fn talk_nl_with<F>(text: &str, progress: F) -> Result<String, String>
where
    F: Fn(&str) + Send + Sync + 'static,
{
    let script = Path::new("/var/lib/motherd/plugins/mother-nl/nl.py");
    if !script.is_file() {
        return Ok(
            "{\"ok\":false,\"error\":\"503 mother-nl\",\"degraded\":true,\"hint\":\"slash still works\"}\n"
                .into(),
        );
    }
    let progress = Arc::new(progress);
    (*progress)("问模型");
    let mut child = Command::new(python())
        .arg(script)
        .env("AIOS_SESSION_DIR", session_dir())
        .env("AIOS_PLUGIN_DIR", "/var/lib/motherd/plugins")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("mother-nl: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    let mut err_pipe = child.stderr.take();
    let p_err = progress.clone();
    let herr = thread::spawn(move || {
        let mut extra = String::new();
        if let Some(ref mut r) = err_pipe {
            let mut br = io::BufReader::new(r);
            let mut line = String::new();
            while br.read_line(&mut line).unwrap_or(0) > 0 {
                let t = line.trim_end();
                if let Some(msg) = t.strip_prefix("#step ") {
                    (*p_err)(&format!("STEP {msg}"));
                } else if let Some(msg) = t.strip_prefix("#progress ") {
                    (*p_err)(msg.trim());
                } else if !t.is_empty() {
                    extra.push_str(&line);
                }
                line.clear();
            }
        }
        extra
    });
    let mut stdout = String::new();
    if let Some(mut so) = child.stdout.take() {
        so.read_to_string(&mut stdout).map_err(|e| e.to_string())?;
    }
    let _ = child.wait();
    let extra = herr.join().unwrap_or_default();
    if stdout.trim().is_empty() {
        return Ok(if extra.trim().is_empty() {
            "{\"ok\":false,\"error\":\"no_model\",\"degraded\":true,\"hint\":\"slash still works\"}\n"
                .into()
        } else {
            extra
        });
    }
    Ok(stdout)
}

fn state_zh(s: &str) -> &str {
    match s {
        "running" => "在跑",
        "idle" => "空闲",
        "stopped" => "已停",
        "archived" => "已归档",
        "tombstoned" => "墓碑",
        "purged" => "已清除",
        other => other,
    }
}

fn err_zh(err: &str) -> String {
    if let Some(st) = err.strip_prefix("not_attachable:") {
        return format!(
            "不能进房（当前 {}）。只对在跑/空闲房间 /attach。",
            state_zh(st)
        );
    }
    match err {
        "no lease" => "无租约".into(),
        "project_id required" => "需要项目 id".into(),
        "same project already running on this disk" => "同项目已在本盘在跑".into(),
        "503 mother-nl" => "母 NL 插件未挂（卸了会 503；slash 仍可用）".into(),
        "no_model" => "无模型".into(),
        "timeout" => "超时".into(),
        "too many args" => "参数太多".into(),
        "basename only" => "只要命令名，不要路径".into(),
        "no live user" => "家里没人".into(),
        "bad arg" => "参数不合法".into(),
        "cap required" => "需要 cap".into(),
        other if other.starts_with("unregistered:") => {
            format!("未登记命令 {}", other.trim_start_matches("unregistered:"))
        }
        other if other.starts_with("missing ") => format!("系统没有 {}", other.trim_start_matches("missing ")),
        other => other.to_string(),
    }
}

fn skip_ip(ip: &str) -> bool {
    ip.starts_with("10.88.") || ip.starts_with("10.0.2.") || ip.starts_with("172.17.")
}

fn advertise_hosts() -> Vec<String> {
    let mut hosts = Vec::new();
    if let Ok(out) = Command::new("ip")
        .args(["-4", "route", "show", "default"])
        .output()
    {
        let s = String::from_utf8_lossy(&out.stdout);
        for line in s.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(i) = parts.iter().position(|p| *p == "src") {
                if let Some(ip) = parts.get(i + 1) {
                    if !skip_ip(ip) && !hosts.iter().any(|h| h == *ip) {
                        hosts.push((*ip).to_string());
                    }
                }
            }
        }
    }
    if let Ok(out) = Command::new("ip")
        .args(["-4", "-o", "addr", "show", "scope", "global"])
        .output()
    {
        let s = String::from_utf8_lossy(&out.stdout);
        for line in s.lines() {
            if let Some(part) = line.split_whitespace().nth(3) {
                let ip = part.split('/').next().unwrap_or("");
                if !ip.is_empty() && !skip_ip(ip) && !hosts.iter().any(|h| h == ip) {
                    hosts.push(ip.to_string());
                }
            }
        }
    }
    if hosts.is_empty() {
        let h = fs::read_to_string("/etc/hostname")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "aios".into());
        hosts.push(h);
    }
    hosts
}

fn room_ssh_lines() -> Vec<String> {
    advertise_hosts()
        .into_iter()
        .map(|h| format!("ssh -p {ROOM_PORT} root@{h}"))
        .collect()
}

fn looks_like_pty_cmd(t: &str) -> bool {
    matches!(
        t.split_whitespace().next().unwrap_or(""),
        "ssh" | "scp" | "sftp" | "tmux" | "mosh"
    )
}

fn pty_hint() -> String {
    let mut out = String::from("母 REPL 不是壳，也不进房。\n\n  请新开终端：\n");
    for c in room_ssh_lines() {
        out.push_str(&format!("    {c}\n"));
    }
    out.push_str("\n  本窗：/help   /status /spawn /attach /idle /reap /archive\n        model   run   闲聊   exit\n");
    out
}

fn banner_text() -> String {
    let mut out = String::from("AIOS 母 · slash 管机 · 闲聊走 NL · 进房请新开终端\n");
    out.push_str("  /status  /spawn  /attach  /idle  /reap  /archive\n");
    let cmds = room_ssh_lines();
    if let Some(first) = cmds.first() {
        out.push_str(&format!("  进房  {first}\n"));
        for c in cmds.iter().skip(1) {
            out.push_str(&format!("        {c}\n"));
        }
    }
    out.push_str("  帮助  /help          模型  model          跑    run          离开  exit\n");
    out
}

fn banner() {
    print!("{}", banner_text());
}

fn format_help(body: &Value) -> String {
    let mut out = String::from("命令\n\n核 slash  来源 motherd · 不经模型\n");
    match body.get("core").and_then(|v| v.as_array()) {
        Some(arr) if !arr.is_empty() => {
            for c in arr {
                let usage = c
                    .get("usage")
                    .and_then(|v| v.as_str())
                    .or_else(|| c.get("name").and_then(|v| v.as_str()))
                    .unwrap_or("-");
                let summary = c.get("summary").and_then(|v| v.as_str()).unwrap_or("");
                out.push_str(&format!("  {usage:<22} {summary}\n"));
            }
        }
        _ => out.push_str("  （无）\n"),
    }
    out.push_str("\n插件命令  来源 已验签插件；未加载当不存在\n");
    match body.get("plugins").and_then(|v| v.as_array()) {
        Some(arr) if !arr.is_empty() => {
            for c in arr {
                let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("-");
                let src = c.get("source").and_then(|v| v.as_str()).unwrap_or("-");
                let summary = c.get("summary").and_then(|v| v.as_str()).unwrap_or("");
                out.push_str(&format!("  {name:<12} {src:<16} {summary}\n"));
            }
        }
        _ => out.push_str("  （无）\n"),
    }
    out.push_str("\n已挂插件  已验签；未加载当不存在\n");
    match body.get("loaded").and_then(|v| v.as_array()) {
        Some(arr) if !arr.is_empty() => {
            let names: Vec<&str> = arr
                .iter()
                .filter_map(|c| c.get("name").and_then(|v| v.as_str()))
                .collect();
            out.push_str(&format!("  {}\n", names.join("  ")));
        }
        _ => out.push_str("  （无）\n"),
    }
    out.push_str(
        "\n本窗  来源 motherctl · 不是 slash\n  exit                  离开\n  help                  无斜杠，转核 /help（避免进闲聊）\n  run                   家里以 aios 跑 PATH 命令名（不是壳）\n  nick                  看/改母昵称（输入栏，默认 OSER）\n",
    );
    out.push_str("\n提示（不是命令）\n  进房请新开终端：\n");
    for c in room_ssh_lines() {
        out.push_str(&format!("    {c}\n"));
    }
    for h in advertise_hosts() {
        out.push_str(&format!("  机舱  http://{h}:7460/\n"));
        out.push_str(&format!("  产品仓 http://{h}:3000/\n"));
    }
    out.push_str("  闲聊直接打字。母可把意愿交给核；人手 / 仍直达核。活数据 /status。\n");
    out
}

fn format_status(body: &Value) -> String {
    let mut out = String::from("状态  正常\n\n房间\n");
    match body.get("leases").and_then(|v| v.as_array()) {
        Some(ls) if !ls.is_empty() => {
            out.push_str(&format!("  {:<16} {:<8} {}\n", "项目", "状态", "房间"));
            for l in ls {
                let id = l.get("project_id").and_then(|v| v.as_str()).unwrap_or("-");
                let st = l.get("state").and_then(|v| v.as_str()).unwrap_or("-");
                let room = l.get("room_id").and_then(|v| v.as_str()).unwrap_or("-");
                out.push_str(&format!(
                    "  {id:<16} {zh:<8} {room}\n",
                    zh = state_zh(st)
                ));
            }
        }
        _ => out.push_str("  （无）\n"),
    }
    out.push_str("\n插件\n");
    let plugins = body.get("plugins");
    let names: Vec<&str> = match plugins {
        Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str()).collect(),
        _ => Vec::new(),
    };
    if names.is_empty() {
        out.push_str("  （无）\n");
    } else {
        for chunk in names.chunks(4) {
            out.push_str("  ");
            out.push_str(&chunk.join("  "));
            out.push('\n');
        }
    }
    out
}

fn format_lease(body: &Value) -> String {
    let id = body
        .get("project_id")
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    let st = body.get("state").and_then(|v| v.as_str()).unwrap_or("-");
    let room = body.get("room_id").and_then(|v| v.as_str()).unwrap_or("");
    let mut out = format!("{id}\n  状态  {}\n", state_zh(st));
    if !room.is_empty() {
        out.push_str(&format!("  房间  {room}\n"));
    }
    out
}

fn format_attach(body: &Value) -> String {
    let id = body
        .get("project_id")
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    let mut cmds: Vec<String> = body
        .get("cmds")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    if cmds.is_empty() {
        if let Some(c) = body.get("cmd").and_then(|v| v.as_str()) {
            cmds.push(c.to_string());
        }
    }
    if cmds.is_empty() {
        cmds.extend(room_ssh_lines());
    }
    let gitea = body.get("gitea").and_then(|v| v.as_str()).unwrap_or("");
    let mut out = format!("进房  {id}\n\n  新开终端（不要在 mother> 里敲 ssh）：\n");
    for c in cmds {
        out.push_str(&format!("    {c}\n"));
    }
    if !gitea.is_empty() {
        out.push_str(&format!("\n  房内 Gitea  {gitea}\n"));
    }
    out.push_str("  母不当 PTY 中间人。\n");
    out
}

fn human(line: &str, raw: &str) -> String {
    let t = raw.trim();
    if !t.starts_with('{') {
        return if raw.ends_with('\n') {
            raw.to_string()
        } else {
            format!("{raw}\n")
        };
    }
    let Ok(v) = serde_json::from_str::<Value>(t) else {
        return if raw.ends_with('\n') {
            raw.to_string()
        } else {
            format!("{raw}\n")
        };
    };
    let ok = v.get("ok").and_then(|x| x.as_bool()).unwrap_or(true);
    let err = v.get("error").and_then(|x| x.as_str()).unwrap_or("");
    if err == "no_model"
        || (v.get("degraded").and_then(|x| x.as_bool()) == Some(true) && (err.is_empty() || err == "no_model"))
    {
        return "无模型（降级）。slash 仍可用。看配置：model\n".into();
    }
    if !ok {
        let err = v
            .get("error")
            .and_then(|x| x.as_str())
            .unwrap_or("unknown");
        return format!("失败  {}\n", err_zh(err));
    }
    let cmd = line.split_whitespace().next().unwrap_or("");
    let body = v.get("body").unwrap_or(&Value::Null);
    match cmd {
        "/status" => format_status(body),
        "/spawn" | "/idle" | "/reap" | "/archive" => format_lease(body),
        "/attach" => format_attach(body),
        "/help" => format_help(body),
        "run" | "/run" => format_run(&v),
        _ => {
            if let Some(msg) = v.get("payload").and_then(|_| v.as_str()) {
                format!("{msg}\n")
            } else if let Ok(s) = serde_json::to_string_pretty(&v) {
                format!("{s}\n")
            } else {
                format!("{raw}\n")
            }
        }
    }
}

fn show(line: &str, raw: &str, pretty: bool) {
    if pretty {
        print!("{}", human(line, raw));
    } else {
        print!("{raw}");
        if !raw.ends_with('\n') {
            println!();
        }
    }
}

fn format_nl(raw: &str) -> String {
    let t = raw.trim();
    let Ok(v) = serde_json::from_str::<Value>(t) else {
        return human("", raw);
    };
    let Some(say) = v.get("say").and_then(|x| x.as_str()) else {
        return human("", raw);
    };
    let mut out = String::new();
    let say = say.trim();
    if !say.is_empty() {
        out.push_str(say);
        out.push('\n');
    }
    let do_line = v.get("do").and_then(|x| x.as_str()).unwrap_or("").trim();
    if let Some(core) = v.get("core").filter(|c| !c.is_null()) {
        let ptr = after_spawn_pointer(do_line, core);
        if !ptr.is_empty() {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&ptr);
        }
    }
    if out.is_empty() {
        human("", raw)
    } else {
        out
    }
}

fn show_nl(raw: &str, pretty: bool) {
    if !pretty {
        print!("{raw}");
        if !raw.ends_with('\n') {
            println!();
        }
        return;
    }
    print!("{}", format_nl(raw));
}

fn after_spawn_pointer(do_line: &str, core: &Value) -> String {
    let mut parts = do_line.split_whitespace();
    let cmd = parts.next().unwrap_or("");
    let pid = parts.next().unwrap_or("");
    if cmd != "/spawn" || pid.is_empty() {
        return String::new();
    }
    let ok = core.get("ok").and_then(|x| x.as_bool()) == Some(true);
    let err = core.get("error").and_then(|x| x.as_str()).unwrap_or("");
    if !ok && err != "same project already running on this disk" {
        return String::new();
    }
    let line = format!("/attach {pid}");
    match call(&line) {
        Ok(raw) => human(&line, &raw),
        Err(e) => format!("{e}\n"),
    }
}

fn default_aliases() -> Value {
    json!({
        "chat-mother": "grok",
        "chat-code": "gpt",
        "cheap": "deepseek"
    })
}

fn load_aliases() -> Value {
    fs::read_to_string(ALIAS_FILE)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(default_aliases)
}

fn save_aliases(v: &Value) -> Result<(), String> {
    fs::write(ALIAS_FILE, format!("{}\n", v))
        .map_err(|e| format!("写别名失败: {e}"))
}

fn secret_present(id: &str) -> bool {
    let req = json!({
        "op": "secret_get",
        "secret_id": id,
        "project_id": "",
        "room_id": "",
        "cap": "",
        "args": {}
    });
    match call_json(&req) {
        Ok(v) if v.get("ok").and_then(|x| x.as_bool()) == Some(true) => v
            .get("body")
            .and_then(|b| b.get("value"))
            .and_then(|x| x.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        _ => false,
    }
}

fn yes_no(v: bool) -> &'static str {
    if v {
        "有"
    } else {
        "无"
    }
}

fn resolve_adapter(ad: &str) -> Option<&'static str> {
    match ad {
        "grok" | "gpt" | "deepseek" | "openai_compat" => ADAPTERS.iter().copied().find(|x| *x == ad),
        "compat" | "openai" | "oc" => Some("openai_compat"),
        _ => None,
    }
}

fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for c in s.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => cur.push(c),
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            None => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn looks_like_key(s: &str) -> bool {
    let s = s.trim();
    if resolve_adapter(s).is_some() {
        return false;
    }
    s.starts_with("sk-")
        || s.starts_with("gsk_")
        || s.starts_with("xai-")
        || (s.len() >= 24 && !s.starts_with("http"))
}

fn looks_like_url(s: &str) -> bool {
    let s = s.trim().trim_matches('"').trim_matches('\'');
    s.starts_with("http://") || s.starts_with("https://")
}

fn history_ok(t: &str) -> bool {
    let s = as_model_line(t).unwrap_or(t);
    let parts = split_args(s);
    if parts.get(1).map(|x| x.as_str()) == Some("key") {
        return false;
    }
    parts.iter().all(|p| !looks_like_key(p))
}

fn history_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".motherctl_history"))
}

fn alias_key(s: &str) -> Option<&'static str> {
    match s {
        "mother" | "chat-mother" => Some("chat-mother"),
        "code" | "chat-code" => Some("chat-code"),
        "cheap" => Some("cheap"),
        _ => None,
    }
}

fn infer_key_adapter() -> String {
    if secret_present("model.openai_compat.base_url") {
        return "openai_compat".into();
    }
    load_aliases()
        .get("chat-mother")
        .and_then(|v| v.as_str())
        .unwrap_or("grok")
        .to_string()
}

fn as_model_line(t: &str) -> Option<&str> {
    let s = t.strip_prefix('/').unwrap_or(t);
    if s == "model" || s.starts_with("model ") {
        Some(s)
    } else {
        None
    }
}

fn as_run_line(t: &str) -> Option<&str> {
    let s = t.strip_prefix('/').unwrap_or(t);
    if s == "run" || s.starts_with("run ") {
        Some(s)
    } else {
        None
    }
}

fn as_nick_line(t: &str) -> Option<&str> {
    let s = t.strip_prefix('/').unwrap_or(t);
    if s == "nick" || s.starts_with("nick ") {
        Some(s)
    } else {
        None
    }
}

const DEFAULT_NICK: &str = "OSER";

fn nick_path() -> PathBuf {
    session_dir().join("nick")
}

fn valid_nick(s: &str) -> bool {
    let n = s.chars().count();
    (1..=16).contains(&n) && s.chars().all(|c| !c.is_control() && c != '/')
}

fn load_nick() -> String {
    fs::read_to_string(nick_path())
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| valid_nick(s) && s != "哥哥")
        .unwrap_or_else(|| DEFAULT_NICK.into())
}

fn handle_nick(t: &str) -> String {
    let rest = t
        .strip_prefix('/')
        .unwrap_or(t)
        .strip_prefix("nick")
        .unwrap_or("")
        .trim();
    if rest.is_empty() {
        return format!(
            "母昵称  {}\n改       nick 某某\n复位     nick 默认\n",
            load_nick()
        );
    }
    if rest == "默认" {
        let _ = fs::remove_file(nick_path());
        return format!("已改  {DEFAULT_NICK}\n");
    }
    if !valid_nick(rest) {
        return "失败  昵称 1–16 字，不要控制符或斜杠\n".into();
    }
    let p = nick_path();
    match fs::write(&p, format!("{rest}\n")) {
        Ok(()) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&p, fs::Permissions::from_mode(0o660));
            }
            format!("已改  {rest}\n")
        }
        Err(e) => format!("失败  {e}\n"),
    }
}

fn format_run(v: &Value) -> String {
    let ok = v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false);
    if !ok {
        let err = v.get("error").and_then(|x| x.as_str()).unwrap_or("unknown");
        let mut out = format!("失败  {}\n", err_zh(err));
        if let Some(cmds) = v
            .get("body")
            .and_then(|b| b.get("commands"))
            .and_then(|c| c.as_array())
        {
            let names: Vec<&str> = cmds.iter().filter_map(|x| x.as_str()).collect();
            if !names.is_empty() {
                out.push_str(&format!("可用  {}\n", names.join("  ")));
            }
        }
        return out;
    }
    let body = v.get("body").unwrap_or(&Value::Null);
    if body.get("any_basename").and_then(|x| x.as_bool()) == Some(true)
        || (body.get("path").is_some() && body.get("argv").is_none())
    {
        let user = body.get("user").and_then(|x| x.as_str()).unwrap_or("aios");
        let path = body
            .get("path")
            .and_then(|x| x.as_str())
            .unwrap_or("/run/current-system/sw/bin");
        let mut out = format!("家里可跑（用户 {user}，PATH {path} 下命令名，不是壳）\n");
        out.push_str("用法  run df -h\n      run cat /etc/os-release\n      run ps aux\n");
        return out;
    }
    if let Some(cmds) = body.get("commands").and_then(|c| c.as_array()) {
        let names: Vec<&str> = cmds.iter().filter_map(|x| x.as_str()).collect();
        let user = body.get("user").and_then(|x| x.as_str()).unwrap_or("aios");
        let mut out = format!("家里可跑（用户 {user}，不是壳）\n  {}\n", names.join("  "));
        out.push_str("用法  run df -h\n      run systemctl status motherd\n");
        return out;
    }
    let argv: Vec<&str> = body
        .get("argv")
        .and_then(|a| a.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
        .unwrap_or_default();
    let user = body.get("user").and_then(|x| x.as_str()).unwrap_or("aios");
    let exit = body.get("exit").and_then(|x| x.as_i64()).unwrap_or(0);
    let mut out = format!("跑  {}    退出 {exit}    身份 {user}\n", argv.join(" "));
    if body.get("truncated").and_then(|x| x.as_bool()) == Some(true) {
        out.push_str("（输出已截断）\n");
    }
    let stdout = body.get("stdout").and_then(|x| x.as_str()).unwrap_or("");
    let stderr = body.get("stderr").and_then(|x| x.as_str()).unwrap_or("");
    if !stdout.is_empty() {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
        out.push_str(stdout);
        if !stdout.ends_with('\n') {
            out.push('\n');
        }
    }
    if !stderr.is_empty() {
        out.push_str("\n标准错误\n");
        out.push_str(stderr);
        if !stderr.ends_with('\n') {
            out.push('\n');
        }
    }
    if stdout.is_empty() && stderr.is_empty() {
        out.push('\n');
    }
    out
}

fn handle_run(t: &str) -> String {
    let parts = split_args(t);
    let argv: Vec<String> = parts.into_iter().skip(1).collect();
    let req = json!({
        "op": "cap_call",
        "project_id": "",
        "room_id": "",
        "cap": "host.run",
        "args": { "argv": argv },
        "secret_id": ""
    });
    match call_json(&req) {
        Ok(v) => format_run(&v),
        Err(e) => format!("失败  {e}\n"),
    }
}

fn alias_label(k: &str) -> &str {
    match k {
        "chat-mother" => "母闲聊",
        "chat-code" => "房内代码",
        "cheap" => "便宜通道",
        _ => k,
    }
}

fn model_status() -> String {
    let aliases = load_aliases();
    let mut out = String::from("模型\n\n别名\n");
    for key in ["chat-mother", "chat-code", "cheap"] {
        let ad = aliases
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("-");
        let on = secret_present(&format!("model.{ad}.api_key"));
        out.push_str(&format!(
            "  {:<10} {key:<14} {ad:<16} 钥 {}\n",
            alias_label(key),
            yes_no(on)
        ));
    }
    out.push_str("\n适配器钥\n");
    for ad in ADAPTERS {
        let on = secret_present(&format!("model.{ad}.api_key"));
        out.push_str(&format!("  {ad:<16} {}\n", yes_no(on)));
    }
    let url_on = secret_present("model.openai_compat.base_url");
    out.push_str(&format!(
        "  {:<16} {}\n",
        "compat url",
        yes_no(url_on)
    ));
    if let Some(ids) = aliases.get("model_ids").and_then(|v| v.as_object()) {
        if !ids.is_empty() {
            out.push_str("\n模型 id\n");
            for (ad, v) in ids {
                out.push_str(&format!(
                    "  {ad:<16} {}\n",
                    v.as_str().unwrap_or("-")
                ));
            }
        }
    }
    out.push_str(
        "\n改别名  model mother|code|cheap  grok|gpt|deepseek|openai_compat\n投钥    model key <适配器> [钥]\n        model key <钥>   （已设 compat url 时默认 openai_compat）\n兼容地址 model url <base_url>\n        model <url>\n模型 id  model name grok-4.6\n        model name cheap deepseek-v4-flash\n全屏    Enter 发送；Ctrl-C 清输入；Ctrl-D 或 exit 离开；PgUp 滚\n",
    );
    out
}

fn set_alias(alias: &str, adapter: &str) -> String {
    let Some(adapter) = resolve_adapter(adapter) else {
        return format!("失败  未知适配器 {adapter}。grok|gpt|deepseek|openai_compat\n");
    };
    let mut cur = load_aliases();
    if let Some(obj) = cur.as_object_mut() {
        obj.insert(alias.to_string(), json!(adapter));
    }
    match save_aliases(&cur) {
        Ok(()) => format!(
            "已改  {} ({alias}) → {adapter}\n",
            alias_label(alias)
        ),
        Err(e) => format!("失败  {e}\n"),
    }
}

fn read_secret_line(prompt: &str) -> Result<String, String> {
    eprint!("{prompt}");
    let _ = io::stderr().flush();
    let tty = io::stdin().is_terminal();
    if tty {
        let _ = Command::new("stty").args(["-echo"]).status();
    }
    let mut s = String::new();
    let n = io::stdin().read_line(&mut s);
    if tty {
        let _ = Command::new("stty").args(["echo"]).status();
        eprintln!();
    }
    n.map_err(|e| e.to_string())?;
    Ok(s.trim().to_string())
}

fn put_vault_in(id: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("空值".into());
    }
    let dir = Path::new(VAULT_IN);
    if !dir.is_dir() {
        return Err("哑柜入口不存在".into());
    }
    fs::write(dir.join(id), value.as_bytes()).map_err(|e| format!("写入口失败: {e}"))?;
    let _ = call("/status");
    Ok(())
}

fn set_key(adapter: &str, inline: Option<&str>) -> String {
    let Some(adapter) = resolve_adapter(adapter) else {
        return format!(
            "失败  未知适配器 {adapter}。grok|gpt|deepseek|openai_compat\n用法  model key openai_compat\n      model key openai_compat <钥>\n"
        );
    };
    let key = if let Some(k) = inline {
        k.trim().to_string()
    } else if io::stdin().is_terminal() {
        match read_secret_line(&format!("钥 {adapter}（不回显，回车结束）：")) {
            Ok(k) => k,
            Err(e) => return format!("失败  {e}\n"),
        }
    } else {
        return "失败  用法  model key <适配器> <钥>\n".into();
    };
    if key.is_empty() {
        return "失败  空钥\n".into();
    }
    match put_vault_in(&format!("model.{adapter}.api_key"), &key) {
        Ok(()) => format!("已收  model.{adapter}.api_key\n"),
        Err(e) => format!("失败  {e}\n"),
    }
}

fn set_model_id(scope: Option<&str>, name: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        return "失败  需要模型 id，例如 model name grok-4.6\n".into();
    }
    let key = match scope {
        None | Some("mother") | Some("chat-mother") => "chat-mother",
        Some("code") | Some("chat-code") => "chat-code",
        Some("cheap") => "cheap",
        Some(ad) => match resolve_adapter(ad) {
            Some(a) => a,
            None => {
                return format!("失败  未知范围 {ad}。mother|code|cheap 或适配器\n");
            }
        },
    };
    let mut cur = load_aliases();
    if let Some(obj) = cur.as_object_mut() {
        let ids = obj.entry("model_ids").or_insert_with(|| json!({}));
        if let Some(m) = ids.as_object_mut() {
            m.insert(key.to_string(), json!(name));
        }
    }
    match save_aliases(&cur) {
        Ok(()) => format!("已改  {} 模型 id → {name}\n", alias_label(key)),
        Err(e) => format!("失败  {e}\n"),
    }
}

fn set_url(url: &str) -> String {
    let url = url.trim().trim_matches('"').trim_matches('\'').trim();
    if !looks_like_url(url) {
        return "失败  需要 http(s) 地址，例如 model url http://192.168.1.168:8317/v1\n"
            .into();
    }
    match put_vault_in("model.openai_compat.base_url", url) {
        Ok(()) => {
            let mut out = String::from("已收  model.openai_compat.base_url\n");
            let mother = load_aliases()
                .get("chat-mother")
                .and_then(|v| v.as_str())
                .unwrap_or("grok")
                .to_string();
            if mother != "openai_compat" {
                out.push_str(&set_alias("chat-mother", "openai_compat"));
            }
            if !secret_present("model.openai_compat.api_key") {
                out.push_str("还缺钥：model key openai_compat\n");
            } else {
                out.push_str("闲聊直接打字。改模型 id：model name grok-4.6\n");
            }
            out
        }
        Err(e) => format!("失败  {e}\n"),
    }
}

fn handle_model(t: &str) -> String {
    let parts = split_args(t);
    let p: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
    match p.as_slice() {
        ["model"] | ["model", "help"] => model_status(),
        ["model", "mother", ad] | ["model", "chat-mother", ad] => set_alias("chat-mother", ad),
        ["model", "code", ad] | ["model", "chat-code", ad] => set_alias("chat-code", ad),
        ["model", "cheap", ad] => set_alias("cheap", ad),
        ["model", "key"] => {
            "用法  model key <适配器> [钥]\n      model key <钥>   （已设 compat url 时默认 openai_compat）\n适配器 grok|gpt|deepseek|openai_compat\n".into()
        }
        ["model", "key", a] if resolve_adapter(a).is_some() => set_key(a, None),
        ["model", "key", maybe_key] if looks_like_key(maybe_key) => {
            let ad = infer_key_adapter();
            let mut out = set_key(&ad, Some(maybe_key));
            out.push_str(&format!("（未写适配器，按 {ad} 收）\n"));
            out
        }
        ["model", "key", a, key] => set_key(a, Some(key)),
        ["model", url] if looks_like_url(url) => set_url(url),
        ["model", "url", url] => set_url(url),
        ["model", "url", "openai_compat", url] => set_url(url),
        ["model", "name", id] | ["model", "id", id] => set_model_id(None, id),
        ["model", "name", scope, id] | ["model", "id", scope, id] => {
            set_model_id(Some(scope), id)
        }
        _ => "用法  model   或  model help\n      model <url>\n".into(),
    }
}

fn is_direct_cmd(t: &str) -> bool {
    as_model_line(t).is_some()
        || as_run_line(t).is_some()
        || as_nick_line(t).is_some()
        || t.starts_with('/')
        || t == "help"
        || t.starts_with("help ")
        || looks_like_pty_cmd(t)
}

fn model_key_needs_prompt(t: &str) -> Option<String> {
    let s = as_model_line(t)?;
    let p = split_args(s);
    if p.len() == 3 && p[1] == "key" && resolve_adapter(&p[2]).is_some() {
        Some(p[2].clone())
    } else {
        None
    }
}

fn dispatch_pretty(t: &str) -> String {
    if let Some(line) = as_model_line(t) {
        return handle_model(line);
    }
    if let Some(line) = as_nick_line(t) {
        return handle_nick(line);
    }
    if let Some(line) = as_run_line(t) {
        return handle_run(line);
    }
    if t == "help" || t.starts_with("help ") {
        return match call("/help") {
            Ok(o) => human("/help", &o),
            Err(e) => format!("{e}\n"),
        };
    }
    if t.starts_with('/') {
        return match call(t) {
            Ok(o) => {
                let mut s = human(t, &o);
                if let Ok(v) = serde_json::from_str::<Value>(o.trim()) {
                    s.push_str(&after_spawn_pointer(t, &v));
                }
                s
            }
            Err(e) => format!("{e}\n"),
        };
    }
    if looks_like_pty_cmd(t) {
        return pty_hint();
    }
    match talk_nl(t) {
        Ok(o) => format_nl(&o),
        Err(e) => format!("{e}\n"),
    }
}

fn handle(t: &str, pretty: bool) {
    if let Some(line) = as_model_line(t) {
        print!("{}", handle_model(line));
        return;
    }
    if let Some(line) = as_nick_line(t) {
        print!("{}", handle_nick(line));
        return;
    }
    if let Some(line) = as_run_line(t) {
        print!("{}", handle_run(line));
        return;
    }
    if t == "help" || t.starts_with("help ") {
        match call("/help") {
            Ok(o) => show("/help", &o, true),
            Err(e) => eprintln!("{e}"),
        }
        return;
    }
    if t.starts_with('/') {
        let humanize = pretty || t.split_whitespace().next() == Some("/help");
        match call(t) {
            Ok(o) => show(t, &o, humanize),
            Err(e) => eprintln!("{e}"),
        }
        return;
    }
    if looks_like_pty_cmd(t) {
        print!("{}", pty_hint());
        return;
    }
    match talk_nl(t) {
        Ok(o) => show_nl(&o, pretty),
        Err(e) => eprintln!("{e}"),
    }
}

fn repl_pipe(pretty: bool) {
    let stdin = io::stdin();
    let mut line = String::new();
    loop {
        line.clear();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t == "exit" || t == "quit" {
            break;
        }
        handle(t, pretty);
    }
}

fn repl_tty(pretty: bool) {
    let mut rl = match DefaultEditor::new() {
        Ok(e) => e,
        Err(_) => {
            repl_pipe(pretty);
            return;
        }
    };
    let hist = history_path();
    if let Some(ref p) = hist {
        let _ = rl.load_history(p);
    }
    loop {
        match rl.readline("mother> ") {
            Ok(line) => {
                let t = line.trim();
                if t.is_empty() {
                    continue;
                }
                if t == "exit" || t == "quit" {
                    break;
                }
                if history_ok(t) {
                    let _ = rl.add_history_entry(t);
                }
                handle(t, pretty);
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => break,
            Err(_) => break,
        }
    }
    if let Some(ref p) = hist {
        let _ = rl.save_history(p);
    }
}

fn repl() {
    if io::stdin().is_terminal() && io::stdout().is_terminal() {
        match tui::run() {
            Ok(()) => return,
            Err(e) => {
                eprintln!("TUI 未启动（{e}），退回行模式。");
            }
        }
    }
    banner();
    let pretty = io::stdout().is_terminal();
    if io::stdin().is_terminal() {
        repl_tty(pretty);
    } else {
        repl_pipe(pretty);
    }
}

fn is_login_flag(s: &str) -> bool {
    matches!(s, "-l" | "--login" | "-")
}

// sshd ForceCommand is `$SHELL -c ForceCommand`. SHELL is motherctl, so
// the -c string is this binary (store path or /run/current-system/...).
fn is_self_invocation(line: &str) -> bool {
    let t = line.trim().trim_matches('\'').trim_matches('"').trim();
    if t.is_empty() || is_login_flag(t) {
        return true;
    }
    Path::new(t)
        .file_name()
        .and_then(|n| n.to_str())
        == Some("motherctl")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest: Vec<&str> = args
        .iter()
        .map(|s| s.as_str())
        .filter(|s| !is_login_flag(s))
        .collect();
    if rest.is_empty() {
        repl();
        return;
    }
    if rest[0] == "-c" {
        let line = rest.get(1..).unwrap_or(&[]).join(" ");
        if is_self_invocation(&line) {
            repl();
            return;
        }
        let pretty = io::stdout().is_terminal();
        handle(&line, pretty);
        return;
    }
    let line = rest.join(" ");
    if is_self_invocation(&line) {
        repl();
        return;
    }
    let pretty = io::stdout().is_terminal();
    handle(&line, pretty);
}
