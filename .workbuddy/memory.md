# CloudSatchel / 云笈 - 项目记忆

## 当前状态

- 产品：云笈 / Cloud Satchel，纯净本地 Windows 桌面工具集
- 技术栈：React 19 + TypeScript + Vite；Tauri 2 + Rust；WebView2
- 当前版本：v0.17.0
- 目标平台：Windows 10 / Windows 11
- 代码位置：`desktop-tools/`
- 需求文档：`CloudSatchel需求文档.md`（工作区根目录，与记忆同步维护，当前 v1.13）

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
- **音频识别（v0.15.0）：右下角媒体面板，SMTC 控制 + WASAPI 波形；v0.17.0 起为主界面功能列表项（不再在设置面板）**

## 最近完成

- 2026-08-18 v0.17.0 移除虚拟桌宠 + 音频识别移入功能列表
  - **删除桌宠全部功能**（用户明确不需要）：pet-window 窗口、PetWindow.tsx、set_pet_enabled/set_pet_position、prefs pet_* 字段、poll_loop 隐私联动、privacy.rs collect_cb 桌宠跳过、CSS .pet-* 样式、FEATURES 桌宠项
  - **音频识别从设置面板移入主界面功能列表**（第 6 项卡片，handleToggle 走 set_audio_panel_enabled）：SettingsPanel 删除「音频识别」分组与 props
  - 需求文档 v1.13：删除 4.16 虚拟桌宠章节（4.17→4.16、4.18→4.17），SC 场景/持久化表/NFR-35/技术架构/工程约定/第 8 节同步清理
  - 实测：功能列表 = 双击隐藏图标 | 任务栏 | 性能监控 | 隐私 | AI 助手 | 音频识别；窗口数 3（pet-window 已消失）

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
- 2026-08-18 v0.16.1 音频面板修复（用户反馈）
  - 虚框：透明窗口上 backdrop-filter 产生矩形边缘伪影（模糊采样窗口外内容失败）→ 移除 blur，用高不透明度背景 + 边框高光模拟玻璃感；面板 inset 12px 留阴影空间（窗口 320×108→344×132）；AI 小窗同步（400×520→420×540，shell inset 10px）；pet-menu 去 blur
  - 应用名：SMTC 的 SourceAppUserModelId 是 AUMID（AppleInc.AppleMusicWin_xxx!App）→ windows::ApplicationModel::AppInfo::GetFromAppUserModelId 查询 DisplayName（"Apple Music"）；副标题改「歌手 · 专辑」优先（MediaState 新增 album 字段）
  - 实测：Apple Music 显示「刘惜君 & 薛之谦 — 尘」✓ 换歌实时更新 ✓
  - 已发布：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.16.1
- 2026-08-18 v0.16.2 彻底消除辅助窗口虚框（用户反馈 v0.16.1 仍虚框）
  - 终案：SetWindowRgn 圆角裁剪——audio-panel/ai-popup/pet-window 窗口区域直接切成圆角（CreateRoundRectRgn + SetWindowRgn，物理像素 r=逻辑12×scale，ai-popup resize 后经 on_window_event Resized 重设）；圆角外不参与绘制，系统阴影/合成边缘全部无从谈起（实测 GetWindowRgn=COMPLEXREGION）
  - shadow: false：三个辅助窗口关闭系统阴影（透明窗口 + DWM 阴影 = 矩形虚影框）
  - 面板精简：无 border、无外 box-shadow、inset 0 铺满，仅 inset 高光（inset 阴影窗口内渲染不裁剪）
  - 坑：tauri hwnd() 返回 windows crate HWND 元组结构体（解包 .0 为 *mut c_void 供 windows-sys）；PowerShell 批量改版本号时 Set-Content -Encoding UTF8 写 BOM 会破坏 JSON（用 [IO.File]::WriteAllText + UTF8Encoding($false)）
  - 已发布：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.16.2
- 2026-08-18 v0.16.3 音频面板重新打开不显示（用户反馈）
  - bug：set_audio_panel_enabled(true) 只恢复数据采集，窗口仍处于关闭时的 hide 状态 → 补 show + unminimize
  - 已发布：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.16.3
