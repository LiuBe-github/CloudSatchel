//! TranslucentTB 便携引擎（Windows 11 任务栏透明后端）
//!
//! 背景：Win11 任务栏背景由 explorer 内的 XAML 岛绘制，没有公开 API 可改；
//! TranslucentTB 通过向 explorer 注入 TAP 模块把任务栏 XAML 背景设为透明，
//! 是目前唯一稳妥的“背景消失”实现，且 2024.1 起宿主进程退出时 TAP 会自动
//! 恢复任务栏外观。
//!
//! 本模块把官方便携版（2026.1）内嵌进二进制，首次开启时释放到
//! `%LOCALAPPDATA%\AsYouWishToolBox\taskbar-engine`，配置为：
//! - desktop_appearance：accent=clear + color=#00000000（全透明、隐藏任务栏线条）
//! - hide_tray=true（隐藏托盘图标）
//! - disable_saving=true（引擎不写自己的设置）
//!
//! 关闭功能/退出应用时结束引擎进程；启动自检兜底清理异常退出遗留的引擎实例。

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
        .join("AsYouWishToolBox")
        .join("taskbar-engine")
}

/// 释放内嵌的引擎文件（已存在且大小一致则跳过），并总是重写我们的配置
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
        if !up_to_date && fs::write(&path, data).is_err() {
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

/// 删除我们自己释放的引擎目录（先验证路径落在 LOCALAPPDATA 下）
fn remove_engine_dir() {
    let dir = engine_dir();
    let base = local_app_data();
    if dir.starts_with(&base)
        && dir
            .file_name()
            .map(|n| n == "taskbar-engine")
            .unwrap_or(false)
    {
        let _ = fs::remove_dir_all(&dir);
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

/// 停止引擎（恢复任务栏），并清理残留实例与释放的引擎文件
pub fn stop() {
    if let Some(pid) = ENGINE_PID.lock().unwrap().take() {
        if is_process_alive(pid) {
            terminate_pid(pid);
        }
    }
    terminate_engine_processes();
    remove_engine_dir();
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
