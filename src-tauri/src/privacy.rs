//! 隐私操作（FR-13）与任务栏自动隐藏（FR-14）
//!
//! 两个功能共享 `GetLastInputInfo` 空闲检测轮询线程（约 1 秒间隔，轻量不阻塞主线程）：
//!
//! - FR-13 隐私操作：空闲超过设定时间后，按顺序执行
//!   「最小化所有窗口（全屏先转窗口化）→ 隐藏桌面图标 → 隐藏任务栏 → 系统静音」；
//!   检测到用户恢复操作（鼠标移动 / 点击 / 键盘输入）后成对还原全部状态。
//!   触发/恢复通过 ACTIVE_LOCK 串行化，恢复前不重复触发；
//!   窗口清单与「本功能执行了隐藏」标记是恢复的唯一依据。
//!
//! - FR-14 任务栏自动隐藏：空闲超时通过 AppBar API（ABS_AUTOHIDE）隐藏任务栏，
//!   隐藏后的边缘弹出 / 移开再隐藏由系统原生行为完成；全屏或云笈最大化期间暂停。
//!   仅改变运行时 AppBar 状态，不写注册表、不改系统「自动隐藏任务栏」设置开关。
//!
//! 纯净性：不写注册表、不联网；退出（stop）时完整恢复所有状态。

#![allow(non_snake_case)]

use std::mem::size_of;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows_sys::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::System::SystemInformation::GetTickCount;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetCursorPos, GetWindow, GetWindowLongW, GetWindowPlacement,
    GetWindowRect, IsIconic, IsWindowVisible, MoveWindow, ShowWindow, GWL_EXSTYLE, GW_OWNER,
    SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SW_SHOWNORMAL, SW_SHOWMAXIMIZED, WS_EX_TOOLWINDOW,
    WINDOWPLACEMENT,
};

use crate::dlog;
use crate::icons;
use crate::taskbar;

/// 空闲时间可设范围：10 秒 ~ 60 分钟（需求 FR-13 / FR-14）
pub const IDLE_CLAMP_MIN: u32 = 10;
pub const IDLE_CLAMP_MAX: u32 = 3600;
/// 轮询 tick：50ms（边缘弹出响应）＋ 每 20 tick（1 秒）做一次空闲检测
const POLL_MS: u64 = 50;
/// 边缘弹出检测时任务栏矩形外扩像素
const EDGE_PADDING: i32 = 4;
/// 鼠标移出任务栏区域后延迟多久重新隐藏（给开始菜单等操作留出时间）
const HIDE_DELAY_MS: u64 = 1500;

// ---------------------------------------------------------------------------
// 配置与运行时状态
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Config {
    privacy_enabled: bool,
    privacy_idle_secs: u32,
    autohide_enabled: bool,
    autohide_idle_secs: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            privacy_enabled: false,
            privacy_idle_secs: 60,
            autohide_enabled: false,
            autohide_idle_secs: 60,
        }
    }
}

static CFG: Mutex<Config> = Mutex::new(Config {
    privacy_enabled: false,
    privacy_idle_secs: 60,
    autohide_enabled: false,
    autohide_idle_secs: 60,
});
static THREAD: OnceLock<()> = OnceLock::new();
static RUNNING: AtomicBool = AtomicBool::new(true);
/// FR-13 是否已触发（触发/恢复通过 ACTIVE_LOCK 串行化，成对执行）
static PRIVACY_TRIGGERED: AtomicBool = AtomicBool::new(false);
/// 触发序列与恢复序列互斥，避免交错
static ACTIVE_LOCK: Mutex<()> = Mutex::new(());
/// 任务栏自动隐藏是否由本应用通过 ABM_SETSTATE 设置（恢复的唯一依据）
static AUTOHIDE_APPLIED: AtomicBool = AtomicBool::new(false);

