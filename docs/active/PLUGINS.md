# 插件生态与接口规范 (Plugins Specification)

本文档定义 AIOS 插件的目录组织规范、生命周期管理、通信契约与异常容错机制。

---

## 1. 现有内置插件矩阵 (Built-in Plugin Matrix)

| 插件名称 | 路径 | 核心能力 | 暴露命令 / 接口 | 依赖资源 |
|---|---|---|---|---|
| `mother-nl` | `plugins/mother-nl/` | 自然语言意图解析与工作集上下文维护 | `nl` | 大模型 API、`identity.md`、`operator.md` |
| `model-router` | `plugins/model-router/` | 多模型端点路由、Token 路由与别名映射 | `model` | Vault 凭证、OpenAI/Claude 兼容 API |
| `mux-skill` | `plugins/mux-skill/` | 工具能力复用与指令分发调度 | `mux` | 插件 RPC 通信通道 |
| `cockpit-web` | `plugins/cockpit-web/` | Web 态势可视化看板与控制台服务 | Web 服务 (HTTP) | 死核状态监控套接字 |
| `archive` | `plugins/archive/` | 任务工作间运行产物与状态快照归档 | `archive` | 存储持久化路径 |
| `metrics-export`| `plugins/metrics-export/` | 系统资源与指标监控导出 | Prometheus 格式端点 | 系统审计日志与配额表 |
| `gitea` | `plugins/gitea/` | 本地代码托管平台接入与身份认证 | Gitea API 客户端 | Gitea 服务实例 |
| `letta` | `plugins/letta/` | 长期记忆与认知状态管理桥接 | 记忆状态接口 | Letta 服务/存储后端 |

---

## 2. 插件规范与通信协议 (Plugin Specification & Contract)

### 2.1 目录结构标准
所有合法插件必须包含以下标准化结构：

```text
plugins/<plugin-name>/
├── manifest.toml    # 插件元数据、命令声明与签名元信息（必选）
├── skill.md         # 技能说明与大模型交互语义定义（必选）
├── <entrypoint>     # 插件执行入口（Python 脚本或二进制 ELF）
└── identity.md      # 角色或交互约束补充说明（可选）
```

### 2.2 `manifest.toml` 规范字段
```toml
name = "model-router"
version = "0.1.0"
description = "多模型动态路由与凭证管理插件"

# 注册至死核的扩展命令列表
[[commands]]
name = "model"
description = "查询当前模型或切换上游端点"
usage = "model [model_id | endpoint_url]"
```

---

## 3. 进程管理与容错降级 (Lifecycle & Fault Tolerance)

1. **动态加载与发现 (Discovery)**：
   - 死核在引导或热重载时遍历 `plugins/` 目录。
   - 提取各插件 `manifest.toml` 中声明的 `[[commands]]`，注册至死核指令路由表，并通过 `/help` 统一输出。
2. **进程隔离与安全沙箱**：
   - 插件作为死核的子进程派生运行，通信基于标准 I/O 管道或 Unix Domain Socket。
   - 敏感配置与密钥（API Key）不通过环境变量广播，插件需调用 `secret_get` 经由验签授权后按需获取。
3. **503 隔离与优雅降级**：
   - 插件故障、未响应或被卸载时，死核对其暴露命令拦截并返回 `503 Service Unavailable`。
   - 任何单点插件异常均不会导致死核 `motherd` 进程崩溃或挂起。
