# MC Launcher

一个从零开始写的开源 Minecraft 启动器。目标是对标 PCL 的功能完整度，但代码全新实现、架构现代化、跨平台。

> 状态：**Phase 0（骨架）** —— 版本清单、任务调度、日志、Java 扫描已可用；安装与启动在 Phase 1。

## 技术栈

| 层 | 技术 |
|---|---|
| 后端核心 | **Rust**（下载引擎 / 版本解析 / 进程管理 / 认证） |
| 前端 UI | **Vue 3 + TypeScript** |
| 桌面壳 | **Tauri 2**（WebView2，产物约 10 MB） |
| 构建 | pnpm + Vite + Cargo |

## 架构

严格分层，`core` 层不依赖任何框架，方便单元测试与跨平台复用。

```
src-tauri/src/
├── core/          纯逻辑，无 IO 副作用
│   └── models.rs      版本 JSON 强类型模型、配置结构
├── infra/         基础设施
│   ├── http.rs        多源（官方 / BMCLAPI）+ 重试 + 磁盘缓存
│   ├── task.rs        任务调度器（进度 / 取消 / 事件推送）
│   ├── logger.rs      分级日志，按天滚动
│   └── paths.rs       应用数据目录
├── services/      业务服务
│   ├── version.rs     版本清单拉取、本地版本扫描
│   └── java.rs        Java 探测（版本 / 厂商 / 位宽）
├── commands/      Tauri IPC 命令（前端唯一入口）
└── state.rs       全局状态（任务、下载器、配置）
```

前端同样分层：

```
src/
├── services/ipc.ts    唯一知道 Rust 命令名的文件
├── services/events.ts Tauri 事件订阅
├── types/index.ts     与 Rust 共享的类型（改这里要同步 models.rs）
├── views/             页面
└── components/        复用组件
```

## 开发

```bash
# 安装依赖
pnpm install

# 开发（Vite HMR + Tauri 窗口）
pnpm tauri:dev

# 构建
pnpm tauri:build
```

### 环境要求

- **Rust** ≥ 1.77（本机已配 `CARGO_HOME` / `RUSTUP_HOME` 指向 D 盘）
- **Node** ≥ 20 + pnpm
- **MSVC**（Visual Studio Build Tools 2022）
- **WebView2**（Windows 10/11 自带）

## 已实现

- [x] 分层项目骨架
- [x] 任务调度器（进度 / 取消 / 事件推送）
- [x] 日志系统（控制台 + 文件，按天滚动）
- [x] HTTP 下载器（官方/镜像双源回退 + 重试 + 缓存）
- [x] 版本清单拉取 + 缓存（1 小时 TTL）
- [x] 本地版本扫描
- [x] Java 探测（版本 / 厂商 / 64 位）
- [x] 苹果风浅色 UI（版本列表 / 设置 / 任务栏）

## 路线图

- [ ] **Phase 1** 安装原版并启动（下载引擎 + classpath + natives + 进程）
- [ ] **Phase 2** 微软正版登录（OAuth 6 步）+ 多账户 + 实例隔离
- [ ] **Phase 3** 模组加载器（Fabric / Quilt / Forge / NeoForge / OptiFine）
- [ ] **Phase 4** Mod 管理与整合包（Modrinth / CurseForge）
- [ ] **Phase 5** 崩溃分析、主题、多语言、自动更新

## 设计要点

**多源下载**：官方源与 BMCLAPI 镜像按用户偏好排序，任一失败自动切换另一个，避免国内下载卡死。

**任务调度**：所有耗时操作都包成任务，向前端推送 `task-updated` 事件，UI 只订阅事件、不轮询。

**版本合并**：`inheritsFrom` 递归合并父版本 —— 这是支持 Forge/Fabric 的前提（Phase 1 实现）。

**变量替换顺序**：`${classpath_separator}` 必须在 `${classpath}` 之前替换，否则会被前缀吃掉。

## 许可证

MIT