/// 隐私触发时保存的现场（恢复的唯一依据，见工程约定 10）
struct PrivacySnapshot {
    windows: Vec<SavedWindow>,
    hid_icons: bool,   // 本功能执行了图标隐藏（触发前已隐藏则不恢复）
    hid_taskbar: bool, // 本功能执行了任务栏隐藏
    muted_by_us: bool, // 本功能执行了静音
}

static PRIVACY_SNAPSHOT: Mutex<Option<PrivacySnapshot>> = Mutex::new(None);

#[derive(Clone, Copy)]
struct SavedWindow {
    /// 窗口句柄（存为 isize 保证 Send，供 static Mutex 使用）
    hwnd: isize,
    was_maximized: bool,
    was_fullscreen: bool,
}

// ---------------------------------------------------------------------------
// 公共接口（lib.rs 调用）
// ---------------------------------------------------------------------------

/// 启动空闲检测轮询线程（幂等）
pub fn start() {
    RUNNING.store(true, Ordering::SeqCst);
    THREAD.get_or_init(|| {
        std::thread::spawn(poll_loop);
    });
}

/// 停止轮询并完整恢复：若隐私已触发 → 执行恢复序列；若任务栏由本应用隐藏 → 恢复显示。
/// 退出应用 / 关闭全部相关开关时调用。
pub fn stop() {
    RUNNING.store(false, Ordering::SeqCst);
    if PRIVACY_TRIGGERED.load(Ordering::SeqCst) {
        // 同步执行恢复（此时触发线程若在跑，等它写完现场再恢复）
        let _guard = ACTIVE_LOCK.lock().unwrap();
        if PRIVACY_TRIGGERED.swap(false, Ordering::SeqCst) {
            let snap = PRIVACY_SNAPSHOT.lock().unwrap().take();
            if let Some(s) = snap {
                restore_sequence(&s);
            }
        }
    }
    if AUTOHIDE_APPLIED.swap(false, Ordering::SeqCst) {
        let _ = taskbar::set_autohide(false);
    }
}

/// 同步配置（开关与空闲时间），并处理关闭时的即时恢复
pub fn configure(
    privacy_enabled: bool,
    privacy_idle_secs: u32,
    autohide_enabled: bool,
    autohide_idle_secs: u32,
) {
    let mut cfg = CFG.lock().unwrap();
    cfg.privacy_enabled = privacy_enabled;
    cfg.privacy_idle_secs = privacy_idle_secs.clamp(IDLE_CLAMP_MIN, IDLE_CLAMP_MAX);
    cfg.autohide_enabled = autohide_enabled;
    cfg.autohide_idle_secs = autohide_idle_secs.clamp(IDLE_CLAMP_MIN, IDLE_CLAMP_MAX);
    drop(cfg);

    if !privacy_enabled && PRIVACY_TRIGGERED.load(Ordering::SeqCst) {
        dlog::write("[privacy] switch off while triggered -> restore");
        restore_privacy_async();
    }
    if !autohide_enabled && AUTOHIDE_APPLIED.load(Ordering::SeqCst) {
        AUTOHIDE_APPLIED.store(false, Ordering::SeqCst);
        let _ = taskbar::set_autohide(false);
    }
}

/// FR-13 当前是否处于「已触发」状态（供前端状态提示）
pub fn is_triggered() -> bool {
    PRIVACY_TRIGGERED.load(Ordering::SeqCst)
}

// ---------------------------------------------------------------------------
// 空闲轮询
// ---------------------------------------------------------------------------

