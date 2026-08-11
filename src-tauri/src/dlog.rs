//! 临时调试日志：统一写入 `%LOCALAPPDATA%\AsYouWishToolBox\hooks-debug.log`
//!
//! 用于在真实桌面上核对“双击快捷方式误隐藏图标”的消息序列与命中测试细节，
//! 确认修复后应移除。
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static LOG_LOCK: Mutex<()> = Mutex::new(());
static DIR_READY: AtomicBool = AtomicBool::new(false);

pub fn write(msg: &str) {
    let _guard = LOG_LOCK.lock().unwrap();
    if let Ok(base) = std::env::var("LOCALAPPDATA") {
        let dir = std::path::Path::new(&base).join("AsYouWishToolBox");
        let path = dir.join("hooks-debug.log");
        if !DIR_READY.load(Ordering::Relaxed) {
            let _ = std::fs::create_dir_all(&dir);
            DIR_READY.store(true, Ordering::Relaxed);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(f, "{}", msg);
        }
    }
}
