//! 任务栏透明控制（双后端 + 遗留清理）
//!
//! - Windows 10（build < 22000，经典任务栏）：向 `Shell_TrayWnd` / `Shell_SecondaryTrayWnd`
//!   下发未文档化 API `SetWindowCompositionAttribute` 的 WCA_ACCENT_POLICY，
//!   ACCENT_ENABLE_TRANSPARENTGRADIENT + 全透明渐变 → 背景消失。
//! - Windows 11（build >= 22000，XAML 任务栏）：Accent API 只能改底色，
//!   改由内置 TranslucentTB 便携引擎实现（见 `taskbar_engine.rs`）。
//! - `ensure_restored()`：启动自检，清理旧版“注册表 OLED 开关”实验遗留的
//!   备份值，并兜底结束异常退出残留的引擎进程。
#![allow(non_snake_case)]

use std::mem::size_of;

use windows_sys::core::{s, w};
use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_DWORD,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowW, GetClassNameW, GetWindowRect, IsWindowVisible, SetWindowPos,
    ShowWindow, SWP_NOACTIVATE, SWP_NOZORDER, SW_HIDE, SW_SHOW,
};

const ACCENT_DISABLED: u32 = 0;
const ACCENT_ENABLE_TRANSPARENTGRADIENT: u32 = 2;
const WCA_ACCENT_POLICY: u32 = 19;
const MAX_CLASS_LEN: usize = 256;

// ---- 旧版注册表实验的遗留清理 ----
const VALUE_TRANSPARENT: windows_sys::core::PCWSTR = w!("UseOLEDTaskbarTransparency");
const VALUE_BACKUP: windows_sys::core::PCWSTR = w!("DesktopToolsTaskbarBackup");
/// 备份哨兵：原值本来不存在（恢复时应删除而不是写回）
const BACKUP_NOT_EXISTED: u32 = u32::MAX;

#[repr(C)]
struct AccentPolicy {
    accent_state: u32,
    accent_flags: u32,
    gradient_color: u32,
    animation_id: u32,
}

#[repr(C)]
struct WindowCompositionAttribData {
    attribute: u32,
    data: *mut std::ffi::c_void,
    size_of_data: usize,
}

type SetWindowCompositionAttributeFn =
    unsafe extern "system" fn(HWND, *mut WindowCompositionAttribData) -> i32;

#[repr(C)]
struct RtlOsVersionInfoW {
    dw_os_version_info_size: u32,
    dw_major_version: u32,
    dw_minor_version: u32,
    dw_build_number: u32,
    dw_platform_id: u32,
    sz_csd_version: [u16; 128],
}

type RtlGetVersionFn = unsafe extern "system" fn(*mut RtlOsVersionInfoW) -> i32;

/// Windows 11（含 25H2/26200）：任务栏是 XAML 岛，build >= 22000
fn is_windows_11() -> bool {
    unsafe {
        let ntdll = LoadLibraryW(w!("ntdll.dll"));
        if ntdll.is_null() {
            return false;
        }
        let Some(proc) = GetProcAddress(ntdll, s!("RtlGetVersion")) else {
            return false;
        };
        let func: RtlGetVersionFn = std::mem::transmute(proc);
        let mut info = RtlOsVersionInfoW {
            dw_os_version_info_size: size_of::<RtlOsVersionInfoW>() as u32,
            dw_major_version: 0,
            dw_minor_version: 0,
            dw_build_number: 0,
            dw_platform_id: 0,
            sz_csd_version: [0; 128],
        };
        func(&mut info) == 0 && info.dw_build_number >= 22000
    }
}

// ---------------------------------------------------------------------------
// Windows 10 后端：SetWindowCompositionAttribute
// ---------------------------------------------------------------------------

