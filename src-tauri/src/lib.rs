//! 云笈（Cloud Satchel）· Tauri 2 应用
//!
//! 前端：React + TS + CSS（ui/ 目录，Vite 构建）
//! 后端：本模块 —— Win32 桌面图标控制 + 全局双击检测
//!
//! 纯净性：本地运行，不联网、不写注册表、无开机启动、退出即恢复。
//! 后台驻留：关闭窗口时询问「最小化到托盘 / 直接退出」；托盘常驻，
//!           左键单击或托盘菜单可随时恢复主窗口。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod background;
mod dlog;
mod hooks;
mod icons;
mod taskbar;
mod taskbar_engine;
mod tray;

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

// ---------------------------------------------------------------------------
// 应用状态
// ---------------------------------------------------------------------------

struct AppState {
    enabled: Mutex<bool>,        // 功能是否激活
    icons_hidden: Mutex<bool>,   // 图标当前是否隐藏
    animating: Mutex<bool>,      // 是否正在动画
    pending_toggle: Mutex<bool>, // 动画期间到达的切换请求（动画结束后立即执行，保证每次双击都不被吞）
    taskbar_transparent: Mutex<bool>, // 任务栏是否透明
    theme: Mutex<String>,        // light / dark / system
    autostart: Mutex<bool>,      // 开机自启动（启动文件夹快捷方式）
    close_to_tray: Mutex<bool>,  // 关闭到托盘：true=点击关闭最小化到托盘；false=点击关闭直接退出
    background: Mutex<background::BackgroundSettings>, // 背景图片设置
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            enabled: Mutex::new(true),
            icons_hidden: Mutex::new(false),
            animating: Mutex::new(false),
            pending_toggle: Mutex::new(false),
            taskbar_transparent: Mutex::new(false),
            theme: Mutex::new("system".to_string()),
            autostart: Mutex::new(false),
            close_to_tray: Mutex::new(true),
            background: Mutex::new(background::BackgroundSettings::default()),
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
    autostart: bool,
    close_to_tray: bool,
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
        autostart: *state.autostart.lock().unwrap(),
        close_to_tray: *state.close_to_tray.lock().unwrap(),
        background_image_path: bg.image_path.clone(),
        background_fit: bg.fit.clone(),
        background_dim: bg.dim,
        background_blur: bg.blur,
        background_scale: bg.scale,
        background_position_x: bg.position_x,
        background_position_y: bg.position_y,
    }
}

/// 退出前的统一清理：停止全局钩子、恢复桌面图标、恢复任务栏
fn cleanup_on_exit(app: &AppHandle) {
    hooks::stop();
    icons::restore_icons();
    let state = app.state::<std::sync::Arc<AppState>>();
    if *state.taskbar_transparent.lock().unwrap() {
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
    let snap = snapshot(&state);
    let _ = app.emit("state-updated", snap.clone());
    snap
}

#[tauri::command]
async fn set_taskbar_transparent(
    app: AppHandle,
    state: State<'_, std::sync::Arc<AppState>>,
    enabled: bool,
) -> Result<Snapshot, String> {
    let state = state.inner().clone();
    let current = *state.taskbar_transparent.lock().unwrap();
    if current == enabled {
        return Ok(snapshot(&state));
    }
    // 先翻转状态并推送前端，让开关即时响应；
    // 引擎的启停（可能耗时数百毫秒到数秒）放到后台线程执行，
    // 避免阻塞主线程导致窗口“无响应”。
    *state.taskbar_transparent.lock().unwrap() = enabled;
    let snap = snapshot(&state);
    let _ = app.emit("state-updated", snap.clone());

    let ok = tauri::async_runtime::spawn_blocking(move || taskbar::set_transparent(enabled))
        .await
        .unwrap_or(false);

    if !ok {
        // 引擎启动失败：回滚状态（仅当用户没有再次切换时）
        let mut cur = state.taskbar_transparent.lock().unwrap();
        if *cur == enabled {
            *cur = false;
        }
        drop(cur);
        let snap = snapshot(&state);
        let _ = app.emit("state-updated", snap.clone());
    }
    Ok(snapshot(&state))
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
    let snap = snapshot(&state);
    let _ = app.emit("state-updated", snap.clone());
    Ok(snap)
}

#[tauri::command]
fn set_background(
    app: AppHandle,
    state: State<std::sync::Arc<AppState>>,
    settings: background::BackgroundSettings,
) -> Snapshot {
    let settings = settings.clamped();
    *state.background.lock().unwrap() = settings.clone();
    let _ = background::save(&settings);
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
    loop {
        std::thread::sleep(std::time::Duration::from_millis(80));
        if icons::take_hook_event() {
            dlog::write("[poll_loop] take_hook_event=true -> start_toggle");
            start_toggle(&app, &state);
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
    // 以启动文件夹快捷方式的实际存在情况初始化自启动状态
    *state.autostart.lock().unwrap() = autostart::is_enabled();
    // 加载持久化的背景图片设置
    *state.background.lock().unwrap() = background::load();

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
            // 系统托盘（后台驻留入口）
            tray::setup_tray(app)?;
            // 激活时安装钩子 + 启动轮询
            if *state.enabled.lock().unwrap() {
                hooks::start();
            }
            let app_handle = app.handle().clone();
            let st = state.clone();
            std::thread::spawn(move || poll_loop(app_handle, st));
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::Destroyed => {
                cleanup_on_exit(window.app_handle());
            }
            tauri::WindowEvent::CloseRequested { api, .. } => {
                // 关闭行为由设置「关闭到托盘」决定：
                // 开 → 隐藏到托盘继续后台运行；关 → 清理后直接退出
                api.prevent_close();
                let app = window.app_handle();
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
