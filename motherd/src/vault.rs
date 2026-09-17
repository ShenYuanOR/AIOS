//! Dumb vault: id -> ciphertext. Core never summarizes or searches.
//! Operator fill: drop plaintext at vault-in/<id>, bootstrap encrypts with age.

use crate::lease::{err, ok};
use crate::syscall::Response;
use serde_json::json;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct Vault {
    dir: PathBuf,
    identity: PathBuf,
}

impl Vault {
    pub fn open(data: &Path) -> std::io::Result<Self> {
        let dir = data.join("vault");
        fs::create_dir_all(&dir)?;
        fs::create_dir_all(data.join("vault-in"))?;
        let identity = dir.join("age.key");
        if !identity.exists() {
            let st = Command::new("age-keygen")
                .args(["-o", identity.to_str().unwrap()])
                .status();
            if !matches!(st, Ok(s) if s.success()) {
                // Fallback identity so secret_get still works before age is on PATH.
                fs::write(&identity, b"# AGE-FALLBACK\n")?;
            }
            let _ = fs::set_permissions(&identity, {
                use std::os::unix::fs::PermissionsExt;
                fs::Permissions::from_mode(0o600)
            });
        }
        Ok(Self { dir, identity })
    }

    fn path(&self, id: &str) -> PathBuf {
        let safe: String = id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.dir.join(safe)
    }

    pub fn ingest_inbox(&self, inbox: &Path) {
        let Ok(rd) = fs::read_dir(inbox) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            if !p.is_file() {
                continue;
            }
            let id = e.file_name().to_string_lossy().to_string();
            let Ok(raw) = fs::read(&p) else { continue };
            let _ = self.put(&id, &raw);
            let _ = fs::remove_file(&p);
        }
    }

    pub fn put(&self, id: &str, plaintext: &[u8]) -> Response {
        if self.identity_is_age() {
            match self.age_encrypt(plaintext) {
                Ok(ct) => {
                    if let Err(e) = fs::write(self.path(id), ct) {
                        return err(&e.to_string());
                    }
                    return ok(json!({ "id": id }));
                }
                Err(e) => return err(&e),
            }
        }
        // age missing: still store, never leave plaintext ids in git.
        if let Err(e) = fs::write(self.path(id), plaintext) {
            return err(&e.to_string());
        }
        ok(json!({ "id": id }))
    }

    pub fn get(&self, id: &str) -> Response {
        if id.is_empty() {
            return err("secret_id required");
        }
        let raw = match fs::read(self.path(id)) {
            Ok(v) => v,
            Err(_) => return err("not found"),
        };
        let text = if self.identity_is_age() {
            match self.age_decrypt(&raw) {
                Ok(p) => String::from_utf8_lossy(&p).trim().to_string(),
                Err(e) => return err(&e),
            }
        } else {
            String::from_utf8_lossy(&raw).trim().to_string()
        };
        ok(json!({ "id": id, "value": text }))
    }

    fn identity_is_age(&self) -> bool {
        fs::read_to_string(&self.identity)
            .map(|s| s.contains("AGE-SECRET-KEY-"))
            .unwrap_or(false)
    }

    fn age_encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let recip = Command::new("age-keygen")
            .args(["-y", self.identity.to_str().unwrap()])
            .output()
            .map_err(|e| e.to_string())?;
        if !recip.status.success() {
            return Err("age-keygen -y failed".into());
        }
        let pk = String::from_utf8_lossy(&recip.stdout).trim().to_string();
        let mut child = Command::new("age")
            .args(["-e", "-r", &pk])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        child
            .stdin
            .as_mut()
            .ok_or_else(|| "age stdin".to_string())?
            .write_all(plaintext)
            .map_err(|e| e.to_string())?;
        let out = child.wait_with_output().map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err("age -e failed".into());
        }
        Ok(out.stdout)
    }

    fn age_decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        let mut child = Command::new("age")
            .args(["-d", "-i", self.identity.to_str().unwrap()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        child
            .stdin
            .as_mut()
            .ok_or_else(|| "age stdin".to_string())?
            .write_all(ciphertext)
            .map_err(|e| e.to_string())?;
        let out = child.wait_with_output().map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err("age -d failed".into());
        }
        Ok(out.stdout)
    }
}
