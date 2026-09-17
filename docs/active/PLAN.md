# 方案（现行）

v1 一口气流已验收，归档：[`docs/worked/v1-stream.md`](../worked/v1-stream.md)。

工地 `aios`（`192.168.110.99`）即第一台母。规格仍冻结：`SPEC-v1.md`，不扩 syscall。

## 本气流（母 REPL 可用性）

框架在，细节按哥哥现场：格式乱、NL 和进房抢同一条 SSH、欢迎语堆叠、没有模型配置口。

| 面 | 本轮 | 不本轮 |
|---|---|---|
| 输出 | motherctl 人读块；TTY 全屏（U4） | 独立 CLI 程序 |
| 进房 | 新开终端 `ssh -p 2222 root@<母LAN>`；母不当 PTY | 母 REPL 里嵌套壳 |
| 欢迎语 | 压 SSH motd/lastlog；REPL 单块横幅 | 花哨动画 |
| 模型 | 皮命令 `model` + 机舱表单；钥进哑柜 | 新 slash / 新 syscall |
| 行编辑 | TTY 全屏输入栏（U4）；失败退 rustyline（U5：Ctrl-C 清、不掉线） | 嵌套壳 |
| 全屏家里 | 有 TTY 进全屏；NL 进度不冻；slash/run/model 同脸（U4） | 新登录程序；母当 PTY；fork 别的 agent TUI |
| 帮助 | 核 slash `/help`（U7）；插件 `[[commands]]` 登记；来源分列 | 新 syscall |
| 对话注入 | 母 NL 短索引（U8）；活数据 `/help`；插件 `nl` 自报；感知新挂件 | 核写人格；mux 工具表进母 |
| 操作员 | 母 NL 代交核 slash（U9）；allowlist；缺参就问 | 模型 secret_get/cap_call；母当 PTY；每轮确认框 |
| 对话 CLI 机制 | md 人格/skill、会话账本、工作集（U10）；TTY 全屏（U4） | 独立 CLI 程序；摘要压缩器 |
| 角色 | 核=NixOS+定制加法；母=全能用户；家里能办的在家里办，工具不够才另开工作间（U11） | 母只开房；事事 spawn；废 NixOS 命令 |
| 回家 | `aios` 身份登录即母（U12）；本机不是 `$` | 拿掉 root 的 `$` |
| 本机中文 | kmscon + Sarasa Term SC（U13） | 内核 vgacon 补汉字；Noto CJK VF/TTC |
| 对话执行 | `run` / `cap_call host.run` PATH 命令名 argv，以 aios 跑（U12；U18 去掉命令白名单） | 核解析管道；母当 PTY；`sh -c` |
| 对话自查 | `do` 后回看结果再答；账本带输出（U14）；回看按完整行，不半行截断（U19） | 只 dump 不读；模型自己 cap_call；按折行误判 |

核仍不管厂商。别名文件和投钥是插件+哑柜入口；motherctl 只是人读皮。
