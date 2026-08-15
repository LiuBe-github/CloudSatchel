//! 用户偏好持久化（settings.json）
//!
//! 保存「上次关闭时」所有功能开关与背景设置的状态，下次启动时自动恢复并应用效果。
//! 文件：`%LOCALAPPDATA%\CloudSatchel\settings.json`
//!
//! 兼容性：v0.8 及更早版本该文件只含背景设置（顶层字段，camelCase）。
//! 本结构把背景字段保持在同一层并全部带 `serde(default)`，旧文件可直接反序列化，
//! 缺失的开关字段回落到默认值，无需迁移逻辑；写入时开关与背景同文件原子更新。

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::background::BackgroundSettings;

fn default_true() -> bool {
    true
}
fn default_theme() -> String {
    "system".to_string()
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

/// 持久化的应用偏好（与 AppState 中可持久化的字段一一对应）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPrefs {
    #[serde(default = "default_true")]
    pub enabled: bool, // 桌面图标功能是否激活
    #[serde(default)]
    pub taskbar_transparent: bool, // 透明任务栏
    #[serde(default)]
    pub performance_monitor: bool, // 主机性能监控
    #[serde(default = "default_theme")]
    pub theme: String, // light / dark / system
    #[serde(default = "default_true")]
    pub close_to_tray: bool, // 关闭到托盘

    // —— 背景设置（保持旧 settings.json 的顶层布局，向后兼容）——
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

impl Default for AppPrefs {
    fn default() -> Self {
        Self {
            enabled: true,
            taskbar_transparent: false,
            performance_monitor: false,
            theme: default_theme(),
            close_to_tray: true,
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

impl AppPrefs {
    /// 提取背景设置（数值已夹紧，非法枚举回落默认）
    pub fn background(&self) -> BackgroundSettings {
        BackgroundSettings {
            image_path: self.image_path.clone(),
            fit: self.fit.clone(),
            dim: self.dim,
            blur: self.blur,
            scale: self.scale,
            position_x: self.position_x,
            position_y: self.position_y,
        }
        .clamped()
    }
}

fn settings_path() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("CloudSatchel")
        .join("settings.json")
}

/// 读取持久化偏好；文件缺失 / 损坏时回落到默认值
pub fn load() -> AppPrefs {
    let Ok(text) = fs::read_to_string(settings_path()) else {
        return AppPrefs::default();
    };
    let mut prefs = serde_json::from_str::<AppPrefs>(&text).unwrap_or_default();
    if !matches!(prefs.theme.as_str(), "light" | "dark" | "system") {
        prefs.theme = default_theme();
    }
    prefs
}

/// 原子写入（tmp + rename），失败返回错误信息
pub fn save(prefs: &AppPrefs) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())
}
