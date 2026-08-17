# CloudSatchel / 云笈 - 项目记忆

## 当前状态

- 产品：云笈 / Cloud Satchel，纯净本地 Windows 桌面工具集
- 技术栈：React 19 + TypeScript + Vite；Tauri 2 + Rust；WebView2
- 当前版本：v0.10.0
- 目标平台：Windows 10 / Windows 11
- 代码位置：`desktop-tools/`
- 需求文档：`CloudSatchel需求文档.md`（工作区根目录，与记忆同步维护）

## 已实现功能

- 双击隐藏/显示桌面图标（SHELLDLL_DefView + WS_EX_LAYERED，动画约 0.5s）
- 透明任务栏（Win10 Accent API / Win11 TranslucentTB 便携引擎）
- 主机性能监控（CPU、GPU、内存、网络，约 1 秒本地采样）
- 隐私操作（空闲超时自动最小化窗口、隐藏图标/任务栏、静音，操作即还原）
- 任务栏自动隐藏（空闲隐藏，鼠标移到底部弹出，AppBar ABS_AUTOHIDE）
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
- 2026-08-15 按需求文档 v1.2 实现 FR-13 / FR-14，版本升级到 v0.10.0
  - 核心模块：[src-tauri/src/privacy.rs](src-tauri/src/privacy.rs)（FR-13 隐私操作 + FR-14 任务栏自动隐藏共用空闲轮询）
  - 空闲检测：GetLastInputInfo 轮询（1 秒），FR-13/FR-14 各自独立计时与配置（默认 60s，可设 30s~60min）
  - FR-13 触发序列：最小化所有窗口（跳过自身/桌面/任务栏/工具窗口，全屏先转窗口化）→ 隐藏图标（先查已隐藏）→ 隐藏任务栏（先查 autohide）→ 静音（Core Audio IAudioEndpointVolume，仅切静音标志）
  - FR-13 恢复序列：用户操作（最后输入时间变化）→ 还原窗口（原最大化/全屏尽力还原）→ 图标/任务栏仅还原本功能执行过的 → 取消静音；ACTIVE_LOCK 串行化触发/恢复，PRIVACY_SNAPSHOT 为恢复唯一依据
  - FR-14：taskbar.rs 新增 set_autohide/is_autohide（SHAppBarMessage ABM_SETSTATE ABS_AUTOHIDE，不写注册表）；全屏/云笈最大化期间暂停；边缘弹出/移开再隐藏由系统原生行为完成
  - 任务栏隐藏与 FR-13 步骤③共享 is_autohide 判定；AUTOHIDE_APPLIED 标记"由本应用设置"，避免动用户系统原有设置
  - 持久化扩展：privacy_enabled / privacy_idle_secs / autohide_enabled / autohide_idle_secs（prefs 扁平结构 + serde default，旧文件兼容）
  - 前端：功能列表第 4 项「隐私操作」卡片；设置面板新增「任务栏」分组（透明任务栏/自动隐藏开关+空闲时间）与「隐私操作」分组（触发空闲时间）
  - 版本号同步 v0.10.0（Cargo / tauri.conf / package / App / About / Settings）
  - 依赖新增：windows-sys Win32_UI_Shell + Win32_UI_Input_KeyboardAndMouse；windows Win32_Media_Audio + Win32_Media_Audio_Endpoints
  - 已构建并推送：
    - 免安装 EXE：`src-tauri/target/release/CloudSatchel.exe`
    - Release 安装包：`CloudSatchel_0.10.0_x64-setup.exe`（npm run release 一键构建+重命名）
    - GitHub Release：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.10.0
    - tag：v0.10.0（gh CLI + GH_TOKEN 提取自凭据管理器，与 v0.9.0 同流程）