/// 动态加载未文档化 API 并对单个窗口下发强调色策略
unsafe fn apply_accent(hwnd: HWND, enabled: bool) -> bool {
    if hwnd.is_null() {
        return false;
    }
    let user32 = LoadLibraryW(w!("user32.dll"));
    if user32.is_null() {
        return false;
    }
    let Some(proc) = GetProcAddress(user32, s!("SetWindowCompositionAttribute")) else {
        return false;
    };
    let func: SetWindowCompositionAttributeFn = std::mem::transmute(proc);

    let policy = AccentPolicy {
        accent_state: if enabled {
            ACCENT_ENABLE_TRANSPARENTGRADIENT
        } else {
            ACCENT_DISABLED
        },
        accent_flags: 0,
        gradient_color: 0, // 全透明渐变 → 背景消失
        animation_id: 0,
    };
    let mut data = WindowCompositionAttribData {
        attribute: WCA_ACCENT_POLICY,
        data: &policy as *const AccentPolicy as *mut std::ffi::c_void,
        size_of_data: size_of::<AccentPolicy>(),
    };
    func(hwnd, &mut data) != 0
}

unsafe extern "system" fn enum_apply(hwnd: HWND, lparam: LPARAM) -> i32 {
    let enabled = lparam != 0;
    let mut buf = vec![0u16; MAX_CLASS_LEN];
    let n = GetClassNameW(hwnd, buf.as_mut_ptr(), MAX_CLASS_LEN as i32);
    if n > 0 {
        let cls = String::from_utf16_lossy(&buf[..n as usize]);
        if cls == "Shell_TrayWnd" || cls == "Shell_SecondaryTrayWnd" {
            apply_accent(hwnd, enabled);
        }
    }
    1 // 继续枚举
}

fn apply_accent_to_taskbars(enabled: bool) {
    unsafe {
        EnumWindows(Some(enum_apply), enabled as isize as LPARAM);
    }
}

// ---------------------------------------------------------------------------
// 旧版注册表实验的遗留清理
// ---------------------------------------------------------------------------

unsafe fn open_advanced_key() -> Option<HKEY> {
    let mut key: HKEY = std::ptr::null_mut();
    let status = RegOpenKeyExW(
        HKEY_CURRENT_USER,
        w!("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced"),
        0,
        KEY_QUERY_VALUE | KEY_SET_VALUE,
        &mut key,
    );
    (status == 0).then_some(key)
}

unsafe fn read_dword(hkey: HKEY, name: windows_sys::core::PCWSTR) -> Option<u32> {
    let mut ty = 0u32;
    let mut value = 0u32;
    let mut size = size_of::<u32>() as u32;
    let status = RegQueryValueExW(
        hkey,
        name,
        std::ptr::null(),
        &mut ty,
        &mut value as *mut u32 as *mut u8,
        &mut size,
    );
    if status == 0 && ty == REG_DWORD && size == size_of::<u32>() as u32 {
        Some(value)
    } else {
        None
    }
}

unsafe fn write_dword(hkey: HKEY, name: windows_sys::core::PCWSTR, value: u32) -> bool {
    RegSetValueExW(
        hkey,
        name,
        0,
        REG_DWORD,
        &value as *const u32 as *const u8,
        size_of::<u32>() as u32,
    ) == 0
}

unsafe fn delete_value(hkey: HKEY, name: windows_sys::core::PCWSTR) {
    let _ = RegDeleteValueW(hkey, name);
}

/// 旧版实验写了 `UseOLEDTaskbarTransparency` 并留了备份：按备份恢复并删除备份
unsafe fn restore_old_registry_backup(hkey: HKEY) {
    let Some(backup) = read_dword(hkey, VALUE_BACKUP) else {
        return; // 没有备份，不擅自动用户自己的设置
    };
    if backup == BACKUP_NOT_EXISTED {
        delete_value(hkey, VALUE_TRANSPARENT);
    } else {
        write_dword(hkey, VALUE_TRANSPARENT, backup);
    }
    delete_value(hkey, VALUE_BACKUP);
}

// ---------------------------------------------------------------------------
// 公共接口
// ---------------------------------------------------------------------------

