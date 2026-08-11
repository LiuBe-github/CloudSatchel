//! TranslucentTB 便携引擎（Windows 11 任务栏透明后端）
//!
//! 背景：Win11 任务栏背景由 explorer 内的 XAML 岛绘制，没有公开 API 可改；
//! TranslucentTB 通过向 explorer 注入 TAP 模块把任务栏 XAML 背景设为透明，
//! 是目前唯一稳妥的“背景消失”实现，且 2024.1 起宿主进程退出时 TAP 会自动
//! 恢复任务栏外观。
//!
//! 本模块把官方便携版（2026.1）内嵌进二进制，首次开启时释放到
//! `%LOCALAPPDATA%\CloudSatchel\taskbar-engine`，配置为：
//! - desktop_appearance：accent=clear + color=#00000000（全透明、隐藏任务栏线条）
//! - hide_tray=true（隐藏托盘图标）
//! - disable_saving=true（引擎不写自己的设置）
//!
//! 关闭功能/退出应用时结束引擎进程；启动自检兜底清理异常退出遗留的引擎实例。
//! 引擎文件保留在 `%LOCALAPPDATA%\CloudSatchel\taskbar-engine`（不随关闭删除）：
//! 删除后重新释放会让文件时间戳变新，TranslucentTB 复制 TAP 到临时目录时
//! 会因目标被 explorer 锁定而失败，误弹“已更新，请重启 Windows”对话框。

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};

const SETTINGS_JSON: &str = r##"{
  "desktop_appearance": {
    "accent": "clear",
    "color": "#00000000",
    "show_peek": false,
    "show_line": false
  },
  "hide_tray": true,
  "disable_saving": true,
  "verbosity": "off"
}"##;

/// 本次会话启动的引擎进程 PID（用于关闭时精准结束，避免误杀用户自己的 TranslucentTB）
static ENGINE_PID: Mutex<Option<u32>> = Mutex::new(None);

const ENGINE_FILES: &[(&str, &[u8])] = &[
    (
        "TranslucentTB.exe",
        include_bytes!("../resources/taskbar-engine/TranslucentTB.exe"),
    ),
    (
        "ExplorerTAP.dll",
        include_bytes!("../resources/taskbar-engine/ExplorerTAP.dll"),
    ),
    (
        "ExplorerHooks.dll",
        include_bytes!("../resources/taskbar-engine/ExplorerHooks.dll"),
    ),
    (
        "ProgramLog.dll",
        include_bytes!("../resources/taskbar-engine/ProgramLog.dll"),
    ),
    ("Xaml.dll", include_bytes!("../resources/taskbar-engine/Xaml.dll")),
    (
        "resources.pri",
        include_bytes!("../resources/taskbar-engine/resources.pri"),
    ),
    (
        "Assets/SplashScreen.jpeg",
        include_bytes!("../resources/taskbar-engine/Assets/SplashScreen.jpeg"),
    ),
    (
        "Assets/SplashScreen.scale-400.jpeg",
        include_bytes!("../resources/taskbar-engine/Assets/SplashScreen.scale-400.jpeg"),
    ),
    (
        "README.txt",
        include_bytes!("../resources/taskbar-engine/README.txt"),
    ),
];

fn local_app_data() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

fn engine_dir() -> PathBuf {
    local_app_data()
        .join("CloudSatchel")
        .join("taskbar-engine")
}

/// 与 TranslucentTB 的临时副本对齐 DLL 时间戳
///
/// TranslucentTB 每次启动都会把引擎目录中的 DLL 用
/// `copy_file(update_existing)` 复制到 `%TEMP%\TranslucentTB\`，仅当“源比目标新”时复制。
/// Explorer 注入后会把临时 TAP 锁住；若引擎目录文件的修改时间比临时副本新
/// （例如旧版本在关闭时删除引擎目录、开启时重新释放），复制会因共享冲突失败，
/// TranslucentTB 误判为“已更新”，弹出“请重启 Windows”对话框并退出。
/// 这里在内容一致（字节数相同）时把引擎目录文件的时间戳压到不晚于临时副本，
/// 使 update_existing 判定为无需复制；内容不一致（真正的版本更新）则保留新时间戳，
/// 让复制照常发生（此时确实需要重启资源管理器/系统才能生效）。
fn align_dll_timestamps_with_temp(dir: &std::path::Path, name: &str) {
    let path = dir.join(name);
    let Ok(src_meta) = fs::metadata(&path) else {
        return;
    };
    let temp_path = std::env::temp_dir().join("TranslucentTB").join(name);
    let Ok(temp_meta) = fs::metadata(&temp_path) else {
        return;
    };
    if temp_meta.len() != src_meta.len() {
        return; // 内容不同：真·版本更新，保留新时间戳
    }
    let (Ok(src_t), Ok(temp_t)) = (src_meta.modified(), temp_meta.modified()) else {
        return;
    };
    if src_t > temp_t {
        if let Ok(f) = fs::OpenOptions::new().write(true).open(&path) {
            let _ = f.set_modified(temp_t);
        }
    }
}

