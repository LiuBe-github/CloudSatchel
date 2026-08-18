//! 云笈（Cloud Satchel）· Tauri 2 应用
//!
//! 前端：React + TS + CSS（ui/ 目录，Vite 构建）
//! 后端：本模块 —— Win32 桌面图标控制 + 全局双击检测
//!
//! 纯净性：本地运行，不联网、不写注册表、无开机启动、退出即恢复。
//! 后台驻留：关闭窗口时询问「最小化到托盘 / 直接退出」；托盘常驻，
//!           左键单击或托盘菜单可随时恢复主窗口。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai;
mod audio;
mod autostart;
mod background;
mod dlog;
mod fullscreen;
mod hooks;
mod hotkey;
mod icons;
mod perf;
mod prefs;
mod privacy;
mod taskbar;
mod taskbar_engine;
mod tray;

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

// ---------------------------------------------------------------------------
// 应用状态
// ---------------------------------------------------------------------------

/// 应用状态（pub(crate)：托盘模块需要读写开关字段以支持快捷开关）
pub(crate) struct AppState {
    pub(crate) enabled: Mutex<bool>,     // 功能是否激活
    pub(crate) icons_hidden: Mutex<bool>, // 图标当前是否隐藏
    animating: Mutex<bool>,      // 是否正在动画
    pending_toggle: Mutex<bool>, // 动画期间到达的切换请求（动画结束后立即执行，保证每次双击都不被吞）
    pub(crate) taskbar_transparent: Mutex<bool>, // 任务栏是否透明
    fullscreen_active: Mutex<bool>, // 当前前台是否有应用处于全屏（用于暂时取消任务栏透明）
    taskbar_applied: Mutex<bool>, // 任务栏“实际”当前是否已透明（用户意图与全屏叠加后的结果）
    theme: Mutex<String>,        // light / dark / system
    autostart: Mutex<bool>,      // 开机自启动（启动文件夹快捷方式）
    close_to_tray: Mutex<bool>,  // 关闭到托盘：true=点击关闭最小化到托盘；false=点击关闭直接退出
    background: Mutex<background::BackgroundSettings>, // 背景图片设置
    performance_monitor: Mutex<bool>, // 主机性能监控是否激活
    pub(crate) privacy_enabled: Mutex<bool>, // 隐私操作（FR-13）是否激活
    privacy_idle_secs: Mutex<u32>,   // 隐私操作空闲触发时间（秒）
    privacy_active: Mutex<bool>,     // 隐私操作当前是否已触发（运行时状态，不入盘）
    pub(crate) autohide_enabled: Mutex<bool>, // 任务栏自动隐藏（FR-02 开关二，开启即隐藏）
    perf_interval_ms: Mutex<u32>,    // 性能监控采样间隔（毫秒）
    ai_model: Mutex<String>,         // AI 助手模型名
    ai_base_url: Mutex<String>,      // AI 助手接口地址（OpenAI 兼容，默认 OpenAI 官方）
    privacy_boss_key: Mutex<String>, // 隐私老板键（FR-13 扩展，默认 Ctrl+`）
    boss_key_registered: Mutex<bool>, // 老板键热键当前是否注册成功（运行时状态，不入盘）
    ai_popup_enabled: Mutex<bool>,   // AI 小窗（FR-17）开关（默认开）
    ai_popup_hotkey: Mutex<String>,  // AI 小窗呼出快捷键（默认 Ctrl+Shift+Space）
    ai_popup_registered: Mutex<bool>, // AI 小窗热键当前是否注册成功（运行时状态，不入盘）
    audio_panel_enabled: Mutex<bool>, // 音频识别面板（FR-18）开关（默认开）
    audio_panel_x: Mutex<i32>,        // 面板位置 X（物理像素，-1 = 未设置 → 右下角默认）
    audio_panel_y: Mutex<i32>,        // 面板位置 Y
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            enabled: Mutex::new(true),
            icons_hidden: Mutex::new(false),
            animating: Mutex::new(false),
            pending_toggle: Mutex::new(false),
            taskbar_transparent: Mutex::new(false),
            fullscreen_active: Mutex::new(false),
            taskbar_applied: Mutex::new(false),
            theme: Mutex::new("system".to_string()),
            autostart: Mutex::new(false),
            close_to_tray: Mutex::new(true),
            background: Mutex::new(background::BackgroundSettings::default()),
            performance_monitor: Mutex::new(false),
            privacy_enabled: Mutex::new(false),
            privacy_idle_secs: Mutex::new(60),
            privacy_active: Mutex::new(false),
            autohide_enabled: Mutex::new(false),
            perf_interval_ms: Mutex::new(1000),
            ai_model: Mutex::new("gpt-4o-mini".to_string()),
            ai_base_url: Mutex::new(ai::DEFAULT_BASE_URL.to_string()),
            privacy_boss_key: Mutex::new("Ctrl+`".to_string()),
            boss_key_registered: Mutex::new(false),
            ai_popup_enabled: Mutex::new(true),
            ai_popup_hotkey: Mutex::new("Ctrl+Shift+Space".to_string()),
            ai_popup_registered: Mutex::new(false),
            audio_panel_enabled: Mutex::new(true),
            audio_panel_x: Mutex::new(-1),
            audio_panel_y: Mutex::new(-1),
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    enabled: bool,
    icons_hidden: bool,
    taskbar_transparent: bool,
    theme: String,
    animating: bool,
    fullscreen_active: bool,
    autostart: bool,
    close_to_tray: bool,
    performance_monitor: bool,
    privacy_enabled: bool,
    privacy_idle_secs: u32,
    privacy_active: bool,
    autohide_enabled: bool,
    perf_interval_ms: u32,
    ai_model: String,
    ai_base_url: String,
    privacy_boss_key: String,
    boss_key_registered: bool,
    ai_popup_enabled: bool,
    ai_popup_hotkey: String,
    ai_popup_registered: bool,
    audio_panel_enabled: bool,
    audio_panel_x: i32,
    audio_panel_y: i32,
    background_image_path: String,
    background_fit: String,
    background_dim: f64,
    background_blur: f64,
    background_scale: f64,
    background_position_x: f64,
    background_position_y: f64,
}

