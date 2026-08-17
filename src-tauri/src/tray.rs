//! 系统托盘（后台驻留 + 功能快捷开关）
//!
//! 托盘图标随应用常驻：左键单击恢复窗口；右键菜单包含 4 个功能快捷开关
//! （双击隐藏桌面图标 / 透明任务栏 / 自动隐藏任务栏 / 隐私操作），带勾选标记
//! （√ = 已开启），点击即切换，与主界面开关双向实时同步（FR-06 扩展，v0.12.0）。
//! 菜单「显示主窗口」恢复窗口；「退出」先恢复桌面图标 / 任务栏，再结束进程。

use std::sync::Mutex;

use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

pub const TRAY_ID: &str = "main-tray";
const TRAY_SHOW_ID: &str = "tray-show";
const TRAY_QUIT_ID: &str = "tray-quit";
const TRAY_TOGGLE_ICONS: &str = "tray-toggle-icons";
const TRAY_TOGGLE_TRANSPARENT: &str = "tray-toggle-transparent";
const TRAY_TOGGLE_AUTOHIDE: &str = "tray-toggle-autohide";
const TRAY_TOGGLE_PRIVACY: &str = "tray-toggle-privacy";

/// 4 个功能开关菜单项缓存（勾选状态由 update_checks 双向同步）
static TOGGLE_ITEMS: Mutex<Option<Vec<CheckMenuItem<tauri::Wry>>>> = Mutex::new(None);

/// 创建托盘图标与右键菜单
pub fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let handle = app.handle();

    let toggles = [
        CheckMenuItem::with_id(handle, TRAY_TOGGLE_ICONS, "双击隐藏桌面图标", true, false, None::<&str>)?,
        CheckMenuItem::with_id(handle, TRAY_TOGGLE_TRANSPARENT, "透明任务栏", true, false, None::<&str>)?,
        CheckMenuItem::with_id(handle, TRAY_TOGGLE_AUTOHIDE, "自动隐藏任务栏", true, false, None::<&str>)?,
        CheckMenuItem::with_id(handle, TRAY_TOGGLE_PRIVACY, "隐私操作", true, false, None::<&str>)?,
    ];
    *TOGGLE_ITEMS.lock().unwrap() = Some(toggles.to_vec());

    let separator = PredefinedMenuItem::separator(handle)?;
    let show = MenuItem::with_id(handle, TRAY_SHOW_ID, "显示主窗口", true, None::<&str>)?;
    let separator2 = PredefinedMenuItem::separator(handle)?;
    let quit = MenuItem::with_id(handle, TRAY_QUIT_ID, "退出", true, None::<&str>)?;
    let menu = Menu::with_items(
        handle,
        &[
            &toggles[0],
            &toggles[1],
            &toggles[2],
            &toggles[3],
            &separator,
            &show,
            &separator2,
            &quit,
        ],
    )?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(
            app.default_window_icon()
                .expect("missing default window icon")
                .clone(),
        )
        .tooltip("云笈")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            TRAY_SHOW_ID => show_main_window(app),
            TRAY_QUIT_ID => crate::cleanup_and_quit(app),
            TRAY_TOGGLE_ICONS => {
                let state = app.state::<std::sync::Arc<crate::AppState>>();
                let next = !*state.enabled.lock().unwrap();
                crate::set_enabled(app.clone(), state.clone(), next);
            }
            TRAY_TOGGLE_TRANSPARENT => {
                let state = app.state::<std::sync::Arc<crate::AppState>>();
                let next = !*state.taskbar_transparent.lock().unwrap();
                crate::set_taskbar_transparent(app.clone(), state.clone(), next);
            }
            TRAY_TOGGLE_AUTOHIDE => {
                let state = app.state::<std::sync::Arc<crate::AppState>>();
                let next = !*state.autohide_enabled.lock().unwrap();
                crate::set_autohide_enabled(app.clone(), state.clone(), next);
            }
            TRAY_TOGGLE_PRIVACY => {
                let state = app.state::<std::sync::Arc<crate::AppState>>();
                let next = !*state.privacy_enabled.lock().unwrap();
                crate::set_privacy_enabled(app.clone(), state.clone(), next);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Windows 上左键单击托盘图标直接恢复主窗口
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

/// 同步 4 个功能开关的勾选状态（开关变化后由 lib.rs 的 persist 统一调用，
/// 主界面 / 托盘任一侧切换，另一侧立即一致）
pub fn update_checks(enabled: bool, transparent: bool, autohide: bool, privacy: bool) {
    if let Some(items) = TOGGLE_ITEMS.lock().unwrap().as_ref() {
        let _ = items[0].set_checked(enabled);
        let _ = items[1].set_checked(transparent);
        let _ = items[2].set_checked(autohide);
        let _ = items[3].set_checked(privacy);
    }
}

/// 显示并聚焦主窗口（若隐藏 / 最小化则先恢复）
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