/// 释放内嵌的引擎文件（已存在且大小一致则跳过），并总是重写我们的配置。
/// 注意：不要删除已释放的文件 —— 删除后下次会重新写入新时间戳，进而触发
/// TranslucentTB 的“已更新，请重启 Windows”误弹窗（见 align_dll_timestamps_with_temp）。
fn release_engine() -> bool {
    let dir = engine_dir();
    if fs::create_dir_all(&dir).is_err() {
        return false;
    }
    for (name, data) in ENGINE_FILES {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            if fs::create_dir_all(parent).is_err() {
                return false;
            }
        }
        let up_to_date = fs::metadata(&path)
            .map(|m| m.len() == data.len() as u64)
            .unwrap_or(false);
        if up_to_date {
            align_dll_timestamps_with_temp(&dir, name);
        } else if fs::write(&path, data).is_err() {
            return false;
        }
    }
    fs::write(dir.join("settings.json"), SETTINGS_JSON).is_ok()
}

/// 进程是否还活着（STILL_ACTIVE）
fn is_process_alive(pid: u32) -> bool {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        let mut code: u32 = 0;
        let ok = GetExitCodeProcess(handle, &mut code);
        CloseHandle(handle);
        ok != 0 && code == 259 // STILL_ACTIVE
    }
}

fn wait_exit(pid: u32, ms: u64) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(ms);
    while is_process_alive(pid) && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// 先优雅结束（taskkill 默认发 WM_CLOSE），超时再强制结束。
///
/// 注意：`taskkill /PID`（不带 /F）会一直等待目标进程退出后才返回，
/// 如果引擎清理较慢，`Command::status()` 会把调用方阻塞数秒。
/// 因此这里改为 `spawn()` 把 taskkill 放后台，由我们自己按固定超时
/// 轮询进程是否退出，超时立即强杀 —— 总耗时可控（约 1.5~2.5s）。
fn terminate_pid(pid: u32) {
    // 优雅关闭：发 WM_CLOSE 让引擎自行恢复任务栏；不等待 taskkill 返回
    let _ = Command::new("taskkill")
        .arg("/PID")
        .arg(pid.to_string())
        .spawn();
    wait_exit(pid, 1500);
    if is_process_alive(pid) {
        let _ = Command::new("taskkill")
            .arg("/PID")
            .arg(pid.to_string())
            .arg("/F")
            .status();
        wait_exit(pid, 1000);
    }
}

unsafe fn process_image_path(pid: u32) -> Option<PathBuf> {
    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if handle.is_null() {
        return None;
    }
    let mut buf = [0u16; 1024];
    let mut size = buf.len() as u32;
    let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
    CloseHandle(handle);
    if ok == 0 {
        return None;
    }
    Some(PathBuf::from(String::from_utf16_lossy(
        &buf[..size as usize],
    )))
}

/// 结束所有可执行路径为“我们释放的引擎目录”的 TranslucentTB 实例
/// （异常退出后的兜底清理；按完整路径比对，不误伤用户自己安装的 TranslucentTB）
fn terminate_engine_processes() {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return;
        }
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            cntUsage: 0,
            th32ProcessID: 0,
            th32DefaultHeapID: 0,
            th32ModuleID: 0,
            cntThreads: 0,
            th32ParentProcessID: 0,
            pcPriClassBase: 0,
            dwFlags: 0,
            szExeFile: [0; 260],
        };
        let engine_exe = engine_dir().join("TranslucentTB.exe");
        let mut ok = Process32FirstW(snapshot, &mut entry);
        while ok != 0 {
            let name_len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..name_len]);
            if name.eq_ignore_ascii_case("TranslucentTB.exe") {
                if let Some(path) = process_image_path(entry.th32ProcessID) {
                    if path
                        .to_string_lossy()
                        .eq_ignore_ascii_case(&engine_exe.to_string_lossy())
                    {
                        terminate_pid(entry.th32ProcessID);
                    }
                }
            }
            ok = Process32NextW(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
    }
}

/// 启动引擎（开启透明）
pub fn start() -> bool {
    if !release_engine() {
        return false;
    }
    let exe = engine_dir().join("TranslucentTB.exe");
    match Command::new(&exe).current_dir(engine_dir()).spawn() {
        Ok(child) => {
            *ENGINE_PID.lock().unwrap() = Some(child.id());
            true
        }
        Err(_) => false,
    }
}

/// 停止引擎（恢复任务栏），并清理残留实例。
/// 注意：保留已释放的引擎文件（不删除），避免下次开启时因时间戳变新
/// 触发 TranslucentTB 误弹“已更新，请重启 Windows”对话框。
pub fn stop() {
    if let Some(pid) = ENGINE_PID.lock().unwrap().take() {
        if is_process_alive(pid) {
            terminate_pid(pid);
        }
    }
    terminate_engine_processes();
}

/// 开关入口：返回是否成功（开启失败时返回 false）
pub fn set(enabled: bool) -> bool {
    if enabled {
        start()
    } else {
        stop();
        true
    }
}