fn snapshot(state: &AppState) -> Snapshot {
    let bg = state.background.lock().unwrap();
    Snapshot {
        enabled: *state.enabled.lock().unwrap(),
        icons_hidden: *state.icons_hidden.lock().unwrap(),
        taskbar_transparent: *state.taskbar_transparent.lock().unwrap(),
        theme: state.theme.lock().unwrap().clone(),
        animating: *state.animating.lock().unwrap(),
        fullscreen_active: *state.fullscreen_active.lock().unwrap(),
        autostart: *state.autostart.lock().unwrap(),
        close_to_tray: *state.close_to_tray.lock().unwrap(),
        performance_monitor: *state.performance_monitor.lock().unwrap(),
        privacy_enabled: *state.privacy_enabled.lock().unwrap(),
        privacy_idle_secs: *state.privacy_idle_secs.lock().unwrap(),
        privacy_active: *state.privacy_active.lock().unwrap(),
        autohide_enabled: *state.autohide_enabled.lock().unwrap(),
        perf_interval_ms: *state.perf_interval_ms.lock().unwrap(),
        ai_model: state.ai_model.lock().unwrap().clone(),
        ai_base_url: state.ai_base_url.lock().unwrap().clone(),
        privacy_boss_key: state.privacy_boss_key.lock().unwrap().clone(),
        boss_key_registered: *state.boss_key_registered.lock().unwrap(),
        ai_popup_enabled: *state.ai_popup_enabled.lock().unwrap(),
        ai_popup_hotkey: state.ai_popup_hotkey.lock().unwrap().clone(),
        ai_popup_registered: *state.ai_popup_registered.lock().unwrap(),
        audio_panel_enabled: *state.audio_panel_enabled.lock().unwrap(),
        audio_panel_x: *state.audio_panel_x.lock().unwrap(),
        audio_panel_y: *state.audio_panel_y.lock().unwrap(),
        background_image_path: bg.image_path.clone(),
        background_fit: bg.fit.clone(),
        background_dim: bg.dim,
        background_blur: bg.blur,
        background_scale: bg.scale,
        background_position_x: bg.position_x,
        background_position_y: bg.position_y,
    }
}

/// 把当前「可持久化」的开关与背景设置写入 settings.json。
///
/// 每次开关变化后立即保存（与 background::save 的旧行为一致），
/// 退出 / 崩溃都不丢状态；保存失败只记日志，不打断用户操作。
fn persist(state: &AppState) {
    let bg = state.background.lock().unwrap();
    let prefs = prefs::AppPrefs {
        enabled: *state.enabled.lock().unwrap(),
        taskbar_transparent: *state.taskbar_transparent.lock().unwrap(),
        performance_monitor: *state.performance_monitor.lock().unwrap(),
        theme: state.theme.lock().unwrap().clone(),
        close_to_tray: *state.close_to_tray.lock().unwrap(),
        privacy_enabled: *state.privacy_enabled.lock().unwrap(),
        privacy_idle_secs: *state.privacy_idle_secs.lock().unwrap(),
        autohide_enabled: *state.autohide_enabled.lock().unwrap(),
        perf_interval_ms: *state.perf_interval_ms.lock().unwrap(),
        ai_model: state.ai_model.lock().unwrap().clone(),
        ai_base_url: state.ai_base_url.lock().unwrap().clone(),
        privacy_boss_key: state.privacy_boss_key.lock().unwrap().clone(),
        ai_popup_enabled: *state.ai_popup_enabled.lock().unwrap(),
        ai_popup_hotkey: state.ai_popup_hotkey.lock().unwrap().clone(),
        audio_panel_enabled: *state.audio_panel_enabled.lock().unwrap(),
        audio_panel_x: *state.audio_panel_x.lock().unwrap(),
        audio_panel_y: *state.audio_panel_y.lock().unwrap(),
        image_path: bg.image_path.clone(),
        fit: bg.fit.clone(),
        dim: bg.dim,
        blur: bg.blur,
        scale: bg.scale,
        position_x: bg.position_x,
        position_y: bg.position_y,
    };
    if let Err(e) = prefs::save(&prefs) {
        dlog::write(&format!("[prefs] 保存设置失败: {e}"));
    }
    // 托盘快捷开关勾选双向同步（开关变化后立即更新，任意一侧切换另一侧一致）
    tray::update_checks(
        *state.enabled.lock().unwrap(),
        *state.taskbar_transparent.lock().unwrap(),
        *state.autohide_enabled.lock().unwrap(),
        *state.privacy_enabled.lock().unwrap(),
    );
}

/// 串行化任务栏视觉切换：用户开关与全屏切换可能同时触发，避免 stop/start 竞态
static TASKBAR_LOCK: Mutex<()> = Mutex::new(());

/// 任务栏“应当”呈现的视觉状态 = 用户开启透明 && 当前无全屏应用
fn desired_taskbar_visual(state: &std::sync::Arc<AppState>) -> bool {
    *state.taskbar_transparent.lock().unwrap() && !*state.fullscreen_active.lock().unwrap()
}

/// 把任务栏同步到目标视觉状态（幂等，异步执行避免阻塞主线程）
///
/// 全屏检测到变化、用户切换任务栏开关时都会调用这里。真正的引擎启停可能耗时
/// 数百毫秒到数秒（Win11 走 TranslucentTB），因此放到 spawn_blocking 后台执行；
/// 内部再用 TASKBAR_LOCK 串行化，防止多次调用交错。
fn sync_taskbar(app: &AppHandle, state: &std::sync::Arc<AppState>) {
    let desired = desired_taskbar_visual(state);
    let applied = *state.taskbar_applied.lock().unwrap();
    if desired == applied {
        return;
    }

    let app2 = app.clone();
    let state2 = state.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = TASKBAR_LOCK.lock().unwrap();

        // 排队期间状态可能又变了：以“最新意图”为准，幂等跳过
        let desired_now = desired_taskbar_visual(&state2);
        let applied_now = *state2.taskbar_applied.lock().unwrap();
        if desired_now == applied_now {
            return;
        }

        let ok = taskbar::set_transparent(desired_now);
        {
            let mut applied = state2.taskbar_applied.lock().unwrap();
            *applied = if ok { desired_now } else { false };
        }
        if !ok && desired_now {
            // 开启透明失败（例如引擎启动失败）：回滚用户开关，让前端状态与真实一致
            *state2.taskbar_transparent.lock().unwrap() = false;
            persist(&state2);
        }
        let _ = app2.emit("state-updated", snapshot(&state2));
    });
}

/// 退出前的统一清理：停止全局钩子、恢复桌面图标、恢复任务栏、恢复隐私/自动隐藏状态
fn cleanup_on_exit(app: &AppHandle) {
    hooks::stop();
    perf::stop();
    privacy::stop();
    audio::stop();
    hotkey::unregister_all();
    icons::restore_icons();
    let state = app.state::<std::sync::Arc<AppState>>();
    if *state.taskbar_transparent.lock().unwrap() || *state.taskbar_applied.lock().unwrap() {
        taskbar::restore();
    }
}