fn last_input_tick() -> u32 {
    unsafe {
        let mut lii = LASTINPUTINFO {
            cbSize: size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if GetLastInputInfo(&mut lii) == 0 {
            return 0;
        }
        lii.dwTime
    }
}

fn poll_loop() {
    dlog::write("[privacy] poll loop started");
    let mut prev_input: u32 = last_input_tick();
    let mut fullscreen = false;
    let mut last_leave: Option<std::time::Instant> = None;
    let mut tick: u64 = 0;
    loop {
        std::thread::sleep(Duration::from_millis(POLL_MS));
        if !RUNNING.load(Ordering::SeqCst) {
            break;
        }
        tick += 1;
        let cfg = *CFG.lock().unwrap();

        // —— FR-14 边缘弹出 / 再隐藏（每 tick，约 50ms 响应；动画期间暂缓）——
        if cfg.autohide_enabled
            && !fullscreen
            && AUTOHIDE_APPLIED.load(Ordering::SeqCst)
            && !taskbar::is_animating()
        {
            let pos = cursor_pos();
            if in_taskbar_area(pos) {
                // 鼠标在任务栏（或其边缘）：弹出显示
                if taskbar::is_autohide() {
                    let _ = taskbar::set_autohide(false);
                    dlog::write("[privacy] autohide: edge popup -> show");
                }
                last_leave = None;
            } else if !taskbar::is_autohide() {
                // 鼠标已离开任务栏：延迟后重新隐藏
                let now = std::time::Instant::now();
                match last_leave {
                    None => last_leave = Some(now),
                    Some(t) if now.duration_since(t).as_millis() as u64 >= HIDE_DELAY_MS => {
                        let _ = taskbar::set_autohide(true);
                        last_leave = None;
                        dlog::write("[privacy] autohide: re-hide after leave");
                    }
                    _ => {}
                }
            } else {
                last_leave = None;
            }
        }

        // —— 空闲检测（每 1 秒 = 20 tick）——
        if tick % 20 == 0 {
            fullscreen = crate::fullscreen::is_fullscreen_effective();
            let input = last_input_tick();
            // 32 位 tick 回绕安全（wrapping_sub）
            let idle_ms = unsafe { GetTickCount() }.wrapping_sub(input);
            let input_changed = input != prev_input;
            prev_input = input;

            // —— FR-14 任务栏自动隐藏 ——
            if cfg.autohide_enabled {
                if fullscreen {
                    // 全屏暂停：由本应用设置的隐藏先恢复显示，避免遮挡全屏内容
                    if AUTOHIDE_APPLIED.swap(false, Ordering::SeqCst) {
                        let _ = taskbar::set_autohide(false);
                        last_leave = None;
                        dlog::write("[privacy] autohide: fullscreen pause -> show");
                    }
                } else if !AUTOHIDE_APPLIED.load(Ordering::SeqCst)
                    && !taskbar::is_animating()
                    && u64::from(idle_ms) >= u64::from(cfg.autohide_idle_secs) * 1000
                {
                    if taskbar::set_autohide(true) {
                        AUTOHIDE_APPLIED.store(true, Ordering::SeqCst);
                        last_leave = None;
                        dlog::write("[privacy] autohide: idle timeout -> hide");
                    }
                }
                // 用户操作只重置计时（下一轮 idle_ms 重新累计），不改变任务栏显示状态：
                // 未隐藏 → 保持显示；已隐藏 → 保持隐藏（鼠标到边缘再弹出）。
            }

            // —— FR-13 隐私操作 ——
            if cfg.privacy_enabled {
                if !PRIVACY_TRIGGERED.load(Ordering::SeqCst)
                    && u64::from(idle_ms) >= u64::from(cfg.privacy_idle_secs) * 1000
                {
                    trigger_privacy_async();
                } else if PRIVACY_TRIGGERED.load(Ordering::SeqCst) && input_changed {
                    dlog::write("[privacy] user input detected -> restore");
                    restore_privacy_async();
                }
            }
        }
    }
    dlog::write("[privacy] poll loop stopped");
}

fn cursor_pos() -> POINT {
    let mut p = POINT { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut p);
    }
    p
}