/// 设置任务栏透明：true=背景消失，false=恢复系统默认（无动画）。
/// 返回是否成功（Win11 引擎启动失败时返回 false）。
pub fn set_transparent(enabled: bool) -> bool {
    if is_windows_11() {
        crate::taskbar_engine::set(enabled)
    } else {
        apply_accent_to_taskbars(enabled);
        true
    }
}

/// 恢复系统默认任务栏（供关闭功能 / 退出时调用）
pub fn restore() {
    let _ = set_transparent(false);
}

// ---------------------------------------------------------------------------
// 任务栏自动隐藏（FR-14 / FR-13 步骤③ 共用）
// ---------------------------------------------------------------------------
//
// 注：早期实现使用 SHAppBarMessage(ABM_SETSTATE, ABS_AUTOHIDE)。实测 Windows 11
// 25H2（build 26200）上该调用只改变 AppBar 状态位，任务栏视觉上不隐藏，
// 因此改为直接控制任务栏窗口（滑动动画 + SW_HIDE/SW_SHOW），边缘弹出/再隐藏
// 由 privacy.rs 的空闲轮询线程检测鼠标位置实现。不写注册表。

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

fn taskbar_hwnd() -> HWND {
    unsafe { FindWindowW(w!("Shell_TrayWnd"), std::ptr::null()) }
}

unsafe extern "system" fn enum_taskbars(hwnd: HWND, lparam: LPARAM) -> i32 {
    let list = &mut *(lparam as *mut Vec<HWND>);
    let mut buf = [0u16; 128];
    let n = GetClassNameW(hwnd, buf.as_mut_ptr(), 128);
    if n > 0 {
        let cls = String::from_utf16_lossy(&buf[..n as usize]);
        if cls == "Shell_TrayWnd" || cls == "Shell_SecondaryTrayWnd" {
            list.push(hwnd);
        }
    }
    1
}

/// 所有任务栏窗口（主任务栏 + 多显示器副任务栏）
pub fn taskbar_windows() -> Vec<HWND> {
    let mut list: Vec<HWND> = Vec::new();
    unsafe {
        EnumWindows(Some(enum_taskbars), &mut list as *mut Vec<HWND> as LPARAM);
    }
    list
}

/// 任务栏“期望”的隐藏状态（由 set_autohide 写入，动画线程按最新意图执行）
static AUTOHIDE_EXPECTED: AtomicBool = AtomicBool::new(false);
/// 动画线程是否正在运行
static AUTOHIDE_ANIMATING: AtomicBool = AtomicBool::new(false);

/// 任务栏当前是否处于隐藏状态（窗口不可见，或已完全滑出所在显示器底部）
pub fn is_autohide() -> bool {
    let hwnd = taskbar_hwnd();
    if hwnd.is_null() {
        return false;
    }
    unsafe {
        if IsWindowVisible(hwnd) == 0 {
            return true;
        }
        let mut r = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if GetWindowRect(hwnd, &mut r) == 0 {
            return false;
        }
        let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
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
        if GetMonitorInfoW(mon, &mut mi) == 0 {
            return false;
        }
        // 窗口整体在所在显示器底部下方 → 视为已隐藏
        r.top >= mi.rcMonitor.bottom
    }
}

/// 任务栏动画是否进行中（动画期间调用方应暂缓状态判断）
pub fn is_animating() -> bool {
    AUTOHIDE_ANIMATING.load(Ordering::SeqCst)
}

