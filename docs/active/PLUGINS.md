# AIOS 插件生态全景说明

AIOS 死核（`motherd`）保持绝对收敛，系统的所有交互进化面、大模型对接与扩展能力均以插件（Plugins）形式实现。

---

## 一、官方内置插件一览

| 插件名称 | 目录路径 | 核心能力与职责 | 挂载命令 |
|---|---|---|---|
| **mother-nl** | `plugins/mother-nl/` | 自然语言解析中枢，负责意图校验、身份（`identity.md`）注入、会话账本管理与核 Slash 代交 | `nl` |
| **model-router** | `plugins/model-router/` | 大模型多端路由与别名管理，对接 OpenAI 兼容端点、Grok、DeepSeek 等模型 | `model` |
| **mux-skill** | `plugins/mux-skill/` | 技能复用与多路调度器，负责把外挂工具与指令流聚合给对话层 | `mux` |
| **cockpit-web** | `plugins/cockpit-web/` | Web 态势看板与控制台，提供图形化仪表盘与实时状态监控 | Web UI 服务 |
| **archive** | `plugins/archive/` | 工作间产物归档与环境快照保存器，用于持久化任务执行结果 | `archive` |
| **metrics-export** | `plugins/metrics-export/` | 资源监控与性能指标导出器（Prometheus / Grafana 兼容格式） | 监控端点 |
| **gitea** | `plugins/gitea/` | 代码仓库集成插件，提供项目与代码护照管理 | Gitea 客户端 |
| **letta** | `plugins/letta/` | 长期记忆与认知状态管理插件，维护全局与会话级认知闭环 | 记忆桥接 |

---

## 二、插件结构与规范要求

每个合法插件目录必须包含以下标准文件：

```text
plugins/<plugin-name>/
├── manifest.toml    # 插件元数据、签名与命令登记（必选）
├── skill.md         # 技能说明与大模型注入提示词（必选）
├── <entry>.py / bin # 插件可执行入口逻辑
└── identity.md      # 人格或特定角色注入说明（可选）
```

### manifest.toml 示例
```toml
name = "model-router"
version = "0.1.0"
description = "多模型动态路由与凭证接入"

[[commands]]
name = "model"
description = "查询或切换当前使用的大模型"
usage = "model [model_id | endpoint_url]"
```

---

## 三、插件生命周期与安全约束

1. **注册机制**：插件启动时由死核扫描 `plugins/` 目录，读取 `manifest.toml` 中 `[[commands]]` 并由死核 `/help` 分列输出。
2. **503 优雅降级**：卸载或关闭某插件仅影响其自持的扩展命令（对应报错 503），死核的所有系统调用与 Slash 命令不受任何影响。
3. **安全边界**：插件禁止直接读写未经死核授权的系统关键目录，敏感 API 凭证必须通过 `secret_get` 向死核哑柜申请。
