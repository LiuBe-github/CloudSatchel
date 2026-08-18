# CloudSatchel / 云笈 - 项目记忆

## 当前状态

- 产品：云笈 / Cloud Satchel，纯净本地 Windows 桌面工具集
- 技术栈：React 19 + TypeScript + Vite；Tauri 2 + Rust；WebView2
- 当前版本：v0.16.0
- 目标平台：Windows 10 / Windows 11
- 代码位置：`desktop-tools/`
- 需求文档：`CloudSatchel需求文档.md`（工作区根目录，与记忆同步维护，当前 v1.12）

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
- **隐私老板键（v0.13.0）：全局热键立即触发/恢复隐私序列**
- **AI 小窗（v0.14.0）：全局快捷键呼出小型 AI 问答窗**
- **音频识别（v0.15.0）：右下角媒体面板，SMTC 控制 + WASAPI 波形**
- **虚拟桌宠（v0.16.0）：CSS 自绘桌面精灵，拖拽与菜单**

## 最近完成

- 2026-08-18 v0.13.0 隐私老板键（FR-13 扩展）
  - 新模块 [src-tauri/src/hotkey.rs](src-tauri/src/hotkey.rs)：通用全局热键（RegisterHotKey + 独立消息循环线程 + MOD_NOREPEAT 防连发 + 注册结果回执），老板键与 AI 小窗共用
  - privacy.rs：BOSS_TRIGGERED 状态——老板键触发后鼠标/键盘不恢复（仅老板键/关开关/退出可恢复）；触发/恢复复用现有序列
  - 默认 Ctrl+`，设置面板可自定义并持久化（prefs.privacy_boss_key）；注册失败提示并降级仅空闲触发
  - 自动化实测：热键占用探测 err=1409 ✓ 老板键切换 ✓ 老板键模式鼠标不恢复 ✓ 空闲触发后真实输入恢复 ✓
  - 已发布：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.13.0
- 2026-08-18 v0.14.0 AI 小窗（FR-17）
  - tauri.conf 新增 ai-popup 窗口（400×520 无边框透明置顶）；前端按窗口 label 路由（main.tsx）
  - 默认 Ctrl+Shift+Space 切换呼出/隐藏；复用 FR-15 Key/模型/BaseURL；关闭（隐藏）即清空对话不落盘（后端 hide 时 emit ai-popup-cleared）
  - 设置面板「AI 小窗」分组：开关 + 快捷键自定义（持久化）
  - 关键：on_window_event 区分窗口 label——ai-popup 关闭 = 隐藏（不触发退出清理）；Destroyed 清理仅 main
  - 自动化实测：热键占用 ✓ 呼出 ✓ 再按隐藏 ✓
  - 已发布：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.14.0
- 2026-08-18 v0.15.0 音频识别（FR-18）
  - 新模块 [src-tauri/src/audio.rs](src-tauri/src/audio.rs)：SMTC（GlobalSystemMediaTransportControlsSessionManager）读会话 + 控制；WASAPI loopback + 512 点 FFT → 16 档对数频段能量
  - 波形能量驱动：有声音即采集，静音约 2 秒释放采集客户端（低开销）；不依赖 SMTC playing 状态（SoundPlayer 等不注册 SMTC 的播放器也能出波形）
  - 面板窗口 audio-panel（320×108 液态玻璃），默认右下角（可拖拽持久化），无播放淡出、全屏隐藏（复用 snapshot.fullscreen_active——本轮把 fullscreen_active 加入 Snapshot）
  - **Tauri 2 ACL 坑**：辅助窗口的 show/hide/setPosition 等前端 window 操作被 capabilities 拒绝（"not allowed by ACL"）；需在 app.security.capabilities 配置 core:window:allow-show/hide/set-position/set-focus/set-size（core:window:default 只含只读查询）；**capabilities 位置在 app.security 下**（不在顶层/app 直接子属性）
  - **WASAPI 坑**：GetMixFormat 通常返回 WAVEFORMATEXTENSIBLE，只复制 WAVEFORMATEX 头部（18 字节）会导致 Initialize E_INVALIDARG；必须完整复制头部+cbSize 字节再传指针
  - windows crate 0.61 需加 features：Media_Control、Media_MediaProperties、Foundation（SMTC）；PlaybackStatus 在 PlaybackInfo 上，IsNextEnabled 等在 playback.Controls() 上
  - 自动化实测：SMTC 真实会话（Apple Music 标题/艺术家/进度）✓ 播放波形跳动 ✓ 窗口右下角定位 ✓
  - 已发布：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.15.0
- 2026-08-18 v0.16.0 虚拟桌宠（FR-16）
  - pet-window 透明置顶小窗（140×160）；精灵为 CSS 自绘「竹灵小猫」（呼吸/眨眼/尾巴/阴影动画），无第三方素材无版权风险
  - 左键拖拽（data-tauri-drag-region）位置持久化；双击/右键弹出小菜单（隐藏桌宠/退出桌宠）
  - 隐私联动：poll_loop 检测 privacy_active 变化 → hide/show pet 窗口；privacy.rs collect_cb 按标题「云笈桌宠」跳过（避免被最小化与 hide 冲突）
  - 功能列表第 6 项「虚拟桌宠」卡片（默认关闭）；capabilities windows 列表需含 pet-window
  - 自动化实测：显示定位 ✓ 精灵渲染 ✓ 拖拽持久化（settings.json petX/petY）✓ 双击菜单 ✓ 隐私触发隐藏/恢复还原 ✓
  - 已发布：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.16.0

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
- 2026-08-15 性能优化：v0.11.5
  - 任务栏窗口句柄缓存：privacy 边缘检测每 50ms 调 taskbar_windows() → EnumWindows 全窗口枚举（每秒 20 次）；改为 TASKBAR_CACHE 5 秒 TTL 缓存（句柄存 isize 保 Send），覆盖 Explorer 重启场景
  - dlog 文件句柄缓存：每次写日志 open/close 文件 → Mutex<Option<File>> 复用
  - perf 静态明细降频：200ms 档下 process_thread_counts（Toolhelp 快照）/cpu_temperature（sysinfo 全量）/cpu_frequencies 每轮重算 → maybe_refresh_details 按 1 秒刷新缓存（every = 1000/interval）；动态指标（usage/内存/网络速率）每轮
  - 回归实测：自动隐藏立即生效/边缘弹出/移开再隐藏全过
  - 已发布 GitHub Release：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.11.5
- 2026-08-15 托盘右键菜单功能快捷开关：v0.12.0
  - FR-06 扩展：托盘菜单新增 4 个 CheckMenuItem（双击隐藏桌面图标/透明任务栏/自动隐藏任务栏/隐私操作），带勾选标记
  - 点击切换复用同一套 set_* command：command 函数保持**普通 fn**，tray.rs 作为 lib.rs 子模块可直接调用 crate::set_enabled（父模块私有项对子模块可见）
  - 经验：tauri::command 宏对 `pub fn` / `pub(crate) fn` 展开会报 __cmd__ 重复定义（E0255）——**command 必须保持普通 fn**；AppState 开关字段加 pub(crate)（实际子模块可见父私有，非必需但无害）
  - CheckMenuItem（勾选项）的 set_checked 在 CheckMenuItem 上（普通 MenuItem 没有）；with_id 第 5 参为初始 checked
  - 勾选双向同步：persist 统一调 tray::update_checks；setup 后按恢复状态初始化
  - 已发布 GitHub Release：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.12.0
- 2026-08-15 修复 TranslucentTB 误弹「已更新，请重启 Windows」：v0.12.1
  - 用户反馈：新机器开启透明任务栏后 TranslucentTB 一直弹窗要求更新重启
  - 根因：TranslucentTB 每次启动把引擎目录 DLL 复制到 %TEMP%\TranslucentTB（update_existing，仅当源比目标新）；目标被 explorer 锁定（异常退出残留）→ 复制失败 → 误判「已更新」弹窗
  - 原 align_dll_timestamps_with_temp 仅当 temp 副本存在且字节一致时压时间戳，覆盖不全
  - 修复：释放的引擎文件统一 stamp_fixed 为 2000-01-01 UTC（fs::File::set_modified）——源永远不新于任何 temp 副本，永不触发复制、永不弹窗；内容变化（字节不同）时保留新时间戳（真更新需重启）
  - 实测：开启透明后引擎 DLL/EXE 时间戳均为 2000/1/1 8:00（+08）
  - 已发布 GitHub Release：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.12.1（创建时遇 GitHub API 网络 EOF，重试成功）

## 关键工程约定

- 纯净性第一：不主动联网、不写注册表、退出恢复桌面图标与任务栏
- 桌面图标窗口定位不向 Progman 发送 `WM_SPAWN_WORKERW`，避免与 Wallpaper Engine 冲突
- 桌面列表操作只使用无指针消息，避免 Explorer 崩溃
- 任何耗时 Win32 操作使用 async + spawn_blocking 或后台线程
- 性能监控关闭后立即停止采样；开关状态随 settings.json 持久化，启动时若上次为开则自动恢复采样
- 设置持久化：开关与背景设置统一存 `%LOCALAPPDATA%\CloudSatchel\settings.json`（prefs 模块，原子写入），变更实时保存、启动自动恢复；运行时状态（图标隐藏/动画/全屏叠加）不入盘
- 全局热键统一走 hotkey.rs（RegisterHotKey + MOD_NOREPEAT），注册失败降级提示，不影响其他功能
- 辅助窗口（ai-popup / audio-panel / pet-window）：关闭=隐藏不销毁；前端 window 操作需在 capabilities 配置权限；on_window_event 按 label 分支
- 多窗口路由：main.tsx 按 getCurrentWindow().label 渲染 App / AiPopup / AudioPanel / PetWindow
- Release 资产统一英文名：`CloudSatchel_<版本>_x64-setup.exe`

## 待办 / 可继续方向

- 如需要，可补充 CPU/GPU 温度等传感器不可用时的状态提示
- 可增加性能监控采集日志开关
- dlog.rs 临时调试日志移除/加开关（hooks-debug.log 持续增长）
- 需求文档第 10 节迭代建议剩余项：设置引导 / 日志开关 / 背景图缓存 / Win 版本能力说明 / 冒烟强化 / 隐私恢复托盘气泡 / AI 对话持久化与导出 / 音频面板音量歌词 / 桌宠更多外观
- 后续版本继续同步 Cargo、Tauri、前端 package 和 About 版本号

## 会话交接状态（2026-08-18 更新，供新会话"读取记忆"恢复上下文）

**当前版本与发布**
- 最新版本：v0.16.0（最后发布 https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.16.0）
- 工作区 git 干净，main 与远端同步（最后 commit `3250a22`）
- 版本线：v0.9.0 开关记忆 → v0.10.x 隐私/自动隐藏/动画 → v0.11.x AI 助手+BaseURL/主题/性能 → v0.12.x 托盘快捷开关/TranslucentTB 修复 → v0.13.0 老板键 → v0.14.0 AI 小窗 → v0.15.0 音频识别 → v0.16.0 虚拟桌宠

**需求文档当前状态**
- `CloudSatchel需求文档.md`（根目录，v1.12）：FR-01~FR-13、FR-15~FR-18 全部已实现；v0.12.0 规划范围（托盘快捷开关 / 老板键 / AI 小窗 / 音频识别 / 桌宠）全部交付
- 文档第 10 节迭代建议中**未实现**：设置引导 / 日志开关 / 背景图缓存 / Win 版本能力说明 / 冒烟强化 / 隐私恢复托盘气泡 / AI 对话持久化导出 / 音频面板扩展 / 桌宠外观扩展

**待用户验证事项**
- v0.12.1 TranslucentTB 弹窗修复：需在出问题的那台机器上验证（反复开关透明任务栏不再弹"请重启 Windows"）
- v0.13.0+ 老板键 / AI 小窗 / 音频识别 / 桌宠：自动化已覆盖核心链路，真实桌面体验待用户确认
  - AI 小窗对话（需配置 Key）与音频面板控制（真实播放器 SMTC 控制按钮）
  - 桌宠拖拽手感与菜单交互
- AI 助手对话（v0.11.x）：用户配置 DeepSeek/OpenAI 实测

**遗留小问题**
- dlog.rs 是临时调试日志（hooks 每点击写 FIRST/DBLCLK 多行），产品发布前应移除或加开关（记忆原话"确认修复后应移除"）
- hooks-debug.log 会持续增长

**已知技术坑（详见各版本记录）**
- Win11 25H2：ABM_SETSTATE 不隐藏任务栏（用 ShowWindow+轮询）；SetWindowPos 移任务栏被系统拉回（动画用 alpha 渐变）
- TranslucentTB 弹窗 = 文件时间戳比 temp 副本新（引擎文件固定 2000-01-01 时间戳解决）
- tauri::command 宏对 pub fn 报 E0255（command 保持普通 fn，子模块可直接调用）
- CSS 必须用 :root 已定义主题变量（--color-paper/ink/bamboo 系列）
- settings.json 读取代容忍 UTF-8 BOM；前端 build 在沙箱需 danger-full-access（Node 子进程 EPERM）
- Tauri 2 ACL：前端 window 操作（show/hide/setPosition）需 app.security.capabilities 配置 core:window:allow-*；core:window:default 只含只读查询；每个辅助窗口都要列入 windows 列表
- WASAPI loopback：GetMixFormat 的 WAVEFORMATEXTENSIBLE 必须完整复制（头部+cbSize）再 Initialize，否则 E_INVALIDARG；波形采集以能量驱动（不依赖 SMTC playing，SoundPlayer 等不注册 SMTC 也能出波形）
- SMTC：PlaybackStatus 在 PlaybackInfo 上；IsNextEnabled/IsPlayEnabled 等在 playback.Controls() 上；GetCurrentSession 无会话返回 Err；需要 windows crate features Media_Control/Media_MediaProperties/Foundation
- CDP 远程调试（WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port）可验证多窗口 DOM 状态；PowerShell Add-Type P/Invoke 可探测热键占用（err=1409）与窗口状态

**新会话快速恢复**
1. 第一句：「读取记忆」（读 `.workbuddy/memory.md`）+ 读取 `CloudSatchel需求文档.md`
2. 即可继续开发/修 bug；发布流程：`npm run release`（一键构建+重命名安装包）→ git tag/push → gh release create（GH_TOKEN 从 `git credential fill` 提取）
