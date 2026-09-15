# 方案（现行）

一口气流。开发仓与产品 Gitea 隔离。工地 `aios` 即 v1 第一台母。

工作面（可并行，互不改 syscall）：

1. `infra/` flake、aios 模块、guest、verity、ISO
2. `motherd/` 死核（syscall/slash/租约/验签/哑柜/审计/microVM/rescue）
3. `plugins/` 八个预装：路由（四路接入）、母 NL、mux、产品 Gitea、Letta、归档、机舱、metrics

验收：无模型 slash 管机；一间房能写代码并推 **产品 Gitea**；卸插件 503、slash 还在；能打签名净装 ISO。
