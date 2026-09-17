//! Living-room host.run: any basename in PATH, dropped to aios. Not a shell.

use crate::lease::{err, ok};
use crate::syscall::Response;
use serde_json::json;
use std::fs;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const LIVE_USER: &str = "aios";
pub const BIN_DIR: &str = "/run/current-system/sw/bin";
pub const TIMEOUT_MS: u64 = 8000;
pub const MAX_OUT: usize = 256 * 1024;
pub const MAX_ARGS: usize = 8;

fn passwd(name: &str) -> Option<(u32, u32, PathBuf)> {
    let text = fs::read_to_string("/etc/passwd").ok()?;
    for line in text.lines() {
        let mut it = line.split(':');
        let n = it.next()?;
        if n != name {
            continue;
        }
        let _pw = it.next()?;
        let uid: u32 = it.next()?.parse().ok()?;
        let gid: u32 = it.next()?.parse().ok()?;
        let _gecos = it.next()?;
        let home = PathBuf::from(it.next()?);
        return Some((uid, gid, home));
    }
    None
}

fn arg_ok(a: &str) -> bool {
    if a.is_empty() || a.len() > 128 {
        return false;
    }
    if a.contains('\0') || a.contains("..") {
        return false;
    }
    a.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || matches!(c, '_' | '.' | '/' | ':' | '=' | ',' | '+' | '-' | '@' | '%')
    })
}

pub fn help_body() -> serde_json::Value {
    json!({
        "any_basename": true,
        "path": BIN_DIR,
        "user": LIVE_USER,
        "timeout_ms": TIMEOUT_MS,
        "max_args": MAX_ARGS,
    })
}

pub fn catalog() -> Response {
    let mut body = help_body();
    if let Some(obj) = body.as_object_mut() {
        obj.insert("usage".into(), json!("run <cmd> [args]"));
    }
    ok(body)
}

pub fn host_run(args: &serde_json::Value) -> Response {
    let argv: Vec<String> = args
        .get("argv")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    if argv.is_empty() {
        return catalog();
    }
    if argv.len() > MAX_ARGS {
        return err("too many args");
    }
    for a in &argv {
        if !arg_ok(a) {
            return err("bad arg");
        }
    }
    let bin = &argv[0];
    if bin.contains('/') || bin.starts_with('.') {
        return err("basename only");
    }
    let path = Path::new(BIN_DIR).join(bin);
    if !path.is_file() {
        return err(&format!("missing {bin}"));
    }
    let Some((uid, gid, home)) = passwd(LIVE_USER) else {
        return err("no live user");
    };
    let cwd = if home.is_dir() {
        home.clone()
    } else {
        PathBuf::from("/tmp")
    };
    let mut cmd = Command::new(&path);
    cmd.args(&argv[1..])
        .current_dir(&cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .env("PATH", BIN_DIR)
        .env("HOME", &home)
        .env("USER", LIVE_USER)
        .env("LOGNAME", LIVE_USER)
        .env("LANG", "C.UTF-8")
        .env("LC_ALL", "C.UTF-8")
        .env("PAGER", "cat")
        .env("SYSTEMD_PAGER", "cat");
    // motherd is root; drop to the at-home user before exec.
    unsafe {
        cmd.uid(uid);
        cmd.gid(gid);
    }
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return err(&format!("spawn: {e}")),
    };
    let mut stdout_p = child.stdout.take();
    let mut stderr_p = child.stderr.take();
    let deadline = Instant::now() + Duration::from_millis(TIMEOUT_MS);
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return err("timeout");
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return err(&format!("wait: {e}")),
        }
    };
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    if let Some(ref mut o) = stdout_p {
        let _ = o.read_to_end(&mut stdout);
    }
    if let Some(ref mut e) = stderr_p {
        let _ = e.read_to_end(&mut stderr);
    }
    let (out, ot) = clip_out(stdout);
    let (errb, et) = clip_out(stderr);
    let code = status.code().unwrap_or(128);
    ok(json!({
        "argv": argv,
        "user": LIVE_USER,
        "exit": code,
        "stdout": out,
        "stderr": errb,
        "truncated": ot || et,
    }))
}

fn clip_out(buf: Vec<u8>) -> (String, bool) {
    if buf.len() <= MAX_OUT {
        return (String::from_utf8_lossy(&buf).into_owned(), false);
    }
    let mut end = MAX_OUT;
    if let Some(i) = buf[..MAX_OUT].iter().rposition(|&c| c == b'\n') {
        end = i + 1;
    }
    (
        String::from_utf8_lossy(&buf[..end]).into_owned(),
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arg_ok_examples() {
        assert!(arg_ok("-h"));
        assert!(arg_ok("df"));
        assert!(arg_ok("motherd.service"));
        assert!(!arg_ok("a;b"));
        assert!(!arg_ok("x|y"));
        assert!(!arg_ok("../etc"));
        assert!(!arg_ok(""));
    }

    #[test]
    fn catalog_is_open() {
        let v = help_body();
        assert_eq!(v.get("any_basename").and_then(|x| x.as_bool()), Some(true));
        assert_eq!(v.get("path").and_then(|x| x.as_str()), Some(BIN_DIR));
    }

    #[test]
    fn clip_out_keeps_complete_lines() {
        let mut buf = Vec::new();
        while buf.len() < MAX_OUT + 50 {
            buf.extend_from_slice(b"alpha bravo charlie\n");
        }
        buf.extend_from_slice(b"TAIL SHOULD DROP\n");
        let (s, t) = clip_out(buf);
        assert!(t);
        assert!(s.ends_with('\n'));
        assert!(!s.contains("TAIL SHOULD DROP"));
        assert!(!s.ends_with("charl"));
    }
}
