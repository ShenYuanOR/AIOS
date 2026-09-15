# 方案（现行）

工地已在：NixOS 26.05 `aios` / 192.168.110.99 / KVM。产品 ISO 还不存在。

## 分期

0. 工地收口：本仓骨架、`flake.nix`、把 `aios` 最小配置收回 `infra/nixos`
1. 死核：`motherd` 能无模型管机（slash + 租约 + 验签加载空插件 + microVM 驱动）
2. 一间房：guest 模板 + `/attach`；16GB 只保证同时一间能写代码的房
3. 首期 8 插件：路由、母 NL、mux/SKILL、Gitea、Letta、归档、机舱、metrics
4. 产品 ISO：verity 签名 2–4GB 净装；验收=刷盘后能 `/spawn`

v1 不做：热迁、多母、跨机 GPU。

## 验收（v1）

一台机 · 一张净装 ISO · slash+母 NL · 路由 · mux · Gitea · Letta+哑柜 · 归档 · 机舱 · 一间房能写代码并推仓。