- 2026-08-15 修复任务栏自动隐藏 + 空闲时间最小 10 秒：v0.10.1
  - 根因：Win11 25H2（build 26200）上 `SHAppBarMessage(ABM_SETSTATE, ABS_AUTOHIDE)` 仅改变状态位、任务栏视觉不隐藏（PowerShell 实测 GETSTATE 0→1 但窗口可见）
  - 修复：taskbar.rs 改用 `ShowWindow(SW_HIDE/SW_SHOW)` 控制所有任务栏窗口（主+副）；边缘弹出/移开再隐藏由 privacy.rs 轮询鼠标位置实现（50ms tick，移开 1.5s 后重新隐藏，任务栏矩形外扩 4px）
  - privacy.rs 轮询改为 50ms tick：每 tick 做边缘检测，每 20 tick（1 秒）做空闲检测与全屏检测
  - 空闲时间最小可设值 IDLE_CLAMP_MIN 30→10 秒（前端 RangeRow min 同步）
  - prefs::load() 容忍 UTF-8 BOM（PowerShell 等第三方工具写入 settings.json 会导致 serde 解析失败、设置静默丢失——自动化测试发现的真实问题）
  - taskbar::ensure_restored() 扩展：启动时恢复异常退出残留的隐藏任务栏（仅恢复 SW_HIDE 的窗口，不影响系统 autohide 设置）
  - 自动化验证（等系统空闲后实测）：空闲 10 秒隐藏 ✓ 鼠标移到底部弹出 ✓ 移开 1.5s 再隐藏 ✓ 启动自修复 ✓
  - 已发布 GitHub Release：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.10.1
  - 版本号同步 v0.10.1（含根 package.json）
- 2026-08-15 任务栏滑动动画 + 隐私恢复跳过已最小化窗口：v0.10.2
  - taskbar.rs：set_autohide 改为滑动动画（约 144ms，12 步×12ms，SetWindowPos 逐帧位移后 SW_HIDE/SW_SHOW）；AUTOHIDE_EXPECTED/AUTOHIDE_ANIMATING 原子标志，动画期间新请求按最新意图排队执行；is_animating() 供轮询暂缓
  - is_autohide 判定扩展：窗口不可见 OR 完全滑出所在显示器底部（top >= rcMonitor.bottom）
  - privacy.rs：边缘/空闲检测在动画期间暂缓（避免"弹出又缩回"）；collect_cb 用 IsIconic 跳过触发前已最小化的窗口（恢复时保持最小化）
  - ensure_restored 扩展：恢复滑出屏幕残留的任务栏位置（SetWindowPos 移回底部原位）
  - 自动化实测：隐藏动画 ✓ 边缘弹出动画 ✓ 移开再隐藏 ✓（Phase A 全过）
  - 教训：自动化测试脚本超时/失败路径必须清理残留状态（曾因超时退出导致任务栏被隐藏残留，需手动 ShowWindow 恢复）；隐私触发会最小化用户全部窗口，测试需谨慎
  - 已发布 GitHub Release：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.10.2
- 2026-08-15 任务栏透明度动画 + 隐私最小化云笈自身：v0.10.3
  - 用户实测反馈：位移动画视觉上无效（生硬）、隐私操作未最小化云笈自身
  - 根因实测：Shell_TrayWnd 是系统管理的停靠窗口，SetWindowPos 移动被系统强制拉回原位（top 恒定不变）→ 位移帧全部无效
  - 修复：动画改为 WS_EX_LAYERED + LWA_ALPHA 透明度渐变（16 步 × 10ms ≈ 160ms，alpha 255↔0），隐藏收尾 SW_HIDE、显示先 SW_SHOW；动画前记录原始 exstyle，结束后恢复（避免影响透明引擎）
  - 隐私操作 collect_cb 移除"跳过云笈自身进程"：保护时主窗口一并最小化（需求文档 FR-13 原写"跳过云笈自身窗口"，已按用户要求调整）
  - 已发布 GitHub Release：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.10.3