/// 退出进程（托盘菜单「退出」与前端「直接退出」均走这里）
fn cleanup_and_quit(app: &AppHandle) {
    cleanup_on_exit(app);
    app.exit(0);
}

/// 单实例保护：已有实例在运行时返回 true（本实例应退出）
///
/// 两个实例会各自安装全局鼠标钩子、各自维护一份状态，
/// 同时切换同一个桌面图标窗口会导致“显示完又被另一个实例隐藏”。
fn is_already_running() -> bool {
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;

    unsafe {
        let handle = CreateMutexW(
            std::ptr::null(),
            0,
            windows_sys::core::w!("CloudSatchel_SingleInstance_v1"),
        );
        if handle.is_null() {
            return false;
        }
        // 故意不关闭句柄：让互斥体随进程存活，进程退出后系统自动清理
        let _ = handle;
        GetLastError() == ERROR_ALREADY_EXISTS
    }
}

/// 已有实例时，把它的窗口带到前台
fn activate_existing_window() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };
    unsafe {
        let hwnd = FindWindowW(std::ptr::null(), windows_sys::core::w!("云笈"));
        if !hwnd.is_null() {
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
        }
    }
}

/// WM_NCCALCSIZE 全客户区子类化：强制客户区 = 窗口区。
///
/// 目的：彻底消除任何残留的「幽灵标题栏 / 非客户区」——即使 DWM 或
/// tao/wry 在窗口激活等时机重新计算非客户区（用户实测点击面板时标题栏
/// 会弹出），这里直接让系统把整个窗口当作客户区，无任何标题栏可画。
/// 老式子类化（替换 GWLP_WNDPROC 并转发），不依赖 comctl32。
#[cfg(target_os = "windows")]
fn install_nccalc_fix(window: &tauri::WebviewWindow) {
    use std::collections::HashMap;
    use std::sync::Mutex as StdMutex;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::SystemInformation::GetTickCount64;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, DefWindowProcW, GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos,
        GWLP_WNDPROC, GWL_EXSTYLE, GWL_STYLE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
        SWP_NOSIZE, SWP_NOZORDER,
    };

    static ORIG_PROCS: StdMutex<Option<HashMap<isize, isize>>> = StdMutex::new(None);
    static LAST_FIX_TICK: std::sync::atomic::AtomicU64 =
        std::sync::atomic::AtomicU64::new(0);

    unsafe extern "system" fn aux_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        // ===== 排查日志（调试期）：记录焦点 / 非客户区 / 命中测试消息与窗口样式 =====
        const WM_NCCALCSIZE_MSG: u32 = 0x0083;
        const WM_NCPAINT_MSG: u32 = 0x0085;
        const WM_NCACTIVATE_MSG: u32 = 0x0086;
        const WM_ACTIVATE_MSG: u32 = 0x0006;
        const WM_SETFOCUS_MSG: u32 = 0x0007;
        const WM_KILLFOCUS_MSG: u32 = 0x0008;
        const WM_NCHITTEST_MSG: u32 = 0x0084;
        let interesting =
            matches!(msg, WM_NCCALCSIZE_MSG | WM_NCPAINT_MSG | WM_ACTIVATE_MSG | WM_SETFOCUS_MSG | WM_KILLFOCUS_MSG | WM_NCHITTEST_MSG);
        if interesting {
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            crate::dlog::write(&format!(
                "[aux] h={hwnd:?} msg=0x{msg:04X} w={wparam} style=0x{style:08X} ex=0x{ex:08X}"
            ));
        }

        // 吞掉非客户区激活/重绘消息：不给系统任何绘制经典标题栏的机会。
        // tao 子类链在上层已完成焦点事件派发，这里直接短路 DefWindowProc 的 NC 绘制。
        if msg == WM_NCACTIVATE_MSG {
            return 1; // TRUE = 已处理，无需重绘非客户区
        }
        if msg == WM_NCPAINT_MSG {
            return 0;
        }

        if msg == WM_NCCALCSIZE_MSG {
            // 客户区 = 窗口区（无标题栏 / 无边框空间）
            return 0;
        }

        let orig = ORIG_PROCS
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|m| m.get(&(hwnd as isize)).copied())
            .unwrap_or(0);
        let result = if orig != 0 {
            CallWindowProcW(Some(std::mem::transmute(orig)), hwnd, msg, wparam, lparam)
        } else {
            DefWindowProcW(hwnd, msg, wparam, lparam)
        };

        // ===== 防御：任何消息处理后，若窗口样式被重置为「带标题栏」状态，立即修复 =====
        // 背景：实测点击面板（激活）时，样式会被重置回 0x14CB0000
        // （WS_CAPTION|WS_SYSMENU|WS_BORDER + APPWINDOW）→ DWM 画标题框；
        // 轮询（2 秒）修复太慢，这里在消息处理返回后立即检查并修复（节流 500ms 防风暴）。
        {
            const WS_CAPTION_ALL: u32 = 0x00C00000;
            const WS_SYSMENU: u32 = 0x00080000;
            const WS_BORDER: u32 = 0x00800000;
            const WS_MINMAX: u32 = 0x00030000;
            const WS_POPUP: u32 = 0x80000000;
            const WS_EX_APPWINDOW: u32 = 0x00040000;
            const WS_EX_TOOLWINDOW: u32 = 0x00000080;
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            let bad = (style & (WS_CAPTION_ALL | WS_SYSMENU | WS_BORDER | WS_MINMAX)) != 0
                || (style & WS_POPUP) == 0
                || (ex & WS_EX_APPWINDOW) != 0
                || (ex & WS_EX_TOOLWINDOW) == 0;
            if bad {
                let now = unsafe { GetTickCount64() };
                if now.wrapping_sub(LAST_FIX_TICK.load(std::sync::atomic::Ordering::Relaxed)) >= 500 {
                    LAST_FIX_TICK.store(now, std::sync::atomic::Ordering::Relaxed);
                    let new_style = (style
                        & !(WS_CAPTION_ALL | WS_SYSMENU | WS_BORDER | WS_MINMAX))
                        | WS_POPUP;
                    let new_ex = (ex | WS_EX_TOOLWINDOW) & !WS_EX_APPWINDOW;
                    SetWindowLongPtrW(hwnd, GWL_STYLE, new_style as isize);
                    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex as isize);
                    SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                    );
                    crate::dlog::write(&format!(
                        "[aux] FIXED after msg=0x{msg:04X} h={hwnd:?}: style=0x{new_style:08X} ex=0x{new_ex:08X} (was 0x{style:08X}/0x{ex:08X})"
                    ));
                }
            }
        }

        if msg == WM_NCHITTEST_MSG {
            crate::dlog::write(&format!("[aux] h={hwnd:?} NCHITTEST -> 0x{result:X}"));
        }
        result
    }

    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    let hwnd: *mut core::ffi::c_void = hwnd.0;
    unsafe {
        let mut guard = ORIG_PROCS.lock().unwrap();
        let map = guard.get_or_insert_with(HashMap::new);
        if !map.contains_key(&(hwnd as isize)) {
            let orig = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, aux_wnd_proc as isize);
            map.insert(hwnd as isize, orig);
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            crate::dlog::write(&format!(
                "[aux] nccalc installed hwnd={hwnd:?} style=0x{style:08X} ex=0x{ex:08X} orig=0x{orig:X}"
            ));
        }
    }
}

