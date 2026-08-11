//! 桌面双击检测 —— WH_MOUSE_LL 全局低层鼠标钩子
//!
//! 自检双击序列（系统对注入事件不生成 WM_LBUTTONDBLCLK）：
//! 两次 LEFT DOWN 间隔 <= 500ms 且位移 <= 8px 视为一次双击。
//! 同时兼容真实点击下系统把第二击转换为 WM_LBUTTONDBLCLK 的情况。
//!
//! 触发判定（v0.6.0 重做，修复“双击桌面图标误隐藏”与 Explorer 崩溃）：
//! - 首击与第二击都必须确认在桌面体系内（Progman/WorkerW 且被桌面列表覆盖）；
//! - 图标可见：仅当首击没有选中任何图标才触发 —— 桌面列表在空白处点击会取消选中、
//!   在图标上会选中该项，用无指针的 LVM_GETNEXTITEM 区分“点的是图标”还是空白；
//! - 图标隐藏/动画中：任意桌面双击都触发（意图是重新显示图标）；
//! - 选中状态读取不确定（列表无响应/超时）→ 不触发（宁可漏一次，绝不误隐藏）。

#![allow(non_snake_case)]

use std::sync::atomic::{AtomicI64, AtomicPtr, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

use windows_sys::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, WH_MOUSE_LL, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN,
    WM_QUIT, MSG, MSLLHOOKSTRUCT,
};

use crate::dlog;
use crate::icons;

const DOUBLE_CLICK_MS: i64 = 500; // 双击判定窗口
const MOVE_TOLERANCE: i64 = 8; // 两次点击位移容差（px）
// 防抖：仅用于吸收同一物理按击的重复消息尾巴（如 DOWN+DBLCLK 双送、三连击的第三次）。
// 不能长到吞掉用户真实的连续双击——用户连续双击节奏通常 1~1.3s。
// 真实的“下一次双击”必须能再次触发，否则状态会与输入错位（表现为“显示完又隐藏”）。
const DEBOUNCE_MS: i64 = 250;

static HOOK: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());
static THREAD_ID: AtomicU64 = AtomicU64::new(0);
static RUNNING: AtomicI64 = AtomicI64::new(0); // 0 停止, 1 运行

static LAST_DOWN_T: AtomicI64 = AtomicI64::new(-1);
static LAST_DOWN_X: AtomicI64 = AtomicI64::new(0);
static LAST_DOWN_Y: AtomicI64 = AtomicI64::new(0);
static LAST_DOWN_DESKTOP: AtomicI64 = AtomicI64::new(-1); // -1 未知 / 0 首击非桌面 / 1 首击确认桌面
static FIRST_DOWN_SEL: AtomicI64 = AtomicI64::new(-2); // 首击前的选中状态：-2 未知 / -1 无选中 / >=0 选中项索引
static LAST_FIRE_T: AtomicI64 = AtomicI64::new(-1); // -1：启动后第一次双击不应被防抖吞掉

/// 单调时钟起点：时间只增不减，避免系统墙钟回拨（NTP 校时等）导致误判双击/绕过防抖
static CLOCK_EPOCH: OnceLock<Instant> = OnceLock::new();

fn now_ms() -> i64 {
    CLOCK_EPOCH
        .get_or_init(Instant::now)
        .elapsed()
        .as_millis() as i64
}

