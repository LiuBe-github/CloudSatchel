//! 鼠标选取翻译（v0.20.0）
//!
//! - 全局低层鼠标钩子（WH_MOUSE_LL）监听左键抬起：松手后经 UI Automation
//!   （IUIAutomation TextPattern）读取前台窗口的选中文本与选区位置，
//!   在选区下方弹出「翻译」小按钮（translate-button 窗口）。
//! - 点击按钮打开翻译弹窗（translate-popup 窗口），翻译引擎二选一：
//!   - AI 助理：复用 FR-15 的 BaseURL / 模型 / DPAPI Key，把文本翻译为简体中文；
//!   - 微软翻译：Azure Translator v3（需在设置中配置 API Key 与区域）。
//! - 点击按钮/弹窗以外任意位置（或弹窗失焦 / Esc）→ 两者一并隐藏。
//! - 纯净性：仅用户选中文字并点击翻译时才发起网络请求；Key 经 DPAPI 加密落盘。

#![allow(non_snake_case)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, OnceLock};

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::System::Ole::{
    SafeArrayAccessData, SafeArrayGetLBound, SafeArrayGetUBound, SafeArrayUnaccessData,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationTextPattern, IUIAutomationTextRange,
    UIA_TextPatternId,
};
use windows_sys::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetClassNameW, GetForegroundWindow, GetMessageW,
    GetWindowRect, GetWindowThreadProcessId, IsWindowVisible, PostThreadMessageW,
    SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_MOUSE_LL, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_QUIT, MSG, MSLLHOOKSTRUCT,
};

use crate::dlog;

/// 翻译按钮尺寸（与 tauri.conf.json translate-button 窗口一致）
const BUTTON_W: i32 = 75;
const BUTTON_H: i32 = 30;
/// 翻译弹窗尺寸（与 tauri.conf.json translate-popup 窗口一致）
const POPUP_W: i32 = 400;
const POPUP_H: i32 = 300;
/// 松手后等待选区稳定再读取（应用处理鼠标抬起通常需要几十毫秒）
const SELECT_SETTLE_MS: u64 = 130;
/// 按钮/弹窗相对选区底部的间距
const GAP: i32 = 8;
/// 单次翻译文本上限（防止超大选区拖垮请求）
const MAX_TEXT: usize = 8000;
/// 微软翻译 Key 加密文件名（%LOCALAPPDATA%\CloudSatchel）
pub const MS_KEY_FILE: &str = "ms-translate-key.bin";
/// 请求总超时（秒）
const TIMEOUT_SECS: u64 = 60;

static RUNNING: AtomicBool = AtomicBool::new(false);
static ENABLED: AtomicBool = AtomicBool::new(true);
static HOOK: std::sync::atomic::AtomicPtr<std::ffi::c_void> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());
static HOOK_THREAD_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static APP: OnceLock<AppHandle> = OnceLock::new();
static TX: Mutex<Option<Sender<HookMsg>>> = Mutex::new(None);

/// 当前选区（按钮定位与弹窗取词共用）
#[derive(Clone)]
struct Selection {
    text: String,
    x: i32, // 按钮左上角（已 clamp 进工作区）
    y: i32,
}
static SELECTION: Mutex<Option<Selection>> = Mutex::new(None);

enum HookMsg {
    MouseUp(windows::Win32::Foundation::POINT),
}

/// 翻译配置（lib.rs 从 AppState 组装传入）
pub struct TranslateConfig {
    pub engine: String, // "ai" | "microsoft"
    pub ai_model: String,
    pub ai_base_url: String,
    pub ms_region: String,
    pub target_lang: String, // 目标语言代码（默认 auto-zh-Hans）
    pub source_lang: String, // 源语言代码（默认 auto：自动检测）
}

// ---------------------------------------------------------------------------
// 生命周期
// ---------------------------------------------------------------------------

/// 启动翻译钩子与选区检测线程（幂等）
pub fn start(app: AppHandle) {
    let _ = APP.set(app.clone());
    if RUNNING.load(Ordering::SeqCst) {
        return;
    }
    let (tx, rx) = mpsc::channel::<HookMsg>();
    *TX.lock().unwrap() = Some(tx);
    RUNNING.store(true, Ordering::SeqCst);
    // 启动即把按钮窗口压到目标尺寸（tauri.conf 创建极小窗口会被钳制）
    if let Some(win) = app.get_webview_window("translate-button") {
        prepare_button(&win);
    }
    let hook_handle = std::thread::spawn(|| hook_thread());
    let worker_handle = std::thread::spawn(move || worker_thread(rx));
    std::mem::forget(hook_handle);
    std::mem::forget(worker_handle);
}