/// 动画式隐藏 / 恢复显示所有任务栏窗口（异步，不阻塞调用线程）。
///
/// 隐藏：任务栏从所在显示器底部平滑滑出屏幕（约 144ms），随后 SW_HIDE；
/// 显示：SW_SHOW 后平滑滑回原位。动画期间的新请求按最新意图在完成后继续执行。
/// 不写注册表、不改系统「自动隐藏任务栏」设置开关。
pub fn set_autohide(enabled: bool) -> bool {
    if taskbar_windows().is_empty() {
        return false;
    }
    AUTOHIDE_EXPECTED.store(enabled, Ordering::SeqCst);
    if AUTOHIDE_ANIMATING.swap(true, Ordering::SeqCst) {
        return true; // 动画线程会在当前动画完成后按最新意图继续执行
    }
    std::thread::spawn(|| {
        loop {
            let hide = AUTOHIDE_EXPECTED.load(Ordering::SeqCst);
            if hide == is_autohide() {
                // 二次确认：期间可能又收到新请求
                if AUTOHIDE_EXPECTED.load(Ordering::SeqCst) == is_autohide() {
                    break;
                }
            } else {
                animate_taskbars(hide);
            }
        }
        AUTOHIDE_ANIMATING.store(false, Ordering::SeqCst);
    });
    true
}

struct AnimTarget {
    hwnd: HWND,
    x: i32,
    y0: i32,
    y1: i32,
    w: i32,
    h: i32,
}

/// 单轮滑动动画：hide=true 滑出所在显示器底部，false 滑回原位。
fn animate_taskbars(hide: bool) {
    let mut targets: Vec<AnimTarget> = Vec::new();
    unsafe {
        for hwnd in taskbar_windows() {
            let mut r = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if GetWindowRect(hwnd, &mut r) == 0 {
                continue;
            }
            let w = r.right - r.left;
            let h = r.bottom - r.top;
            // 所在显示器的底部（隐藏目标：整体滑到屏幕下方）
            let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
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
            let bottom = if GetMonitorInfoW(mon, &mut mi) != 0 {
                mi.rcMonitor.bottom
            } else {
                r.bottom
            };
            let (y0, y1) = if hide { (r.top, bottom) } else { (r.top, bottom - h) };
            // 显示前确保窗口可见（从屏外或隐藏状态恢复）
            if !hide && IsWindowVisible(hwnd) == 0 {
                ShowWindow(hwnd, SW_SHOW);
            }
            targets.push(AnimTarget {
                hwnd,
                x: r.left,
                y0,
                y1,
                w,
                h,
            });
        }

        // 步进动画（12 步 × 12ms ≈ 144ms）
        const STEPS: i32 = 12;
        for step in 0..=STEPS {
            for t in &targets {
                let y = t.y0 + (t.y1 - t.y0) * step / STEPS;
                SetWindowPos(
                    t.hwnd,
                    std::ptr::null_mut(),
                    t.x,
                    y,
                    t.w,
                    t.h,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            std::thread::sleep(Duration::from_millis(12));
        }

        // 收尾：隐藏 → SW_HIDE；显示 → 已滑回原位
        if hide {
            for t in &targets {
                ShowWindow(t.hwnd, SW_HIDE);
            }
        }
    }
}

/// 启动自检：清理旧版注册表实验遗留、结束异常退出残留的引擎进程，
/// 并恢复异常退出时残留的隐藏任务栏（仅恢复被隐藏/滑出屏幕的任务栏窗口，
/// 不影响系统自身的「自动隐藏任务栏」设置——该设置下窗口仍为可见且在原位）。
pub fn ensure_restored() {
    unsafe {
        if let Some(hkey) = open_advanced_key() {
            restore_old_registry_backup(hkey);
            RegCloseKey(hkey);
        }
        for hwnd in taskbar_windows() {
            if IsWindowVisible(hwnd) == 0 {
                ShowWindow(hwnd, SW_SHOW);
            }
            // 滑出屏幕残留（动画中途强杀）：移回所在显示器底部原位
            let mut r = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if GetWindowRect(hwnd, &mut r) == 0 {
                continue;
            }
            let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
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
            if GetMonitorInfoW(mon, &mut mi) != 0 && r.top >= mi.rcMonitor.bottom {
                let h = r.bottom - r.top;
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    r.left,
                    mi.rcMonitor.bottom - h,
                    r.right - r.left,
                    h,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }
    }
    crate::taskbar_engine::stop();
}
