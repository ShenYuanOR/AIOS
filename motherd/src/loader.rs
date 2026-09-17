//! Unsigned plugins do not exist.

use crate::lease::{err, ok};
use crate::syscall::Response;
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Deserialize)]
pub struct PluginCommand {
    pub name: String,
    #[serde(default)]
    pub summary: String,
}

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub coeffects: Vec<String>,
    #[serde(default)]
    pub commands: Vec<PluginCommand>,
    /// One-line card for mother-nl. Empty = listed without speech.
    #[serde(default)]
    pub nl: String,
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
                let name = e.file_name().to_string_lossy().to_string();
                if self.load(&name).ok {
                    names.push(name);
                }
            }
        }
        names.sort();
        ok(json!(names))
    }

    fn clip_nl(s: &str) -> String {
        s.trim().chars().take(80).collect()
    }

    fn signed_manifests(&self) -> Vec<(String, Manifest)> {
        let mut out = Vec::new();
        let Ok(rd) = fs::read_dir(&self.dir) else {
            return out;
        };
        let mut plugs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        plugs.sort();
        for dest in plugs {
            let name = dest
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            if name.is_empty() || !self.load(&name).ok {
                continue;
            }
            let raw = match fs::read_to_string(dest.join("manifest.toml")) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let man: Manifest = match toml::from_str(&raw) {
                Ok(m) => m,
                Err(_) => continue,
            };
            out.push((name, man));
        }
        out
    }

    /// Commands + loaded plugin cards from the same signed scan.
    pub fn help_catalog(&self) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
        let mut commands = Vec::new();
        let mut loaded = Vec::new();
        for (name, man) in self.signed_manifests() {
            let src = if man.name.is_empty() {
                name
            } else {
                man.name
            };
            loaded.push(json!({
                "name": src,
                "nl": Self::clip_nl(&man.nl),
            }));
            for cmd in man.commands {
                let n = cmd.name.trim();
                if n.is_empty() || crate::slash::is_reserved(n) {
                    continue;
                }
                commands.push(json!({
                    "name": n,
                    "summary": cmd.summary,
                    "source": src,
                }));
            }
        }
        (commands, loaded)
    }

    pub fn install_from(&self, src: &Path, seckey: &Path) -> Result<Vec<String>, String> {
        if !src.exists() {
            return Err("plugin src missing".into());
        }
        let mut signed = Vec::new();
        let rd = fs::read_dir(src).map_err(|e| e.to_string())?;
        for e in rd.flatten() {
            let p = e.path();
            if !p.join("manifest.toml").is_file() {
                continue;
            }
            let name = e.file_name();
            let dest = self.dir.join(&name);
            let _ = Command::new("rm")
                .args(["-rf", dest.to_str().unwrap()])
                .status();
            let st = Command::new("cp")
                .args(["-a", p.to_str().unwrap(), dest.to_str().unwrap()])
                .status()
                .map_err(|e| e.to_string())?;
            if !st.success() {
                continue;
            }
            let man = dest.join("manifest.toml");
            let sig = dest.join("manifest.toml.minisig");
            let st = Command::new("minisign")
                .args([
                    "-S",
                    "-s",
                    seckey.to_str().unwrap(),
                    "-m",
                    man.to_str().unwrap(),
                    "-x",
                    sig.to_str().unwrap(),
                    "-t",
                    "aios-plugin",
                ])
                .status()
                .map_err(|e| e.to_string())?;
            if st.success() {
                signed.push(name.to_string_lossy().to_string());
            }
        }
        Ok(signed)
    }
}
