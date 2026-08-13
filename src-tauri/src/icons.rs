//! 桌面图标显隐控制（Win32）
//!
//! 原理：桌面图标渲染在 SHELLDLL_DefView 窗口上（宿主 Progman / WorkerW）。
//! 为其启用 WS_EX_LAYERED 后，用 SetLayeredWindowAttributes 调节全局 alpha，
//! 即可让所有桌面图标整体淡出/淡入，不影响壁纸与其他窗口。
//!
//! v0.6.0 起不再向 Explorer 发送任何“带指针的 LVM 消息”（LVM_GETITEMAT /
//! LVM_GETITEMPOSITION 等）：跨进程时这类消息不可靠（实测结构体原样返回），
//! 且 Explorer 会在 comctl32 内解引用本进程栈指针造成访问违例、资源管理器
//! 崩溃重启。桌面判定只用系统 API（WindowFromPoint / GetWindowRect）与
//! 无指针的 LVM_GETNEXTITEM（读取选中状态）。

#![allow(non_snake_case)]

use std::sync::atomic::{AtomicIsize, AtomicU8, Ordering};
use std::sync::Mutex;

use windows_sys::Win32::Foundation::{HWND, LPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowExW, FindWindowW, GetClassNameW, GetLayeredWindowAttributes,
    GetParent, GetWindowLongW, GetWindowRect, IsWindow, SendMessageTimeoutW,
    SetLayeredWindowAttributes, SetWindowLongW, WindowFromPoint, GWL_EXSTYLE, LWA_ALPHA,
    SMTO_ABORTIFHUNG, WS_EX_LAYERED,
};

const LVM_FIRST: u32 = 0x1000;
const LVM_GETNEXTITEM: u32 = LVM_FIRST + 12; // 按状态找下一项（选中项）
const LVNI_SELECTED: u32 = 0x0002;
const MAX_CLASS_LEN: usize = 256;

/// 串行化动画：即使事件意外重叠，也不会同时操作同一个图标窗口
static FADE_LOCK: Mutex<()> = Mutex::new(());

/// 查窗口类名
fn window_class(hwnd: HWND) -> String {
    if hwnd.is_null() {
        return String::new();
    }
    let mut buf = vec![0u16; MAX_CLASS_LEN];
    unsafe {
        let n = GetClassNameW(hwnd, buf.as_mut_ptr(), MAX_CLASS_LEN as i32);
        if n <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buf[..n as usize])
    }
}

/// 定位桌面图标宿主窗口 SHELLDLL_DefView
static FOUND_SHELL_VIEW: AtomicIsize = AtomicIsize::new(0);

/// 定位桌面图标宿主窗口 SHELLDLL_DefView（带缓存）
///
/// 旧实现每次鼠标按下都会重新 EnumWindows + 向 Progman 发 WM_SPAWN_WORKERW 并等待，
/// 钩子回调因此可能被拖慢数百毫秒，是“显示完又隐藏”等竞态的诱因之一。
/// 这里缓存句柄（并用 IsWindow 校验有效性），桌面句柄极少变化。
static CACHED_SHELL_VIEW: AtomicIsize = AtomicIsize::new(0);

unsafe extern "system" fn enum_shell_view_proc(hwnd: HWND, _lparam: LPARAM) -> i32 {
    let sv = FindWindowExW(hwnd, std::ptr::null_mut(), windows_sys::core::w!("SHELLDLL_DefView"), std::ptr::null());
    if !sv.is_null() {
        FOUND_SHELL_VIEW.store(sv as isize, Ordering::SeqCst);
        return 0; // 停止枚举
    }
    1 // 继续
}

