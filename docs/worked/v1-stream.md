# v1 一口气流（已验收）

归档日期：2026-09-15。权威规格仍是根目录 `SPEC-v1.md`。核 syscall 未扩。

原 `docs/active/PLAN.md` / `TASKS.md` 的 T0–T12 气流并入本文件，不再在 active 双份存放。

## 方案（当时）

一口气流。开发仓与产品 Gitea 隔离。工地 `aios` 即 v1 第一台母。

工作面：

1. `infra/` flake、aios 模块、guest（NAT+10.0.2.2）、verity 占位、ISO
2. `motherd/` 死核（syscall/slash/租约/验签/age 哑柜/审计/microVM/rescue/REPL）
3. `plugins/` 八个预装：路由（四路）、母 NL、mux、产品 Gitea（LAN）、官方 Letta 0.16.8、归档、机舱、metrics

验收（均已过）：无模型 slash 管机；`/spawn hello` 能写代码并推产品 Gitea；aios 上打出签名 ISO 文件。卸插件 503 为规格项，本轮未做卸载演练。

## 任务表

| id | 状态 | 项 |
|---|---|---|
| T0 | 完成 | `flake.lock`：nixpkgs `c3eea5b2156d`（nixos-26.05.20260914.c3eea5b），microvm `614e954`；已拷回 Win `F:\AIOS\flake.lock` |
| T1 | 完成 | motherd-0.1.0 编出 |
| T2 | 完成 | `/spawn` → virtiofsd-run + microvm-run；guest mem 2176 |
| T3 | 完成 | 四路空钥均 `no_model` + `degraded` |
| T4 | 完成 | `aios` SSH ForceCommand motherctl；套接字 `root:mother` `0660` |
| T5 | 完成 | mux `127.0.0.1:7461` |
| T6 | 完成 | 产品 Gitea LAN http://192.168.110.99:3000/ ；管理员 ShenYuan；仓 hello |
| T7 | 完成 | Letta `0.16.8` 容器 Up；`127.0.0.1:8283`；插件 :8284；LAN 不通 8283 |
| T8 | 完成 | cockpit :7460 LAN；metrics :9105 本机 |
| T9 | 完成 | `plugins.sec` 仅 aios；8 个插件 `.minisig` |
| T10 | 完成 | `nixos-rebuild switch #aios` |
| T11 | 完成 | 房 `hello` SSH；`ROOM.txt` 推入 hello 仓 `65931bf` |
| T12 | 完成 | ISO 1.8G + minisign 验签 ok（不刷第二台） |

气流结束后补：motherctl REPL 人读输出；非 slash 走 `mother-nl` 插件（空钥降级，不再 JSON 解析失败）。

## 工地快照（验收当时）

- 主机：`aios` / `192.168.110.99`，NixOS 26.05，KVM
- 系统闭包（REPL 格式化那次 switch）：`/nix/store/g6y4yb79hav3vlvp4mi9hm3masif4jpa-nixos-system-aios-26.05.20260914.c3eea5b`
- 产品 Gitea：http://192.168.110.99:3000/ShenYuan/hello/src/branch/main/ROOM.txt
- 房间：`hello` running；进房 `ssh -p 2222 root@127.0.0.1`（仅母本机，LAN 不放行 2222）
- ISO：`/nix/store/nkn61c8v31wagcif4j36d3krwdi83k5v-aios.iso/iso/aios.iso`（1.8G）
- 签名：`/root/AIOS/aios.iso.minisig`（trusted comment `aios-iso`）；Win 副本 `F:\AIOS\aios.iso.minisig`
- 私钥：`/var/lib/motherd/keys/plugins.sec` 不出机、不进 git

## 工程记录（避免重踩）

- qemu `microvm` + virtiofs **正好 2048M + ACPI** 会停在 KASLR。guest `mem = 2176`。
- `/spawn` 必须先删残留 `room-virtiofs-ro-store.sock`，等新套接字出现再启 qemu；只看文件存在会连上死套接字。
- virtiofsd 包装脚本需要 coreutils（`id`/`nproc`）；`systemd-run` 必须 `--setenv PATH=`。
- 套接字：`User=root` `Group=mother` `UMask=0007`，避免 `ExecStartPost chown` 竞态。
- `sign-iso`：ISO 在 nix store 只读，`.minisig` 写到可写路径（`-x`）。
- 母 NL 由 **motherctl** 调插件，不在 motherd 请求里同步 exec（插件会回连 `secret_get`，单线程会死锁）。
- Windows OpenSSH 卡密码；用 Git Bash 或 paramiko。Git Bash `known_hosts` 在 rebuild sshd 后可能要 `ssh-keygen -R 192.168.110.99`。
- 源码：Win `F:\AIOS`（LF）SFTP → `/root/AIOS`。不要在 lock 过程中整树覆盖把 `flake.lock` 冲掉。
- `infra/verity/` 仅密钥路径占位，**不是** dm-verity 根。ISO 配置未挂 `guestRunner`，净装 ISO 要能 `/spawn` 还需后续把 guest-runner 打进 ISO 闭包。

## 未做（不是失败）

- 不刷第二台
- 模型钥空（允许；slash 管机）
- 插件卸载 503 演练
- 开发仓 192.168.1.168 本轮 aios 到不了，未从工地推 AIOS 自身
