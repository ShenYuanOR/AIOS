# 已拍板

- 一口气流，不按阶段关门。断流只看 `TASKS.md`。
- 工地即第一台母：motherd 常驻 `aios`，ISO 同一 flake。
- 开发仓：http://192.168.1.168:3000/ShenYuan/AIOS（只开发 AIOS 本身）。
- 产品 Gitea：AIOS 自持，插件拉起，与 192.168.1.168 隔离。房间推产品 Gitea。
- 记忆：aios 上记忆插件拉起 Letta。
- 签名钥：aios 现生，私钥不出机、不进 git。
- 模型路由四路：Grok、GPT、Deepseek、OpenAI 格式。密钥进哑柜，本轮允许空钥（slash 仍管机）。
- guest：NixOS + git + tmux/vim + 通用编译链，推产品 Gitea。
- 16GB：同时只一间 running 写代码房。
- motherd：Rust。房间：microvm.nix + qemu/KVM。

自行定（不问）：机舱只 LAN；产品 Gitea 绑本机端口不与 ssh 冲突；vault id 见 TASKS。
