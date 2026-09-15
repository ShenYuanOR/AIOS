use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("motherctl /status | /spawn <id> | /attach <id> | /idle <id> | /reap <id> | /archive <id>");
        std::process::exit(2);
    }
    let line = args.join(" ");
    let payload = if line.starts_with('/') {
        format!("{line}\n")
    } else {
        format!("{line}\n")
    };
    let mut s = UnixStream::connect("/run/motherd/mother.sock").expect("connect motherd");
    s.write_all(payload.as_bytes()).unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).unwrap();
    print!("{out}");
}
