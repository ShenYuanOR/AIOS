# AIOS

基于 NixOS 的母子以及多虚拟环境机制 AI 操作系统（工位 OS）。

> **工位 OS 核心原则**：NixOS 自身 + 定制命令（加法）+ 签名热插拔插件 + 要落实才开的工作间。  
> 模型是脑，母是全能用户（哥哥只表达需求），家里能办的在家里办，脑没了工位还在。

---

## 快速导航与核心文档

- **规格说明书**：[`SPEC-v1.md`](SPEC-v1.md)
- **快速上手指南**：[`docs/active/USE.md`](docs/active/USE.md)
- **v1 一口气流验收**：[`docs/worked/v1-stream.md`](docs/worked/v1-stream.md)
- **现行任务列表**：[`docs/active/TASKS.md`](docs/active/TASKS.md)
- **研发助手规则**：[`AGENT.md`](AGENT.md)（不含产品硬约束）
- **原版速记与文档存档**：[`docs/README-ORIGINAL.md`](docs/README-ORIGINAL.md)
- **开发仓库**：`http://192.168.1.168:3000/ShenYuan/AIOS`
- **工地 / 第一台母**：NixOS 26.05 `aios` / `192.168.110.99`（KVM）
- **产品 Gitea**：`http://192.168.110.99:3000/`

---

## 核心系统（非插件）

### 1. 系统调用（syscall）
```text
spawn · attach · reap · cap_call · audit · secret_get · quota + bootstrap · rescue
```

### 2. Slash 命令（人手直达，不经模型）
```text
/status · /spawn · /attach · /idle · /reap · /archive · /help
```

### 3. 日常交互与接入
- **回家进母**：本机或 `ssh aios@192.168.110.99`（TTY 全屏，非普通 `$` 终端）。
- **运行命令**：`run` 以 `aios` 用户执行 PATH 下命令。
- **工作间隔离**：工具不在家里或做不了实验才开工作间：`ssh -p 2222 root@192.168.110.99`。
- **工程运维**：工程壳 `ssh root@…` 仍使用 NixOS 自身原生命令。
- **帮助直达**：手敲 `/help` 直达死核；`model` 是插件自持命令。

---

## 项目约束与开发准则

> 修改以下约束前必须明确是在演进产品，并同步更新 [`SPEC-v1.md`](SPEC-v1.md) 与 [`docs/active/`](docs/active/)，严禁只改一处。

1. **核的边界**：核 = NixOS 自身 + 定制命令（加法，不阻挡 NixOS 本身命令）。`motherd` 不是自作内核。
2. **syscall 扩张约束**：核 syscall 插件不可私自扩张（现行表见上）。`/help` 走已有 `quota`，不另开 syscall。插件命令在 manifest `[[commands]]` 登记，由 `/help` 分列，未验签当不存在。
3. **slash 指令规则**：slash 人手不经模型。母把意愿交给已登记动作。模型不得执行 `secret_get` / `cap_call` / 发明命令 / 事事 `/spawn`。
4. **对话注入机制**：注入位于 `mother-nl`（`identity.md` / `operator.md` + 按需 `skill.md` + 会话账本工作集）。禁止在 Python 内写死 AGENT。核不管闲聊。
5. **母的定位**：母是 bash 里的对话 CLI 机制，不是独立 CLI 程序。账本不压缩、不丢弃；工作集不堆全量。母是全能用户代理，不是开房专用角色。
6. **就近处理**：家里能办的在家里办。工具不在家里或家里做不了实验，才另开工作间。工作间之间有能力交集，在某些事情上排异。
7. **概念边界明确**：
   - **项目** = Gitea 护照
   - **家里** = 跟母在一起（登录 `aios`）
   - **工作间** = 落实才开的 microVM（口语称「房间」）
   - **子** = 工作间租客
   - **区别**：房内 CLI ≠ 房间 ≠ 母对话 CLI
8. **核的极简解耦**：核不管闲聊 / Gitea / Letta / 厂商 / Grafana / MCP 目录。预装插件 ≠ 特权。卸了对应能力报 503，slash 依然可用。
9. **发布形态**：产品是签名定制安装 ISO，不是官方 ISO + 脚本。换核必须带签名闭包短切和可回滚；无减法清单的版本非法。
10. **工作流驱动**：现行任务查看 [`docs/active/TASKS.md`](docs/active/TASKS.md)。新气流由人下令后再开启。

---

## 目录结构

| 路径 | 作用 |
|---|---|
| `infra/` | ISO、NixOS 配置、verity 占位、guest 环境 |
| `motherd/` | 死核核心实现 |
| `plugins/` | 唯一系统进化面（插件生态） |
| `docs/active/` | 现行方案、任务规划、决策记录与用法 |
| `docs/worked/` | 已完成任务与历史归档 |
