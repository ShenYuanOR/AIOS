# AIOS

工位 OS：NixOS 自身 + 定制命令（加法）+ 签名热插拔插件 + 要落实才开的工作间。

模型是脑。母是全能用户（哥哥只表达需求）。家里能办的在家里办。脑没了工位还在。

- 规格：[`SPEC-v1.md`](SPEC-v1.md)
- 开发仓：http://192.168.1.168:3000/ShenYuan/AIOS
- 工地 / 第一台母：NixOS 26.05 `aios` / `192.168.110.99`（KVM）
- 产品 Gitea：http://192.168.110.99:3000/
- 开发：Win 写代码（LF）；构建和验房间只在 `aios`
- 研发助手规则：[`AGENT.md`](AGENT.md)（不含产品硬约束）

v1 一口气流已验收：[`docs/worked/v1-stream.md`](docs/worked/v1-stream.md)。怎么用：[`docs/active/USE.md`](docs/active/USE.md)。

## 核（非插件）

syscall：`spawn` `attach` `reap` `cap_call` `audit` `secret_get` `quota` + `bootstrap` `rescue`

slash（不经模型）：`/status` `/spawn` `/attach` `/idle` `/reap` `/archive` `/help`

日常：本机或 `ssh aios@192.168.110.99` 回家进母（有 TTY 全屏，不是 `$`）。`run` 以 aios 跑 PATH 下命令名。工具不在家里或做不了实验才开工作间：`ssh -p 2222 root@192.168.110.99`。工程壳 `ssh root@…` 仍是 NixOS 自身命令。人手 `/help` 直达核。`model` 是插件自持命令。

## 项目约束

改这些之前要明确是在演进产品，并同步 `SPEC-v1.md` / `docs/active/`，不要只改一处。

- 核 = NixOS 自身 + 定制命令（加法，不挡 NixOS 本身命令）。motherd 不是自作内核。
- 核 syscall 由插件不可私自扩张；现行表见上。`/help` 走已有 `quota`，不另开 syscall。插件命令在 manifest `[[commands]]` 登记，由 `/help` 分列；未验签当不存在。
- slash 人手不经模型。母把意愿交给已登记动作。模型不得 `secret_get` / `cap_call` / 发明命令 / 事事 `/spawn`。
- 对话注入在 mother-nl：`identity.md` / `operator.md` + 按需 `skill.md` + 会话账本工作集。禁止 py 内写死 AGENT。核不管闲聊。
- 母是 bash 里的对话 CLI 机制，不是独立 CLI 程序。账本不压缩、不丢弃；工作集不堆全量。母是全能用户代理，不是开房专用角色。
- 家里能办的在家里办。工具不在家里或家里做不了实验，才另开工作间。工作间之间有能力交集，在某些事情上排异。
- 概念不许混：项目 = Gitea 护照；家里 = 跟母在一起（登录 `aios`）；工作间 = 落实才开的 microVM；房间 = 工作间的口语；子 = 工作间租客；房内 CLI ≠ 房间 ≠ 母对话 CLI。
- 核不管闲聊 / Gitea / Letta / 厂商 / Grafana / MCP 目录。预装插件 ≠ 特权。卸了对应能力 503，slash 还在。
- 产品是签名定制安装 ISO，不是官方 ISO + 脚本。换核必须带签名闭包短切和可回滚；无减法清单的版本非法。
- 现行任务看 [`docs/active/TASKS.md`](docs/active/TASKS.md)。新气流由人下令再开。

## 目录

| 路径 | 作用 |
|---|---|
| `infra/` | ISO、NixOS、verity 占位、guest |
| `motherd/` | 死核 |
| `plugins/` | 唯一进化面 |
| `docs/active/` | 现行方案、任务、拍板、用法 |
| `docs/worked/` | 完成与归档 |