/// 把辅助窗口标记为「工具窗口」：加 WS_EX_TOOLWINDOW、清 WS_EX_APPWINDOW。
///
/// 目的：Tauri 2 的 skipTaskbar 在 Windows 上未设置 TOOLWINDOW 样式
/// （实测 EX_APPWINDOW=True），窗口会出现在 Alt+Tab / 任务视图里——
/// 小面板与用户正在使用的应用（如 Photoshop）并列，观感诡异。
/// 工具窗口样式使其从 Alt+Tab、任务栏中彻底消失。
///
/// 同时彻底去除窗口框架：
/// - GWL_STYLE 清 WS_CAPTION/WS_SYSMENU/WS_BORDER/MIN/MAX 并置 WS_POPUP
///   （wry 的 decorations:false 仅隐藏标题栏渲染但保留样式位，失焦时
///   DWM 会按样式位绘制「非活动窗口框架」——实测复现）
/// - DWMWA_NCRENDERING_POLICY=DWMNCRP_DISABLED 禁用 DWM 非客户区渲染
/// - DWMWA_WINDOW_CORNER_PREFERENCE=ROUND：系统圆角（注意不能用 SetWindowRgn
///   裁剪——acrylic 等 DWM 背景按窗口矩形渲染，region 不裁剪 DWM 背景会变直角）
#[cfg(target_os = "windows")]
fn make_tool_window(window: &tauri::WebviewWindow) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, GWL_STYLE,
        SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_BORDER,
        WS_CAPTION, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP,
        WS_SYSMENU,
    };
    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    let hwnd: *mut core::ffi::c_void = hwnd.0;
    unsafe {
        // 样式：置 WS_POPUP 并清除所有框架相关位
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let new_style = (style
            & !(WS_CAPTION | WS_SYSMENU | WS_BORDER | WS_MINIMIZEBOX | WS_MAXIMIZEBOX))
            | WS_POPUP;
        if new_style != style {
            SetWindowLongPtrW(hwnd, GWL_STYLE, new_style as isize);
        }
        // 扩展样式：工具窗口（不进 Alt+Tab / 任务视图）
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let new_ex = (ex | WS_EX_TOOLWINDOW) & !WS_EX_APPWINDOW;
        if new_ex != ex {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex as isize);
        }
        if new_style != style || new_ex != ex {
            // 样式变更后通知系统重算（标题栏/任务栏归属）
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
        crate::dlog::write(&format!(
            "[aux] make_tool_window label={} style=0x{new_style:08X} ex=0x{new_ex:08X}",
            window.label()
        ));
        // 禁用 DWM 非客户区渲染（彻底不绘制标题栏/边框，失焦也不出现）
        let ncr_policy: i32 = 1; // DWMNCRP_DISABLED
        let _ = windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute(
            hwnd,
            2, // DWMWA_NCRENDERING_POLICY
            &ncr_policy as *const i32 as *const core::ffi::c_void,
            std::mem::size_of::<i32>() as u32,
        );
        // 关闭 Win11 的窗口边框（DWMWA_BORDER_COLOR = 34，DWMWA_COLOR_NONE = 0xFFFFFFFE）：
        // Win11 22H2+ 会给窗口（含失焦态）画 1px 边框线，即用户看到的「框」
        let no_border: u32 = 0xFFFFFFFE;
        let _ = windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute(
            hwnd,
            34, // DWMWA_BORDER_COLOR
            &no_border as *const u32 as *const core::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        );
        // 系统圆角（DWMWA_WINDOW_CORNER_PREFERENCE = 33，DWMWCP_ROUND = 2）：
        // 替代 SetWindowRgn 圆角——acrylic 等 DWM 背景按窗口矩形渲染，
        // region 裁剪不到 DWM 背景会变直角；系统圆角是 DWM 层裁剪，两者兼容
        let round_corner: i32 = 2;
        let _ = windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute(
            hwnd,
            33,
            &round_corner as *const i32 as *const core::ffi::c_void,
            std::mem::size_of::<i32>() as u32,
        );
    }
}

// ---------------------------------------------------------------------------
// Tauri Commands（前端 invoke 调用）
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_state(state: State<std::sync::Arc<AppState>>) -> Snapshot {
    snapshot(&state)
}

#[tauri::command]
fn set_enabled(
    app: AppHandle,
    state: State<std::sync::Arc<AppState>>,
    enabled: bool,
) -> Snapshot {
      {
          let mut e = state.enabled.lock().unwrap();
          if *e == enabled {
              // 先释放锁再取快照，否则 snapshot 会再次锁同一个互斥量造成自锁死
              drop(e);
              return snapshot(&state);
          }
          *e = enabled;
      }
    if enabled {
        hooks::start();
    } else {
        hooks::stop();
        // 等待正在进行的动画结束再恢复，避免动画线程随后把图标又藏回去
        icons::restore_icons_blocking();
        *state.icons_hidden.lock().unwrap() = false;
        *state.pending_toggle.lock().unwrap() = false;
    }
    persist(&state);
    let snap = snapshot(&state);
    let _ = app.emit("state-updated", snap.clone());
    snap
}

#[tauri::command]
fn set_theme(app: AppHandle, state: State<std::sync::Arc<AppState>>, mode: String) -> Snapshot {
    let mode = if mode == "light" || mode == "dark" {
        mode
    } else {
        "system".to_string()
    };
    *state.theme.lock().unwrap() = mode;
    persist(&state);
    let snap = snapshot(&state);
    let _ = app.emit("state-updated", snap.clone());
    snap
}