/// 鼠标是否位于任一任务栏窗口矩形（外扩 EDGE_PADDING 像素）
fn in_taskbar_area(pos: POINT) -> bool {
    unsafe {
        for hwnd in taskbar::taskbar_windows() {
            let mut r = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if GetWindowRect(hwnd, &mut r) != 0 {
                if pos.x >= r.left - EDGE_PADDING
                    && pos.x <= r.right + EDGE_PADDING
                    && pos.y >= r.top - EDGE_PADDING
                    && pos.y <= r.bottom + EDGE_PADDING
                {
                    return true;
                }
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// FR-13 触发 / 恢复
// ---------------------------------------------------------------------------

fn trigger_privacy_async() {
    if PRIVACY_TRIGGERED.swap(true, Ordering::SeqCst) {
        return;
    }
    dlog::write("[privacy] idle timeout -> trigger");
    std::thread::spawn(|| {
        let _guard = ACTIVE_LOCK.lock().unwrap();
        // stop()/关开关可能已抢先恢复
        if !PRIVACY_TRIGGERED.load(Ordering::SeqCst) {
            return;
        }
        // ① 最小化所有窗口（全屏先转窗口化）
        let windows = collect_and_minimize();
        // ② 隐藏桌面图标（先检查当前是否已隐藏，避免与 FR-01 双击功能打架）
        let hid_icons = if icons::is_icons_hidden() {
            false
        } else {
            icons::run_fade(true, 256, 2);
            true
        };
        // ③ 隐藏任务栏（先检查 FR-14 / 系统是否已自动隐藏）
        let hid_taskbar = if taskbar::is_autohide() {
            false
        } else {
            taskbar::set_autohide(true)
        };
        // ④ 系统静音（仅切换静音标志，不改音量级别）
        let muted_by_us = set_system_mute(true);
        let windows_count = windows.len();
        *PRIVACY_SNAPSHOT.lock().unwrap() = Some(PrivacySnapshot {
            windows,
            hid_icons,
            hid_taskbar,
            muted_by_us,
        });
        dlog::write(&format!(
            "[privacy] sequence done: windows={} icons={} taskbar={} mute={}",
            windows_count,
            hid_icons,
            hid_taskbar,
            muted_by_us,
        ));
    });
}

fn restore_privacy_async() {
    if !PRIVACY_TRIGGERED.load(Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(|| {
        let _guard = ACTIVE_LOCK.lock().unwrap();
        if !PRIVACY_TRIGGERED.swap(false, Ordering::SeqCst) {
            return;
        }
        let snap = PRIVACY_SNAPSHOT.lock().unwrap().take();
        if let Some(s) = snap {
            restore_sequence(&s);
        }
        dlog::write("[privacy] restored");
    });
}

/// 恢复序列：① 还原窗口 ② 图标 ③ 任务栏 ④ 取消静音（仅还原本功能执行过的操作）
fn restore_sequence(s: &PrivacySnapshot) {
    restore_windows(&s.windows);
    if s.hid_icons {
        icons::run_fade(false, 256, 2);
    }
    if s.hid_taskbar {
        let _ = taskbar::set_autohide(false);
    }
    if s.muted_by_us {
        let _ = set_system_mute(false);
    }
}

// ---------------------------------------------------------------------------
// 窗口枚举 / 最小化 / 还原
// ---------------------------------------------------------------------------

unsafe extern "system" fn collect_cb(hwnd: HWND, lparam: LPARAM) -> i32 {
    let saved = &mut *(lparam as *mut Vec<SavedWindow>);

    // 仅处理可见的顶层窗口
    if IsWindowVisible(hwnd) == 0 {
        return 1;
    }
    // 触发前已最小化的窗口：不记录、不操作，恢复时保持原样（不弹出来）
    if IsIconic(hwnd) != 0 {
        return 1;
    }
    // 跳过带 owner 的窗口（属于其他窗口）与工具窗口（WS_EX_TOOLWINDOW）
    if !GetWindow(hwnd, GW_OWNER).is_null() {
        return 1;
    }
    if GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW != 0 {
        return 1;
    }
    // 桌面与任务栏体系（云笈自身窗口不跳过：隐私保护时连同主窗口一起最小化）
    let mut buf = [0u16; 128];
    let n = GetClassNameW(hwnd, buf.as_mut_ptr(), 128);
    if n > 0 {
        let cls = String::from_utf16_lossy(&buf[..n as usize]);
        if matches!(
            cls.as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
        ) {
            return 1;
        }
    }

    let mut wp = WINDOWPLACEMENT {
        length: size_of::<WINDOWPLACEMENT>() as u32,
        flags: 0,
        showCmd: 0,
        ptMinPosition: POINT { x: 0, y: 0 },
        ptMaxPosition: POINT { x: 0, y: 0 },
        rcNormalPosition: RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
    };
    let was_maximized = GetWindowPlacement(hwnd, &mut wp) != 0 && wp.showCmd == SW_SHOWMAXIMIZED as u32;
    let was_fullscreen = is_fullscreen_window(hwnd);
    saved.push(SavedWindow {
        hwnd: hwnd as isize,
        was_maximized,
        was_fullscreen,
    });

    // 全屏窗口先切换为窗口化再最小化（独占全屏应用无法切换时保持原状，不影响其余步骤）
    if was_fullscreen {
        ShowWindow(hwnd, SW_SHOWNORMAL);
    }
    ShowWindow(hwnd, SW_MINIMIZE);
    1
}

fn collect_and_minimize() -> Vec<SavedWindow> {
    let mut saved: Vec<SavedWindow> = Vec::new();
    unsafe {
        EnumWindows(Some(collect_cb), &mut saved as *mut Vec<SavedWindow> as LPARAM);
    }
    dlog::write(&format!("[privacy] minimized {} windows", saved.len()));
    saved
}

fn is_fullscreen_window(hwnd: HWND) -> bool {
    let mut wr = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    if unsafe { GetWindowRect(hwnd, &mut wr) } == 0 {
        return false;
    }
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_null() {
        return false;
    }
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
    if unsafe { GetMonitorInfoW(monitor, &mut mi) } == 0 {
        return false;
    }
    let m = &mi.rcMonitor;
    wr.left == m.left && wr.top == m.top && wr.right == m.right && wr.bottom == m.bottom
}

/// 还原窗口到最小化前状态：原最大化 → 还原为最大化；原全屏 → 尽力还原全屏。
/// 对不可控窗口（如独占全屏应用）尽力而为，不得崩溃。
fn restore_windows(saved: &[SavedWindow]) {
    for w in saved {
        let hwnd = w.hwnd as HWND;
        unsafe {
            if w.was_maximized {
                ShowWindow(hwnd, SW_RESTORE);
                let mut wp = WINDOWPLACEMENT {
                    length: size_of::<WINDOWPLACEMENT>() as u32,
                    flags: 0,
                    showCmd: 0,
                    ptMinPosition: POINT { x: 0, y: 0 },
                    ptMaxPosition: POINT { x: 0, y: 0 },
                    rcNormalPosition: RECT {
                        left: 0,
                        top: 0,
                        right: 0,
                        bottom: 0,
                    },
                };
                if GetWindowPlacement(hwnd, &mut wp) != 0 && wp.showCmd != SW_SHOWMAXIMIZED as u32 {
                    ShowWindow(hwnd, SW_MAXIMIZE);
                }
            } else if w.was_fullscreen {
                // 尽力还原全屏：恢复到所在显示器的完整矩形
                ShowWindow(hwnd, SW_RESTORE);
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
                if GetMonitorInfoW(monitor, &mut mi) != 0 {
                    let m = mi.rcMonitor;
                    MoveWindow(hwnd, m.left, m.top, m.right - m.left, m.bottom - m.top, 1);
                }
            } else {
                ShowWindow(hwnd, SW_RESTORE);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 系统静音（Core Audio，仅切换静音标志）
// ---------------------------------------------------------------------------

fn set_system_mute(mute: bool) -> bool {
    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        // 仅当本线程由我们初始化时负责反初始化
        let need_uninit = hr.0 == 0;
        let result = (|| -> windows::core::Result<()> {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
            let volume: IAudioEndpointVolume = device.Activate(CLSCTX_INPROC_SERVER, None)?;
            volume.SetMute(mute, std::ptr::null())?;
            Ok(())
        })();
        if need_uninit {
            CoUninitialize();
        }
        result.is_ok()
    }
}