pub fn find_shell_view() -> HWND {
    unsafe {
        let cached = CACHED_SHELL_VIEW.load(Ordering::SeqCst) as HWND;
        if !cached.is_null() && IsWindow(cached) != 0 {
            return cached;
        }

        let progman = FindWindowW(windows_sys::core::w!("Progman"), std::ptr::null());

        // 直接枚举顶层窗口，在已有的 WorkerW / Progman 下找 SHELLDLL_DefView。
        // 刻意不向 Progman 发 WM_SPAWN_WORKERW：该消息会让 Explorer 重建承载壁纸的
        // WorkerW 窗口，Wallpaper Engine 等动态壁纸的渲染表面随之销毁重建，
        // 表现为“壁纸暂停刷新一段时间后再恢复”。WorkerW 在正常桌面上本就存在，
        // 直接枚举即可定位，无需也不应重建它。
        FOUND_SHELL_VIEW.store(0, Ordering::SeqCst);
        EnumWindows(Some(enum_shell_view_proc), 0);
        let mut found = FOUND_SHELL_VIEW.load(Ordering::SeqCst) as HWND;

        // 兜底：直接查 Progman 子窗口
        if found.is_null() && !progman.is_null() {
            found = FindWindowExW(
                progman,
                std::ptr::null_mut(),
                windows_sys::core::w!("SHELLDLL_DefView"),
                std::ptr::null(),
            );
        }
        if !found.is_null() {
            CACHED_SHELL_VIEW.store(found as isize, Ordering::SeqCst);
        }
        found
    }
}

/// 桌面自己的 SysListView32（SHELLDLL_DefView 的直接子窗口，即图标列表）
fn find_desktop_list_view() -> HWND {
    unsafe {
        let sv = find_shell_view();
        if sv.is_null() {
            return std::ptr::null_mut();
        }
        FindWindowExW(
            sv,
            std::ptr::null_mut(),
            windows_sys::core::w!("SysListView32"),
            std::ptr::null(),
        )
    }
}

/// 读取窗口当前 alpha；非分层窗口返回 None
pub fn get_alpha(hwnd: HWND) -> Option<u8> {
    unsafe {
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
        if ex & (WS_EX_LAYERED as i32) == 0 {
            return None;
        }
        let mut color: u32 = 0;
        let mut alpha: u8 = 0;
        let mut flags: u32 = 0;
        let ok = GetLayeredWindowAttributes(hwnd, &mut color, &mut alpha, &mut flags);
        if ok == 0 {
            return None;
        }
        Some(alpha)
    }
}

/// 将窗口整体 alpha 设为 0-255（自动启用分层样式）
pub fn set_alpha(hwnd: HWND, alpha: u8) -> bool {
    unsafe {
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
        if ex & (WS_EX_LAYERED as i32) == 0 {
            SetWindowLongW(hwnd, GWL_EXSTYLE, ex | (WS_EX_LAYERED as i32));
        }
        SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA) != 0
    }
}

/// 立即恢复桌面图标为完全可见（保持分层样式，alpha=255 即为完全不透明）
///
/// 注意：这里刻意不再调用 clear_layered 移除 WS_EX_LAYERED。
/// 实测在部分系统上，显示动画结束后移除分层样式会让 Explorer 重绘桌面、
/// 图标当场消失（即“显示完又隐藏”）。保持分层且 alpha=255 与正常显示完全一致。
pub fn restore_icons() {
    let hwnd = find_shell_view();
    if !hwnd.is_null() {
        set_alpha(hwnd, 255);
    }
}

/// 等待正在进行的动画结束后恢复图标（供“关闭功能”等场景使用）
pub fn restore_icons_blocking() {
    let _guard = FADE_LOCK.lock().unwrap();
    restore_icons();
}

/// 程序启动时自修复：若发现桌面图标残留半透明（上次异常退出），恢复之
pub fn ensure_icons_restored() {
    let hwnd = find_shell_view();
    if hwnd.is_null() {
        return;
    }
    match get_alpha(hwnd) {
        Some(a) if a < 255 => restore_icons(),
        _ => {}
    }
}

/// 收集到的桌面图标列表视图句柄（各显示器下 SHELLDLL_DefView 的 SysListView32）
/// 注：HWND 是指针不可 Send，故以 isize 存储、使用时再转回。
static DESKTOP_LIST_VIEWS: Mutex<Vec<isize>> = Mutex::new(Vec::new());

