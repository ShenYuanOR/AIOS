//! Slash commands never go through a model.

use crate::syscall::{LeaseState, Request, Syscall};

/// Core slash catalog for `/help`. Usage strings are what humans type.
pub const CORE: &[(&str, &str, &str)] = &[
    ("/help", "/help", "本页"),
    ("/status", "/status", "系统状态"),
    ("/spawn", "/spawn <项目>", "开房"),
    ("/attach", "/attach <项目>", "给进房指针；本窗不进房"),
    ("/idle", "/idle <项目>", "空闲"),
    ("/reap", "/reap <项目>", "停房"),
    ("/archive", "/archive <项目>", "归档（之后不能 /attach）"),
];

pub fn is_reserved(name: &str) -> bool {
    let n = name.strip_prefix('/').unwrap_or(name);
    CORE.iter().any(|(cmd, _, _)| cmd.strip_prefix('/').unwrap_or(cmd) == n)
}

pub fn parse(line: &str) -> Result<Request, String> {
    let line = line.trim();
    if !line.starts_with('/') {
        return Err("slash only".into());
    }
    let mut parts = line.split_whitespace();
    let cmd = parts.next().unwrap_or("");
    let rest: Vec<&str> = parts.collect();
    match cmd {
        "/status" => Ok(Request {
            op: Syscall::Quota,
            project_id: String::new(),
            room_id: String::new(),
            cap: "status".into(),
            args: serde_json::json!({ "kind": "status" }),
            secret_id: String::new(),
        }),
        "/spawn" => {
            let project_id = rest.first().cloned().unwrap_or("").to_string();
            if project_id.is_empty() {
                return Err("/spawn <project_id>".into());
            }
            Ok(Request {
                op: Syscall::Spawn,
                project_id,
                room_id: String::new(),
                cap: String::new(),
                args: serde_json::json!({}),
                secret_id: String::new(),
            })
        }
        "/attach" => {
            let project_id = rest.first().cloned().unwrap_or("").to_string();
            if project_id.is_empty() {
                return Err("/attach <project_id>".into());
            }
            Ok(Request {
                op: Syscall::Attach,
                project_id,
                room_id: rest.get(1).cloned().unwrap_or("").to_string(),
                cap: String::new(),
                args: serde_json::json!({}),
                secret_id: String::new(),
            })
        }
        "/idle" => Ok(state_cmd(Syscall::Quota, &rest, LeaseState::Idle)),
        "/reap" => {
            let project_id = rest.first().cloned().unwrap_or("").to_string();
            Ok(Request {
                op: Syscall::Reap,
                project_id,
                room_id: rest.get(1).cloned().unwrap_or("").to_string(),
                cap: String::new(),
                args: serde_json::json!({}),
                secret_id: String::new(),
            })
        }
        "/archive" => Ok(state_cmd(Syscall::Quota, &rest, LeaseState::Archived)),
        "/help" => Ok(Request {
            op: Syscall::Quota,
            project_id: String::new(),
            room_id: String::new(),
            cap: "help".into(),
            args: serde_json::json!({ "kind": "help" }),
            secret_id: String::new(),
        }),
        other => Err(format!("unknown slash {other}")),
    }
}

fn state_cmd(op: Syscall, rest: &[&str], state: LeaseState) -> Request {
    Request {
        op,
        project_id: rest.first().cloned().unwrap_or("").to_string(),
        room_id: rest.get(1).cloned().unwrap_or("").to_string(),
        cap: format!("{state:?}").to_lowercase(),
        args: serde_json::json!({ "to": state }),
        secret_id: String::new(),
    }
}
