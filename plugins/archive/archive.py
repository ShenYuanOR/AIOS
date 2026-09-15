#!/usr/bin/env python3
"""Team winds Gitea down, child files inventory, core packs. This plugin only requests /archive."""
import json
import socket
import sys

def main() -> int:
    if len(sys.argv) < 2:
        print("archive.py <project_id>", file=sys.stderr)
        return 2
    pid = sys.argv[1]
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect("/run/motherd/mother.sock")
    s.sendall(f"/archive {pid}\n".encode())
    print(s.recv(65536).decode())
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
