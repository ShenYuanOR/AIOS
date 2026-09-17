# AIOS（工位 OS）定版 v1

冻结规格。此后只加签名插件，不改核 syscall 表。
称呼用户为「哥哥」。始终中文。

一句话：核是 NixOS 自身能力 + 定制命令（加法，不挡 NixOS）。母是全能用户（哥哥只表达需求，母去敲和编排命令）。家里能办的在家里办；工具不在家里或家里做不了实验，才另开工作间。模型是脑。脑没了工位还在。

---

## 是 / 不是

是：在不阻碍 NixOS 本身命令的前提下加定制命令；母减轻/替代真人去敲和编排复杂命令、多条命令；家里能办的就在家里办；真正要落实文件或跑起来才另开工作间；子在工作间满权限干活；项目可停可档可迁。

不是：自作内核、废掉 NixOS 命令；无论做什么都开工作间；母只负责 `/spawn`；母当 IDE / PTY；监控/插件当人格；第二套任务系统；用户现场 curl|bash 拼器官。

---

## 概念（不许混）

| 名 | 定义 |
|---|---|
| 项目 | Gitea 护照：代码、Issue/PR、Release、flake |
| 家里 | 跟母在一起。登录 `aios` 就是回家。闲聊、系统状态、编排、模型、插件。家里能办的不另开工作间 |
| 房间 | 口语指工作间。开房 = 开工作间。回家 ≠ 开房 |
| 工作间 | 落实文件或把项目/点子/需求跑起来才开的 microVM（盘 + 配额 + 路由） |
| 子 | 工作间的系统租客（project_id 主键，uid 是租约） |
| 工作人员 | 工作间再派的脑，母不登记 |
| CLI | 工作间内进程，可重启，≠ 房间；也 ≠ 母这条对话 CLI 机制 |
| 母 | 全能用户代理。哥哥只表达需求。母去敲、编排复杂命令和多条命令。闲聊是插件；定制命令是核/插件 |
| 能力 | 已登记、带范围的 cap_call |
| 插件 | 带 manifest 的签名闭包，可卸可逆 |
| 技能 | 说明书，≠ 授权 |

main 分支 Agent 也是子，不是母。系统母子只到工位。

---

## 核（NixOS + 定制，日常零提交 syscall 表）

核 = NixOS 自身能力 + 定制化能力。前提是不阻碍 NixOS 本身命令，再加上定制命令来实现需要的机制。工程壳（`ssh root`）仍是完整 NixOS；`nixos-rebuild` / `systemctl` 不被废掉。`aios` 是家门，`root` 是工程门，两门加法。motherd 的 syscall/slash 是定制命令层，不是自作内核。

syscall 钉死：`spawn / attach / reap / cap_call / audit / secret_get / quota` + `bootstrap` / `rescue`

焊在核里：验签加载器、卸载反演、租约表、slash、cgroup/PSI 感官、哑柜机制、审计 hash 链、SSH/串口针脚、microVM 驱动。

slash（人手不经模型）：`/status /spawn /attach /idle /reap /archive /help`
人手敲的 slash 由 motherctl 直达核。母把意愿编成已登记动作交给核/插件；执行、拒绝仍在核。插件把执行结果交回母看完再答。模型不得 `secret_get` / `cap_call`、不得发明 slash、不得编项目名去做 reap/archive。不得把每个需求都映射成开房。
`/help` 是核短索引，走已有 `quota`，不新增 syscall。核 slash 与已验签插件在 manifest `[[commands]]` 登记的自持命令分列。未加载当不存在。插件不可登记与核 slash 撞名的命令。

物理：到期卸插件；超顶杀到 idle；无签名当不存在；无心跳 reap；记忆超顶拒写；临时 cap 到期切断。第三态=bug。反演失败=插件标毒、依赖房冻结。

核不管：闲聊、Gitea、Letta、厂商、Grafana、MCP 目录、项目对不对。

换核=签名闭包短切+可回滚；无减法清单的版本非法。

---

## 插件（唯一进化面）

预装 ≠ 特权。卸了对应能力 503，slash 还在。

首期预装（进 ISO 闭包，开机自挂）：
1. 模型路由
2. 母 NL
3. mux / SKILL（房内一根 MCP，list_changed）
4. Gitea
5. 记忆 Letta + sleep-time
6. 归档
7. 机舱 Web（按钮走核）
8. metrics 导出（皮可卸，感官在核）

