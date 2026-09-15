use crate::lease::{err, ok};
use crate::syscall::Response;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Audit {
    chain: PathBuf,
}

impl Audit {
    pub fn open(data: &Path) -> std::io::Result<Self> {
        let dir = data.join("audit");
        fs::create_dir_all(&dir)?;
        Ok(Self {
            chain: dir.join("chain.jsonl"),
        })
    }

    pub fn append(&self, event: &str, detail: serde_json::Value) -> Response {
        let prev = self.tail_hash().unwrap_or_else(|| "genesis".into());
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let rec = json!({
            "ts": ts,
            "event": event,
            "detail": detail,
            "prev": prev,
        });
        let line = rec.to_string();
        let mut hasher = Sha256::new();
        hasher.update(prev.as_bytes());
        hasher.update(line.as_bytes());
        let hash = hex::encode(hasher.finalize());
        let mut rec = rec;
        rec["hash"] = json!(hash);
        let mut f = match OpenOptions::new().create(true).append(true).open(&self.chain) {
            Ok(f) => f,
            Err(e) => return err(&e.to_string()),
        };
        if writeln!(f, "{}", rec).is_err() {
            return err("audit write failed");
        }
        ok(rec)
    }

    fn tail_hash(&self) -> Option<String> {
        let raw = fs::read_to_string(&self.chain).ok()?;
        let last = raw.lines().last()?;
        let v: serde_json::Value = serde_json::from_str(last).ok()?;
        v.get("hash")?.as_str().map(|s| s.to_string())
    }
}