unsafe extern "system" fn mouse_callback(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let is_down = wparam == WM_LBUTTONDOWN as usize || wparam == WM_LBUTTONDBLCLK as usize;
    if code == 0 && is_down {
        let info = &*(lparam as *const MSLLHOOKSTRUCT);
        let now = now_ms();
        let last_t = LAST_DOWN_T.load(Ordering::SeqCst);
        let dx = (info.pt.x as i64 - LAST_DOWN_X.load(Ordering::SeqCst)).abs();
        let dy = (info.pt.y as i64 - LAST_DOWN_Y.load(Ordering::SeqCst)).abs();

        let is_dblclk = wparam == WM_LBUTTONDBLCLK as usize;
        let within_window = last_t >= 0
            && (now - last_t) <= DOUBLE_CLICK_MS
            && dx <= MOVE_TOLERANCE
            && dy <= MOVE_TOLERANCE;

        if within_window {
            // 双击第二击。触发规则见模块注释：可见时仅“首击未选中任何图标”才触发；
            // 隐藏/动画中任意桌面双击都触发；不确定一律不触发。
            let first_desktop = LAST_DOWN_DESKTOP.load(Ordering::SeqCst) == 1;
            let st = icons::desktop_point_state(info.pt.x, info.pt.y);
            let on_desktop = first_desktop && st.on_desktop;
            let hidden = icons::is_icons_hidden();

            let mut sel = if on_desktop && !hidden {
                icons::desktop_selection(st.list_view)
            } else {
                icons::SelectionState::Unknown
            };
            // 首击前无选中、第二击也无选中：可能是“首击点空白（正常）”，
            // 也可能是极快双击时 Explorer 尚未处理首击的选中。等 15ms 复查一次，
            // 避免把“双击图标”误判成“双击空白”而误隐藏。
            if matches!(sel, icons::SelectionState::NoneSelected)
                && FIRST_DOWN_SEL.load(Ordering::SeqCst) == -1
            {
                std::thread::sleep(std::time::Duration::from_millis(15));
                sel = icons::desktop_selection(st.list_view);
            }

            let last_fire = LAST_FIRE_T.load(Ordering::SeqCst);
            let since_fire = now - last_fire;
            // last_fire < 0 表示从未触发过：启动后的第一次双击不应被防抖吞掉
            let debounce_ok = last_fire < 0 || since_fire >= DEBOUNCE_MS;
            let blank_click = matches!(sel, icons::SelectionState::NoneSelected);
            let fire = on_desktop && debounce_ok && (hidden || blank_click);
            if fire {
                icons::set_hook_event();
                LAST_FIRE_T.store(now, Ordering::SeqCst);
            }
            let pre_sel = match FIRST_DOWN_SEL.load(Ordering::SeqCst) {
                -2 => "unknown".to_string(),
                -1 => "none".to_string(),
                i => format!("item {}", i),
            };
            dlog::write(&format!(
                "[{}] DBLCLK ({},{}) top={} first_desktop={} on_desktop={} hidden={} pre_sel={} sel={:?} fire={}",
                now_ms(),
                info.pt.x,
                info.pt.y,
                st.top_cls,
                first_desktop,
                on_desktop,
                hidden,
                pre_sel,
                sel,
                fire
            ));
            LAST_DOWN_T.store(-1, Ordering::SeqCst); // 本次双击已消费
            LAST_DOWN_DESKTOP.store(-1, Ordering::SeqCst);
            FIRST_DOWN_SEL.store(-2, Ordering::SeqCst);
        } else if is_dblclk && last_t == -1 {
            // 极少数情况下系统会对同一物理按压既送 DOWN 又送 DBLCLK：
            // 上一击已作为双击第二击消费，忽略这条尾巴，不重新武装序列
            dlog::write(&format!(
                "[{}] TAIL w={} ({},{})",
                now_ms(),
                wparam,
                info.pt.x,
                info.pt.y
            ));
        } else {
            // 首击：轻量桌面判定 + 记录“点击前”的选中状态（不发任何跨进程指针消息）。
            let st = icons::desktop_point_state(info.pt.x, info.pt.y);
            let sel0 = if st.on_desktop {
                icons::desktop_selection(st.list_view)
            } else {
                icons::SelectionState::Unknown
            };
            let sel0_code = match sel0 {
                icons::SelectionState::NoneSelected => -1,
                icons::SelectionState::Selected(i) => i as i64,
                icons::SelectionState::Unknown => -2,
            };
            LAST_DOWN_T.store(now, Ordering::SeqCst);
            LAST_DOWN_X.store(info.pt.x as i64, Ordering::SeqCst);
            LAST_DOWN_Y.store(info.pt.y as i64, Ordering::SeqCst);
            LAST_DOWN_DESKTOP.store(st.on_desktop as i64, Ordering::SeqCst);
            FIRST_DOWN_SEL.store(sel0_code, Ordering::SeqCst);
            let pre_sel = match sel0_code {
                -2 => "unknown".to_string(),
                -1 => "none".to_string(),
                i => format!("item {}", i),
            };
            dlog::write(&format!(
                "[{}] FIRST ({},{}) cls={}(0x{:x}) top={} on_desktop={} pre_sel={}",
                now_ms(),
                info.pt.x,
                info.pt.y,
                st.hwnd_cls,
                st.hwnd,
                st.top_cls,
                st.on_desktop,
                pre_sel
            ));
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

/// 安装全局鼠标钩子（独立线程，自带消息循环）
pub fn start() -> bool {
    if RUNNING.load(Ordering::SeqCst) == 1 {
        return true;
    }
    let handle = std::thread::spawn(|| unsafe {
        let module: HINSTANCE = GetModuleHandleW(std::ptr::null());
        // 钩子回调是静态函数指针，随模块存活，不会被回收
        let cb: unsafe extern "system" fn(i32, usize, isize) -> isize = mouse_callback;
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(cb), module, 0);
        if hook.is_null() {
            RUNNING.store(0, Ordering::SeqCst);
            dlog::write("[HOOK] SetWindowsHookExW FAILED");
            return;
        }
        HOOK.store(hook, Ordering::SeqCst);
        THREAD_ID.store(GetCurrentThreadId() as u64, Ordering::SeqCst);
        RUNNING.store(1, Ordering::SeqCst);
        dlog::write("[HOOK] installed");

        let mut msg = MSG {
            hwnd: std::ptr::null_mut(),
            message: 0,
            wParam: 0,
            lParam: 0,
            time: 0,
            pt: windows_sys::Win32::Foundation::POINT { x: 0, y: 0 },
        };
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        // 消息循环退出：卸载钩子
        let hook = HOOK.swap(std::ptr::null_mut(), Ordering::SeqCst);
        if !hook.is_null() {
            UnhookWindowsHookEx(hook);
        }
        RUNNING.store(0, Ordering::SeqCst);
        dlog::write("[HOOK] thread exit, unhooked");
    });
    // 等待钩子安装完成
    for _ in 0..100 {
        if RUNNING.load(Ordering::SeqCst) == 1 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    std::mem::forget(handle); // 钩子线程为 daemon 性质，随进程退出
    RUNNING.load(Ordering::SeqCst) == 1
}

/// 卸载钩子
pub fn stop() {
    if RUNNING.load(Ordering::SeqCst) != 1 {
        return;
    }
    let tid = THREAD_ID.load(Ordering::SeqCst);
    unsafe {
        // 向钩子线程投递 WM_QUIT
        if tid != 0 {
            PostThreadMessageW(tid as u32, WM_QUIT, 0, 0);
        }
    }
    // 等待线程退出（最多 2s）
    for _ in 0..100 {
        if RUNNING.load(Ordering::SeqCst) == 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}
