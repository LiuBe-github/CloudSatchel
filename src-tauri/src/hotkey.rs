//! 全局热键（RegisterHotKey）—— 老板键（FR-13 扩展）、AI 小窗（FR-17）共用
//!
//! 实现：注册线程持有 Windows 消息队列，`RegisterHotKey(hwnd=NULL)` 把 `WM_HOTKEY`
//! 投递到该线程队列；线程轮询 `PeekMessageW` 分发回调。注册结果经一次性 channel
//! 回执给调用方（注册失败 = 组合键被占用 / 系统保留，可提示用户换键）。
//!
//! 纯净性：不写注册表；退出（unregister_all）时全部注销，不残留热键。
//! 注意：回调在热键线程执行，必须快速返回（耗时工作请自行 spawn）。

use std::collections::HashMap;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TryRecvError};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    PeekMessageW, PM_NOREMOVE, PM_REMOVE, WM_HOTKEY, MSG,
};

/// 热键组合：修饰键位（MOD_* 组合）+ 虚拟键码
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HotKeySpec {
    pub modifiers: u32,
    pub vk: u16,
}

enum Cmd {
    Register {
        id: i32,
        spec: HotKeySpec,
        cb: Box<dyn Fn() + Send>,
        ack: SyncSender<bool>,
    },
    Unregister(i32),
    Quit,
}

static REGISTRY: OnceLock<Mutex<HashMap<i32, (HotKeySpec, Box<dyn Fn() + Send>)>>> = OnceLock::new();
static CHANNEL: OnceLock<SyncSender<Cmd>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<i32, (HotKeySpec, Box<dyn Fn() + Send>)>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

// ---------------------------------------------------------------------------
// 热键字符串解析（"Ctrl+Shift+Space"、"Ctrl+`"、"Alt+F9"）
// ---------------------------------------------------------------------------

fn key_vk(name: &str) -> Option<u16> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse as k;
    let n = name.to_ascii_lowercase();
    // 特殊符号键（单字符，需在字母分支之前判定，否则 `` ` `` 等会被当作未知单字符）
    if let Some(vk) = symbol_vk(n.as_str()) {
        return Some(vk);
    }
    // 单字符：字母 / 数字
    let bytes = n.as_bytes();
    if bytes.len() == 1 {
        let b = bytes[0];
        return match b {
            b'a'..=b'z' => Some(k::VK_A as u16 + (b - b'a') as u16),
            b'0'..=b'9' => Some(k::VK_0 as u16 + (b - b'0') as u16),
            _ => None,
        };
    }
    // F1-F24
    if let Some(rest) = n.strip_prefix('f') {
        if let Ok(num) = rest.parse::<u16>() {
            if (1..=24).contains(&num) {
                return Some(k::VK_F1 as u16 + num - 1);
            }
        }
    }
    Some(match n.as_str() {
        "space" => k::VK_SPACE as u16,
        "tab" => k::VK_TAB as u16,
        "enter" | "return" => k::VK_RETURN as u16,
        "esc" | "escape" => k::VK_ESCAPE as u16,
        "backspace" => k::VK_BACK as u16,
        "delete" | "del" => k::VK_DELETE as u16,
        "insert" | "ins" => k::VK_INSERT as u16,
        "home" => k::VK_HOME as u16,
        "end" => k::VK_END as u16,
        "pageup" | "pgup" => k::VK_PRIOR as u16,
        "pagedown" | "pgdn" => k::VK_NEXT as u16,
        "up" => k::VK_UP as u16,
        "down" => k::VK_DOWN as u16,
        "left" => k::VK_LEFT as u16,
        "right" => k::VK_RIGHT as u16,
        _ => return None,
    })
}

/// 符号键 → 虚拟键码（OEM 键）
fn symbol_vk(n: &str) -> Option<u16> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse as k;
    Some(match n {
        "`" | "backquote" => k::VK_OEM_3 as u16,
        "-" | "minus" => k::VK_OEM_MINUS as u16,
        "=" | "plus" | "equal" => k::VK_OEM_PLUS as u16,
        "[" => k::VK_OEM_4 as u16,
        "]" => k::VK_OEM_6 as u16,
        "\\" | "backslash" => k::VK_OEM_5 as u16,
        ";" | "semicolon" => k::VK_OEM_1 as u16,
        "'" | "quote" => k::VK_OEM_7 as u16,
        "," | "comma" => k::VK_OEM_COMMA as u16,
        "." | "period" | "dot" => k::VK_OEM_PERIOD as u16,
        "/" | "slash" => k::VK_OEM_2 as u16,
        _ => return None,
    })
}

/// 解析热键字符串，如 `Ctrl+Shift+Space`、`Ctrl+\``、`Alt+F9`。
/// 修饰键：Ctrl / Alt / Shift / Win（大小写不敏感）；主键：字母、数字、F1-F24 或特殊键名。
pub fn parse_hotkey(spec: &str) -> Option<HotKeySpec> {
    let parts: Vec<&str> = spec.split('+').map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
    if parts.is_empty() || parts.len() > 4 {
        return None;
    }
    let mut modifiers = 0u32;
    let mut vk = 0u16;
    for (i, part) in parts.iter().enumerate() {
        let lower = part.to_ascii_lowercase();
        let is_modifier = matches!(lower.as_str(), "ctrl" | "alt" | "shift" | "win" | "windows");
        if is_modifier {
            match lower.as_str() {
                "ctrl" => modifiers |= MOD_CONTROL,
                "alt" => modifiers |= MOD_ALT,
                "shift" => modifiers |= MOD_SHIFT,
                _ => modifiers |= MOD_WIN,
            }
            continue;
        }
        if i != parts.len() - 1 {
            return None; // 修饰键必须在前，最后一项必须是主键
        }
        vk = key_vk(part)?;
    }
    if vk == 0 {
        return None;
    }
    Some(HotKeySpec { modifiers, vk })
}

