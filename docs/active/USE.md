# 用法（工地母）

口令按项目约定，不写进仓库。Windows 用 Git Bash，不要用自带 OpenSSH。

## 两扇门（不要混）

| 怎么进 | 身份 | 用途 |
|---|---|---|
| 真机/串口登录 `aios`，或 `ssh aios@192.168.110.99` | 家里（母） | 回家。有 TTY 进全屏。slash / 闲聊 / `model` / `run` / `/help`。无 TTY 或 `motherctl -c` 才是行模式 `mother>` |
| `ssh -p 2222 root@192.168.110.99` | 工作间 | 工具不在家里或做不了实验才开；新开终端进 |
| 真机/SSH `root` | 工程壳 | NixOS 自身命令、rebuild、SFTP；这里才是 `$` |

主机钥变了：`ssh-keygen -R 192.168.110.99` 后再连。房间端口另记一份：`ssh-keygen -R '[192.168.110.99]:2222'`。

母 REPL 里不要敲 `ssh`。敲了会提示你新开终端，不会进房。

本机显示器走 kmscon + Sarasa Term SC，中文应能显示。Windows SSH 若仍是方块，改终端字体（如 Sarasa Term SC / 微软雅黑），不是工位缺字。

## slash（不经模型）

核 slash 七个（六个管机 + `/help` 索引）。人手 `/` 直达核、不经模型。闲聊：母当全能用户，哥哥只说需求；家里能办的不要另开工作间。落实代码/跑实验且家里不够才 `/spawn`。会话账本跨重连还在。`/help` 分列核命令和已加载插件自持命令。无斜杠 `help` 由 REPL 转核。`model` / `run` 是皮命令，不是 slash。

```
/help
/status
run
run df -h
run systemctl status motherd
/spawn hello
/attach hello
/idle hello
/reap hello
/archive hello
exit
```

`run` 以家里用户 `aios` 跑 PATH（`/run/current-system/sw/bin`）下的命令名，例如 `run cat /etc/os-release`、`run ps aux`。不是 bash：没有管道、不能 `sh -c`。要管道或交互，走工程门或工作间。闲聊里母会看完输出再答；账本里已有结果能答就不再跑。

同时只一间 running 写代码房。同项目同盘已 running，`/spawn` 拒绝。归档后不能 `/attach`。

机舱（按钮走核）：http://192.168.110.99:7460/

## 进房

`/attach hello` 只给指针。新开一个 Git Bash：

```
ssh -p 2222 root@192.168.110.99
```

房内推产品 Gitea：

```
git clone http://ShenYuan:root@10.0.2.2:3000/ShenYuan/hello.git
```

LAN 看仓：http://192.168.110.99:3000/

## 模型

母 REPL 或机舱。不是 slash（`/model` 在 REPL 里会转成皮命令，核表仍只有六个 slash）：

```
model
model mother openai_compat
model url http://192.168.1.168:8317/v1
model http://192.168.1.168:8317/v1
model key openai_compat
model key openai_compat sk-...
model name grok-4.6
model name cheap deepseek-v4-flash
```

有 TTY 时是全屏：上头「系统状态」（CPU/内存/存储进度条、负载、任务、温度、交换、网络、房间）。首次进场对话区居中「AIOS」门户字，开口说话后清掉才出对话。输入栏左上角是母昵称（默认 OSER，`nick 某某` 改，`nick 默认` 复位），栏里只有光标，不写 `mother>`。对话里用户侧是「哥哥」。快捷键在输入框下方「键」栏；F1（空输入也可 `?`）打开键位面板。Enter 发送。Ctrl-C 清输入，不掉 SSH。Ctrl-D 或 `exit` 离开。闲聊执行步以「流」展示（推理 / 执行 / 消化，带耗时），OSER 自己消化命令结果，不把原文摊给哥哥。输入栏显示当前步骤和已用秒数。`model key <适配器>` 在框里收钥、不回显。无 TTY 仍是 `mother>` 行模式。`model key …` 不进历史。

仍可在工程壳投钥（母会在下一请求收走）：

```
printf '%s' 'KEY' > /var/lib/motherd/vault-in/model.grok.api_key
```

id 见 `TASKS.md`。空钥闲聊降级，slash 仍可用。

## 入口

| 入口 | 地址 |
|---|---|
| 产品 Gitea | http://192.168.110.99:3000/ |
| 机舱 | http://192.168.110.99:7460/ |
| 房间 SSH | `192.168.110.99:2222`（LAN，先 /spawn） |
| mux | 仅 aios `127.0.0.1:7461` |
| Letta | 仅 aios `127.0.0.1:8283` |
| ISO | `/root/AIOS/result-iso/iso/aios.iso` + `/root/AIOS/aios.iso.minisig` |

私钥 `/var/lib/motherd/keys/plugins.sec` 不出机。
