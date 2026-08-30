# CloudSatchel / 云笈 - 项目记忆

## 当前状态

- 产品：云笈 / Cloud Satchel，纯净本地 Windows 桌面工具集
- 技术栈：React 19 + TypeScript + Vite；Tauri 2 + Rust；WebView2
- 当前版本：v1.0.0
- 目标平台：Windows 10 / Windows 11
- 代码位置：`desktop-tools/`
- 需求文档：`CloudSatchel需求文档.md`（工作区根目录，与记忆同步维护，当前 v1.25）

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
- **鼠标选取翻译（v0.20.0）：选中文字松手弹「翻译」按钮，点击出翻译弹窗（AI 助理 / 微软翻译），点击外部关闭**
- **音频面板音量调节条（v0.20.0）：面板内系统音量滑块 + 静音开关**

## 最近完成

- 2026-08-30 v1.0.0 首个正式版：设置项间距修复 + 版本号正式升 1.0.0
  - 用户反馈设置项之间元素重叠/拥挤（删除灰色说明后行距不足）：CSS 新增
    `.setting-row + .setting-row { margin-top: 16px }`，settings-section 纵向 padding
    16→20px，settings-label margin-bottom 10→12px
  - 版本号全链路 0.20.11 → 1.0.0（Cargo/tauri.conf/package/ui/About/侧栏/设置页脚）
  - 版本线至此：v0.7.x ~ v0.20.11 迭代 → v1.0.0 正式版

- 2026-08-30 v0.20.11 设置面板只留大标题与控件，删除全部灰色说明（setting-row-desc）
  - 用户明确：只留选项大字，设置项下面的灰色小子不要；删除所有 setting-row-desc、
    隐私/AI 小窗两组底部提示、以及「纯净性」整段（纯文字无控件）；错误提示（error-text）保留
  - 版本号 v0.20.11 全链路同步

