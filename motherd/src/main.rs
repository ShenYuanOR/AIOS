mod audit;
mod lease;
mod loader;
mod microvm;
mod run;
mod slash;
mod syscall;
mod vault;

use audit::Audit;
use lease::{err, ok, LeaseStore};
use loader::Loader;
use microvm::Driver;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
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
        self.vault.ingest_inbox(&self.data.join("vault-in"));
        let _ = self.audit.append(
            "syscall",
            serde_json::json!({ "op": req.op, "project_id": req.project_id }),
        );
        match req.op {
            Syscall::Bootstrap => self.bootstrap(),
            Syscall::Rescue => ok(serde_json::json!({ "rescue": true, "slash": slash::CORE
                .iter()
                .map(|(n, _, _)| *n)
                .collect::<Vec<_>>()
            })),
            Syscall::Spawn => {
                let disk = self.data.join("rooms").to_string_lossy().to_string();
                let spawned = self.leases.spawn(&req.project_id, &disk);
                if spawned.ok {
                    let vmr = self.vm.spawn(&req.project_id);
                    if !vmr.ok {
                        let _ = self.leases.set_state(&req.project_id, LeaseState::Stopped);
                        return vmr;
                    }
                }
                spawned
            }
            Syscall::Attach => match self.leases.load(&req.project_id) {
                None => err("no lease"),
                Some(l) => match l.state {
                    LeaseState::Running | LeaseState::Idle => self.vm.attach_hint(&req.project_id),
                    other => {
                        let st = serde_json::to_value(other)
                            .ok()
                            .and_then(|v| v.as_str().map(|s| s.to_string()))
                            .unwrap_or_else(|| format!("{other:?}"));
                        err(&format!("not_attachable:{st}"))
                    }
                },
            },
            Syscall::Reap => {
                let r = self.leases.set_state(&req.project_id, LeaseState::Stopped);
                if r.ok {
                    let _ = self.vm.stop(&req.project_id);
                }
                r
            }
            Syscall::CapCall => {
                if req.cap.is_empty() {
                    return err("cap required");
                }
                let _ = self.audit.append(
                    "cap_call",
                    serde_json::json!({ "cap": req.cap, "project_id": req.project_id, "args": req.args }),
                );
                if req.cap == "host.run" {
                    return crate::run::host_run(&req.args);
                }
                ok(serde_json::json!({ "cap": req.cap, "audited": true }))
            }
            Syscall::Audit => self.audit.append("explicit", req.args),
            Syscall::SecretGet => self.vault.get(&req.secret_id),
            Syscall::Quota => {
                if req.cap == "help" || req.args.get("kind").and_then(|v| v.as_str()) == Some("help")
                {
                    return self.help_index();
                }
                if req.cap == "status" || req.args.get("kind").and_then(|v| v.as_str()) == Some("status")
                {
                    return ok(serde_json::json!({ "leases": self.leases.all(), "plugins": self.loader.list().body }));
                }
                if let Some(to) = req.args.get("to") {
                    if let Ok(st) = serde_json::from_value::<LeaseState>(to.clone()) {
                        let r = self.leases.set_state(&req.project_id, st);
                        if r.ok
                            && matches!(
                                st,
                                LeaseState::Stopped
                                    | LeaseState::Archived
                                    | LeaseState::Tombstoned
                                    | LeaseState::Purged
                            )
                        {
                            let _ = self.vm.stop(&req.project_id);
                        }
                        return r;
                    }
                }
                ok(serde_json::json!({ "leases": self.leases.all() }))
            }
        }
    }

    fn help_index(&self) -> syscall::Response {
        let core: Vec<serde_json::Value> = slash::CORE
            .iter()
            .map(|(name, usage, summary)| {
                serde_json::json!({
                    "name": name,
                    "usage": usage,
                    "summary": summary,
                    "source": "core"
                })
            })
            .collect();
        let (plugins, loaded) = self.loader.help_catalog();
        ok(serde_json::json!({
            "core": core,
            "plugins": plugins,
            "loaded": loaded,
            "host_run": crate::run::help_body(),
        }))
    }

    fn bootstrap(&self) -> syscall::Response {
        let pub_path = self.data.join("keys").join("plugins.pub");
        let sec = self.data.join("keys").join("plugins.sec");
        if !pub_path.exists() {
            let st = std::process::Command::new("minisign")
                .args([
                    "-G",
                    "-p",
                    pub_path.to_str().unwrap(),
                    "-s",
                    sec.to_str().unwrap(),
                    "-W",
                ])
                .status();
            if !matches!(st, Ok(s) if s.success()) {
                return err("minisign -G failed");
            }
            let _ = fs::set_permissions(&sec, fs::Permissions::from_mode(0o600));
        }
        self.vault.ingest_inbox(&self.data.join("vault-in"));
        let mut signed = Vec::new();
        if let Ok(src) = std::env::var("AIOS_PLUGIN_SRC") {
            if let Ok(n) = self.loader.install_from(Path::new(&src), &sec) {
                signed = n;
            }
        }
        ok(serde_json::json!({
            "bootstrapped": true,
            "pubkey": pub_path,
            "signed_plugins": signed
        }))
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
    let _ = fs::set_permissions(&sock, fs::Permissions::from_mode(0o660));
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
                    let _ = writeln!(
                        stream,
                        "{}",
                        serde_json::to_string(&err(&e.to_string())).unwrap()
                    );
                    continue;
                }
            }
        };
        let resp = core.dispatch(req);
        let _ = writeln!(stream, "{}", serde_json::to_string(&resp).unwrap());
    }
}