/// 停止钩子（退出应用时）
pub fn stop() {
    RUNNING.store(false, Ordering::SeqCst);
    // 让钩子线程消息循环退出，同时释放 TX → 工作线程 rx.recv 返回 Err 退出
    let tid = HOOK_THREAD_ID.load(Ordering::SeqCst);
    if tid != 0 {
        unsafe {
            PostThreadMessageW(tid as u32, WM_QUIT, 0, 0);
        }
    }
    *TX.lock().unwrap() = None;
}

/// 开关（设置面板）：关闭时立即隐藏翻译窗口并停止检测
pub fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::SeqCst);
    if !enabled {
        if let Some(app) = APP.get() {
            hide(app);
        }
    }
}

// ---------------------------------------------------------------------------
// 全局鼠标钩子
// ---------------------------------------------------------------------------

unsafe extern "system" fn mouse_callback(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == 0 {
        let info = &*(lparam as *const MSLLHOOKSTRUCT);
        if wparam == WM_LBUTTONUP as usize {
            if ENABLED.load(Ordering::SeqCst) {
                if let Some(tx) = TX.lock().unwrap().as_ref() {
                    let _ = tx.send(HookMsg::MouseUp(windows::Win32::Foundation::POINT {
                        x: info.pt.x,
                        y: info.pt.y,
                    }));
                }
            }
        } else if wparam == WM_LBUTTONDOWN as usize {
            // 点击按钮/弹窗以外 → 隐藏；点击内部由各窗口自行处理
            if let Some(app) = APP.get() {
                if translate_visible(app) && !point_inside_translate(app, info.pt.x, info.pt.y) {
                    hide(app);
                }
            }
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

fn hook_thread() {
    unsafe {
        let module: HINSTANCE = GetModuleHandleW(std::ptr::null());
        let cb: unsafe extern "system" fn(i32, usize, isize) -> isize = mouse_callback;
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(cb), module, 0);
        if hook.is_null() {
            dlog::write("[translate] SetWindowsHookExW FAILED");
            return;
        }
        HOOK.store(hook, Ordering::SeqCst);
        HOOK_THREAD_ID.store(GetCurrentThreadId() as u64, Ordering::SeqCst);
        dlog::write("[translate] hook installed");

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
        let hook = HOOK.swap(std::ptr::null_mut(), Ordering::SeqCst);
        if !hook.is_null() {
            UnhookWindowsHookEx(hook);
        }
        dlog::write("[translate] hook thread exit");
    }
}

// ---------------------------------------------------------------------------
// 选区检测（UI Automation）
// ---------------------------------------------------------------------------

fn worker_thread(rx: mpsc::Receiver<HookMsg>) {
    let need_uninit = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).is_ok() };
    // MouseUp 检测链日志节流：选区操作高频，最多每 5 秒记一条“收到抬起”
    let mut last_mouseup_log = std::time::Instant::now() - std::time::Duration::from_secs(60);
    loop {
        match rx.recv() {
            Ok(HookMsg::MouseUp(pt)) => {
                if !ENABLED.load(Ordering::SeqCst) {
                    continue;
                }
                std::thread::sleep(std::time::Duration::from_millis(SELECT_SETTLE_MS));
                if !ENABLED.load(Ordering::SeqCst) {
                    continue;
                }
                if last_mouseup_log.elapsed() >= std::time::Duration::from_secs(5) {
                    dlog::write(&format!("[translate] mouseup@({},{}) detecting", pt.x, pt.y));
                    last_mouseup_log = std::time::Instant::now();
                }
                let Some(app) = APP.get() else { continue };
                // 前台是自己的窗口 → 不检测
                if foreground_is_ours() {
                    continue;
                }
                // 弹窗显示中（本次点击刚打开它 / 点击在弹窗内）→ 不检测，避免误关或重复弹按钮
                if popup_visible(app) {
                    continue;
                }
                // 点击落在按钮窗口内 → 不检测
                if translate_visible(app) && point_inside_translate(app, pt.x, pt.y) {
                    continue;
                }
                if let Some(sel) = detect_selection(pt) {
                    *SELECTION.lock().unwrap() = Some(sel.clone());
                    if let Some(win) = app.get_webview_window("translate-button") {
                        let _ = win.set_position(tauri::PhysicalPosition::new(sel.x, sel.y));
                        prepare_button(&win);
                        crate::prepare_aux_window(&win);
                        let _ = win.show();
                    }
                }
            }
            Err(_) => break,
        }
    }
    if need_uninit {
        unsafe { CoUninitialize() };
    }
    dlog::write("[translate] worker exit");
}

