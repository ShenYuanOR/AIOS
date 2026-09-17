# 已拍板

- 一口气流，不按阶段关门。断流只看 `TASKS.md`。v1 气流已归档到 `docs/worked/v1-stream.md`。
- 工地即第一台母：motherd 常驻 `aios`（`192.168.110.99`），ISO 同一 flake。
- 开发仓：http://192.168.1.168:3000/ShenYuan/AIOS（只开发 AIOS 本身）。
- 产品 Gitea：AIOS 自持，LAN `http://192.168.110.99:3000/`。与 192.168.1.168 隔离。房间推产品 Gitea。
- 第一间验收房项目名：`hello`。
- 房间：qemu user/NAT；`10.0.2.2` 是母。房间 SSH 绑母 `0.0.0.0:2222`，LAN 放行。用户新开终端 `ssh -p 2222 root@<母LAN>` 进房，不换 root SSH、母 REPL 不当 PTY。
- 母 REPL 皮命令 `model`（看/改别名、投钥）不是 slash，不扩 syscall。钥仍进哑柜。
- `/help` 是核 slash，走 `quota` kind=help，不新增 syscall。核表：六个管机 slash + `/help`。
- 插件自持命令在 `manifest.toml` 的 `[[commands]]` 登记，`/help` 按来源分列（核 / 插件名）。未验签当不存在。`model` 由 `model-router` 登记，执行仍在 motherctl。
- 无斜杠 `help` 由 motherctl 转核 `/help`，避免进闲聊。
- 母是 bash 里的对话 CLI 机制，不是独立 CLI 程序。人格在 `identity.md`/`operator.md`，插件 `skill.md` 按需装，`nl` 一句当目录。会话账本 jsonl 只追加。工作集超窗少装旧轮，不摘要覆盖账本、不暗截当前句。
- 对话注入在 mother-nl，不进核。每轮拉 `/help` JSON（含 `loaded[].nl`）和 `/status` 租约短表进目录层。新插件验签加载后下一轮可见。不灌钥/mux 工具表。
- 核 = NixOS 自身能力 + 定制命令（加法，不挡 NixOS 本身命令）。`aios` 家门、`root` 工程门，两门加法。母 = 全能用户：减轻/替代真人去敲和编排复杂命令、多条命令；哥哥只表达需求。家里能办的在家里办；工具不在家里或家里做不了实验才另开工作间。家里不是一间房。工作间之间有能力交集，在某些事情上排异（同项目同盘已 running 则 `/attach` 给指针，不再 `/spawn`）。
- 母编排已登记动作：NL 输出 `{"say","do"}`，插件校验后交给核。`do` 跑完插件把结果再给模型一拍改 `say`（自查）；账本带结果进工作集，能答就不再跑。人手 `/` 仍不经模型。缺项目名就问、不猜。reap/archive 的项目名必须出现在原话里。不扩 syscall。不得把每个需求映射成开工作间。落地每轮至多一条 `do`；多命令编排另开口。
- 回家有 TTY 进全屏（ratatui + crossterm，同一 `motherctl`，不新开登录程序）。无 TTY / `-c` / 管道 / 启动失败 → 现行行模式。不扩 syscall，母不当 PTY。机舱 `:7460` 仍 LAN 按钮。`root` 仍 `$`。Ctrl-C 清输入，Ctrl-D 或 `exit` 离开。NL 进度走 stderr `#progress`/`#step`，TUI 以「流」展示执行步（推理/执行/消化+耗时），画面不冻。OSER 消化命令结果，不把原文摊给哥哥（`/spawn` 进房指针除外）。系统状态用进度条展示 CPU/内存/存储/交换/网络/房间。键位在输入框下栏，F1 开详情。输入栏不写 `mother>`，标题是母昵称（皮命令 `nick`，默认 OSER）。对话用户侧固定「哥哥」。首次进全屏对话区居中「AIOS」门户字，开口对话后清掉进对话。
- guest mem 2176（qemu microvm+virtiofs 正好 2048M 会 ACPI 卡死）。
- 记忆：官方 Letta `0.16.8`，端口 8283 只绑本机。插件吝啬入库。空钥时进程仍在，模型调用降级。
- 签名 ISO：aios 上打出文件 + minisign 即 v1 验收，不要求再刷一台。私钥不出机、不进 git。
- 模型路由四路：Grok、GPT、Deepseek、OpenAI 格式。密钥进哑柜。空钥允许，slash 仍管机。
- guest：NixOS + git + tmux/vim + 通用编译链。
- 16GB：同时只一间 running 写代码房。
- motherd：Rust。房间：microvm.nix + qemu/KVM。
- 家里身份：`aios` 登录壳即 motherctl（本机/串口/SSH 同一扇门，回家）。SSH 另留 ForceCommand；sshd 会 `$SHELL -c` 再包一层，motherctl 认出自己则进 REPL。`root` 工程壳仍是 `$`。
- 本机显示器中文：kmscon + Sarasa Term SC（内核 TTY 画不出汉字；Noto CJK 在 nixpkgs 是可变字体 TTC，kmscon 会画方块）。Windows SSH 方块是客户端字体，不在核里补。
- 对话执行：不扩 syscall。`cap_call` cap=`host.run`，PATH（`/run/current-system/sw/bin`）下任意命令名 argv，降权 `aios`，超时 8s，输出封顶 256KiB 且按完整行截。人手皮命令 `run`（与 `model` 同类，不是 slash）。禁止管道 / `sh -c` / 母当 PTY。不是命令白名单。回看用完整行（头+尾），不在行中切断；TUI 按词换行，不拆 ASCII 词。
- 产品 Gitea 管理员：`ShenYuan` / 口令与主机约定相同（仅 LAN）。仓 `hello` 开机自建。
- 机舱 `7460` LAN 打开即用，按钮走核，无独立登录页。
- REPL：slash 人读输出在 motherctl（TTY）；套接字仍 JSON。闲聊走 `mother-nl` 插件，不经核 JSON 解析。
- mux/metrics 只本机。Letta 不暴露 LAN。

vault id：`model.grok.api_key` `model.gpt.api_key` `model.deepseek.api_key` `model.openai_compat.api_key` `model.openai_compat.base_url`
