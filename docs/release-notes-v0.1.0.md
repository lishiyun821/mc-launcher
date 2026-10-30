# v0.1.0 — 第一个能用的版本

An open-source Minecraft launcher written from scratch: **Rust + Vue 3 + Tauri 2**, ~2.3 MB installer.

## 这个版本能做什么

### 安装原版

- 版本清单 921 个（含快照 / 远古版），本地缓存 1 小时
- 完整安装：客户端 jar + 全部依赖库 + 资源文件 + natives 解压
- **镜像优先**（BMCLAPI），失败自动降级官方源
- 并发 16 下载，SHA1 全量校验，`.part` 原子落盘，失败重试 3 次

实测安装 1.21.11：**4667 个文件 / 498 MB / 185 秒**，全部校验通过。

### 启动游戏

- classpath 按 `rules` 拼装，20+ 个 Mojang 变量替换
- 离线 UUID 与 Java 的 `UUID.nameUUIDFromBytes` 完全一致（换启动器存档不丢）
- 启动前完整性自检，缺文件直接报，不让游戏莫名其妙崩
- Java 挑选：实例指定 → 满足要求里最低版本 → 兜底最高
- 日志实时落盘，界面里直接看

### 实例管理

每个实例独立：内存 / Java 路径 / 窗口尺寸 / 游戏目录 / 额外 JVM 参数。
配置原子写、可复制、可删除、可校验文件。

### 微软正版登录

- 设备码流程，**密码永不经过启动器**
- 用 Xbox 官方公共 client_id，**下载了就能登**，不用自己注册 Azure
- 完整链路：设备码 → MS token → Xbox Live → XSTS → Minecraft 服务 → profile
- 令牌只存本地 `accounts.json`，界面层永远拿不到 token
- 自动续期、多账户切换、无正版自动回落离线

### Java 自动下载

Adoptium + **清华 TUNA 镜像**（直连 GitHub CDN 在国内会卡死）。
装到启动器自己的目录，不动系统环境变量，装完自动被扫描发现。

### 界面

- 启动台首页：当前实例大卡 + 巨型启动按钮 + 账户状态
- 日志查看器（从文件尾部读，几 GB 日志也不卡）
- 任务栏可展开（进度 / 耗时 / 取消）
- 主题：跟随系统 / 浅色 / 深色
- 下载限速
- 中英双语

## 安装

下载 `MC Launcher_0.1.0_x64-setup.exe`，双击安装。
默认装到当前用户目录，**不需要管理员权限**。

首次启动会自动检测系统 Java；没有也可以直接在设置页下一键装 Java 21。

## 系统要求

- Windows 10 / 11（x64）
- WebView2（Win10 1803+ 和 Win11 自带）
- 想跑游戏还得有 Java —— 启动器可以帮你下

## 已知问题

- **没有代码签名**：Windows SmartScreen 会提示「未知发布者」，点「更多信息 → 仍要运行」即可。
- 更新检查只**提示**新版本，不会自动下载安装（等有 CI 流水线再接 Tauri updater）。
- 还没做模组加载器（Fabric / Forge 等），只能装原版。

## 下一步

- [ ] 模组加载器（Fabric / Quilt / Forge / NeoForge / OptiFine）
- [ ] Mod 管理与整合包（Modrinth / CurseForge）
- [ ] 启动器自动更新
- [ ] 崩溃分析

## 许可证

MIT — 随便用、随便改、随便分发。