/// 前台窗口诊断信息（检测失败时写日志：远程桌面客户端 / 无 TextPattern 的
/// 应用在这里暴露，用于坏机器取证）
fn fg_diag() -> String {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return "fg=none".to_string();
        }
        let mut cls = [0u16; 128];
        let n = GetClassNameW(hwnd, cls.as_mut_ptr(), 128);
        let class = String::from_utf16_lossy(&cls[..(n.max(0) as usize).min(128)]);
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        format!("fg_class={class} fg_pid={pid}")
    }
}

/// 通过 UI Automation 读取前台窗口的选中文本与选区位置
fn detect_selection(pt: windows::Win32::Foundation::POINT) -> Option<Selection> {
    let automation: IUIAutomation = match unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) } {
        Ok(a) => a,
        Err(e) => {
            dlog::write(&format!("[translate] UIA CoCreateInstance failed: {e}; {}", fg_diag()));
            return None;
        }
    };

    // 优先取焦点元素；失败时用鼠标位置处的元素
    let mut element = unsafe { automation.GetFocusedElement() }.ok();
    if element.is_none() {
        element = unsafe { automation.ElementFromPoint(pt) }.ok();
    }
    let walker = unsafe { automation.ControlViewWalker() }.ok();

    // 沿祖先链向上找支持 TextPattern 的元素（浏览器等选中文本常挂在父级文档元素上）
    let mut cur = element.clone();
    let mut range = None;
    let mut found_class: Option<String> = None;
    for _ in 0..10 {
        let Some(el) = cur else { break };
        if let Ok(pattern) =
            unsafe { el.GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId) }
        {
            if let Ok(cls) = unsafe { el.CurrentClassName() } {
                found_class = Some(cls.to_string());
            }
            if let Ok(sel) = unsafe { pattern.GetSelection() } {
                if let Ok(len) = unsafe { sel.Length() } {
                    if len > 0 {
                        if let Ok(r) = unsafe { sel.GetElement(0) } {
                            range = Some(r);
                            break;
                        }
                    }
                }
            }
        }
        cur = match &walker {
            Some(w) => unsafe { w.GetParentElement(&el) }.ok(),
            None => None,
        };
    }

    let r = match range {
        Some(r) => r,
        None => {
            // 常见跨机器失败：选中发生在远程桌面/远程控制窗口内（mstsc/向日葵/
            // ToDesk 无 TextPattern）、或应用不暴露文本模式。日志用于取证。
            dlog::write(&format!(
                "[translate] no TextPattern selection; {}; visited={:?}",
                fg_diag(),
                found_class
            ));
            return None;
        }
    };
    let text = unsafe { r.GetText(-1) }.ok()?.to_string();
    let text = text.trim();
    if text.is_empty() {
        dlog::write(&format!("[translate] selection text empty; {}", fg_diag()));
        return None;
    }
    let text: String = text.chars().take(MAX_TEXT).collect();

    // 选区矩形：优先用选区所在元素，失败退回焦点元素矩形（仅作回退锚点）
    let rect = unsafe { r.GetEnclosingElement() }
        .ok()
        .and_then(|e| unsafe { e.CurrentBoundingRectangle() }.ok())
        .or_else(|| element.as_ref().and_then(|e| unsafe { e.CurrentBoundingRectangle() }.ok()))?;
    if rect.right <= rect.left || rect.bottom <= rect.top {
        return None;
    }

    // 按钮放在选区「末尾」正下方：以最后一个包围矩形的右下角为锚点，
    // clamp 进所在显示器工作区（固定 GAP 间距，不再忽远忽近）
    let (end_x, end_y) = selection_end(&r, &rect);
    let work = work_area(pt.x, pt.y);
    let x = (end_x - BUTTON_W / 2).clamp(work.left, work.right - BUTTON_W);
    let y = (end_y + GAP).clamp(work.top, work.bottom - BUTTON_H);
    dlog::write(&format!(
        "[translate] selection len={} btn=({x},{y}) end=({end_x},{end_y}) rect=[{},{} {} {}]",
        text.len(),
        rect.left,
        rect.top,
        rect.right,
        rect.bottom
    ));
    Some(Selection { text, x, y })
}