- 2026-08-18 v0.16.4 辅助窗口出现在 Alt+Tab（用户截图：与 Photoshop 并列的「音频面板」）
  - 根因：Tauri 2 skipTaskbar 在 Windows 未设置 WS_EX_TOOLWINDOW（实测 EX_APPWINDOW=True，Alt+Tab 不遵守 skipTaskbar）
  - make_tool_window：SetWindowLongPtr 加 WS_EX_TOOLWINDOW(0x80) 清 WS_EX_APPWINDOW(0x40000) + SWP_FRAMECHANGED
  - 坑：wry/Tauri 在窗口 show 时重置 exstyle（仅音频面板被 show 过所以它失败）→ poll_loop 每 25 tick（2 秒）兜底修复，开销可忽略
  - 实测：三窗口 TOOL=True APP=False，关闭/重开循环后保持
  - 已发布：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.16.4
- 2026-08-18 v0.16.5/0.16.6 失焦出现窗口框架（用户反馈）
  - v0.16.5：实测 style=0x14CB0000 含 WS_CAPTION|WS_SYSMENU|WS_BORDER（wry decorations:false 只隐藏渲染、保留样式位）→ 清样式位后按钮消失但标题框仍在
  - v0.16.6 终案：GWL_STYLE 置 WS_POPUP + 清全部框架位 + DwmSetWindowAttribute(DWMWA_NCRENDERING_POLICY=DWMNCRP_DISABLED)（windows-sys 需加 Win32_Graphics_Dwm feature）→ DWM 聚焦/失焦均不绘制任何非客户区；面板移除 inset 1px 内框线
  - 实测：style=0x94000000 popup=True caption/sysmenu/border=False；ex tool=True app=False
  - 已发布：https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.16.6
- 2026-08-18 v0.16.7 关闭 Win11 窗口边框（DWMWA_BORDER_COLOR=DWMWA_COLOR_NONE）——未解决（用户测试后仍有标题框）
- 2026-08-18 v0.16.8 面板背景 100% 不透明（消除透出）——用户反馈「液态玻璃质感消失」
- 2026-08-18 v0.16.9 Acrylic 毛玻璃（DWMWA_SYSTEMBACKDROP_TYPE=DWMSBT_ACRYLIC + DWMWA_COLOR）——用户反馈「无玻璃质感、直角、仍有标题框」；实测 DWM backdrop=3 生效但**acrylic 与 WebView2 透明背景不兼容**（内容整体渲染成灰色）
- 2026-08-18 v0.16.10 移除 SetWindowRgn（acrylic 背景按窗口矩形渲染 region 裁剪不到=直角）→ DWM 系统圆角（DWMWA_WINDOW_CORNER_PREFERENCE=ROUND）；acrylic alpha 80%→60%——用户反馈「全灰、不是透明」
- 2026-08-18 v0.16.11 放弃 acrylic（WebView2 不兼容=全灰根源）→ **CSS 玻璃拟态**（半透明渐变 82%/86% + 噪声纹理 + inset 高光描边，复刻主界面侧边栏质感）；新增 **WM_NCCALCSIZE 全客户区子类化**（install_nccalc_fix，老式 GWLP_WNDPROC 替换+转发）——毛玻璃质感回归 ✓，但「点击面板标题框仍弹出」
- 2026-08-18 v0.16.12 排查日志：aux_wnd_proc 记录焦点/非客户区/命中测试消息+样式——**发现点击面板激活时样式曾被重置为 0x14CB0000**（含 WS_CAPTION）
- 2026-08-18 v0.16.13 防御修复：aux_wnd_proc 每条消息处理后立即检查样式，被重置则毫秒级修复（节流 500ms）——本地模拟验证 FIXED 生效，**但用户测试后标题框依旧**
- 2026-08-18 v0.16.14 **日志铁证**：用户真实点击面板时样式全程正常（popup 0x94000000 保持、无 WS_CAPTION、无 WM_NCPAINT、无 FIXED）——**标题框与窗口样式/系统非客户区完全无关**；新增激活瞬间自动截屏（save_window_shot 保存 BMP 到 %LOCALAPPDATA%\CloudSatchel\shot-activate-<tick>.bmp，激活后 300ms 抓 DWM 合成画面）
  - 尚未解决：标题框之谜（见「待解决」）；本地无法复现（SetForegroundWindow 编程激活无标题框，仅用户真实鼠标点击出现）
