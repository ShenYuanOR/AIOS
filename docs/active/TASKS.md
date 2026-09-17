# 任务（运行时）

断流从本表第一个非「完成」行接着干。无开口气流时不要空转。

v1 T0–T12 已归档：[`docs/worked/v1-stream.md`](../worked/v1-stream.md)。
用法：[`USE.md`](USE.md)。拍板：[`DECISIONS.md`](DECISIONS.md)。

## 现在

| id | 状态 | 项 |
|---|---|---|
| U1 | 完成 | motherctl 人读块；欢迎语单块；REPL 里敲 ssh 拦截并给新开终端命令 |
| U2 | 完成 | LAN 放行 2222；`/attach` 指针；归档后拒绝 attach 且停 VM。工地已 switch |
| U3 | 完成 | 皮命令 `model` + 机舱模型表单。不扩 slash |
| U3b | 完成 | `/model` 皮拦截；一行投钥；compat 要模型 id（现 grok-4.6） |
| U4 | 完成 | 客厅 TTY 全屏（ratatui + crossterm，同一 motherctl）。无 TTY / `-c` / 管道 / 启动失败退行模式。NL 底栏进度。工地已 switch |
| U15 | 完成 | 态势=本机参数/运行；键位下栏 + F1 面板；输入栏无 mother>，标题为可改昵称 nick。工地已 switch |
| U16 | 完成 | 闲聊进度写入对话「进度」行；输入栏显示当前步骤+秒数。工地已 switch |
| U17 | 完成 | 母昵称 OSER；命令结果不摊给哥哥；执行步「流」；态势→系统状态进度条。工地已 switch |
| U18 | 完成 | 放开 OSER/`host.run` 命令白名单：PATH 下任意命令名，aios argv，仍非壳/非 PTY。工地已 switch |
| U19 | 完成 | 回看不截断半行；TUI 按词换行；host.run 输出 256KiB 完整行。工地已验：ps aux 307 行完整装入 |
| U20 | 完成 | 首次进全屏居中 AIOS 门户字，开口对话后清掉。工地已验 |
| U21 | 完成 | 淡化客厅：改称家里。回家找母，房间只指工作间。工地已验 |
| U5 | 完成 | 母 REPL rustyline：方向键/退格；Ctrl-C 清行不掉 SSH。cliproxy `192.168.1.168:8317`：三别名 openai_compat；母/代码 grok-4.6，便宜 deepseek-v4-flash。`model <url>` 当设地址。工地已 switch |
| U6 | 完成 | `/help` 皮命令（`help` 同样拦）。短索引：六个 slash、进房、model、机舱。核套接字仍 unknown slash。工地已 switch |
| U7 | 完成 | `/help` 进核 slash（走 quota，不新增 syscall）。插件 `[[commands]]` 登记；/help 分列核/插件。`model` 来源 model-router。工地已 switch |
| U8 | 完成 | 母 NL 对话注入短索引。每轮拉 `/help`（含 `loaded`）。插件 `nl` 自报。新挂件下一轮可见。工地已 switch |
| U9 | 完成 | 母=系统操作员。NL 校验后把意愿交给核 slash。缺项目名就问。工地已 switch |
| U10 | 完成 | 对话 CLI 机制：identity.md/operator.md/skill.md；会话账本；工作集不压缩不堆。工地已 switch |
| U11 | 完成 | 角色对齐：核=NixOS+定制加法；母=全能用户；客厅也是房间，工具不够才另开工作间。工地已 switch |
| U12 | 完成 | 客厅身份登录即母；对话 `run` 走 `cap_call host.run`（白名单、aios、非 PTY）。工地已 switch |
| U13 | 完成 | 本机 TTY 中文：kmscon + Sarasa Term SC。工地已 switch |
| U14 | 完成 | 对话自查：`do` 后回看结果再答；账本带输出。工地已 switch |

## 交接

权威：`SPEC-v1.md` + `DECISIONS.md` + 本表。不扩 syscall。称呼哥哥，中文。
源码：Win `F:\AIOS`（LF）→ SFTP `/root/AIOS`。构建只在 aios。
Windows OpenSSH 卡密码，用 Git Bash 或 paramiko。
NIX_CONFIG：flakes；substituters USTC + cache.nixos.org。
vault id：`model.grok.api_key` `model.gpt.api_key` `model.deepseek.api_key` `model.openai_compat.api_key` `model.openai_compat.base_url`
私钥 `/var/lib/motherd/keys/plugins.sec` 不出机、不进 git。
