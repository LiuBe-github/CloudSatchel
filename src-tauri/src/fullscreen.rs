//! 全屏窗口检测
//!
//! 用途：当某个前台应用进入“真全屏”状态（窗口覆盖整个显示器）时，
//! 通知上层暂时取消任务栏透明；全屏退出后再恢复透明。
//!
//! 判定规则：
//! - 取当前前台窗口 `GetForegroundWindow`；
//! - 排除桌面/任务栏体系（Progman / WorkerW / Shell_TrayWnd / Shell_SecondaryTrayWnd）
//!   以及云笈自己的窗口，避免误判；
//! - 用 `MonitorFromWindow` 找到窗口所在显示器，取 `rcMonitor`（完整显示器矩形），
//!   若窗口矩形与显示器矩形几乎重合（容差 2px）即视为全屏。
//!   这里刻意对比 `rcMonitor` 而不是 `rcWork`：最大化窗口只覆盖工作区（含任务栏区域），
//!   真全屏（F11 / 视频全屏 / 游戏）才覆盖整个显示器，从而把“最大化”与“全屏”区分开。

#![allow(non_snake_case)]

use std::mem::size_of;

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowTextW, IsZoomed,
};

const MAX_LEN: usize = 256;
const TOLERANCE_PX: i32 = 2;

fn window_class(hwnd: HWND) -> String {
    if hwnd.is_null() {
        return String::new();
    }
    let mut buf = vec![0u16; MAX_LEN];
    unsafe {
        let n = GetClassNameW(hwnd, buf.as_mut_ptr(), MAX_LEN as i32);
        if n <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buf[..n as usize])
    }
}

fn window_title(hwnd: HWND) -> String {
    if hwnd.is_null() {
        return String::new();
    }
    let mut buf = vec![0u16; MAX_LEN];
    unsafe {
        let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), MAX_LEN as i32);
        if n <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buf[..n as usize])
    }
}

/// 前台窗口句柄（0 = 无前台窗口）
pub fn foreground_hwnd() -> isize {
    unsafe { GetForegroundWindow() as isize }
}

/// 云笈主窗口句柄（0 = 未找到）
pub fn find_own_window() -> isize {
    unsafe { FindWindowW(std::ptr::null(), windows_sys::core::w!("云笈")) as isize }
}

/// 窗口是否处于最大化状态（用户把“云笈最大化”视为“全屏”）
pub fn is_zoomed(hwnd: isize) -> bool {
    if hwnd == 0 {
        return false;
    }
    unsafe { IsZoomed(hwnd as HWND) != 0 }
}

/// 当前前台窗口是否处于真全屏状态
pub fn is_fullscreen_now() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return false;
        }

        let cls = window_class(hwnd);
        if matches!(
            cls.as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
        ) {
            return false;
        }
        // 云笈自身窗口（最大化时也不应被当作全屏）
        if window_title(hwnd) == "云笈" {
            return false;
        }

        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            rcWork: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            dwFlags: 0,
        };
        if GetMonitorInfoW(monitor, &mut mi) == 0 {
            return false;
        }

        let mut wr = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if GetWindowRect(hwnd, &mut wr) == 0 {
            return false;
        }

        let m = &mi.rcMonitor;
        wr.left <= m.left + TOLERANCE_PX
            && wr.top <= m.top + TOLERANCE_PX
            && wr.right >= m.right - TOLERANCE_PX
            && wr.bottom >= m.bottom - TOLERANCE_PX
    }
}
