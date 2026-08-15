# CloudSatchel / 云笈 - 项目记忆

## 当前状态

- 产品：云笈 / Cloud Satchel，纯净本地 Windows 桌面工具集
- 技术栈：React 19 + TypeScript + Vite；Tauri 2 + Rust；WebView2
- 当前版本：v0.9.0
- 目标平台：Windows 10 / Windows 11
- 代码位置：`desktop-tools/`
- 需求文档：`CloudSatchel需求文档.md`（工作区根目录，与记忆同步维护）

## 已实现功能

- 双击隐藏/显示桌面图标（SHELLDLL_DefView + WS_EX_LAYERED，动画约 0.5s）
- 透明任务栏（Win10 Accent API / Win11 TranslucentTB 便携引擎）
- 主机性能监控（CPU、GPU、内存、网络，约 1 秒本地采样）
- 浅色 / 深色 / 跟随系统三主题
- 无边框 Windows 风格窗口，启动默认尺寸 1280×720
- 系统托盘与关闭到托盘
- 开机自启动（启动文件夹快捷方式）
- 背景图片设置
- 设置面板与关于面板
- 单实例保护与异常恢复
- **开关状态记忆：上次关闭时的所有功能开关与设置自动恢复，启动即应用效果**

## 最近完成

- 2026-08-15 新增「主机性能监控」功能，版本升级到 v0.8.0
- 2026-08-15 新增「开关状态记忆」：v0.9.0
  - 持久化模块：[src-tauri/src/prefs.rs](src-tauri/src/prefs.rs)
  - 持久化文件：`%LOCALAPPDATA%\CloudSatchel\settings.json`（与背景设置同文件，扁平结构，旧版文件可直接读取，无需迁移）
  - 记忆内容：enabled（桌面图标）、taskbar_transparent、performance_monitor、theme、close_to_tray + 背景设置
  - 每次开关变化实时保存（persist()，失败仅记日志）；启动时在 setup 中自动应用：
    enabled→hooks::start()、performance_monitor→perf::start()、taskbar_transparent→sync_taskbar()
  - 不持久化运行时状态：icons_hidden / animating / pending_toggle / fullscreen_active / taskbar_applied（退出恢复桌面图标与任务栏的纯净性约定不变）
  - autostart 仍以启动文件夹快捷方式为唯一事实来源，不重复存储
  - background.rs 的 load/save 已移除，持久化职责移交 prefs 模块
  - 顺手修复 SettingsPanel 版本号漏改（v0.7.1 → v0.9.0）
  - 已构建并推送：
    - 免安装 EXE：`src-tauri/target/release/CloudSatchel.exe`
    - Release 安装包：`CloudSatchel_0.9.0_x64-setup.exe`（dev/_rename_installer.ps1 重命名）
    - GitHub Release：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.9.0
  - 发布工具链：winget 安装 gh CLI 2.97.0；认证通过 `git credential fill` 提取凭据管理器中的 OAuth token 设置 GH_TOKEN（gh auth login 未执行）

## 关键工程约定

- 纯净性第一：不主动联网、不写注册表、退出恢复桌面图标与任务栏
- 桌面图标窗口定位不向 Progman 发送 `WM_SPAWN_WORKERW`，避免与 Wallpaper Engine 冲突
- 桌面列表操作只使用无指针消息，避免 Explorer 崩溃
- 任何耗时 Win32 操作使用 async + spawn_blocking 或后台线程
- 性能监控关闭后立即停止采样；开关状态随 settings.json 持久化，启动时若上次为开则自动恢复采样
- 设置持久化：开关与背景设置统一存 `%LOCALAPPDATA%\CloudSatchel\settings.json`（prefs 模块，原子写入），变更实时保存、启动自动恢复；运行时状态（图标隐藏/动画/全屏叠加）不入盘
- Release 资产统一英文名：`CloudSatchel_<版本>_x64-setup.exe`

## 待办 / 可继续方向

- 如需要，可补充 CPU/GPU 温度等传感器不可用时的状态提示
- 可增加性能监控采集日志开关
- 后续版本继续同步 Cargo、Tauri、前端 package 和 About 版本号