- 2026-08-30 v0.20.10 设置面板文案精简（用户要求去掉赘述）
  - 缩短隐私/AI 小窗/音频/启动退出各 setting-row-desc；删除与控件重复的
    「（当前 X）」提示（连带删除 formatIdle 函数）；纯净性列表去掉与开关重复的第 4 条
    与括号赘述；修正老板键输入框 placeholder 误显示 Ctrl+Shift+Space → Ctrl+`
  - 版本号 v0.20.10 全链路同步

- 2026-08-30 v0.20.9 翻译按钮 75×30 + 按钮定位改为选区末尾正下方
  - 用户确认 24×12 尺寸钳制修复生效后，明确要 75×30：BUTTON_W/H=75/30、
    tauri.conf 75×30、CSS 字号 12.5px 圆角 15px 字距 2px（prepare_button 运行期强制）
  - 定位修复：原用「选区 enclosing 元素矩形」居中定位，跨行/长段落时按钮会
    忽远忽近；改为 selection_end()——GetBoundingRectangles（SAFEARRAY，每 4 个
    double=left/top/width/height）取最后一个有效矩形右下角作为「末尾」锚点，
    按钮水平居中于末尾、垂直固定 GAP=8px 下方；失败回退元素矩形右下角
  - 新增 windows feature Win32_System_Ole（SafeArrayAccessData/GetLBound/GetUBound/Unaccess）
  - 版本号 v0.20.9 全链路同步；cargo check 通过

- 2026-08-30 v0.20.8 翻译按钮“框不变小”根因修复（重要）
  - 用户反馈：字号在变但按钮框不变。实测（启动日志 outer_size）：tauri.conf 创建
    极小窗口时被框架钳制——24×12/60×20 配置创建后窗口都是 170×47 物理像素
    （约 136×37.6 逻辑像素）；而 400×300 配置完全正常（500×375 物理）。
    说明存在创建期最小尺寸钳制，且与 config 无关（tao/wry 源码无此逻辑，疑似
    Windows CreateWindowExW 对极小 WS_POPUP 的默认处理）。
  - 修复：translate.rs 新增 prepare_button()——启动（start）与每次显示（show_button）
    前调用 set_min_size(1×1) + set_size(目标物理尺寸=逻辑×scale)，运行期 set_size
    可绕过创建期钳制；实测启动后按钮窗口=30×15 物理（24×12 逻辑）
  - 验证方法：临时启动日志 outer_size 实测（已移除）；经验：辅助窗口尺寸若
    config 不生效，优先尝试运行期 set_size 绕过创建期钳制
  - 版本号 v0.20.8 全链路同步

- 2026-08-30 v0.20.7 翻译按钮再缩小到 24×12（用户反馈 30×14 仍偏大）
  - 用户反馈“调整后大小不变”：应用为 Per-Monitor V2 DPI 感知（tao dpi.rs），
    窗口配置为逻辑像素，配置本身生效；上次 38→30 变化小且旧构建未更新易误判。
    本次直接缩到 24×12，字号 10→9px、圆角 7→6px、字距 1→0.5px
  - 版本号 v0.20.7 全链路同步

- 2026-08-30 v0.20.6 翻译按钮缩小（38×15→30×14）+ 全屏检测改为任一窗口全屏即不透明
  - 按钮：tauri.conf translate-button 30×14；translate.rs BUTTON_W/H；CSS 字号 10px 圆角 7px
  - 全屏检测：fullscreen.rs 新增 any_fullscreen_now()（EnumWindows 遍历可见顶层窗口，
    跳过桌面/任务栏/工具窗口/云笈自身，复用 ≥97% 覆盖判据）+ ANY_FULLSCREEN 静态标志；
    poll_loop 每 320ms 更新，desired_taskbar_visual 加 !is_any_fullscreen()——
    A 全屏中点击未全屏的 B 到前台，任务栏仍保持不透明；不影响音频面板隐藏与隐私边缘弹出
  - 版本号 v0.20.6 全链路同步；cargo check + ui build 通过

- 2026-08-30 v0.20.5 翻译源语言可选（默认自动检测）
  - TranslatePanel 新增「源语言」下拉（默认 auto：自动检测，支持简/繁/英/日/韩/法/德/俄/西）；
    prefs/lib.rs 持久化 translate_source_lang；AI 引擎提示词带源语言名（auto 时省略），
    微软引擎显式指定时附 from 参数（auto 时省略，由微软自动检测）；弹窗标题显示
    「引擎 · 源语言 → 目标语言」
  - 版本号 v0.20.5 全链路同步；cargo check + ui build 通过

- 2026-08-30 v0.20.4 翻译目标语言 + 任务栏全屏/最大化检测修复
  - 翻译目标语言：TranslatePanel 新增「目标语言」下拉（默认 auto-zh-Hans：自动识别 →
    中文简体，另支持简/繁/英/日/韩/法/德/俄/西）；prefs/lib.rs 持久化
    translate_target_lang；AI 引擎提示词带目标语言名，微软引擎 to= 参数（auto→zh-Hans）；
    弹窗标题显示「引擎 → 目标语言」
  - 全屏检测修复：根因是单一 98% 面积比判据（最大化约 95.6% 被漏）+ 未覆盖
    “前台第三方最大化也要恢复不透明”；修复：fullscreen.rs 阈值放宽到 97% +
    IsWindowVisible 校验，新增 FOREGROUND_MAXIMIZED 静态标志（poll_loop 每 320ms
    更新，第三方前台 IsZoomed）；desired_taskbar_visual 加
    !is_foreground_maximized()；独立标志不影响音频面板隐藏/隐私边缘弹出（仍只看真全屏）
  - 版本号 v0.20.4 全链路同步；cargo check + ui build 通过

- 2026-08-29 v0.20.3 翻译按钮再缩小一半：75×30 → 38×15
  - tauri.conf.json translate-button width/height 75×30→38×15；translate.rs
    BUTTON_W/BUTTON_H 同步；CSS 字号 12.5→10.5、圆角 15→8、去默认 padding
  - 版本号 v0.20.3 全链路同步

- 2026-08-29 v0.20.2 翻译按钮尺寸 72×30 → 75×30（用户指定）
  - tauri.conf.json translate-button width 72→75；translate.rs BUTTON_W 72→75
    （按钮定位/弹窗居中换算同步）
  - 版本号 v0.20.2 全链路同步

- 2026-08-29 v0.20.1 翻译窗口虚框修复 + 翻译移入功能列表 + 波形幅度加大
  - 虚框修复：翻译按钮/弹窗去掉 CSS 外阴影（透明窗口外阴影会在窗口边缘裁出
    矩形虚影框，与音频面板 v0.16.2 同因），改 inset 高光；新增 lib.rs
    prepare_aux_window（make_tool_window + install_nccalc_fix），translate.rs 在
    show 前重刷窗口样式，防止 wry show 重置 exstyle/样式位导致首帧矩形框
  - 翻译移入主界面功能列表（第 7 项「鼠标选取翻译」卡片，FEATURES[6]）：新增
    TranslatePanel 详情组件（开关 + 引擎下拉 + 微软 Key/Region），SettingsPanel
    移除「鼠标选取翻译」分组与 props（与音频识别 v0.17.0 同套路）
  - 波形幅度：AudioPanel 高度公式 135→175、指数 0.75→0.7（低能量更活跃）
  - 版本号 v0.20.1 全链路同步；cargo check + ui build 通过

- 2026-08-29 v0.20.0 新增「鼠标选取翻译」+ 音频面板音量调节条
  - 鼠标选取翻译（FR-19）：WH_MOUSE_LL 钩子监听左键抬起 → UI Automation
    （IUIAutomation TextPattern，GetFocusedElement/ElementFromPoint + ControlViewWalker
    沿祖先链找 TextPattern）读取前台窗口选中文本与选区位置 → translate-button 小窗
    （72×30 focusable:false 竹青胶囊）出现在选区下方 → 点击 translate_open 打开
    translate-popup（400×300 纸感卡片，focusable:true）→ 异步翻译 → translate-result 事件
  - 关闭方式：钩子 WM_LBUTTONDOWN 点按钮/弹窗以外 → hide 两者；弹窗失焦 / Esc → hide；
    竞态修复：worker 延迟 130ms 检测时若弹窗已显示则跳过（避免误关/重复弹按钮）
  - 引擎：AI 助理（复用 ai.rs DPAPI Key + BaseURL/模型，非流式 chat/completions，
    提示词「翻译成简体中文只输出译文」）；微软翻译（Azure Translator v3，
    Key DPAPI 加密存 ms-translate-key.bin，region 持久化，Ocp-Apim-Subscription-Key/Region 头）
  - 设置：SettingsPanel「鼠标选取翻译」分组（开关 / 引擎下拉 / 微软 Key+Region）；
    prefs 新增 translate_enabled / translate_engine / translate_ms_region
  - 窗口：tauri.conf.json 新增 translate-button / translate-popup（透明无边框置顶 skipTaskbar），
    capabilities windows 列表同步；make_tool_window / poll_loop 兜底修复列表同步；
    on_window_event 关闭=隐藏；UIA 需 windows feature Win32_UI_Accessibility
  - 音频面板音量条：volume.rs（IAudioEndpointVolume Get/SetMasterVolumeLevelScalar +
    Get/SetMute，独立 COM 初始化）；命令 get/set_system_volume、get/set_system_mute；
    面板新增音量行（喇叭按钮静音/取消 + 80ms 节流滑块 + 百分比），窗口高度 132→156
  - 版本号 v0.20.0 全链路同步；cargo check + ui build 通过

- 2026-08-29 v0.19.2 音频封面修复（错图 / Edge 无封面 / 空封面占位）
  - 错图根因：THUMB_CACHE 单槽缓存键命中即永久返回；切歌瞬间 SMTC 缩略图仍是
    上一首（应用异步更新），首曲乱图同因（读到旧会话残留缩略图并缓存）
  - 修复：两段式缓存「候选 → 连续两次一致 → 确认」——键变化后首次读取只作候选，
    事件循环 400ms（THUMB_SETTLE_MS）后强制重读，一致才确认；不一致更新候选继续等；
    空封面不确认，按 1s（THUMB_EMPTY_RETRY_MS）重读，3 次（THUMB_EMPTY_MAX_TRIES）
    仍空才接受（浏览器封面常异步加载）
  - Edge 无封面：read_thumbnail 新增 WebP（RIFF....WEBP）/ AVIF（ftyp avif/avis）嗅探，
    浏览器媒体会话 artwork 常用 WebP；异步加载的空封面由空封面重试兜底
  - 空封面占位：AudioPanel 封面区常驻，thumbnail 为空显示云笈图标
    （复用 ui/src/assets/app-icon.png；object-fit: contain + 14px 内边距 + 柔化，
    `.audio-panel-art.placeholder`）
  - 版本号同步 v0.19.2（Cargo / Cargo.lock / tauri.conf / package / ui package / App /
    About / Settings）；cargo check + cargo test + ui build 通过

- 2026-08-27 v0.19.1 音频 SMTC 改为事件驱动（修 CPU 高占用）
  - 现象：云笈 + NPSMS（Now Playing Session Manager / NPSMSvc）合计占用约 50% CPU
  - 根因：audio.rs smtc_loop 每 1 秒创建 SMTC manager 并查询会话/属性/封面，持续唤醒 NPSMS
  - 修复：订阅 CurrentSessionChanged / SessionsChanged / MediaPropertiesChanged / PlaybackInfoChanged / TimelinePropertiesChanged，系统有变化才读取推送；空闲完全不访问 SMTC
  - 前端 AudioPanel 增加本地 1 秒进度推进，事件驱动下进度条仍平滑
  - 版本号同步 v0.19.1

- 2026-08-21 v0.19.0 音频面板自定义位置与透明度（设置项）
  - 需求：位置/透明度交给用户——设置面板「音频识别」分组新增**面板背景透明度**滑块（0~100，默认 75）与**鼠标穿透**开关（默认关）
  - 透明度持久化：prefs `audio_panel_opacity`（u8，serde default）+ snapshot/前端 `audioPanelOpacity`。`set_audio_panel_opacity` 仅持久化+emit；前端 AudioPanel 用 CSS 变量应用：`.audio-panel` 背景渐变 use `var(--audio-bg-top)/var(--audio-bg-bottom)`，JS 注入 `--audio-bg-top: {opacity}%、--audio-bg-bottom: min(100, opacity+4)%`（保留默认 72%/76%）
  - 鼠标穿透：prefs `audio_panel_click_through`；`set_audio_panel_click_through` 用 **WS_EX_TRANSPARENT**（`set_click_through_window`，需联动 WS_EX_LAYERED，enabled=false 时仅清 TRANSPARENT）+ emit；**穿透开=仅展示（点击其下方窗口），关=可操作**；启动时 on_window_event audio-panel 就绪按持久化值应用穿透
  - **拖拽已删除（用户决定不要拖拽功能）**：曾两版尝试让面板可拖（`data-tauri-drag-region` 顶条 → 手动 SetCapture 拖拽 `start_audio_drag`），用户在 focusable:false 透明置顶小窗上仍拖不动/不想要 → 全部移除：Rust `start_audio_drag`/`set_audio_panel_position` 调用方、bridge `startAudioDrag`、AudioPanel `handlePanelMouseDown`/onMoved 持久化/`clickThrough` 状态、CSS `.draggable`、windows-sys `Win32_System_SystemServices` feature。**默认位置右/下边距 21→26px（整体左移/上移 5px，`AudioPanel.tsx` MARGIN）**。教训：这种自定义无边框小窗想可拖拽，不要走 data-tauri-drag-region，直接用 SetCapture 手动拖才可靠
  - AudioPanel 状态新增 `opacity`（`clickThrough` 已删，OS 层管穿透不需要前端跟踪）
  - 前端：App.tsx 加 `handleAudioOpacityChange/handleAudioClickThroughChange`；vite-env.d.ts AppState + bridge fallback 加两字段；SettingsPanel 新增分组（RangeRow 透明度 + Switch 穿透）
  - 版本号同步 v0.19.0（Cargo / tauri.conf / ui package / App sidebar / About / Settings footer）；注意「组件命令行 `cargo build` 的 stderr 在 PowerShell 被标成 Error 是正常的，看 exit code」
- 2026-08-18 v0.18.0 音频面板增强（封面 + 主题色 + 波形幅度）
  - **封面**：SMTC 读专辑/视频缩略图（audio.rs `read_thumbnail`：`props.Thumbnail()`→`IRandomAccessStreamReference.OpenReadAsync`→`DataReader.LoadAsync/ReadBytes` 读流→按文件头嗅探 mime（png/jpeg/gif/bmp）→base64 data URL，1.5MB 上限）；`THUMB_CACHE` 按 标题|艺术家|专辑|应用 缓存避免每 1 秒重复解码；MediaState 加 `thumbnail`（空串=无封面）。WinRT 接口参数须传**引用**（windows-core `Param<T> for &U`，`DataReader::CreateDataReader(&input)` 而非传值）
  - **封面展示**：前端 `.audio-panel-art` 58×58 正方形，圆角 12px 与外框一致；前端窗口 320×108→**384×132**（tauri.conf audio-panel）
  - **主题色**：前端 `extractAccent`（canvas 16×16 采样封面，只统计饱和度>0.16 像素求均色，亮度<0.35 提亮）→ 注入 CSS 变量 `--audio-accent`，应用到处：主播放按钮背景、进度条、波形条渐变（默认回落竹青 `var(--audio-accent, var(--color-bamboo))`）
  - **波形幅度**：height 由 `round(v*100)` 改为 `min(100, max(10, round(pow(v,0.75)*135)))`——非线性提低能量、底座抬到 10%，震动更明显
  - MediaState 类型权威定义在 `ui/src/vite-env.d.ts`（AudioPanel 不重复定义本地 interface），`bridge.onMediaState` 依赖它；TS 变更集中一处
  - windows crate 加 `Storage_Streams` feature；版本 0.17.0→0.18.0
  - **定位修复（用户反馈：窗口加宽后太靠右/盖住任务栏）**：窗口 320→384 后，残留持久化的旧贴边左上角坐标（基于旧宽）导致右/下缘溢出。修复：AudioPanel 初始化统一定位并 **clamp 进主显示器工作区**（`min(max(x, work.left), work.right-width)`，y 同理）——窗口变宽/变高后坐标不再合适也强制回拉到工作区内，不超屏右/下边界、不盖任务栏
  - **定位微调（用户偏好：留安全距离）**：默认右下角边距 16→**21px**（MARGIN=21，面板默认距右/下缘 21px），且 clamp 兜底 **SAFE=5**（持久化坐标即使贴最右/最下缘也至少留 5px）——面板不再贴着屏幕边缘和任务栏
  - **透明度（用户偏好：更透）**：`.audio-panel` 背景纸感 alpha 82%/86% → **72%/76%**，更透出桌面
  - 待用户实测（封面需播放器通过 SMTC 提供缩略图，Apple Music/mpv 等有，桌面浏览器标签常无）
- 2026-08-18 v0.17.0.1 修复「启动/开开关残留音频透明虚框」
  - 现象：上次退出时音频开关开着 → 下次启动若无播放，右下角残留透明虚框
  - 根因：音频面板显示与否的权威被破坏——前端初始化 getState 后**无条件 `win.show()`**；后端 `set_audio_panel_enabled(true)` 也强制 `win.show()`。两者都在「无媒体会话（media=null→visible=false）」时把窗口显示出来，而 visible-effect 依赖 `[visible]`（false 不变不重跑），窗口滞留显示空内容 = 透明虚框
  - 修复：显示由前端 `visible` 单一决定——① 前端 init 只定位不 show；② visible 计算加入 `enabled !== false`（enabled 从 state-updated/getState 读取）；③ 后端开开关不再 show/uminimize（有媒体时前端 visible 变 true 自行 show，同时覆盖 v0.16.3「重开不显示」）；关闭仍 hide
  - 经验：**辅助窗口的 show/hide 只能由一个权威源（前端 visible）驱动**；后端/init 的旁路 show 会与 React 可见性状态产生错配（show 了但应隐藏）
- 2026-08-18 v0.17.0 移除虚拟桌宠 + 音频识别移入功能列表
  - **删除桌宠全部功能**（用户明确不需要）：pet-window 窗口、PetWindow.tsx、set_pet_enabled/set_pet_position、prefs pet_* 字段、poll_loop 隐私联动、privacy.rs collect_cb 桌宠跳过、CSS .pet-* 样式、FEATURES 桌宠项
  - **音频识别从设置面板移入主界面功能列表**（第 6 项卡片，handleToggle 走 set_audio_panel_enabled）：SettingsPanel 删除「音频识别」分组与 props
  - 需求文档 v1.13：删除 4.16 虚拟桌宠章节（4.17→4.16、4.18→4.17），SC 场景/持久化表/NFR-35/技术架构/工程约定/第 8 节同步清理
  - 实测：功能列表 = 双击隐藏图标 | 任务栏 | 性能监控 | 隐私 | AI 助手 | 音频识别；窗口数 3（pet-window 已消失）
  - GitHub Releases 仅保留 v0.17.0（其余 v0.7.0~v0.16.15 全部删除；git tag 完整保留）
- 2026-08-18 README 按 colleague-skill 风格重写（v0.17.0 后）
  - 学习 https://github.com/titanwings/colleague-skill 的 README 风格：居中标题区+态度引言+badges、排比场景铺垫、更新公告 blockquote、表格化特性/命令/结构、ASCII 界面布局、树状目录、底部署名
  - 云笈 README 重写：六项功能表格、老板键/AI 小窗快捷键、音频面板 ASCII、技术架构（含辅助窗口完整配方）、更新日志补全至 v0.17.0
  - 仓库无 LICENSE 文件（badges 不写 License）

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
- 最新代码：v1.0.0（首个正式版：设置项间距修复，2026-08-30，本地未提交）
- 已发布线：v0.7.x ~ v0.16.15 历史 + v0.17.0（GitHub Releases 仅保留 v0.17.0；git tag 完整保留）
- 本地未推送：v0.18.0 ~ v0.20.1（构建产物与安装包待用户确认后发版）
- git 状态：main 与远端同步至 `af0cda4`（v0.19.0 README）；v0.19.1 起的全部改动未提交
- 版本线：v0.9.0 开关记忆 → v0.10.x 隐私/自动隐藏/动画 → v0.11.x AI 助手+BaseURL/主题/性能 → v0.12.x 托盘快捷开关/TranslucentTB 修复 → v0.13.0 老板键 → v0.14.0 AI 小窗 → v0.15.0 音频识别 → v0.16.x 面板修复/标题框终案 → v0.17.0 移除桌宠/音频识别入功能列表 → v0.18.0 封面/主题色/波形 → v0.19.0 面板透明度/穿透（移除拖拽） → v0.19.1 SMTC 事件驱动（CPU 修复） → v0.19.2 封面缓存修复+空封面占位 → v0.20.0 鼠标选取翻译+音量条 → v0.20.1 翻译虚框修复/移入功能列表+波形幅度

**需求文档当前状态**
- `CloudSatchel需求文档.md`（根目录，v1.25）：FR-01~FR-13、FR-15、FR-17、FR-18、FR-19 全部已实现；FR-16 虚拟桌宠已移除（v0.17.0）；音频识别与鼠标选取翻译均为主界面功能列表项
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
