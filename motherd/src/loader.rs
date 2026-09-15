//! Unsigned plugins do not exist.

use crate::lease::{err, ok};
use crate::syscall::Response;
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub coeffects: Vec<String>,
}

pub struct Loader {
    dir: PathBuf,
    pubkeys: PathBuf,
}

impl Loader {
    pub fn open(data: &Path) -> std::io::Result<Self> {
        let dir = data.join("plugins");
        fs::create_dir_all(&dir)?;
        Ok(Self {
            dir,
            pubkeys: data.join("keys").join("plugins.pub"),
        })
    }

    pub fn load(&self, name: &str) -> Response {
        let plug = self.dir.join(name);
        let man_path = plug.join("manifest.toml");
        if !man_path.exists() {
            return err("plugin not found");
        }
        let sig = plug.join("manifest.toml.minisig");
        if !sig.exists() {
            return err("no signature: treat as missing");
        }
        if !self.pubkeys.exists() {
            return err("no plugin pubkey");
        }
        let status = Command::new("minisign")
            .args([
                "-Vm",
                man_path.to_str().unwrap(),
                "-p",
                self.pubkeys.to_str().unwrap(),
            ])
            .status();
        match status {
            Ok(s) if s.success() => ok(json!({ "name": name, "loaded": true })),
            _ => err("bad signature: treat as missing"),
        }
    }

    pub fn list(&self) -> Response {
        let mut names = Vec::new();
        if let Ok(rd) = fs::read_dir(&self.dir) {
            for e in rd.flatten() {
                if e.path().join("manifest.toml").exists() {
                    names.push(e.file_name().to_string_lossy().to_string());
                }
            }
        }
        ok(json!(names))
    }
}
