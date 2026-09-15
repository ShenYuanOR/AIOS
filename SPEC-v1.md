# AIOS（工位 OS）定版 v1

冻结规格。此后只加签名插件，不改核 syscall 表。
称呼用户为「哥哥」。始终中文。

一句话：NixOS + verity 死核（motherd）+ 签名热插拔插件 + 每项目一间 microVM。用户刷定制安装 ISO。模型是脑，房间是身体，母是房东。脑没了身体还在。

---

## 是 / 不是

是：给 AI 独立身份和近乎完整开发身体的 OS；母闲聊+态势+开房+追进度；子在房内满权限干活；项目可停可档可迁。

不是：AI 全机 root；自作内核；滚动 Arch 生产根；母当 IDE；监控/插件当人格；第二套任务系统；用户现场 curl|bash 拼器官。

---

## 概念（不许混）

| 名 | 定义 |
|---|---|
| 项目 | Gitea 护照：代码、Issue/PR、Release、flake |
| 房间 | microVM + 盘 + 配额 + 路由。母开停闲档 |
| 子 | 房间的系统租客（project_id 主键，uid 是租约） |
| 工作人员 | 房内再派的脑，母不登记 |
| CLI | 房内进程，可重启，≠ 房间 |
| 母 | 闲聊是插件；slash 是核 |
| 能力 | 已登记、带范围的 cap_call |
| 插件 | 带 manifest 的签名闭包，可卸可逆 |
| 技能 | 说明书，≠ 授权 |

main 分支 Agent 也是子，不是母。系统母子只到工位。

---

## 核（非插件，日常零提交）

syscall 钉死：`spawn / attach / reap / cap_call / audit / secret_get / quota` + `bootstrap` / `rescue`

焊在核里：验签加载器、卸载反演、租约表、slash、cgroup/PSI 感官、哑柜机制、审计 hash 链、SSH/串口针脚、microVM 驱动。

slash（不经模型）：`/status /spawn /attach /idle /reap /archive`

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

只 microVM。/nix/store 只读共享。房内无开发墙。出房只有 cap_call。同项目同盘两个 running → spawn 拒绝。

生命周期：running → idle → stopped → archived → tombstoned → purged
归档：团队收摊 Gitea → 子交清单 → 核打包。停 ≠ 回收 ≠ 销毁。

---

## 交互

SSH → 母 REPL（slash 永远在；NL 是插件）。
/attach → 房内 tmux + 官方 CLI。母不当 PTY 中间人。
MCP/SKILL 热更：CLI 只连 mux；下轮 list_changed；不重启终端。当前 turn 不换工具表。
对话注入：短索引；全量清单是 mux/slash 活数据。授权在核。

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

技术：NixOS · microVM · Gitea · Letta · sops/age 哑柜 · SSH+薄 REPL+官方 CLI · Cordis/DSH 哲学（syscall 表不当可 patch 行）。

---

## 开发环境

代码可在 Win 敲。系统只能在 NixOS 上长出来。SSH 到 NixOS 构建。不要 WSL2/Hyper-V 当生产验房间。
