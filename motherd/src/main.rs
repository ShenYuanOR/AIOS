mod audit;
mod lease;
mod loader;
mod microvm;
mod slash;
mod syscall;
mod vault;

use audit::Audit;
use lease::{err, ok, LeaseStore};
use loader::Loader;
use microvm::Driver;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use syscall::{LeaseState, Request, Syscall};
use vault::Vault;

struct Core {
    data: PathBuf,
    leases: LeaseStore,
    vault: Vault,
    audit: Audit,
    loader: Loader,
    vm: Driver,
}

impl Core {
    fn open(data: &Path) -> std::io::Result<Self> {
        fs::create_dir_all(data)?;
        fs::create_dir_all(data.join("keys"))?;
        Ok(Self {
            data: data.to_path_buf(),
            leases: LeaseStore::open(data)?,
            vault: Vault::open(data)?,
            audit: Audit::open(data)?,
            loader: Loader::open(data)?,
            vm: Driver::open(data)?,
        })
    }

    fn dispatch(&self, req: Request) -> syscall::Response {
        let _ = self.audit.append(
            "syscall",
            serde_json::json!({ "op": req.op, "project_id": req.project_id }),
        );
        match req.op {
            Syscall::Bootstrap => self.bootstrap(),
            Syscall::Rescue => ok(serde_json::json!({ "rescue": true, "slash": [
                "/status","/spawn","/attach","/idle","/reap","/archive"
            ]})),
            Syscall::Spawn => {
                let disk = self.data.join("rooms").to_string_lossy().to_string();
                let spawned = self.leases.spawn(&req.project_id, &disk);
                if spawned.ok {
                    let _ = self.vm.spawn(&req.project_id);
                }
                spawned
            }
            Syscall::Attach => self.vm.attach_hint(&req.project_id),
            Syscall::Reap => self.leases.set_state(&req.project_id, LeaseState::Stopped),
            Syscall::CapCall => {
                if req.cap.is_empty() {
                    return err("cap required");
                }
                self.audit.append(
                    "cap_call",
                    serde_json::json!({ "cap": req.cap, "project_id": req.project_id, "args": req.args }),
                )
            }
            Syscall::Audit => self.audit.append("explicit", req.args),
            Syscall::SecretGet => self.vault.get(&req.secret_id),
            Syscall::Quota => {
                if req.cap == "status" || req.args.get("kind").and_then(|v| v.as_str()) == Some("status")
                {
                    return ok(serde_json::json!({ "leases": self.leases.all(), "plugins": self.loader.list().body }));
                }
                if let Some(to) = req.args.get("to") {
                    if let Ok(st) = serde_json::from_value::<LeaseState>(to.clone()) {
                        return self.leases.set_state(&req.project_id, st);
                    }
                }
                ok(serde_json::json!({ "leases": self.leases.all() }))
            }
        }
    }

    fn bootstrap(&self) -> syscall::Response {
        let pub_path = self.data.join("keys").join("plugins.pub");
        if !pub_path.exists() {
            let sec = self.data.join("keys").join("plugins.sec");
            let st = std::process::Command::new("minisign")
                .args(["-G", "-p", pub_path.to_str().unwrap(), "-s", sec.to_str().unwrap(), "-W"])
                .status();
            if !matches!(st, Ok(s) if s.success()) {
                return err("minisign -G failed");
            }
        }
        ok(serde_json::json!({ "bootstrapped": true, "pubkey": pub_path }))
    }
}

fn main() {
    let mut data = PathBuf::from("/var/lib/motherd");
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--data" {
            if let Some(v) = args.next() {
                data = PathBuf::from(v);
            }
        }
    }
    let sock = PathBuf::from("/run/motherd/mother.sock");
    let _ = fs::create_dir_all("/run/motherd");
    let _ = fs::remove_file(&sock);
    let core = Core::open(&data).expect("core");
    let _ = core.bootstrap();
    let listener = UnixListener::bind(&sock).expect("bind");
    let _ = fs::set_permissions(&sock, fs::Permissions::from_mode(0o666));
    eprintln!("motherd listen {}", sock.display());
    for conn in listener.incoming() {
        let Ok(mut stream) = conn else { continue };
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            continue;
        }
        let req = if line.starts_with('/') {
            match slash::parse(line.trim()) {
                Ok(r) => r,
                Err(e) => {
                    let _ = writeln!(stream, "{}", serde_json::to_string(&err(&e)).unwrap());
                    continue;
                }
            }
        } else {
            match serde_json::from_str::<Request>(line.trim()) {
                Ok(r) => r,
                Err(e) => {
                    let _ = writeln!(stream, "{}", serde_json::to_string(&err(&e.to_string())).unwrap());
                    continue;
                }
            }
        };
        let resp = core.dispatch(req);
        let _ = writeln!(stream, "{}", serde_json::to_string(&resp).unwrap());
    }
}

use std::os::unix::fs::PermissionsExt;
