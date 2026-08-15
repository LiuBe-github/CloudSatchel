//! 背景图片功能（参照「花笺 Floral Notepaper」实现）
//!
//! 流程：前端点击「选择」→ 原生文件对话框选图 → `copy_background_image` 把图复制到
//! `%LOCALAPPDATA%\CloudSatchel\backgrounds`（并清理上一张）→ 前端把返回路径存入设置；
//! 渲染时由 `read_background_image` 读文件并以 data URL 返回，前端 `<img>` 直接显示，
//! 支持 填充/完整/平铺、遮罩、模糊、缩放、横向/纵向偏移。
//!
//! 与花笺的差异：花笺用 `@tauri-apps/plugin-dialog` + `convertFileSrc` + 资产协议，
//! 云笈为保持「纯净、少依赖、离线可构建」改用原生 Win32 `IFileOpenDialog`（windows crate
//! 已有）+ data URL（base64 crate 已在锁文件中），无需新增插件或资产协议依赖。

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use windows::core::{w, HSTRING, PWSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{FileOpenDialog, IFileOpenDialog, SIGDN_FILESYSPATH};

const ALLOWED_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp"];

// ---------------------------------------------------------------------------
// 设置结构（与前端 BackgroundSettings 对应，camelCase 序列化）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundSettings {
    #[serde(default)]
    pub image_path: String,
    #[serde(default = "default_fit")]
    pub fit: String,
    #[serde(default = "default_dim")]
    pub dim: f64,
    #[serde(default = "default_blur")]
    pub blur: f64,
    #[serde(default = "default_scale")]
    pub scale: f64,
    #[serde(default = "default_position")]
    pub position_x: f64,
    #[serde(default = "default_position")]
    pub position_y: f64,
}

fn default_fit() -> String {
    "cover".to_string()
}
fn default_dim() -> f64 {
    0.25
}
fn default_blur() -> f64 {
    0.0
}
fn default_scale() -> f64 {
    1.0
}
fn default_position() -> f64 {
    50.0
}

impl Default for BackgroundSettings {
    fn default() -> Self {
        Self {
            image_path: String::new(),
            fit: default_fit(),
            dim: default_dim(),
            blur: default_blur(),
            scale: default_scale(),
            position_x: default_position(),
            position_y: default_position(),
        }
    }
}

impl BackgroundSettings {
    /// 归一化并夹紧各数值，防止非法值破坏渲染
    pub fn clamped(mut self) -> Self {
        self.image_path = self.image_path.trim().to_string();
        if !matches!(self.fit.as_str(), "cover" | "contain" | "repeat") {
            self.fit = "cover".to_string();
        }
        self.dim = self.dim.clamp(0.0, 1.0);
        self.blur = self.blur.clamp(0.0, 20.0);
        self.scale = self.scale.clamp(0.5, 2.0);
        self.position_x = self.position_x.clamp(0.0, 100.0);
        self.position_y = self.position_y.clamp(0.0, 100.0);
        self
    }
}

// ---------------------------------------------------------------------------
// 数据目录与持久化
// ---------------------------------------------------------------------------

fn local_app_data() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("CloudSatchel")
}

fn backgrounds_dir() -> PathBuf {
    local_app_data().join("backgrounds")
}

// 注：背景设置的读写（settings.json）已统一移交 prefs 模块，
// 与各功能开关同文件持久化，见 crate::prefs。

// ---------------------------------------------------------------------------
// 选择 / 复制 / 读取
// ---------------------------------------------------------------------------

/// 弹出原生文件选择框（COM STA 线程调用），返回选中的图片绝对路径；取消返回 None
pub fn choose_background_image() -> Result<Option<String>, String> {
    let owner = owner_hwnd();
    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        // 仅当本线程由我们初始化时负责反初始化
        let need_uninit = hr.0 == 0;
        let result = (|| -> windows::core::Result<Option<String>> {
            let dialog: IFileOpenDialog =
                CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
            let title = HSTRING::from("选择背景图片");
            dialog.SetTitle(&title)?;
            let filters = [
                COMDLG_FILTERSPEC {
                    pszName: w!("图片文件"),
                    pszSpec: w!("*.png;*.jpg;*.jpeg;*.webp;*.gif;*.bmp"),
                },
                COMDLG_FILTERSPEC {
                    pszName: w!("所有文件"),
                    pszSpec: w!("*.*"),
                },
            ];
            dialog.SetFileTypes(&filters)?;
            dialog.SetFileTypeIndex(1)?;
            // 用户取消或关闭对话框时返回 HRESULT 失败，统一视为「未选择」
            if dialog.Show(owner).is_err() {
                return Ok(None);
            }
            let item = dialog.GetResult()?;
            let pwstr: PWSTR = item.GetDisplayName(SIGDN_FILESYSPATH)?;
            let path = pwstr.to_string().unwrap_or_default();
            CoTaskMemFree(Some(pwstr.as_ptr() as *const core::ffi::c_void));
            if path.trim().is_empty() {
                Ok(None)
            } else {
                Ok(Some(path))
            }
        })();
        if need_uninit {
            CoUninitialize();
        }
        result.map_err(|e| format!("选择背景图片失败: {e}"))
    }
}

/// 把用户选择的图片复制到数据目录，返回目标绝对路径
pub fn copy_background_image(source_path: &str) -> Result<String, String> {
    let source = PathBuf::from(source_path.trim());
    if !source.is_file() {
        return Err("背景图片文件不存在".to_string());
    }
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_ascii_lowercase();
    if !ALLOWED_EXTS.contains(&ext.as_str()) {
        return Err("不支持的图片格式".to_string());
    }

    let dir = backgrounds_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    // 清理上一张已复制的背景图（仅限我们自己的 backgrounds 目录，不碰用户原始文件）
    let old = crate::prefs::load().image_path;
    if !old.is_empty() {
        let old_path = PathBuf::from(&old);
        if old_path.starts_with(&dir) && old_path.is_file() {
            let _ = fs::remove_file(&old_path);
        }
    }

    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let dest = dir.join(format!("bg-{millis}.{ext}"));
    fs::copy(&source, &dest).map_err(|e| e.to_string())?;
    dest.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "目标路径编码失败".to_string())
}

/// 读取背景图片并以 data URL 返回（空路径 / 文件不存在返回 None）
pub fn read_background_image(path: &str) -> Result<Option<String>, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let p = Path::new(trimmed);
    if !p.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(p).map_err(|e| e.to_string())?;
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        _ => "image/png",
    };
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    Ok(Some(format!(
        "data:{};base64,{}",
        mime,
        STANDARD.encode(&bytes)
    )))
}

/// 找到主窗口句柄作为文件对话框的所有者，避免对话框跑到窗口背后
fn owner_hwnd() -> Option<windows::Win32::Foundation::HWND> {
    unsafe {
        let hwnd = windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW(
            std::ptr::null(),
            windows_sys::core::w!("云笈"),
        );
        if hwnd.is_null() {
            None
        } else {
            Some(windows::Win32::Foundation::HWND(
                hwnd as *mut core::ffi::c_void,
            ))
        }
    }
}