/// 选区「末尾」坐标：取选区各包围矩形中的最后一个（文本结束处）右下角。
/// 优先用 TextRange::GetBoundingRectangles（SAFEARRAY，每 4 个 double =
/// left/top/width/height）；失败或全部退化时回退到元素矩形右下角。
/// 之前用「整段元素矩形」居中定位，跨行/长段选区会导致按钮离文本末尾忽远忽近。
fn selection_end(
    range: &IUIAutomationTextRange,
    fallback: &windows::Win32::Foundation::RECT,
) -> (i32, i32) {
    unsafe {
        if let Ok(sa) = range.GetBoundingRectangles() {
            if !sa.is_null() {
                let lb = SafeArrayGetLBound(sa, 1).unwrap_or(0);
                let ub = SafeArrayGetUBound(sa, 1).unwrap_or(-1);
                let count = ((ub - lb + 1).max(0) as usize).min(4096);
                if count >= 4 {
                    let mut data: *mut core::ffi::c_void = std::ptr::null_mut();
                    if SafeArrayAccessData(sa, &mut data).is_ok() && !data.is_null() {
                        let vals = std::slice::from_raw_parts(data as *const f64, count);
                        let n_rects = count / 4;
                        for i in (0..n_rects).rev() {
                            let left = vals[i * 4] as i32;
                            let top = vals[i * 4 + 1] as i32;
                            let width = vals[i * 4 + 2] as i32;
                            let height = vals[i * 4 + 3] as i32;
                            if width > 0 && height > 0 {
                                let _ = SafeArrayUnaccessData(sa);
                                return (left + width, top + height);
                            }
                        }
                        let _ = SafeArrayUnaccessData(sa);
                    }
                }
            }
        }
    }
    (fallback.right, fallback.bottom)
}

/// 点击坐标所在显示器的工作区（取不到时回退 1080p 全屏）
fn work_area(x: i32, y: i32) -> windows_sys::Win32::Foundation::RECT {
    unsafe {
        let monitor = MonitorFromPoint(
            windows_sys::Win32::Foundation::POINT { x, y },
            MONITOR_DEFAULTTONEAREST,
        );
        if !monitor.is_null() {
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                rcMonitor: windows_sys::Win32::Foundation::RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
                rcWork: windows_sys::Win32::Foundation::RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
                dwFlags: 0,
            };
            if GetMonitorInfoW(monitor, &mut mi) != 0 {
                return mi.rcWork;
            }
        }
    }
    windows_sys::Win32::Foundation::RECT {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1080,
    }
}

fn foreground_is_ours() -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.is_null() {
            return false;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(fg, &mut pid);
        pid == std::process::id()
    }
}

fn window_hwnd(app: &AppHandle, label: &str) -> Option<*mut core::ffi::c_void> {
    let win = app.get_webview_window(label)?;
    let hwnd = win.hwnd().ok()?;
    Some(hwnd.0)
}

fn translate_visible(app: &AppHandle) -> bool {
    for label in ["translate-button", "translate-popup"] {
        if let Some(h) = window_hwnd(app, label) {
            if unsafe { IsWindowVisible(h) } != 0 {
                return true;
            }
        }
    }
    false
}

fn popup_visible(app: &AppHandle) -> bool {
    window_hwnd(app, "translate-popup")
        .map(|h| unsafe { IsWindowVisible(h) } != 0)
        .unwrap_or(false)
}