v1 不做：热迁、多母共管、跨机 live GPU。

学 Cordis：卸载可逆；coeffect 声明依赖。学 DSH：模型可见 ⇒ 必须进审计。拒绝：agent loop 当可 patch 核。

可视化/母 NL/CLI 皮/mux=插件。采集/熔断/slash ≠ 插件。

---

## 房间与权限

家里不是一间房。工具不在家里、或家里不能做实验，才另开工作间。不是无论做什么都开工作间。

工作间只 microVM。工作间之间有能力交集（例如 `/nix/store` 只读共享），在某些事情上排异（同项目同盘两个 running → spawn 拒绝；实验不在家里做）。工作间内无开发墙。出工作间只有 cap_call。

生命周期：running → idle → stopped → archived → tombstoned → purged
归档：团队收摊 Gitea → 子交清单 → 核打包。停 ≠ 回收 ≠ 销毁。

---

## 交互

回家 `aios`（本机登录或 SSH）→ 母对话 CLI（有 TTY 全屏，无 TTY / `-c` / 管道为行模式）；slash 永远在；NL 是插件。不是只有 SSH 才进母。SSH `root` / 本机 `root` → 工程壳（NixOS 自身命令）。
家里 PATH 下命令名走已有 `cap_call`（cap=`host.run`），以 `aios` 跑 argv，不是 bash、不是 PTY、不扩 syscall、不设命令白名单。人手皮命令 `run`。管道和交互壳仍在工程门/工作间。
/attach → 工作间 tmux + 官方 CLI。母不当 PTY 中间人。本窗不进工作间。
MCP/SKILL 热更：CLI 只连 mux；下轮 list_changed；不重启终端。当前 turn 不换工具表。
对话注入：母是 bash 里的对话 CLI 机制（不是独立 CLI 程序）。人格/规程在插件 `identity.md` `operator.md`；能力表述在各插件 `skill.md`（按需装）。会话账本只追加、不摘要覆盖、不自动删。本轮工作集 = 身份 + 活目录 + 按需 skill + 装得下的账本原文。超窗少装旧轮并留指针，不压缩、不暗截。全量清单是账本/mux/slash 活数据。授权在核。未验签当不存在。当前 turn 不换表。
母的目的是编排复杂命令和多条命令，替代哥哥去敲。落地：先核 slash 已登记动作或 `run` PATH 命令名（每轮至多一条）；插件动作另开口登记；NixOS 工程命令走 `root` 壳，不被定制层废掉。人手 `/` 仍不经模型。家里能办则不 `/spawn`。
`/help`：核给出命令短索引（来源分核/插件）和已挂名录。活数据仍看 `/status`。

---

## 模型

别名 chat-mother / chat-code / cheap。熔断切供应商不切 CLI。兼容通道空了 → 该房 stopped。无模型：slash 仍管机。

---

## 记忆与保险箱

Letta + A-MEM 机制：格式开放、入库吝啬。项目决策在 Gitea。冲突：用户指令 > 仓库 > 记忆。

哑柜：id → 密文 + 归属 + 轮换时间。无检索无摘要。
本房间本项目钥默认注入 CLI。测试/生产=不同 id，同一套 secret_get。
长期钥跟项目；临时钥跟房间。回收先发新再废旧。模型不裁决给不给钥。

---

## 资源与熵

母保底不可压。子 guarantee+ceiling。PSI 高先杀子。自进化=插件生灭。

---

## 发行

产品=签名定制安装 ISO（2–4 GB 净装），不是干净官方 ISO+脚本。
不打进 ISO：大模型、全套驱动、各官方 CLI。
用户：U 盘向导写盘 → 开机自挂插件 → 能 /spawn。
开发：Win 可写代码（LF）；构建/验在 NixOS。Hyper-V 只当 SSH 开发机，spawn/ISO 要 KVM 或真机。

---

## v1 范围

一台机 · 一张净装 ISO · slash+母 NL · 路由 · mux · Gitea · Letta+哑柜 · 归档 · 机舱 · 一间房真能写代码并推仓。

技术：NixOS · microVM · Gitea · Letta · sops/age 哑柜 · SSH+母对话（TTY 全屏）+官方 CLI · Cordis/DSH 哲学（syscall 表不当可 patch 行）。

---

## 开发环境

代码可在 Win 敲。系统只能在 NixOS 上长出来。SSH 到 NixOS 构建。不要 WSL2/Hyper-V 当生产验房间。