- 2026-08-18 v0.16.15 **音频面板标题框终案**（用户截图实锤：顶部灰带+「音频面板」文字，文字发虚=在内容**后面**透出）
  - 根因：标题框 = 窗口**激活**时 DWM 合成到窗面非客户区的 caption；音频面板半透明背景（82%）让它透出。样式/NC 消息层面全程干净（v0.16.14 铁证），无法从样式层阻止 DWM 合成
  - 修复：① tauri.conf.json 给 audio-panel / pet-window 加 `focusable: false`（tao 原生加 WS_EX_NOACTIVATE，点击永不激活 → 激活态 caption 永不合成；媒体面板/桌宠本就不需要键盘焦点，点击不抢焦点反而是更好的 UX）；② aux_wnd_proc 吞 WM_NCACTIVATE（返回 TRUE）/ WM_NCPAINT（返回 0）兜底（保护需要键盘的 ai-popup）；③ 移除激活截屏调试代码（save_window_shot 及 shot 线程）
  - ai-popup 保留 focusable（输入框需要键盘），其内容背景不透明（.ai-panel background 实色）天然免疫透出
  - 经验：**不需要键盘输入的透明辅助窗口一律 focusable:false**，从根本上避免激活 caption；这是 tao 自己的 flag 配方，不会被 wry show 重置（区别于手动 SetWindowLongPtr）
  - 待用户实测确认（点击面板不再出现标题框；拖拽/按钮/双击菜单仍正常）

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
- 辅助窗口（ai-popup / audio-panel）：关闭=隐藏不销毁；前端 window 操作需在 capabilities 配置权限；on_window_event 按 label 分支
- 多窗口路由：main.tsx 按 getCurrentWindow().label 渲染 App / AiPopup / AudioPanel
- Release 资产统一英文名：`CloudSatchel_<版本>_x64-setup.exe`

## 待办 / 可继续方向

- 如需要，可补充 CPU/GPU 温度等传感器不可用时的状态提示
- 可增加性能监控采集日志开关
- dlog.rs 临时调试日志移除/加开关（hooks-debug.log 持续增长）
- 需求文档第 10 节迭代建议剩余项：设置引导 / 日志开关 / 背景图缓存 / Win 版本能力说明 / 冒烟强化 / 隐私恢复托盘气泡 / AI 对话持久化与导出 / 音频面板音量歌词
- 后续版本继续同步 Cargo、Tauri、前端 package 和 About 版本号

## 会话交接状态（2026-08-18 更新，供新会话"读取记忆"恢复上下文）

**当前版本与发布**
- 最新已发布：v0.16.15（标题框终案，https://github.com/LiuBe-github/CloudSatchel/releases/tag/v0.16.15）
- **本地待推送**：v0.17.0（移除桌宠 + 音频识别移入功能列表，已构建待用户确认后推送）
- git 状态：main 与远端同步至 `9a1709e`（v0.16.15）；v0.17.0 改动未提交
- 版本线：v0.9.0 开关记忆 → v0.10.x 隐私/自动隐藏/动画 → v0.11.x AI 助手+BaseURL/主题/性能 → v0.12.x 托盘快捷开关/TranslucentTB 修复 → v0.13.0 老板键 → v0.14.0 AI 小窗 → v0.15.0 音频识别 → v0.16.x 面板修复/标题框终案 → v0.17.0 移除桌宠/音频识别入功能列表

**需求文档当前状态**
- `CloudSatchel需求文档.md`（根目录，v1.13）：FR-01~FR-13、FR-15、FR-17、FR-18 全部已实现；FR-16 虚拟桌宠已移除（v0.17.0）；音频识别为主界面功能列表项
- 文档第 10 节迭代建议中**未实现**：设置引导 / 日志开关 / 背景图缓存 / Win 版本能力说明 / 冒烟强化 / 隐私恢复托盘气泡 / AI 对话持久化导出 / 音频面板扩展

**待用户验证事项**
- v0.12.1 TranslucentTB 弹窗修复：需在出问题的那台机器上验证（反复开关透明任务栏不再弹"请重启 Windows"）
- v0.13.0+ 老板键 / AI 小窗 / 音频识别：自动化已覆盖核心链路，真实桌面体验待用户确认
  - AI 小窗对话（需配置 Key）与音频面板控制（真实播放器 SMTC 控制按钮）
- AI 助手对话（v0.11.x）：用户配置 DeepSeek/OpenAI 实测

**遗留小问题**
- dlog.rs 是临时调试日志（hooks 每点击写 FIRST/DBLCLK 多行 + aux 排查日志），产品发布前应移除或加开关（记忆原话"确认修复后应移除"）
- hooks-debug.log 会持续增长；激活截屏调试已于 v0.16.15 移除，磁盘上旧 `%LOCALAPPDATA%\CloudSatchel\shot-activate-*.bmp` 可手动删除
- dlog 坑：进程运行中若日志文件被外部删除重建，缓存文件句柄指向孤儿文件，可见文件恒 0 字节（v0.16.15 排查时遇到）；读日志前注意此假象，必要时重启应用重建句柄