fn point_inside_translate(app: &AppHandle, x: i32, y: i32) -> bool {
    for label in ["translate-button", "translate-popup"] {
        if let Some(h) = window_hwnd(app, label) {
            let mut r = windows_sys::Win32::Foundation::RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if unsafe { GetWindowRect(h, &mut r) } != 0 {
                if x >= r.left && x < r.right && y >= r.top && y < r.bottom {
                    return true;
                }
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// 翻译弹窗与请求
// ---------------------------------------------------------------------------

/// 点击翻译按钮：隐藏按钮、显示弹窗，并按所选引擎发起翻译（异步）
pub fn open_popup(app: AppHandle, cfg: TranslateConfig) {
    let Some(sel) = SELECTION.lock().unwrap().clone() else {
        return;
    };
    if let Some(b) = app.get_webview_window("translate-button") {
        let _ = b.hide();
    }
    let work = work_area(sel.x, sel.y);
    let cx = sel.x + BUTTON_W / 2;
    let x = (cx - POPUP_W / 2).clamp(work.left, work.right - POPUP_W);
    let y = (sel.y + BUTTON_H + GAP).clamp(work.top, work.bottom - POPUP_H);
    if let Some(p) = app.get_webview_window("translate-popup") {
        let _ = p.set_position(tauri::PhysicalPosition::new(x, y));
        crate::prepare_aux_window(&p);
        let _ = p.show();
        let _ = p.set_focus();
    }

    let engine_label = if cfg.engine == "microsoft" {
        "微软翻译".to_string()
    } else {
        "AI 助理".to_string()
    };
    let target_label = target_lang_label(&cfg.target_lang).to_string();
    let source_label = source_lang_label(&cfg.source_lang).to_string();
    let text = sel.text.clone();
    let _ = app.emit_to(
        "translate-popup",
        "translate-pending",
        json!({ "source": &text, "engine": engine_label, "sourceLabel": source_label, "targetLabel": target_label }),
    );

    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = translate_text(&text, &cfg).await;
        let payload = match result {
            Ok(target) => json!({
                "source": &text,
                "target": target,
                "engine": engine_label,
                "sourceLabel": source_label,
                "targetLabel": target_label,
                "ok": true,
                "error": "",
            }),
            Err(e) => json!({
                "source": &text,
                "target": "",
                "engine": engine_label,
                "sourceLabel": source_label,
                "targetLabel": target_label,
                "ok": false,
                "error": e,
            }),
        };
        let _ = app2.emit_to("translate-popup", "translate-result", payload);
    });
}

/// 强制按钮窗口为目标尺寸：tauri.conf 创建极小窗口时会被框架钳制
/// （实测 24×12 配置创建后仍约 136×37 逻辑像素），运行期 set_size 可绕过；
/// 每次显示前重设，防止 wry show 重置尺寸。
fn prepare_button(win: &tauri::WebviewWindow) {
    let scale = win.scale_factor().unwrap_or(1.0);
    let w = (BUTTON_W as f64 * scale).round() as u32;
    let h = (BUTTON_H as f64 * scale).round() as u32;
    let _ = win.set_min_size(Some(tauri::PhysicalSize::new(1, 1)));
    let _ = win.set_size(tauri::PhysicalSize::new(w, h));
}

/// 隐藏翻译按钮与弹窗并清空选区（点击外部 / 失焦 / Esc）
pub fn hide(app: &AppHandle) {
    if let Some(b) = app.get_webview_window("translate-button") {
        let _ = b.hide();
    }
    if let Some(p) = app.get_webview_window("translate-popup") {
        let _ = p.hide();
    }
    *SELECTION.lock().unwrap() = None;
    let _ = app.emit_to("translate-popup", "translate-cleared", ());
}

async fn translate_text(text: &str, cfg: &TranslateConfig) -> Result<String, String> {
    if cfg.engine == "microsoft" {
        translate_ms(text, cfg).await
    } else {
        translate_ai(text, cfg).await
    }
}

/// AI 助理引擎：OpenAI 兼容 chat/completions（非流式），提示词要求输出简体中文译文
async fn translate_ai(text: &str, cfg: &TranslateConfig) -> Result<String, String> {
    let key = crate::ai::load_key()?;
    let lang_label = target_lang_label(&cfg.target_lang);
    let src_label = source_lang_label(&cfg.source_lang);
    let prompt = if cfg.source_lang == "auto" {
        format!(
            "把下面的文本翻译成{lang_label}。只输出译文本身，不要任何解释或额外内容。

{text}"
        )
    } else {
        format!(
            "把下面的{src_label}文本翻译成{lang_label}。只输出译文本身，不要任何解释或额外内容。

{text}"
        )
    };
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    let url = crate::ai::chat_url(&cfg.ai_base_url);
    let body = json!({
        "model": cfg.ai_model,
        "messages": [{ "role": "user", "content": prompt }],
        "stream": false,
    });
    let resp = client
        .post(&url)
        .bearer_auth(&key)
        .json(&body)
        .send()
        .await
        .map_err(friendly_net_error)?;
    if !resp.status().is_success() {
        let status = resp.status();
        let detail = resp
            .text()
            .await
            .ok()
            .and_then(|b| {
                serde_json::from_str::<serde_json::Value>(&b)
                    .ok()
                    .and_then(|v| v["error"]["message"].as_str().map(|s| s.to_string()))
            })
            .filter(|s| !s.is_empty());
        return Err(http_error_msg(status, detail));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("解析 AI 回复失败: {e}"))?;
    v["choices"][0]["message"]["content"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "AI 未返回译文，请检查模型 / 接口配置".to_string())
}

/// 微软翻译引擎：Azure Translator v3（需 Key 与区域；Key DPAPI 加密落盘）
async fn translate_ms(text: &str, cfg: &TranslateConfig) -> Result<String, String> {
    if !crate::ai::has_encrypted_key(MS_KEY_FILE) {
        return Err("未配置微软翻译 API Key，请在「设置 → 鼠标选取翻译」中填写".to_string());
    }
    let key = crate::ai::load_encrypted_key(MS_KEY_FILE)?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    // 目标语言：默认「自动识别 → 简体中文」；显式语言直接走 BCP-47 代码
    let to = if cfg.target_lang == "auto-zh-Hans" {
        "zh-Hans"
    } else {
        cfg.target_lang.as_str()
    };
    let mut url = format!(
        "https://api.cognitive.microsofttranslator.com/translate?api-version=3.0&to={to}"
    );
    // 源语言：默认自动检测（省略 from），显式指定时附带 from 参数
    if cfg.source_lang != "auto" {
        url.push_str(&format!("&from={}", cfg.source_lang));
    }
    let mut req = client
        .post(&url)
        .header("Ocp-Apim-Subscription-Key", &key)
        .json(&json!([{ "text": text }]));
    if !cfg.ms_region.is_empty() {
        req = req.header("Ocp-Apim-Subscription-Region", &cfg.ms_region);
    }
    let resp = req.send().await.map_err(friendly_net_error)?;
    if !resp.status().is_success() {
        let status = resp.status();
        let detail = resp.text().await.ok().filter(|s| !s.is_empty());
        return Err(match detail {
            Some(d) => format!("微软翻译返回错误（HTTP {}）：{d}", status.as_u16()),
            None => format!("微软翻译返回错误（HTTP {}）", status.as_u16()),
        });
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("解析翻译结果失败: {e}"))?;
    v[0]["translations"][0]["text"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "微软翻译未返回译文（请检查 Key 与区域配置）".to_string())
}

/// 源语言代码 → 中文名（"auto" = 自动检测）
fn source_lang_label(code: &str) -> &'static str {
    match code {
        "zh-Hans" => "简体中文",
        "zh-Hant" => "繁體中文",
        "en" => "英语",
        "ja" => "日语",
        "ko" => "韩语",
        "fr" => "法语",
        "de" => "德语",
        "ru" => "俄语",
        "es" => "西班牙语",
        _ => "自动检测",
    }
}

/// 目标语言代码 → 中文名（AI 提示词 / 弹窗标题展示用）
fn target_lang_label(code: &str) -> &'static str {
    match code {
        "zh-Hans" | "auto-zh-Hans" => "简体中文",
        "zh-Hant" => "繁體中文",
        "en" => "英语",
        "ja" => "日语",
        "ko" => "韩语",
        "fr" => "法语",
        "de" => "德语",
        "ru" => "俄语",
        "es" => "西班牙语",
        _ => "简体中文",
    }
}

fn friendly_net_error(e: reqwest::Error) -> String {
    if e.is_timeout() {
        format!("请求超时（超过 {TIMEOUT_SECS} 秒无响应）")
    } else if e.is_connect() {
        "无法连接到翻译服务，请检查网络与配置".to_string()
    } else {
        format!("网络请求失败: {e}")
    }
}

fn http_error_msg(status: reqwest::StatusCode, detail: Option<String>) -> String {
    if status.as_u16() == 401 {
        match detail {
            Some(d) => format!("API Key 无效（401）：{d}"),
            None => "API Key 无效（401 Unauthorized）".to_string(),
        }
    } else if status.as_u16() == 429 {
        "请求过于频繁（429），请稍后再试".to_string()
    } else {
        match detail {
            Some(d) => format!("接口返回错误（HTTP {}）：{d}", status.as_u16()),
            None => format!("接口返回错误（HTTP {}）", status.as_u16()),
        }
    }
}