#[tauri::command]
fn set_taskbar_transparent(
    app: AppHandle,
    state: State<std::sync::Arc<AppState>>,
    enabled: bool,
) -> Snapshot {
    let state = state.inner().clone();
    let current = *state.taskbar_transparent.lock().unwrap();
    if current == enabled {
        return snapshot(&state);
    }
    // 先记录用户意图并推送前端，让开关即时响应；
    // 实际的任务栏切换（含全屏叠加逻辑）交给 sync_taskbar 异步执行。
    *state.taskbar_transparent.lock().unwrap() = enabled;
    let snap = snapshot(&state);
    let _ = app.emit("state-updated", snap.clone());
    persist(&state);
    sync_taskbar(&app, &state);
    snap
}

#[tauri::command]
fn set_autostart(
    app: AppHandle,
    state: State<std::sync::Arc<AppState>>,
    enabled: bool,
) -> Result<Snapshot, String> {
    autostart::set_enabled(enabled)?;
    *state.autostart.lock().unwrap() = enabled;
    let snap = snapshot(&state);
    let _ = app.emit("state-updated", snap.clone());
    Ok(snap)
}

#[tauri::command]
fn set_close_to_tray(
    app: AppHandle,
    state: State<std::sync::Arc<AppState>>,
    enabled: bool,
) -> Result<Snapshot, String> {
    *state.close_to_tray.lock().unwrap() = enabled;
    persist(&state);
    let snap = snapshot(&state);
    let _ = app.emit("state-updated", snap.clone());
    Ok(snap)
}

#[tauri::command]
fn set_performance_monitor(
    app: AppHandle,
    state: State<std::sync::Arc<AppState>>,
    enabled: bool,
) -> Snapshot {
    {
        let mut current = state.performance_monitor.lock().unwrap();
        if *current == enabled {
            drop(current);
            return snapshot(&state);
        }
        *current = enabled;
    }
    if enabled {
        perf::start();
    } else {
        perf::stop();
    }
    persist(&state);
    let snap = snapshot(&state);
    let _ = app.emit("state-updated", snap.clone());
    snap
}

#[tauri::command]
fn get_perf_snapshot() -> Option<perf::PerfSnapshot> {
    perf::latest()
}

/// 把空闲类配置同步给 privacy 模块（含开关变化时的即时动作）
fn sync_idle(state: &AppState) {
    privacy::configure(
        *state.privacy_enabled.lock().unwrap(),
        *state.privacy_idle_secs.lock().unwrap(),
        *state.autohide_enabled.lock().unwrap(),
    );
}

/// 老板键全局热键 id（WM_HOTKEY 的 wParam）
const BOSS_KEY_HOTKEY_ID: i32 = 0x1301;

/// 同步老板键热键注册：跟随「隐私操作」开关（FR-13 扩展）。
/// 注册失败（被占用 / 系统保留）返回 Err 并置 boss_key_registered=false，
/// 降级为仅空闲触发，不影响应用运行。
fn sync_boss_key(app: &AppHandle, state: &std::sync::Arc<AppState>) -> Result<(), String> {
    let enabled = *state.privacy_enabled.lock().unwrap();
    if !enabled {
        hotkey::unregister(BOSS_KEY_HOTKEY_ID);
        *state.boss_key_registered.lock().unwrap() = false;
        return Ok(());
    }
    let spec_str = state.privacy_boss_key.lock().unwrap().clone();
    let spec = hotkey::parse_hotkey(&spec_str).ok_or_else(|| {
        format!("快捷键格式无效：{spec_str}（示例：Ctrl+`、Ctrl+Shift+Space）")
    })?;
    let app2 = app.clone();
    let ok = hotkey::register(BOSS_KEY_HOTKEY_ID, spec, move || {
        privacy::boss_key_pressed();
        let st = app2.state::<std::sync::Arc<AppState>>();
        let _ = app2.emit("state-updated", snapshot(&st));
    });
    *state.boss_key_registered.lock().unwrap() = ok;
    if ok {
        Ok(())
    } else {
        Err(format!("快捷键注册失败：{spec_str}（可能被其他程序占用）"))
    }
}

/// AI 小窗全局热键 id
const AI_POPUP_HOTKEY_ID: i32 = 0x1701;

/// 同步 AI 小窗热键注册：跟随「AI 小窗」开关（FR-17）。
/// 热键按下切换小窗显示/隐藏；注册失败返回 Err（前端提示，降级为不可呼出）。
fn sync_ai_popup(app: &AppHandle, state: &std::sync::Arc<AppState>) -> Result<(), String> {
    let enabled = *state.ai_popup_enabled.lock().unwrap();
    if !enabled {
        hotkey::unregister(AI_POPUP_HOTKEY_ID);
        *state.ai_popup_registered.lock().unwrap() = false;
        return Ok(());
    }
    let spec_str = state.ai_popup_hotkey.lock().unwrap().clone();
    let spec = hotkey::parse_hotkey(&spec_str)
        .ok_or_else(|| format!("快捷键格式无效：{spec_str}（示例：Ctrl+Shift+Space）"))?;
    let app2 = app.clone();
    let ok = hotkey::register(AI_POPUP_HOTKEY_ID, spec, move || {
        if let Some(win) = app2.get_webview_window("ai-popup") {
            if win.is_visible().unwrap_or(false) {
                // 隐藏并通知前端清空对话上下文（不落盘）
                let _ = app2.emit("ai-popup-cleared", ());
                let _ = win.hide();
            } else {
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_focus();
                let _ = app2.emit("ai-popup-shown", ());
            }
        }
    });
    *state.ai_popup_registered.lock().unwrap() = ok;
    if ok {
        Ok(())
    } else {
        Err(format!("快捷键注册失败：{spec_str}（可能被其他程序占用）"))
    }
}

#[tauri::command]
fn set_privacy_enabled(
    app: AppHandle,
    state: State<std::sync::Arc<AppState>>,
    enabled: bool,
) -> Snapshot {
    {
        let mut e = state.privacy_enabled.lock().unwrap();
        if *e == enabled {
            drop(e);
            return snapshot(&state);
        }
        *e = enabled;
    }
    sync_idle(&state);
    // 老板键热键跟随开关注册/注销（注册失败仅降级为空闲触发，不影响开关本身）
    let _ = sync_boss_key(&app, &state);
    persist(&state);
    let snap = snapshot(&state);
    let _ = app.emit("state-updated", snap.clone());
    snap
}

