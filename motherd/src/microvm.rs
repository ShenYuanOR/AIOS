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
            "#!/bin/sh\n# room {project_id} — qemu started by motherd\nexec qemu-system-x86_64 -enable-kvm -m 2048 -smp 4 -nographic -serial mon:stdio -drive if=virtio,file={disk} -netdev user,id=n0,hostfwd=tcp::{ssh}-:22 -device virtio-net-pci,netdev=n0\n",
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
        let ssh = Self::ssh_port(project_id);
        if let Some(msg) = self.start_runner(project_id, &dir) {
            return err(&msg);
        }
        ok(json!({
            "project_id": project_id,
            "dir": dir.display().to_string(),
            "ssh_port": ssh,
            "run": script.display().to_string(),
            "note": "guest rootfs from infra/guest; qemu via guest-runner; attach is a pointer"
        }))
    }

    fn units(project_id: &str) -> (String, String) {
        (
            format!("aios-virtiofs-{project_id}"),
            format!("aios-room-{project_id}"),
        )
    }

    fn start_runner(&self, project_id: &str, dir: &Path) -> Option<String> {
        let runner = std::env::var("AIOS_GUEST_RUNNER").ok()?;
        let virtio = format!("{runner}/virtiofsd-run");
        let run = format!("{runner}/microvm-run");
        if !Path::new(&run).exists() {
            return Some("guest-runner missing microvm-run".into());
        }
        let (vfs, room) = Self::units(project_id);
        let _ = Command::new("systemctl")
            .args(["stop", &vfs, &room])
            .status();
        let _ = Command::new("systemctl")
            .args(["reset-failed", &vfs, &room])
            .status();
        let dir_s = dir.to_str()?;
        let path = std::env::var("PATH").unwrap_or_else(|_| {
            "/run/current-system/sw/bin:/nix/var/nix/profiles/default/bin".into()
        });
        let setenv = format!("PATH={path}");
        let sock = dir.join("room-virtiofs-ro-store.sock");
        let _ = fs::remove_file(&sock);
        let _ = fs::remove_file(dir.join("room.sock"));
        let _ = fs::remove_file(dir.join("room-virtiofs-ro-store.sock.pid"));
        let vfs_st = Command::new("systemd-run")
            .args([
                &format!("--unit={vfs}"),
                "--collect",
                "--working-directory",
                dir_s,
                "--setenv",
                &setenv,
                &virtio,
            ])
            .status();
        if !matches!(vfs_st, Ok(s) if s.success()) {
            return Some("virtiofsd-run failed".into());
        }
        let mut ready = false;
        for _ in 0..50 {
            if sock.exists() {
                ready = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        if !ready {
            let _ = Command::new("systemctl").args(["stop", &vfs]).status();
            return Some("virtiofs socket not ready".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
        let room_st = Command::new("systemd-run")
            .args([
                &format!("--unit={room}"),
                "--collect",
                "--working-directory",
                dir_s,
                "--setenv",
                &setenv,
                &run,
            ])
            .status();
        if !matches!(room_st, Ok(s) if s.success()) {
            let _ = Command::new("systemctl").args(["stop", &vfs]).status();
            return Some("microvm-run failed".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(400));
        let live = Command::new("systemctl")
            .args(["is-active", "--quiet", &room])
            .status();
        if !matches!(live, Ok(s) if s.success()) {
            let _ = Command::new("systemctl").args(["stop", &vfs]).status();
            return Some("microvm-run exited".into());
        }
        None
    }

    pub fn stop(&self, project_id: &str) -> Response {
        let (vfs, room) = Self::units(project_id);
        let _ = Command::new("systemctl").args(["stop", &room, &vfs]).status();
        let pidfile = self.rooms.join(project_id).join("qemu.pid");
        if let Ok(pid) = fs::read_to_string(&pidfile) {
            let _ = Command::new("kill").args([pid.trim()]).status();
            let _ = fs::remove_file(&pidfile);
        }
        ok(json!({ "project_id": project_id, "stopped": true }))
    }

    fn ssh_port(_project_id: &str) -> u32 {
        2222
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
                        if !Self::skip_ip(ip) && !hosts.iter().any(|h| h == ip) {
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
                    if !ip.is_empty() && !Self::skip_ip(ip) && !hosts.iter().any(|h| h == ip) {
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

    pub fn attach_hint(&self, project_id: &str) -> Response {
        let ssh = Self::ssh_port(project_id);
        let hosts = Self::advertise_hosts();
        let cmds: Vec<String> = hosts
            .iter()
            .map(|h| format!("ssh -p {ssh} root@{h}"))
            .collect();
        let cmd = cmds
            .first()
            .cloned()
            .unwrap_or_else(|| format!("ssh -p {ssh} root@aios"));
        ok(json!({
            "project_id": project_id,
            "port": ssh,
            "hosts": hosts,
            "cmd": cmd,
            "cmds": cmds,
            "gitea": "http://10.0.2.2:3000/",
            "note": "open a new terminal; mother is not a PTY middlebox"
        }))
    }
}
