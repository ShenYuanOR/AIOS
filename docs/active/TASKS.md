# 任务（运行时）

一口气流。断流从本表第一个非「完成」行接着干。不要改 syscall 表。
开发仓：192.168.1.168:3000/ShenYuan/AIOS。产品 Gitea 是插件，隔离。

| id | 状态 | 项 | 目录 |
|---|---|---|---|
| T0 | 进行 | flake + 三面骨架 + aios 配置入库 | `flake.nix` `infra/` |
| T1 | 待做 | motherd 死核可跑（socket+slash+租约+审计+哑柜+验签+rescue） | `motherd/` |
| T2 | 待做 | microVM 驱动 + guest 模板；同项目双 running 拒绝 | `motherd/` `infra/guest/` |
| T3 | 待做 | 模型路由四路（Grok/GPT/Deepseek/OpenAI格式），空钥可降级 | `plugins/model-router/` |
| T4 | 待做 | 母 NL（走路由；无钥时说明降级） | `plugins/mother-nl/` |
| T5 | 待做 | mux/SKILL | `plugins/mux-skill/` |
| T6 | 待做 | 产品 Gitea 插件（自持，非 192.168.1.168） | `plugins/gitea/` |
| T7 | 待做 | Letta 插件在 aios 拉起 | `plugins/letta/` |
| T8 | 待做 | 归档 / 机舱 Web / metrics | `plugins/archive/` `cockpit-web/` `metrics-export/` |
| T9 | 待做 | aios 现生签名钥；私钥 `/var/lib/motherd/keys/` | 工地机 |
| T10 | 待做 | motherd 常驻 aios；slash 无模型可用 | 工地机 |
| T11 | 待做 | 一间房写代码并推产品 Gitea | 工地机 |
| T12 | 待做 | 签名净装 ISO | `infra/iso/` `infra/verity/` |

vault id：`model.grok.api_key` `model.gpt.api_key` `model.deepseek.api_key` `model.openai_compat.api_key` `model.openai_compat.base_url`

完成行删掉，对应短记录进 `docs/worked/`。
