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

use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT};
use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowW, GetClassNameW, GetForegroundWindow, GetWindowLongW,
    GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    IsZoomed, GWL_EXSTYLE, WS_EX_TOOLWINDOW,
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

/// DWM cloaked 窗口判定：挂起/后台的 UWP 应用、被虚拟桌面隐藏的窗口等
/// 常保留整屏尺寸的“隐形”窗口（IsWindowVisible 仍返回 true）。这类窗口
/// 必须从全屏判定中排除，否则任何含一个挂起 UWP 应用的机器上
/// `any_fullscreen_now` 会永久为 true → 任务栏透明开关“无反应”。
fn is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked: u32 = 0;
    unsafe {
        let hr = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED as u32,
            &mut cloaked as *mut u32 as *mut core::ffi::c_void,
            size_of::<u32>() as u32,
        );
        hr == 0 && cloaked != 0
    }
}

/// 当前前台窗口是否处于真全屏状态
pub fn is_fullscreen_now() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() || IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
            return false;
        }
        // 挂起的 UWP 等 cloaked 窗口不算全屏
        if is_cloaked(hwnd) {
            return false;
        }

        let cls = window_class(hwnd);
        if matches!(
            cls.as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
        ) {
            return false;
        }
        // 工具窗口（云笈的音频面板/AI 小窗/翻译窗等）不可能成为全屏应用
        if GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW != 0 {
            return false;
        }
        // 云笈自身窗口（最大化时也不应被当作全屏）
        if window_title(hwnd) == "云笈" {
            return false;
        }
        // 系统输入/体验宿主（TextInputHost 等）不算全屏应用
        if is_system_app_window(hwnd) {
            return false;
        }

        window_covers_monitor(hwnd)
    }
}

/// 进程可执行文件完整路径（诊断用；失败返回空串）
fn process_image_path(pid: u32) -> String {
    if pid == 0 {
        return String::new();
    }
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return String::new();
        }
        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
        CloseHandle(handle);
        if ok == 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buf[..size as usize])
    }
}

/// 窗口是否属于 Windows 系统应用宿主（SystemApps 目录）。
///
/// Windows 11 的输入体验（TextInputHost.exe，触摸键盘 / 语音输入等）常驻一个
/// 覆盖整个显示器的 CoreWindow（标题「Windows 输入体验」），按几何判据会被当成
/// “全屏应用”，导致任务栏在全屏退出后不再恢复透明——这类系统窗口必须排除。
fn is_system_app_window(hwnd: HWND) -> bool {
    let mut pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut pid);
    }
    if pid == 0 {
        return false;
    }
    let path = process_image_path(pid);
    if path.is_empty() {
        return false;
    }
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let sysapps = format!("{}\\SystemApps", windir.trim_end_matches('\\'));
    // 用 starts_with（字符安全）而不是字节切片：Windows 路径可能含非 ASCII
    // （如安装目录「云笈」），按 sysapps 字节长度切片可能落在多字节字符中间，
    // 触发 "byte index is not a char boundary" panic → release panic=abort 闪退。
    path.to_lowercase().starts_with(&sysapps.to_lowercase())
}

/// 窗口矩形是否覆盖其所在显示器（真全屏判据）。
///
/// 两个条件同时满足才视为全屏：
/// 1. 覆盖面积比 ≥ FULLSCREEN_RATIO（容错 DPI 缩放下 1~2px 偏移）；
/// 2. 窗口矩形覆盖显示器的四条边（容差 8px 与边长 0.5% 取大者）——最大化窗口
///    不盖任务栏条带，必然至少有一条边差一个任务栏高度，因此不会误判；而纯
///    面积比判据在任务栏较薄 / 自动隐藏 / 多显示器布局下会把最大化窗口误判为
///    全屏（曾表现为：全屏退出后任务栏不再恢复透明）。
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
        let mw = (m.right - m.left).max(1);
        let mh = (m.bottom - m.top).max(1);
        let iw = wr.right.min(m.right) - wr.left.max(m.left);
        let ih = wr.bottom.min(m.bottom) - wr.top.max(m.top);
        if iw <= 0 || ih <= 0 {
            return false;
        }
        // 条件 1：覆盖面积比
        let ratio = (iw as i64 * ih as i64) as f64 / (mw as i64 * mh as i64) as f64;
        if ratio < FULLSCREEN_RATIO {
            return false;
        }
        // 条件 2：四条边都覆盖（排除最大化窗口——其底边会留出任务栏条带）
        let tol_x = ((mw / 200).max(8)) as i32;
        let tol_y = ((mh / 200).max(8)) as i32;
        wr.left <= m.left + tol_x
            && wr.right >= m.right - tol_x
            && wr.top <= m.top + tol_y
            && wr.bottom >= m.bottom - tol_y
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
        // 最小化的窗口不算（GetWindowRect 返回的是最小化位置，且语义上不占屏）
        if IsIconic(hwnd) != 0 {
            return 1;
        }
        // 挂起的 UWP / 被虚拟桌面隐藏的 cloaked 窗口不算（见 is_cloaked 注释）
        if is_cloaked(hwnd) {
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
        // 系统输入/体验宿主（TextInputHost 等）常驻全屏 CoreWindow，不算全屏应用。
        // 只对几何上已是全屏的窗口做进程检查，避免每次枚举都 OpenProcess。
        if window_covers_monitor(hwnd) && !is_system_app_window(hwnd) {
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

/// 诊断：描述所有“几何上覆盖显示器”的可见窗口（含被排除的系统窗口）。
/// 用于在用户机器上定位“任务栏透明/音频面板被全屏抑制”的肇事窗口：
/// 日志里出现 class/title/exe 后即可知道是远程桌面（mstsc）、云笈自身最大化、
/// 还是某个 OEM 覆盖层窗口在作怪。
pub fn fullscreen_culprits() -> String {
    let mut lines: Vec<String> = Vec::new();
    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> i32 {
        let lines = &mut *(lparam as *mut Vec<String>);
        if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
            return 1;
        }
        if GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW != 0 {
            return 1;
        }
        if !window_covers_monitor(hwnd) {
            return 1;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        lines.push(format!(
            "cls={} title={} pid={pid} exe={}",
            window_class(hwnd),
            window_title(hwnd),
            process_image_path(pid)
        ));
        1
    }
    unsafe {
        EnumWindows(Some(enum_cb), &mut lines as *mut Vec<String> as LPARAM);
    }
    lines.join(" | ")
}
