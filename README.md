# AIOS

工位 OS：NixOS + verity 死核 `motherd` + 签名热插拔插件 + 每项目一间 microVM。

模型是脑，房间是身体，母是房东。脑没了身体还在。

- 冻结规格：[`SPEC-v1.md`](SPEC-v1.md)（不改核 syscall）
- 仓库：http://192.168.1.168:3000/ShenYuan/AIOS
- 工地机：NixOS 26.05 `aios` / `192.168.110.99`（KVM）
- 开发：Win 写代码（LF）；构建和验房间只在 `aios`

## 核（非插件）

syscall：`spawn` `attach` `reap` `cap_call` `audit` `secret_get` `quota` + `bootstrap` `rescue`

slash（不经模型）：`/status` `/spawn` `/attach` `/idle` `/reap` `/archive`

## 目录

| 路径 | 作用 |
|---|---|
| `infra/` | ISO、NixOS、verity、guest、CI |
| `motherd/` | 死核 |
| `plugins/` | 唯一进化面 |
| `docs/active/` | 规划与运行时任务 |
| `docs/worked/` | 完成与归档 |

`infra/` `motherd/` `plugins/` 尚未落代码。当前任务见 `docs/active/`。