// ---------------------------------------------------------------------------
// 注册 / 注销
// ---------------------------------------------------------------------------

fn start_thread() -> &'static SyncSender<Cmd> {
    CHANNEL.get_or_init(|| {
        let (tx, rx) = sync_channel::<Cmd>(16);
        std::thread::Builder::new()
            .name("hotkey-loop".into())
            .spawn(move || hotkey_loop(rx))
            .expect("failed to spawn hotkey thread");
        tx
    })
}

fn hotkey_loop(rx: Receiver<Cmd>) {
    // 先触碰一次消息队列：RegisterHotKey(hwnd=NULL) 要求调用线程已有队列
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    unsafe {
        PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
    }
    let mut registered: HashMap<i32, HotKeySpec> = HashMap::new();

    loop {
        // 处理消息队列中的 WM_HOTKEY
        unsafe {
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_HOTKEY {
                    let id = msg.wParam as i32;
                    if let Some((_, cb)) = registry().lock().unwrap().get(&id) {
                        cb();
                    }
                }
            }
        }
        // 处理注册 / 注销命令
        match rx.try_recv() {
            Ok(Cmd::Register { id, spec, cb, ack }) => {
                // 同 id 先注销旧热键再注册新的
                if registered.contains_key(&id) {
                    unsafe {
                        UnregisterHotKey(std::ptr::null_mut(), id);
                    }
                    registered.remove(&id);
                }
                // MOD_NOREPEAT：按住不连发（老板键 / AI 小窗都是开关语义，连发会反复切换）
                let ok = unsafe {
                    RegisterHotKey(
                        std::ptr::null_mut(),
                        id,
                        spec.modifiers | MOD_NOREPEAT,
                        spec.vk.into(),
                    )
                } != 0;
                if ok {
                    registered.insert(id, spec);
                    registry().lock().unwrap().insert(id, (spec, cb));
                }
                let _ = ack.send(ok);
            }
            Ok(Cmd::Unregister(id)) => {
                if registered.contains_key(&id) {
                    unsafe {
                        UnregisterHotKey(std::ptr::null_mut(), id);
                    }
                    registered.remove(&id);
                }
                registry().lock().unwrap().remove(&id);
            }
            Ok(Cmd::Quit) => break,
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => break,
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    // 退出前注销全部热键，不残留
    for &id in registered.keys() {
        unsafe {
            UnregisterHotKey(std::ptr::null_mut(), id);
        }
    }
    registry().lock().unwrap().clear();
}

/// 注册（或更新）一个全局热键。返回 false 表示注册失败（组合键被占用等）。
/// 回调在热键线程执行，必须快速返回。
pub fn register(id: i32, spec: HotKeySpec, cb: impl Fn() + Send + 'static) -> bool {
    let tx = start_thread();
    let (ack_tx, ack_rx) = sync_channel::<bool>(1);
    if tx
        .send(Cmd::Register {
            id,
            spec,
            cb: Box::new(cb),
            ack: ack_tx,
        })
        .is_err()
    {
        return false;
    }
    ack_rx.recv_timeout(Duration::from_millis(1000)).unwrap_or(false)
}

/// 注销指定热键（幂等）
pub fn unregister(id: i32) {
    if let Some(tx) = CHANNEL.get() {
        let _ = tx.send(Cmd::Unregister(id));
    }
}

/// 注销全部热键（退出应用时调用）
pub fn unregister_all() {
    if let Some(tx) = CHANNEL.get() {
        let _ = tx.send(Cmd::Quit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_common_combos() {
        let s = parse_hotkey("Ctrl+`").unwrap();
        assert_eq!(s.modifiers & MOD_CONTROL, MOD_CONTROL);
        assert_eq!(s.vk, 0xC0); // VK_OEM_3

        let s = parse_hotkey("Ctrl+Shift+Space").unwrap();
        assert_eq!(s.modifiers & MOD_CONTROL, MOD_CONTROL);
        assert_eq!(s.modifiers & MOD_SHIFT, MOD_SHIFT);
        assert_eq!(s.vk, 0x20);

        let s = parse_hotkey("alt+f9").unwrap();
        assert_eq!(s.modifiers, MOD_ALT);
        assert_eq!(s.vk, 0x78); // VK_F9

        let s = parse_hotkey("Ctrl+Alt+P").unwrap();
        assert_eq!(s.modifiers, MOD_CONTROL | MOD_ALT);
        assert_eq!(s.vk, 0x50); // VK_P
    }

    #[test]
    fn parse_invalid() {
        assert!(parse_hotkey("").is_none());
        assert!(parse_hotkey("Ctrl").is_none()); // 只有修饰键
        assert!(parse_hotkey("Space+Ctrl").is_none()); // 主键不在最后
        assert!(parse_hotkey("Ctrl+X+Y").is_none()); // 两个主键
        assert!(parse_hotkey("Ctrl+@@@").is_none());
        assert!(parse_hotkey("Ctrl+Shift+Alt+Win+F5").is_none()); // 超出 4 段
    }
}