unsafe extern "system" fn enum_desktop_list_proc(hwnd: HWND, _lparam: LPARAM) -> i32 {
    let sv = FindWindowExW(
        hwnd,
        std::ptr::null_mut(),
        windows_sys::core::w!("SHELLDLL_DefView"),
        std::ptr::null(),
    );
    if !sv.is_null() {
        let lv = FindWindowExW(
            sv,
            std::ptr::null_mut(),
            windows_sys::core::w!("SysListView32"),
            std::ptr::null(),
        );
        if !lv.is_null() {
            if let Ok(mut guard) = DESKTOP_LIST_VIEWS.lock() {
                if !guard.contains(&(lv as isize)) {
                    guard.push(lv as isize);
                }
            }
        }
    }
    1
}

/// 获取有效的桌面图标列表句柄集合（带缓存；全部失效时重新枚举）
fn desktop_list_views() -> Vec<HWND> {
    {
        let guard = DESKTOP_LIST_VIEWS.lock().unwrap();
        let valid: Vec<HWND> = guard
            .iter()
            .map(|h| *h as HWND)
            .filter(|h| unsafe { IsWindow(*h) != 0 })
            .collect();
        if !valid.is_empty() {
            return valid;
        }
    }
    // 缓存全失效：清空后重新枚举
    DESKTOP_LIST_VIEWS.lock().unwrap().clear();
    unsafe {
        EnumWindows(Some(enum_desktop_list_proc), 0);
    }
    let mut result: Vec<HWND> = DESKTOP_LIST_VIEWS
        .lock()
        .unwrap()
        .iter()
        .map(|h| *h as HWND)
        .collect();
    if result.is_empty() {
        // 兜底：主桌面列表
        let primary = find_desktop_list_view();
        if !primary.is_null() {
            result.push(primary);
        }
    }
    result
}

/// 窗口屏幕矩形是否包含该点
fn window_contains(hwnd: HWND, pt: windows_sys::Win32::Foundation::POINT) -> bool {
    unsafe {
        let mut r = windows_sys::Win32::Foundation::RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if GetWindowRect(hwnd, &mut r) == 0 {
            return false;
        }
        pt.x >= r.left && pt.x < r.right && pt.y >= r.top && pt.y < r.bottom
    }
}

/// 沿父链爬到顶层窗口
fn climb_top(mut hwnd: HWND) -> HWND {
    unsafe {
        loop {
            let parent = GetParent(hwnd);
            if parent.is_null() || parent == hwnd {
                break;
            }
            hwnd = parent;
        }
        hwnd
    }
}

/// 点击点的轻量桌面判定结果（仅系统 API，无跨进程消息）
pub struct PointState {
    /// 点击点是否属于桌面体系：顶层为 Progman/WorkerW，且被某个桌面图标列表覆盖
    pub on_desktop: bool,
    /// 覆盖点击点的桌面 SysListView32（0=无）；选中项查询以它为目标
    pub list_view: isize,
    /// WindowFromPoint 直接命中的窗口
    pub hwnd: usize,
    pub hwnd_cls: String,
    /// 顶层窗口类名
    pub top_cls: String,
}

/// 轻量桌面判定：WindowFromPoint + 类名 + 列表矩形包含检测。
/// 不向 Explorer 发送任何消息，供鼠标钩子每次按键时调用。
pub fn desktop_point_state(screen_x: i32, screen_y: i32) -> PointState {
    unsafe {
        let pt = windows_sys::Win32::Foundation::POINT {
            x: screen_x,
            y: screen_y,
        };
        let hwnd = WindowFromPoint(pt);
        let hwnd_cls = window_class(hwnd);
        let top = climb_top(hwnd);
        let top_cls = window_class(top);
        let mut st = PointState {
            on_desktop: false,
            list_view: 0,
            hwnd: hwnd as usize,
            hwnd_cls,
            top_cls,
        };
        if hwnd.is_null() || !(st.top_cls == "Progman" || st.top_cls == "WorkerW") {
            return st;
        }
        if let Some(lv) = desktop_list_views().into_iter().find(|lv| window_contains(*lv, pt)) {
            st.on_desktop = true;
            st.list_view = lv as isize;
        }
        st
    }
}

/// 桌面图标列表的选中状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionState {
    /// 列表确认无任何选中项 —— 说明首击点的是空白（桌面空白点击会取消选中）
    NoneSelected,
    /// 首击选中了该项 —— 说明点的是图标
    Selected(i32),
    /// 列表无响应/超时/无列表 —— 不确定，调用方应保守处理（不触发）
    Unknown,
}