/// 设置隐私老板键快捷键（FR-13 扩展）：
/// 解析成功 → 持久化 → 若隐私开关开启则立即重新注册；注册失败回滚旧键并返回错误。
#[tauri::command]
fn set_privacy_boss_key(
    app: AppHandle,
    state: State<std::sync::Arc<AppState>>,
    key: String,
) -> Result<Snapshot, String> {
    let key = key.trim().to_string();
    hotkey::parse_hotkey(&key)
        .ok_or_else(|| format!("快捷键格式无效：{key}（示例：Ctrl+`、Ctrl+Shift+Space）"))?;
    let old = state.privacy_boss_key.lock().unwrap().clone();
    *state.privacy_boss_key.lock().unwrap() = key.clone();
    persist(&state);
    match sync_boss_key(&app, &state) {
        Ok(()) => {
            let snap = snapshot(&state);
            let _ = app.emit("state-updated", snap.clone());
            Ok(snap)
        }
        Err(e) => {
            // 回滚旧键并尽力恢复旧热键
            *state.privacy_boss_key.lock().unwrap() = old;
            persist(&state);
            let _ = sync_boss_key(&app, &state);
            Err(e)
        }
    }
}

#[tauri::command]
fn set_privacy_idle_secs(
    app: AppHandle,
    state: State<std::sync::Arc<AppState>>,
    secs: u32,
) -> Snapshot {
    let secs = secs.clamp(privacy::IDLE_CLAMP_MIN, privacy::IDLE_CLAMP_MAX);
    *state.privacy_idle_secs.lock().unwrap() = secs;
    sync_idle(&state);
    persist(&state);
    let snap = snapshot(&state);
    let _ = app.emit("state-updated", snap.clone());
    snap
}

#[tauri::command]
fn set_autohide_enabled(
    app: AppHandle,
    state: State<std::sync::Arc<AppState>>,
    enabled: bool,
) -> Snapshot {
    {
        let mut e = state.autohide_enabled.lock().unwrap();
        if *e == enabled {
            drop(e);
            return snapshot(&state);
        }
        *e = enabled;
    }
    sync_idle(&state);
    persist(&state);
    let snap = snapshot(&state);
    let _ = app.emit("state-updated", snap.clone());
    snap
}

#[tauri::command]
fn set_perf_interval_ms(
    app: AppHandle,
    state: State<std::sync::Arc<AppState>>,
    ms: u32,
) -> Snapshot {
    let ms = ms.clamp(200, 1000);
    *state.perf_interval_ms.lock().unwrap() = ms;
    perf::set_interval_ms(ms as u64);
    persist(&state);
    let snap = snapshot(&state);
    let _ = app.emit("state-updated", snap.clone());
    snap
}

#[tauri::command]
fn set_ai_model(app: AppHandle, state: State<std::sync::Arc<AppState>>, model: String) -> Snapshot {
    let model = if model.trim().is_empty() {
        "gpt-4o-mini".to_string()
    } else {
        model.trim().to_string()
    };
    *state.ai_model.lock().unwrap() = model;
    persist(&state);
    let snap = snapshot(&state);
    let _ = app.emit("state-updated", snap.clone());
    snap
}

#[tauri::command]
fn set_ai_base_url(
    app: AppHandle,
    state: State<std::sync::Arc<AppState>>,
    base_url: String,
) -> Snapshot {
    *state.ai_base_url.lock().unwrap() = ai::normalize_base_url(&base_url);
    persist(&state);
    let snap = snapshot(&state);
    let _ = app.emit("state-updated", snap.clone());
    snap
}

/// 开关 AI 小窗（FR-17）：关闭后快捷键失效且小窗不可呼出，不影响 FR-15 主 AI 助手
#[tauri::command]
fn set_ai_popup_enabled(
    app: AppHandle,
    state: State<std::sync::Arc<AppState>>,
    enabled: bool,
) -> Snapshot {
    {
        let mut e = state.ai_popup_enabled.lock().unwrap();
        if *e == enabled {
            drop(e);
            return snapshot(&state);
        }
        *e = enabled;
    }
    if !enabled {
        // 关闭时隐藏已打开的小窗并清空对话上下文
        let _ = app.emit("ai-popup-cleared", ());
        if let Some(win) = app.get_webview_window("ai-popup") {
            let _ = win.hide();
        }
    }
    let _ = sync_ai_popup(&app, &state);
    persist(&state);
    let snap = snapshot(&state);
    let _ = app.emit("state-updated", snap.clone());
    snap
}

/// 设置 AI 小窗呼出快捷键（FR-17）：解析成功 → 持久化 → 立即重新注册；失败回滚旧键
#[tauri::command]
fn set_ai_popup_hotkey(
    app: AppHandle,
    state: State<std::sync::Arc<AppState>>,
    key: String,
) -> Result<Snapshot, String> {
    let key = key.trim().to_string();
    hotkey::parse_hotkey(&key)
        .ok_or_else(|| format!("快捷键格式无效：{key}（示例：Ctrl+Shift+Space）"))?;
    let old = state.ai_popup_hotkey.lock().unwrap().clone();
    *state.ai_popup_hotkey.lock().unwrap() = key.clone();
    persist(&state);
    match sync_ai_popup(&app, &state) {
        Ok(()) => {
            let snap = snapshot(&state);
            let _ = app.emit("state-updated", snap.clone());
            Ok(snap)
        }
        Err(e) => {
            *state.ai_popup_hotkey.lock().unwrap() = old;
            persist(&state);
            let _ = sync_ai_popup(&app, &state);
            Err(e)
        }
    }
}

/// 开关音频识别面板（FR-18，默认开）：关闭后停止采集并隐藏面板
#[tauri::command]
fn set_audio_panel_enabled(
    app: AppHandle,
    state: State<std::sync::Arc<AppState>>,
    enabled: bool,
) -> Snapshot {
    {
        let mut e = state.audio_panel_enabled.lock().unwrap();
        if *e == enabled {
            drop(e);
            return snapshot(&state);
        }
        *e = enabled;
    }
    audio::set_enabled(enabled);
    if let Some(win) = app.get_webview_window("audio-panel") {
        if enabled {
            // 重新打开：恢复窗口显示（无播放时前端自动淡出，播放时出现）
            let _ = win.show();
            let _ = win.unminimize();
        } else {
            let _ = win.hide();
        }
    }
    persist(&state);
    let snap = snapshot(&state);
    let _ = app.emit("state-updated", snap.clone());
    snap
}

/// 保存音频面板位置（拖拽结束时调用，持久化）
#[tauri::command]
fn set_audio_panel_position(
    app: AppHandle,
    state: State<std::sync::Arc<AppState>>,
    x: i32,
    y: i32,
) -> Snapshot {
    *state.audio_panel_x.lock().unwrap() = x;
    *state.audio_panel_y.lock().unwrap() = y;
    persist(&state);
    let snap = snapshot(&state);
    let _ = app.emit("state-updated", snap.clone());
    snap
}

