//! 临时调试日志：统一写入 `%LOCALAPPDATA%\CloudSatchel\hooks-debug.log`
//!
//! 用于在真实桌面上核对“双击快捷方式误隐藏图标”的消息序列与命中测试细节，
//! 确认修复后应移除。
//!
//! 性能：文件句柄缓存（首次打开后复用），避免每次写日志都 open/close 文件。

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);
static DIR_READY: AtomicBool = AtomicBool::new(false);

pub fn write(msg: &str) {
    if let Ok(base) = std::env::var("LOCALAPPDATA") {
        let dir = std::path::Path::new(&base).join("CloudSatchel");
        let path = dir.join("hooks-debug.log");
        if !DIR_READY.load(Ordering::Relaxed) {
            let _ = std::fs::create_dir_all(&dir);
            DIR_READY.store(true, Ordering::Relaxed);
        }
        let mut guard = LOG_FILE.lock().unwrap();
        if guard.is_none() {
            *guard = OpenOptions::new().create(true).append(true).open(&path).ok();
        }
        if let Some(file) = guard.as_mut() {
            let _ = writeln!(file, "{}", msg);
        }
    }
}
