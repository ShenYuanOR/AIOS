use crate::lease::{err, ok};
use crate::syscall::Response;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct Driver {
    rooms: PathBuf,
}

impl Driver {
    pub fn open(data: &Path) -> std::io::Result<Self> {
        let rooms = data.join("rooms");
        fs::create_dir_all(&rooms)?;
        Ok(Self { rooms })
    }

    pub fn spawn(&self, project_id: &str) -> Response {
        let dir = self.rooms.join(project_id);
        if let Err(e) = fs::create_dir_all(&dir) {
            return err(&e.to_string());
        }
        let script = dir.join("run.sh");
        let body = format!(
            "#!/bin/sh\n# room {project_id} — qemu started by motherd\nexec qemu-system-x86_64 -enable-kvm -m 4096 -smp 4 -nographic -serial mon:stdio -drive if=virtio,file={disk} -netdev user,id=n0,hostfwd=tcp::{ssh}-:22 -device virtio-net-pci,netdev=n0\n",
            project_id = project_id,
            disk = dir.join("disk.qcow2").display(),
            ssh = 22000 + (project_id.bytes().fold(0u32, |a, b| a.wrapping_add(b as u32)) % 1000),
        );
        let _ = fs::write(&script, body);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&script, fs::Permissions::from_mode(0o750));
        }
        if !dir.join("disk.qcow2").exists() {
            let st = Command::new("qemu-img")
                .args([
                    "create",
                    "-f",
                    "qcow2",
                    dir.join("disk.qcow2").to_str().unwrap(),
                    "40G",
                ])
                .stdout(Stdio::null())
                .status();
            if !matches!(st, Ok(s) if s.success()) {
                return err("qemu-img failed (guest image not baked yet)");
            }
        }
        ok(json!({
            "project_id": project_id,
            "dir": dir,
            "note": "guest rootfs from infra/guest; overlay disk ready"
        }))
    }

    pub fn attach_hint(&self, project_id: &str) -> Response {
        let ssh = 22000 + (project_id.bytes().fold(0u32, |a, b| a.wrapping_add(b as u32)) % 1000);
        ok(json!({
            "project_id": project_id,
            "cmd": format!("ssh -p {ssh} root@127.0.0.1"),
            "note": "mother is not a PTY middlebox; attach is a pointer"
        }))
    }
}