/// 媒体控制：play / pause / next / prev（FR-18，SMTC 控制命令）
#[tauri::command]
fn audio_media_control(app: AppHandle, action: String) {
    audio::control(app, &action);
}

#[tauri::command]
fn get_ai_config(state: State<std::sync::Arc<AppState>>) -> ai::AiConfig {
    ai::get_ai_config(
        state.ai_model.lock().unwrap().clone(),
        state.ai_base_url.lock().unwrap().clone(),
    )
}

#[tauri::command]
fn set_background(
    app: AppHandle,
    state: State<std::sync::Arc<AppState>>,
    settings: background::BackgroundSettings,
) -> Snapshot {
    let settings = settings.clamped();
    *state.background.lock().unwrap() = settings.clone();
    persist(&state);
    let snap = snapshot(&state);
    let _ = app.emit("state-updated", snap.clone());
    snap
}

#[tauri::command]
async fn choose_background_image() -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(background::choose_background_image)
        .await
        .unwrap_or_else(|e| Err(format!("选择背景图片失败: {e}")))
}

#[tauri::command]
fn copy_background_image(source_path: String) -> Result<String, String> {
    background::copy_background_image(&source_path)
}

#[tauri::command]
async fn read_background_image(path: String) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || background::read_background_image(&path))
        .await
        .unwrap_or_else(|e| Err(format!("读取背景图片失败: {e}")))
}

#[tauri::command]
fn minimize_window(window: tauri::Window) {
    let _ = window.minimize();
}

#[tauri::command]
fn toggle_maximize_window(window: tauri::Window) {
    if window.is_maximized().unwrap_or(false) {
        let _ = window.unmaximize();
    } else {
        let _ = window.maximize();
    }
}

#[tauri::command]
fn close_window(window: tauri::Window) {
    let _ = window.close();
}

#[tauri::command]
fn hide_to_tray(window: tauri::Window) {
    let _ = window.hide();
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    cleanup_and_quit(&app);
}

/// 执行一次桌面双击切换（由轮询线程调用；动画结束后也会再次调用以执行排队的请求）
///
/// 关键设计：每次物理双击都必须对应一次切换。
/// - 动画进行中到达的请求不丢弃，而是置 pending_toggle，动画结束后立即执行；
/// - 重复触发的防护完全在钩子层完成（配对消费 + DBLCLK 尾巴忽略 + 250ms 防抖），
///   因此这里不再设置长冷却，避免吞掉用户真实的连续双击。
fn start_toggle(app: &AppHandle, state: &std::sync::Arc<AppState>) {
    dlog::write("[start_toggle] begin");
    let target_hidden = {
        let animating = *state.animating.lock().unwrap();
        let enabled = *state.enabled.lock().unwrap();
        if !enabled {
            dlog::write("[start_toggle] enabled=false -> return");
            return;
        }
        if animating {
            // 动画中：排队，动画结束后立即执行（不丢任何真实双击）
            *state.pending_toggle.lock().unwrap() = true;
            dlog::write("[start_toggle] animating -> pending");
            return;
        }

        // 切换方向以桌面图标的“真实可见状态”为准（而非内存中的布尔值）：
        // 隐藏中 → 显示，显示中 → 隐藏；若窗口正处于动画中则排队等待。
        let target = match icons::icon_visibility() {
            icons::IconVisibility::Hidden => false,
            icons::IconVisibility::Visible => true,
            icons::IconVisibility::Animating => {
                // 窗口真实处于动画中（状态错位场景）：排队等待其结束
                *state.pending_toggle.lock().unwrap() = true;
                dlog::write("[start_toggle] animating(real) -> pending");
                return;
            }
        };
        dlog::write(&format!("[start_toggle] target_hidden={}", target));
        *state.animating.lock().unwrap() = true;
        target
    };
    let snap = snapshot(state);
    let _ = app.emit("state-updated", snap);

    // 在后台线程执行动画，避免阻塞主线程
    let app2 = app.clone();
    let state2 = state.clone();
    std::thread::spawn(move || {
        // 约 0.5s 的渐变（256 步 × 2ms），与「渐渐隐藏」的交互预期一致
        icons::run_fade(target_hidden, 256, 2);
        let queued = {
            let mut animating = state2.animating.lock().unwrap();
            if *state2.enabled.lock().unwrap() {
                *state2.icons_hidden.lock().unwrap() = target_hidden;
            } else {
                // 动画期间功能被关闭：撤销本次动画结果，保证图标可见
                icons::restore_icons();
                *state2.icons_hidden.lock().unwrap() = false;
            }
            *animating = false;
            // 动画期间到达的切换请求：取走并立即执行（跳过冷却）
            let mut p = state2.pending_toggle.lock().unwrap();
            let q = *p;
            *p = false;
            q
        };
        let _ = app2.emit("state-updated", snapshot(&state2));
        if queued {
            start_toggle(&app2, &state2);
        }
    });
}

