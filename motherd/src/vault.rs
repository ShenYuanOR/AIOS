//! Dumb vault: id -> ciphertext. Core never summarizes or searches.

use crate::lease::{err, ok};
use crate::syscall::Response;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Vault {
    dir: PathBuf,
}

impl Vault {
    pub fn open(data: &Path) -> std::io::Result<Self> {
        let dir = data.join("vault");
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    fn path(&self, id: &str) -> PathBuf {
        let safe: String = id
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' { c } else { '_' })
            .collect();
        self.dir.join(safe)
    }

    pub fn put(&self, id: &str, plaintext: &[u8]) -> Response {
        // Ciphertext at rest: xor with machine-local key file (replaced by age on aios).
        let key = self.load_or_create_key();
        let mut out = plaintext.to_vec();
        for (i, b) in out.iter_mut().enumerate() {
            *b ^= key[i % key.len()];
        }
        if let Err(e) = fs::write(self.path(id), out) {
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
        let key = self.load_or_create_key();
        let mut out = raw;
        for (i, b) in out.iter_mut().enumerate() {
            *b ^= key[i % key.len()];
        }
        let text = String::from_utf8_lossy(&out).to_string();
        ok(json!({ "id": id, "value": text }))
    }

    fn load_or_create_key(&self) -> Vec<u8> {
        let p = self.dir.join(".vaultkey");
        if let Ok(k) = fs::read(&p) {
            if k.len() >= 32 {
                return k;
            }
        }
        let k: Vec<u8> = (0..32).map(|i| ((i * 47 + 13) % 251) as u8).collect();
        let _ = fs::write(p, &k);
        k
    }
}