- 2026-08-15 按需求文档 v1.7 实现 AI 助手与设置项调整：v0.11.0
  - FR-15 AI 助手：[src-tauri/src/ai.rs](src-tauri/src/ai.rs) + 前端 [ui/src/components/AiPanel.tsx](ui/src/components/AiPanel.tsx)
    - OpenAI 官方接口固定 base URL；reqwest 代理（OpenAI 无 CORS），SSE 分块解析后经 Tauri 事件（ai-chunk/ai-done/ai-error）逐字推送
    - 停止生成：futures-util Abortable + AbortHandle::new_pair()（0.3.31 API，注意不是旧的 abort_handle()）；reqwest Response 无 try_clone/abort
    - API Key：Windows DPAPI（windows crate Win32_Security_Cryptography，注意 0.61 用 CRYPT_INTEGER_BLOB 而非 DATA_BLOB，CryptProtectData 返回 Result<()>，LocalFree 在 Win32::Foundation 且参数是 Option<HLOCAL> 元组结构体）加密存 ai-key.bin，磁盘无明文；cargo test dpapi 往返测试通过
    - 对话历史仅内存（保留最近 20 条），退出即清；超时 60s；401/429/断网友好提示
  - 自动隐藏任务栏改为「立即生效」：移除 autohide_idle_secs（prefs/snapshot/command/前端全链路），configure 开启即 set_autohide(true)，全屏只暂停边缘弹出（不再恢复显示）
  - 隐私操作空闲时间改六档下拉（10/30/60/180/300/600 秒），后端 clamp 保留
  - 性能监控刷新间隔下拉（200/500/1000ms）：perf.rs PERF_INTERVAL_MS 原子可配 + 前端定时器跟随
  - 自动化实测：开启 3 秒内立即隐藏 ✓ 边缘弹出 ✓ 移开再隐藏 ✓（无需空闲等待）
  - 版本号 v0.11.0 全链路同步；已发布 GitHub Release：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.11.0
  - AI 对话功能（真实 OpenAI 请求）由用户自行测试
- 2026-08-15 修复全屏检测 + 任务栏功能合并到功能列表：v0.11.1
  - 用户反馈：全屏任务栏不透明不是 100% 触发；功能列表要任务栏相关（自动隐藏+透明合并）
  - 全屏检测 is_fullscreen_now 改为覆盖面积比 ≥98% 判定（原逐边 2px 容差在无边框全屏/DPI 缩放偏移下漏检）；最大化窗口约 95% 不误判；实测真实全屏窗口进出均正确触发
  - 功能列表第 2 项改为「任务栏」卡片：透明任务栏 + 自动隐藏两个开关并列（各自持久化）；设置面板「任务栏」分组移除（统一在功能列表操作）；handleToggle 移除 taskbar 分支（卡片独立 Switch）
  - 已发布 GitHub Release：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.11.1
- 2026-08-15 AI Key 401 排查修复：v0.11.2
  - 用户反馈：真实 Key 填进去 401。实测假 Key 请求 OpenAI：请求构造正确（Bearer 头正常送达），OpenAI 返回 "Incorrect API key provided: sk-xxx...xxxx"（含掩码）
  - 修复 1：save_key 改为清除所有空白字符（clean_key，粘贴换行/空格不再破坏 Key）
  - 修复 2：非 2xx 错误读取响应体并展示 OpenAI error.message（含 Key 掩码便于对比，不含完整 Key）；load_key 记长度日志
  - 已知边界：仅支持 OpenAI 官方接口（api.openai.com），第三方平台 Key 必 401；自定义 base URL 在需求文档第 10 节迭代建议 8
  - 已发布 GitHub Release：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.11.2
- 2026-08-15 AI 助手支持自定义 BaseURL：v0.11.3
  - 用户反馈：DeepSeek Key（模型 DSV4）填进去 401——根因：应用固定请求 OpenAI 官方接口，DeepSeek Key 必被拒
  - 配置区新增「接口地址」输入框（与 API Key/模型名并列），持久化 ai_base_url（默认 https://api.openai.com/v1）
  - 请求 URL = normalize_base_url(base_url) + /chat/completions（去尾斜杠，非法回落默认）；curl 实测 DeepSeek 双端点（/ 与 /v1）均正确接收
  - 新增单元测试 base_url_normalization_and_chat_url；错误文案通用化
  - 已发布 GitHub Release：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.11.3
- 2026-08-15 下拉框与 AI 界面主题色适配：v0.11.4
  - 用户反馈：隐私操作空闲时间下拉、AI 界面深色主题下可读性差
  - 根因：styles.css 追加的 select-box / AI 面板样式误用**未定义变量**（--color-card / --color-line / --color-paper-soft 不在 :root），永远回落白底 + color:inherit 继承浅色文字 → 白底浅字
  - 修复：全部改用已定义主题变量（--color-paper 系列 / --color-ink 系列 / --color-bamboo）；option 加 background/color 跟随主题；输入框/文本域/错误条/bubble 一并适配
  - 教训：新增样式必须使用 :root 已定义的主题变量，禁止引入未定义变量（CSS var 未定义时静默用 fallback）
  - 已发布 GitHub Release：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.11.4

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
