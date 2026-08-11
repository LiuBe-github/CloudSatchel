//! 系统托盘（后台驻留）
//!
//! 托盘图标随应用常驻：左键单击或菜单「显示主窗口」恢复窗口；
//! 菜单「退出」先恢复桌面图标 / 任务栏，再结束进程。

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

pub const TRAY_ID: &str = "main-tray";
const TRAY_SHOW_ID: &str = "tray-show";
const TRAY_QUIT_ID: &str = "tray-quit";

/// 创建托盘图标与右键菜单
pub fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let handle = app.handle();

    let show = MenuItem::with_id(handle, TRAY_SHOW_ID, "显示主窗口", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(handle)?;
    let quit = MenuItem::with_id(handle, TRAY_QUIT_ID, "退出", true, None::<&str>)?;
    let menu = Menu::with_items(handle, &[&show, &separator, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(
            app.default_window_icon()
                .expect("missing default window icon")
                .clone(),
        )
        .tooltip("如意工具箱")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            TRAY_SHOW_ID => show_main_window(app),
            TRAY_QUIT_ID => crate::cleanup_and_quit(app),
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

/// 显示并聚焦主窗口（若隐藏 / 最小化则先恢复）
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
