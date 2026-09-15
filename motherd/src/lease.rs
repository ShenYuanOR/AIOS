use crate::syscall::{Lease, LeaseState, Response};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

pub struct LeaseStore {
    dir: PathBuf,
}

impl LeaseStore {
    pub fn open(data: &Path) -> std::io::Result<Self> {
        let dir = data.join("leases");
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    fn path(&self, project_id: &str) -> PathBuf {
        self.dir.join(format!("{project_id}.json"))
    }

    pub fn load(&self, project_id: &str) -> Option<Lease> {
        let p = self.path(project_id);
        let raw = fs::read_to_string(p).ok()?;
        serde_json::from_str(&raw).ok()
    }

    pub fn save(&self, lease: &Lease) -> std::io::Result<()> {
        let p = self.path(&lease.project_id);
        fs::write(p, serde_json::to_vec_pretty(lease).unwrap())
    }

    pub fn all(&self) -> Vec<Lease> {
        let mut out = Vec::new();
        if let Ok(rd) = fs::read_dir(&self.dir) {
            for e in rd.flatten() {
                if let Ok(raw) = fs::read_to_string(e.path()) {
                    if let Ok(l) = serde_json::from_str::<Lease>(&raw) {
                        out.push(l);
                    }
                }
            }
        }
        out
    }

    pub fn spawn(&self, project_id: &str, disk: &str) -> Response {
        if project_id.is_empty() {
            return err("project_id required");
        }
        if let Some(existing) = self.load(project_id) {
            if existing.state == LeaseState::Running && existing.disk == disk {
                return err("same project already running on this disk");
            }
        }
        let lease = Lease {
            project_id: project_id.to_string(),
            room_id: format!("room-{project_id}"),
            state: LeaseState::Running,
            disk: disk.to_string(),
        };
        if let Err(e) = self.save(&lease) {
            return err(&e.to_string());
        }
        ok(json!(lease))
    }

    pub fn set_state(&self, project_id: &str, state: LeaseState) -> Response {
        let Some(mut lease) = self.load(project_id) else {
            return err("no lease");
        };
        lease.state = state;
        if let Err(e) = self.save(&lease) {
            return err(&e.to_string());
        }
        ok(json!(lease))
    }
}

pub fn ok(body: serde_json::Value) -> Response {
    Response {
        ok: true,
        error: None,
        body,
    }
}

pub fn err(msg: &str) -> Response {
    Response {
        ok: false,
        error: Some(msg.to_string()),
        body: json!({}),
    }
}