/// 读取桌面图标列表当前选中项。
///
/// 只发送无指针的 LVM_GETNEXTITEM（跨进程安全）；绝不使用带指针的
/// LVM_GETITEMAT / LVM_GETITEMPOSITION（跨进程不可靠且会让 Explorer 崩溃）。
pub fn desktop_selection(list_view: isize) -> SelectionState {
    if list_view == 0 {
        return SelectionState::Unknown;
    }
    unsafe {
        let mut sr: usize = 0xDEAD_BEEF;
        let ok = SendMessageTimeoutW(
            list_view as HWND,
            LVM_GETNEXTITEM,
            usize::MAX, // iStart = -1：从头查找
            LVNI_SELECTED as LPARAM,
            SMTO_ABORTIFHUNG,
            100,
            &mut sr,
        );
        if ok == 0 || sr == 0xDEAD_BEEF {
            return SelectionState::Unknown; // 超时 / 列表未处理
        }
        if sr == usize::MAX {
            return SelectionState::NoneSelected; // -1：无选中项
        }
        if sr < 1_000_000 {
            SelectionState::Selected(sr as i32)
        } else {
            SelectionState::Unknown
        }
    }
}

/// 桌面图标当前是否“非完全可见”（隐藏或动画中）。
/// 此时任意桌面双击都应触发（用户意图是重新显示图标），不再区分是否点在图标上。
pub fn is_icons_hidden() -> bool {
    let hwnd = find_shell_view();
    if hwnd.is_null() {
        return false;
    }
    matches!(get_alpha(hwnd), Some(a) if a < 255)
}

/// 桌面图标当前的真实可见状态（以窗口实际 alpha 为准）
pub enum IconVisibility {
    Visible,
    Hidden,
    Animating,
}

/// 读取桌面图标当前状态：0=隐藏，255/无分层样式=可见，其余=动画中
pub fn icon_visibility() -> IconVisibility {
    let hwnd = find_shell_view();
    if hwnd.is_null() {
        // 找不到桌面视图时按“可见”处理，避免误触发隐藏
        return IconVisibility::Visible;
    }
    match get_alpha(hwnd) {
        Some(0) => IconVisibility::Hidden,
        Some(255) => IconVisibility::Visible,
        Some(_) => IconVisibility::Animating,
        None => IconVisibility::Visible,
    }
}

// ---------------------------------------------------------------------------
// 动画执行（供桥接层调用）
// ---------------------------------------------------------------------------

pub fn run_fade(target_hidden: bool, steps: u32, step_ms: u64) {
    let _guard = FADE_LOCK.lock().unwrap();

    let hwnd = find_shell_view();
    if hwnd.is_null() {
        return;
    }

    // 幂等保护：当前真实状态已等于目标状态时直接返回，
    // 从根上杜绝“已显示又执行隐藏/已隐藏又执行显示”的重复动画
    match icon_visibility() {
        IconVisibility::Hidden if target_hidden => return,
        IconVisibility::Visible if !target_hidden => return,
        _ => {}
    }

    let end: u8 = if target_hidden { 0 } else { 255 };
    // 起始方向：隐藏从 255 递减；显示从当前值递增
    let start: u8 = if target_hidden {
        255
    } else {
        get_alpha(hwnd).unwrap_or(0)
    };

    let mut alpha = start as i32;
    let step = if target_hidden { -1 } else { 1 };

    for _ in 0..steps {
        alpha = (alpha + step).clamp(0, 255);
        set_alpha(hwnd, alpha as u8);
        if alpha == end as i32 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(step_ms));
    }

}

pub static HOOK_EVENT_FLAG: AtomicU8 = AtomicU8::new(0); // 1 = 触发过双击

/// 协变包装：钩子回调里检查该标志，桥接层轮询并触发动画
pub fn set_hook_event() {
    HOOK_EVENT_FLAG.store(1, Ordering::SeqCst);
}

pub fn take_hook_event() -> bool {
    HOOK_EVENT_FLAG.swap(0, Ordering::SeqCst) == 1
}
