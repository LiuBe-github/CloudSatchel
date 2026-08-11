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
use windows_sys::Win32::Foundation::{HWND, LPARAM};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_DWORD,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GetClassNameW};

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

/// 启动自检：清理旧版注册表实验遗留，并结束异常退出残留的引擎进程
pub fn ensure_restored() {
    unsafe {
        if let Some(hkey) = open_advanced_key() {
            restore_old_registry_backup(hkey);
            RegCloseKey(hkey);
        }
    }
    crate::taskbar_engine::stop();
}