**已知技术坑（详见各版本记录）**
- Win11 25H2：ABM_SETSTATE 不隐藏任务栏（用 ShowWindow+轮询）；SetWindowPos 移任务栏被系统拉回（动画用 alpha 渐变）
- TranslucentTB 弹窗 = 文件时间戳比 temp 副本新（引擎文件固定 2000-01-01 时间戳解决）
- tauri::command 宏对 pub fn 报 E0255（command 保持普通 fn，子模块可直接调用）
- CSS 必须用 :root 已定义主题变量（--color-paper/ink/bamboo 系列）
- settings.json 读取代容忍 UTF-8 BOM；前端 build 在沙箱需 danger-full-access（Node 子进程 EPERM）
- Tauri 2 ACL：前端 window 操作（show/hide/setPosition）需 app.security.capabilities 配置 core:window:allow-*；core:window:default 只含只读查询；每个辅助窗口都要列入 windows 列表
- WASAPI loopback：GetMixFormat 的 WAVEFORMATEXTENSIBLE 必须完整复制（头部+cbSize）再 Initialize，否则 E_INVALIDARG；波形采集以能量驱动（不依赖 SMTC playing，SoundPlayer 等不注册 SMTC 也能出波形）
- SMTC：PlaybackStatus 在 PlaybackInfo 上；IsNextEnabled/IsPlayEnabled 等在 playback.Controls() 上；GetCurrentSession 无会话返回 Err；需要 windows crate features Media_Control/Media_MediaProperties/Foundation；SourceAppUserModelId 是 AUMID（用 AppInfo::GetFromAppUserModelId 查显示名，feature ApplicationModel）
- 透明窗口 + backdrop-filter：WebView2 模糊不了窗口外内容，会在面板矩形边缘产生「虚框」伪影 → 辅助窗口一律不用 backdrop-filter，用高不透明度背景 + 阴影（阴影需窗口比面板大，面板留边距）
- 透明辅助窗口「虚框」终案：SetWindowRgn 圆角裁剪窗口区域 + shadow:false + 无 border/外阴影（见 v0.16.2）
- Tauri 2 skipTaskbar 在 Windows 不设 WS_EX_TOOLWINDOW（Alt+Tab 仍显示）→ Rust 侧强制 TOOLWINDOW + 轮询兜底（wry show 会重置 exstyle，见 v0.16.4）
- 无边框辅助窗口完整配方（v0.16.15 终案）：WS_POPUP + 清 WS_CAPTION/SYSMENU/BORDER/MIN/MAX + WS_EX_TOOLWINDOW - APPWINDOW + DWMWA_NCRENDERING_POLICY=DISABLED + DWMWA_BORDER_COLOR=NONE + DWMWA_WINDOW_CORNER_PREFERENCE=ROUND + shadow:false + WM_NCCALCSIZE 返回 0 + aux_wnd_proc 吞 WM_NCACTIVATE/WM_NCPAINT + **不需要键盘的窗口 focusable:false（WS_EX_NOACTIVATE，杜绝激活态 caption 合成；v0.16.15 标题框终案根因）**
- **acrylic（DWMSBT_ACRYLIC）与 WebView2 透明背景不兼容**：WebView2 内容在 acrylic 窗口上整体渲染成灰色（v0.16.9/0.16.10 实测）→ 辅助窗口玻璃质感一律用 CSS 拟态（半透明渐变 + 噪声纹理 + inset 高光），不用系统 acrylic
- CDP 远程调试（WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port）可验证多窗口 DOM 状态；PowerShell Add-Type P/Invoke 可探测热键占用（err=1409）、窗口状态与 GetWindowRgn；PowerShell 写 JSON 用 [IO.File]::WriteAllText + UTF8Encoding($false) 防 BOM
- 窗口排查套路：窗口子类化（GWLP_WNDPROC 替换+转发）记录 WM_ACTIVATE/NCPAINT/NCCALCSIZE/NCHITTEST + 样式快照；激活自动截屏（BitBlt 屏幕 DC→BMP）抓 DWM 合成画面（CopyFromScreen/PrintWindow 对透明合成窗口均不可靠，程序内截屏才有效）

**新会话快速恢复**
1. 第一句：「读取记忆」（读 `.workbuddy/memory.md`）+ 读取 `CloudSatchel需求文档.md`
2. 即可继续开发/修 bug；发布流程：`npm run release`（一键构建+重命名安装包）→ git tag/push → gh release create（GH_TOKEN 从 `git credential fill` 提取）