/// 轮询桌面双击事件（后台线程）
fn poll_loop(app: AppHandle, state: std::sync::Arc<AppState>) {
    let mut tick: u64 = 0;
    loop {
        std::thread::sleep(std::time::Duration::from_millis(80));
        if icons::take_hook_event() {
            dlog::write("[poll_loop] take_hook_event=true -> start_toggle");
            start_toggle(&app, &state);
        }

        tick += 1;
        // 每约 2 秒（25 个 80ms 周期）强制修复辅助窗口样式：
        // wry/Tauri 在窗口 show 时会重置 exstyle（实测音频面板 show 后 EX_APPWINDOW 复现），
        // 需周期确保 TOOLWINDOW 生效（不出现 Alt+Tab / 任务视图）
        if tick % 25 == 0 {
            for label in ["ai-popup", "audio-panel"] {
                if let Some(win) = app.get_webview_window(label) {
                    make_tool_window(&win);
                }
            }
        }
        // 每约 320ms（4 个 80ms 周期）检查一次前台窗口是否全屏
        if tick % 4 == 0 {
            // 隐私操作「已触发」状态同步给前端（触发/恢复可能发生在 privacy 轮询线程）
            let pa = privacy::is_triggered();
            {
                let mut cur = state.privacy_active.lock().unwrap();
                if *cur != pa {
                    *cur = pa;
                    drop(cur);
                    let _ = app.emit("state-updated", snapshot(&state));
                }
            }
            let own = fullscreen::find_own_window();
            let fg = fullscreen::foreground_hwnd();
            // 云笈自身前台且最大化 → 按用户的理解视为“全屏”；
            // 否则按“前台第三方窗口覆盖整个显示器”判断真全屏。
            let fullscreen = if fg != 0 && fg == own {
                fullscreen::is_zoomed(fg)
            } else {
                fullscreen::is_fullscreen_now()
            };
            let mut cur = state.fullscreen_active.lock().unwrap();
            if *cur != fullscreen {
                *cur = fullscreen;
                drop(cur);
                dlog::write(&format!(
                    "[poll_loop] fullscreen changed -> {}",
                    fullscreen
                ));
                sync_taskbar(&app, &state);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 生命周期
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dlog::write("[APP] CloudSatchel start");
    if is_already_running() {
        activate_existing_window();
        return;
    }

    let state = std::sync::Arc::new(AppState::default());
    // 恢复上次退出前的开关与设置（settings.json）
    let prefs = prefs::load();
    *state.enabled.lock().unwrap() = prefs.enabled;
    *state.taskbar_transparent.lock().unwrap() = prefs.taskbar_transparent;
    *state.performance_monitor.lock().unwrap() = prefs.performance_monitor;
    *state.theme.lock().unwrap() = prefs.theme.clone();
    *state.close_to_tray.lock().unwrap() = prefs.close_to_tray;
    *state.privacy_enabled.lock().unwrap() = prefs.privacy_enabled;
    *state.privacy_idle_secs.lock().unwrap() = prefs.privacy_idle_secs;
    *state.autohide_enabled.lock().unwrap() = prefs.autohide_enabled;
    *state.perf_interval_ms.lock().unwrap() = prefs.perf_interval_ms;
    *state.ai_model.lock().unwrap() = prefs.ai_model.clone();
    *state.ai_base_url.lock().unwrap() = prefs.ai_base_url.clone();
    *state.privacy_boss_key.lock().unwrap() = prefs.privacy_boss_key.clone();
    *state.ai_popup_enabled.lock().unwrap() = prefs.ai_popup_enabled;
    *state.ai_popup_hotkey.lock().unwrap() = prefs.ai_popup_hotkey.clone();
    *state.audio_panel_enabled.lock().unwrap() = prefs.audio_panel_enabled;
    *state.audio_panel_x.lock().unwrap() = prefs.audio_panel_x;
    *state.audio_panel_y.lock().unwrap() = prefs.audio_panel_y;
    *state.background.lock().unwrap() = prefs.background();
    // 以启动文件夹快捷方式的实际存在情况初始化自启动状态
    *state.autostart.lock().unwrap() = autostart::is_enabled();
    perf::set_interval_ms(prefs.perf_interval_ms as u64);

    // 启动自修复：上次异常退出可能残留透明图标
    icons::ensure_icons_restored();
    // 启动自修复：上次异常退出可能残留任务栏透明开关（注册表备份兜底恢复）
    taskbar::ensure_restored();

    tauri::Builder::default()
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![
            get_state,
            set_enabled,
            set_theme,
            set_taskbar_transparent,
            set_autostart,
            set_close_to_tray,
            set_performance_monitor,
            set_privacy_enabled,
            set_privacy_idle_secs,
            set_privacy_boss_key,
            set_autohide_enabled,
            set_perf_interval_ms,
            set_ai_model,
            set_ai_base_url,
            set_ai_popup_enabled,
            set_ai_popup_hotkey,
            set_audio_panel_enabled,
            set_audio_panel_position,
            audio_media_control,
            get_perf_snapshot,
            get_ai_config,
            ai::save_ai_key,
            ai::ai_send,
            ai::ai_stop,
            set_background,
            choose_background_image,
            copy_background_image,
            read_background_image,
            minimize_window,
            toggle_maximize_window,
            close_window,
            hide_to_tray,
            quit_app
        ])
        .setup(move |app| {
            // 系统托盘（后台驻留入口 + 功能快捷开关）
            tray::setup_tray(app)?;
            // 托盘勾选状态按恢复的开关初始化（FR-06 扩展）
            tray::update_checks(
                *state.enabled.lock().unwrap(),
                *state.taskbar_transparent.lock().unwrap(),
                *state.autohide_enabled.lock().unwrap(),
                *state.privacy_enabled.lock().unwrap(),
            );
            // 自动应用上次退出前的功能效果（开关已从 settings.json 恢复）
            if *state.enabled.lock().unwrap() {
                hooks::start();
            }
            if *state.performance_monitor.lock().unwrap() {
                perf::start();
            }
            // 隐私操作 / 任务栏自动隐藏：先同步配置再启动空闲轮询
            sync_idle(&state);
            privacy::start();
            let app_handle = app.handle().clone();
            // 辅助窗口：工具窗口样式 + 系统圆角 + WM_NCCALCSIZE 全客户区（彻底无标题栏）
            for label in ["ai-popup", "audio-panel"] {
                if let Some(win) = app.get_webview_window(label) {
                    make_tool_window(&win);
                    install_nccalc_fix(&win);
                }
            }
            // 老板键热键跟随隐私开关恢复注册（注册失败仅降级为空闲触发）
            let _ = sync_boss_key(&app_handle, &state);
            // AI 小窗热键跟随开关恢复注册（默认开；注册失败仅降级为不可呼出）
            let _ = sync_ai_popup(&app_handle, &state);
            // 音频识别（FR-18）：启动轮询并按持久化开关设置
            audio::set_enabled(*state.audio_panel_enabled.lock().unwrap());
            audio::start(app_handle.clone());
            if *state.taskbar_transparent.lock().unwrap() {
                sync_taskbar(&app_handle, &state);
            }
            let st = state.clone();
            std::thread::spawn(move || poll_loop(app_handle, st));
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::Destroyed => {
                // 仅主窗口销毁时执行退出清理（AI 小窗等辅助窗口隐藏而非销毁，不触发）
                if window.label() == "main" {
                    cleanup_on_exit(window.app_handle());
                }
            }
            tauri::WindowEvent::Resized(_) => {
                // 辅助窗口（AI 小窗可 resize）尺寸变化后无需额外处理：
                // 系统圆角（DWMWCP_ROUND）随窗口尺寸自动适配
            }
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let app = window.app_handle();
                if window.label() == "ai-popup" {
                    // AI 小窗：关闭 = 隐藏（对话上下文由前端在隐藏时清空，不落盘）
                    let _ = window.hide();
                    return;
                }
                // 主窗口关闭行为由设置「关闭到托盘」决定：
                // 开 → 隐藏到托盘继续后台运行；关 → 清理后直接退出
                let close_to_tray = *app
                    .state::<std::sync::Arc<AppState>>()
                    .close_to_tray
                    .lock()
                    .unwrap();
                if close_to_tray {
                    let _ = window.hide();
                } else {
                    cleanup_and_quit(app);
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
