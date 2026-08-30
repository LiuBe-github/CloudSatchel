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
use std::sync::atomic::{AtomicBool, Ordering};

use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowW, GetClassNameW, GetForegroundWindow, GetWindowLongW,
    GetWindowRect, GetWindowTextW, IsWindowVisible, IsZoomed, GWL_EXSTYLE, WS_EX_TOOLWINDOW,
};

/// 全屏覆盖面积比阈值（≥97% 视为全屏，见 is_fullscreen_now）。
/// 最大化窗口只覆盖工作区（约 95.6%），不会误判；97% 可覆盖无边框全屏
/// 与 DPI 缩放下 1~2px 偏移导致的面积损失。
const FULLSCREEN_RATIO: f64 = 0.97;

/// 前台第三方窗口当前是否最大化（任务栏透明暂停用，独立于真全屏）
static FOREGROUND_MAXIMIZED: AtomicBool = AtomicBool::new(false);
/// 是否存在任意全屏窗口（无论是否前台，任务栏透明暂停用）
static ANY_FULLSCREEN: AtomicBool = AtomicBool::new(false);

const MAX_LEN: usize = 256;

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

/// 记录“前台第三方窗口最大化”状态（由 poll_loop 每 320ms 更新）
pub fn set_foreground_maximized(maximized: bool) {
    FOREGROUND_MAXIMIZED.store(maximized, Ordering::SeqCst);
}

/// 前台第三方窗口当前是否最大化（任务栏透明暂停用；真全屏见 is_fullscreen_now）
pub fn is_foreground_maximized() -> bool {
    FOREGROUND_MAXIMIZED.load(Ordering::SeqCst)
}

/// 记录“是否存在任意全屏窗口”（由 poll_loop 每 320ms 更新）
pub fn set_any_fullscreen(fullscreen: bool) {
    ANY_FULLSCREEN.store(fullscreen, Ordering::SeqCst);
}

/// 是否存在任意全屏窗口（任务栏透明暂停用）
pub fn is_any_fullscreen() -> bool {
    ANY_FULLSCREEN.load(Ordering::SeqCst)
}

/// 当前前台窗口是否处于真全屏状态
pub fn is_fullscreen_now() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() || IsWindowVisible(hwnd) == 0 {
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

        window_covers_monitor(hwnd)
    }
}

/// 窗口矩形是否覆盖其所在显示器 ≥ FULLSCREEN_RATIO（真全屏判据）
fn window_covers_monitor(hwnd: HWND) -> bool {
    unsafe {
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
        // 覆盖面积比判定（鲁棒）：窗口与显示器相交面积 ≥ 97% 即视为全屏。
        // 相比逐边容差更可靠：无边框全屏 / DPI 缩放下 1~2px 偏移不影响；
        // 最大化窗口只覆盖工作区（不盖任务栏区域），面积比约 95% 不会误判。
        let mw = (m.right - m.left).max(1);
        let mh = (m.bottom - m.top).max(1);
        let iw = wr.right.min(m.right) - wr.left.max(m.left);
        let ih = wr.bottom.min(m.bottom) - wr.top.max(m.top);
        if iw <= 0 || ih <= 0 {
            return false;
        }
        let ratio = (iw as i64 * ih as i64) as f64 / (mw as i64 * mh as i64) as f64;
        ratio >= FULLSCREEN_RATIO
    }
}

/// 是否存在任意可见顶层窗口处于真全屏（无论是否前台）。
/// 场景：A 应用全屏中，用户点击未全屏的 B 到前台——B 非全屏但 A 仍在全屏，
/// 任务栏应保持不透明。跳过桌面/任务栏、工具窗口与云笈自身窗口。
pub fn any_fullscreen_now() -> bool {
    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> i32 {
        let found = &mut *(lparam as *mut bool);
        if *found {
            return 0; // 已找到，停止枚举
        }
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        // 跳过工具窗口（托盘、云笈辅助窗等）
        if GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW != 0 {
            return 1;
        }
        let cls = window_class(hwnd);
        if matches!(
            cls.as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
        ) {
            return 1;
        }
        // 云笈自身窗口不算
        if window_title(hwnd) == "云笈" {
            return 1;
        }
        if window_covers_monitor(hwnd) {
            *found = true;
            return 0;
        }
        1
    }
    let mut found = false;
    unsafe {
        EnumWindows(Some(enum_cb), &mut found as *mut bool as LPARAM);
    }
    found
}

/// 有效全屏判断（供空闲类功能复用）：云笈自身前台且最大化 → 视为“全屏”；
/// 否则按前台第三方窗口是否覆盖整个显示器判断（同主轮询逻辑）。
pub fn is_fullscreen_effective() -> bool {
    let own = find_own_window();
    let fg = foreground_hwnd();
    if fg != 0 && fg == own {
        is_zoomed(fg)
    } else {
        is_fullscreen_now()
    }
}
