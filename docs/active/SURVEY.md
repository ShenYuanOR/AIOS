# AIOS 技术调研与竞品生态报告

本文档汇总了 AI 操作系统、Agent 执行环境隔离、轻量化虚拟化与不可变系统的前沿调研，为 AIOS 的长期演进提供理论与工程参考。

---

## 一、AI 操作系统与 Agent 沙箱架构对比

| 维度 / 项目 | **AIOS (母子工位系统)** | **OS-Copilot / OpenDevin** | **Letta (MemGPT 体系)** | **Cloudflare Sandbox / Firecracker** |
|---|---|---|---|---|
| **核心架构** | 原生 NixOS + 死核 + MicroVM 工作间 | Python Agent + Docker 容器封装 | 内存管理层 + 外部 LLM API 服务 | 极简 MicroVM 运行时 (轻量 KVM) |
| **执行解耦** | 脑与工位彻底解耦，模型掉线原生系统命令完好 | 深度绑定 Python 进程，脚本故障则环境崩溃 | 侧重记忆持久化，对操作系统管控较浅 | 纯粹的执行沙箱，无原生「母」角色伴随 |
| **隔离级别** | 硬件级 KVM MicroVM 隔离 | 容器级 cgroups/namespace 隔离 | 进程/沙箱隔离 | 极简微虚机隔离 |
| **交互形态** | TTY 全屏客厅（家里） + 独立工作间（房间） | WebUI / 命令行交互 | Web / REST API | Headless API / 终端流 |
| **不可变性** | NixOS Flakes 声明式 + dm-verity 验签 | 传统 Linux 发行版镜像 | 依赖云端或外部基础设施 | 极简 Rootfs |

### AIOS 的架构优势
1. **分级伴随设计（母子架构）**：兼顾了日常陪伴与高危实验隔离。「家里」处理 80% 的日常工程与交互，「工作间」承担 20% 的高危编译与隔离测试。
2. **确定性与可回滚**：依托 NixOS，系统每一个版本变更均具备哈希确定性与即时可回滚能力，从根本上避免 Agent 污染宿主依赖。
3. **严格的受限死核**：死核 `motherd` 不受模型幻觉影响，Slash 命令人手直达，杜绝模型越权调用系统底层能力。

---

## 二、轻量化虚拟化选型调研

| 虚拟化引擎 | 启动耗时 | 内存底噪 | 设备/网络支持 | 适用场景 |
|---|---|---|---|---|
| **NixOS MicroVM (基于 QEMU/KVM)** | ~300ms - 800ms | 32MB - 64MB | 完整 VirtIO、TAP/MacVTap 网络、9p/virtio-fs 共享 | **AIOS 当前最优选**（NixOS 生态原生集成、声明式构建） |
| **Firecracker** | ~100ms - 200ms | 5MB - 10MB | 极简 VirtIO（网络、块设备、vsock），不支持文件系统共享 | 极轻量 Serverless 计算（适合只跑单次无状态任务） |
| **Cloud-Hypervisor** | ~150ms - 300ms | 15MB - 30MB | Rust 原生、virtio-fs、vhost-user 丰富 | 未来高性能多租户演进备选 |

**调研结论**：
当前基于 NixOS Flakes 的 MicroVM 机制最符合 AIOS 现状：可直接复用 NixOS 的包闭包和声明式配置，无需额外引入外部镜像管理层。

---

## 三、插件体系与模型路由演进

1. **多模型热路由（Model Router）**：
   - 区分代码重载模型（如 Claude / Grok-4.6）与轻量/闲聊模型（如 DeepSeek / Flash 系列）。
   - 插件层拦截 `/model`，统一与安全哑柜（Vault）打通，实现 Token 与 API 密钥在内存中的受控调度。
2. **记忆层与知识库结合（Letta & RAG）**：
   - 会话账本（Ledger）记录真实交互轨迹，不压缩、不丢弃。
   - 插件通过工作集（Working Set）机制向模型上下文按需注入，避免超长上下文带来的推理抖动与成本飙升。
