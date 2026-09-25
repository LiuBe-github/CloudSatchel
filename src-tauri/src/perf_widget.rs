//! 主机性能监控 · 任务栏小组件（FR-03 扩展）
//!
//! 在主屏任务栏的系统托盘区（输入法/时钟所在区域）左侧常驻一个原生小组件，
//! 用 Direct2D + DirectWrite 直接绘制文字：`CPU 12% · 内存 45% · ↑1.2M ↓8.4M`。
//!
//! 设计要点：
//! - 独立线程 + 消息循环；窗口 WS_POPUP + WS_EX_TOOLWINDOW|NOACTIVATE|LAYERED|TRANSPARENT
//!   → 不进 Alt+Tab / 任务栏，不抢焦点，鼠标点击直接穿透到任务栏；
//! - Direct2D `ID2D1DCRenderTarget`（预乘 alpha）绑定 32bpp DIB，DirectWrite 排版，
//!   用 `UpdateLayeredWindow` 提交 → 背景全透明，只有文字；
//! - 数据源复用 `perf::latest()`（不新增采样线程）；渲染节奏 = max(采样间隔, 500ms)；
//! - 位置锚定 `TrayNotifyWnd` 左边界（回退：任务栏右边界 − 240 逻辑像素），
//!   右边缘固定 + 用户微调；仅主屏、仅屏幕底部任务栏；
//! - 任务栏隐藏/自动隐藏动画中/全屏/隐私触发/开关关闭 → 隐藏窗口。
//!
//! 纯净性：不联网、不写注册表（仅只读 SystemUsesLightTheme 判断任务栏深浅色）。

#![allow(non_snake_case)]

use std::ffi::c_void;
use std::mem::size_of;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use windows::core::{w, Interface, PCWSTR};
use windows::Win32::Foundation::{
    COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_RECT_F,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Brush, ID2D1DCRenderTarget, ID2D1Factory, ID2D1SolidColorBrush,
    D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT,
    D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE,
    D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat, DWRITE_FACTORY_TYPE_SHARED,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL,
    DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING,
    DWRITE_TEXT_ALIGNMENT_TRAILING,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetMonitorInfoW,
    MonitorFromWindow, SelectObject, AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO,
    BITMAPINFOHEADER, BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ, MONITORINFO,
    MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, FindWindowExW, FindWindowW,
    GetWindowRect, HTTRANSPARENT, HWND_TOPMOST, IsWindowVisible, MSG, PeekMessageW, PM_REMOVE,
    RegisterClassW, SetWindowPos, ShowWindow, TranslateMessage, UnregisterClassW,
    UpdateLayeredWindow, WM_NCHITTEST, WM_QUIT, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE,
    SW_SHOWNOACTIVATE, ULW_ALPHA,
};

use crate::{dlog, fullscreen, perf, privacy};
// ---------------------------------------------------------------------------
// 配置
// ---------------------------------------------------------------------------

/// 显示项
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ItemKind {
    Cpu,
    Memory,
    GpuTemp,
    Net,
}

/// 把前端传来的字符串列表解析为显示项（白名单 + 去重，顺序即显示顺序）
pub(crate) fn items_from_strings(items: &[String]) -> Vec<ItemKind> {
    let mut out: Vec<ItemKind> = Vec::new();
    for raw in items {
        let kind = match raw.as_str() {
            "cpu" => ItemKind::Cpu,
            "memory" => ItemKind::Memory,
            "gpu_temp" => ItemKind::GpuTemp,
            "net" => ItemKind::Net,
            _ => continue,
        };
        if !out.contains(&kind) {
            out.push(kind);
        }
    }
    out
}

struct Config {
    /// 子开关：任务栏显示
    enabled: bool,
    /// 性能监控总开关（关闭时组件隐藏）
    monitor_on: bool,
    items: Vec<ItemKind>,
    offset_x: i32,
}

static CONFIG: Mutex<Config> = Mutex::new(Config {
    enabled: false,
    monitor_on: false,
    items: Vec::new(),
    offset_x: 0,
});

static THREAD: OnceLock<()> = OnceLock::new();
static STOP: AtomicBool = AtomicBool::new(false);

pub const OFFSET_MIN: i32 = -150;
pub const OFFSET_MAX: i32 = 150;

fn config_snapshot() -> (bool, bool, Vec<ItemKind>, i32) {
    match CONFIG.lock() {
        Ok(c) => (c.enabled, c.monitor_on, c.items.clone(), c.offset_x),
        Err(_) => (false, false, Vec::new(), 0),
    }
}

/// 开启/关闭任务栏小组件（开启时惰性启动渲染线程）
pub fn set_enabled(enabled: bool) {
    if let Ok(mut c) = CONFIG.lock() {
        c.enabled = enabled;
    }
    if enabled {
        THREAD.get_or_init(|| {
            std::thread::spawn(thread_main);
        });
    }
}

/// 同步性能监控总开关（关闭时组件隐藏）
pub fn set_monitor_on(on: bool) {
    if let Ok(mut c) = CONFIG.lock() {
        c.monitor_on = on;
    }
}

/// 同步显示项（顺序即显示顺序；空列表 → 组件隐藏）
pub fn set_items(items: Vec<String>) {
    let parsed = items_from_strings(&items);
    if let Ok(mut c) = CONFIG.lock() {
        c.items = parsed;
    }
}

/// 同步左右微调（逻辑像素，-150 ~ 150）
pub fn set_offset_x(offset: i32) {
    if let Ok(mut c) = CONFIG.lock() {
        c.offset_x = offset.clamp(OFFSET_MIN, OFFSET_MAX);
    }
}

/// 退出应用时停止渲染线程（销毁窗口，避免残留）
pub fn stop() {
    STOP.store(true, Ordering::SeqCst);
}

// ---------------------------------------------------------------------------
// 主题与配色
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Theme {
    Dark,
    Light,
}

/// 只读注册表判断系统任务栏深浅色（读取失败回落深色）
fn read_theme() -> Theme {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER, KEY_QUERY_VALUE,
        REG_DWORD,
    };
    unsafe {
        let mut key = std::ptr::null_mut();
        let status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            windows_sys::core::w!(
                "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"
            ),
            0,
            KEY_QUERY_VALUE,
            &mut key,
        );
        if status != 0 {
            return Theme::Dark;
        }
        let mut value: u32 = 0;
        let mut size: u32 = size_of::<u32>() as u32;
        let mut kind: u32 = 0;
        let ok = RegQueryValueExW(
            key,
            windows_sys::core::w!("SystemUsesLightTheme"),
            std::ptr::null_mut(),
            &mut kind,
            &mut value as *mut u32 as *mut u8,
            &mut size,
        ) == 0;
        RegCloseKey(key);
        if ok && kind == REG_DWORD && value != 0 {
            Theme::Light
        } else {
            Theme::Dark
        }
    }
}

type Color = [f32; 4];

fn text_color(theme: Theme) -> Color {
    match theme {
        Theme::Dark => [1.0, 1.0, 1.0, 0.94],
        Theme::Light => [0.07, 0.07, 0.09, 0.92],
    }
}

fn label_color(theme: Theme) -> Color {
    match theme {
        Theme::Dark => [1.0, 1.0, 1.0, 0.62],
        Theme::Light => [0.07, 0.07, 0.09, 0.55],
    }
}

/// 描边色：与文字反相，保证透明任务栏压在任意壁纸上都可读
fn halo_color(theme: Theme) -> Color {
    match theme {
        Theme::Dark => [0.0, 0.0, 0.0, 0.45],
        Theme::Light => [1.0, 1.0, 1.0, 0.62],
    }
}

/// CPU / 内存占用率配色：>90% 红、>80% 橙、其余用主题文字色
pub(crate) fn load_color(usage: f32, theme: Theme) -> Color {
    if usage > 90.0 {
        [1.0, 0.35, 0.33, 0.98]
    } else if usage > 80.0 {
        [1.0, 0.69, 0.13, 0.98]
    } else {
        text_color(theme)
    }
}

/// CPU / GPU 温度配色：>85°C 红、>75°C 橙；读不到温度时用次级色
pub(crate) fn temp_color(celsius: Option<f32>, theme: Theme) -> Color {
    match celsius {
        Some(t) if t > 85.0 => [1.0, 0.35, 0.33, 0.98],
        Some(t) if t > 75.0 => [1.0, 0.69, 0.13, 0.98],
        Some(_) => text_color(theme),
        None => label_color(theme),
    }
}

/// 网速上行（蓝）
fn net_up_color() -> Color {
    [0.35, 0.72, 0.98, 0.96]
}

/// 网速下行（绿）
fn net_down_color() -> Color {
    [0.45, 0.80, 0.50, 0.96]
}
// ---------------------------------------------------------------------------
// 文本布局（纯逻辑，便于单测）
// ---------------------------------------------------------------------------

/// 值列类型：不同指标的值列有各自的最大宽度参考串
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ColKind {
    Percent,
    Rate,
    /// 温度（参考串 `100°C`）
    Temp,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SegKind {
    Label,
    Value(ColKind),
    Separator,
}

#[derive(Clone, Debug)]
pub(crate) struct Segment {
    pub text: String,
    pub kind: SegKind,
    pub color: Color,
}

/// 速率格式化：自动进位 B/K/M/G，<100 保留 1 位小数（如 `1.2K`、`8.4M`、`105M`）
pub(crate) fn format_rate(bytes_per_sec: f64) -> String {
    let v = if bytes_per_sec.is_finite() && bytes_per_sec > 0.0 {
        bytes_per_sec
    } else {
        0.0
    };
    const UNITS: [(&str, f64); 3] = [
        ("G", 1024.0 * 1024.0 * 1024.0),
        ("M", 1024.0 * 1024.0),
        ("K", 1024.0),
    ];
    for (suffix, div) in UNITS {
        if v >= div {
            let x = v / div;
            return if x >= 100.0 {
                format!("{x:.0}{suffix}")
            } else {
                format!("{x:.1}{suffix}")
            };
        }
    }
    format!("{v:.0}B")
}

fn format_percent(usage: f32) -> String {
    let v = if usage.is_finite() {
        usage.clamp(0.0, 999.0)
    } else {
        0.0
    };
    format!("{v:.0}%")
}

/// 温度格式化：四舍五入到整数摄氏度；读不到时显示 `--`
pub(crate) fn format_temp(celsius: Option<f32>) -> String {
    match celsius {
        Some(t) if t.is_finite() => format!("{:.0}°C", t.clamp(-99.0, 999.0)),
        _ => "--".to_string(),
    }
}

/// 组装渲染片段：项顺序 = 用户配置顺序，项间用 `·` 分隔
pub(crate) fn build_segments(
    items: &[ItemKind],
    snap: Option<&perf::PerfSnapshot>,
    theme: Theme,
) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push(Segment {
                text: "·".to_string(),
                kind: SegKind::Separator,
                color: label_color(theme),
            });
        }
        match item {
            ItemKind::Cpu => {
                let usage = snap.map(|s| s.cpu.usage).unwrap_or(0.0);
                out.push(Segment {
                    text: "CPU".to_string(),
                    kind: SegKind::Label,
                    color: label_color(theme),
                });
                out.push(Segment {
                    text: format_percent(usage),
                    kind: SegKind::Value(ColKind::Percent),
                    color: load_color(usage, theme),
                });
            }
            ItemKind::Memory => {
                let usage = snap.map(|s| s.memory.usage).unwrap_or(0.0);
                out.push(Segment {
                    text: "内存".to_string(),
                    kind: SegKind::Label,
                    color: label_color(theme),
                });
                out.push(Segment {
                    text: format_percent(usage),
                    kind: SegKind::Value(ColKind::Percent),
                    color: load_color(usage, theme),
                });
            }
            ItemKind::GpuTemp => {
                let temp = snap.and_then(|s| s.gpu.temperature);
                out.push(Segment {
                    text: "GPU".to_string(),
                    kind: SegKind::Label,
                    color: label_color(theme),
                });
                out.push(Segment {
                    text: format_temp(temp),
                    kind: SegKind::Value(ColKind::Temp),
                    color: temp_color(temp, theme),
                });
            }
            ItemKind::Net => {
                let (up, down) = snap
                    .map(|s| (s.network.upload_bytes_per_sec, s.network.download_bytes_per_sec))
                    .unwrap_or((0.0, 0.0));
                out.push(Segment {
                    text: "↑".to_string(),
                    kind: SegKind::Label,
                    color: net_up_color(),
                });
                out.push(Segment {
                    text: format_rate(up),
                    kind: SegKind::Value(ColKind::Rate),
                    color: net_up_color(),
                });
                out.push(Segment {
                    text: "↓".to_string(),
                    kind: SegKind::Label,
                    color: net_down_color(),
                });
                out.push(Segment {
                    text: format_rate(down),
                    kind: SegKind::Value(ColKind::Rate),
                    color: net_down_color(),
                });
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 几何（纯逻辑，便于单测）
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WidgetRect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

/// 计算组件窗口矩形；不满足条件（非底部任务栏 / 任务栏过窄 / 尺寸非法）→ None（不显示）
pub(crate) fn compute_rect(
    taskbar: RECT,
    monitor: RECT,
    tray_left: i32,
    width: i32,
    scale: f32,
    offset_x: i32,
) -> Option<WidgetRect> {
    if width <= 0 {
        return None;
    }
    let mon_w = monitor.right - monitor.left;
    let mon_h = monitor.bottom - monitor.top;
    let bar_w = taskbar.right - taskbar.left;
    let bar_h = taskbar.bottom - taskbar.top;
    if mon_w <= 0 || mon_h <= 0 || bar_h < 16 {
        return None;
    }
    // 仅支持屏幕底部的横向任务栏：底边贴显示器底边，且横向铺满（排除顶部/竖向任务栏）
    if (taskbar.bottom - monitor.bottom).abs() > 2 {
        return None;
    }
    if bar_w * 10 < mon_w * 8 {
        return None;
    }
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let gap = (6.0 * scale).round() as i32;
    let offset = (offset_x as f32 * scale).round() as i32;
    let right = tray_left - gap + offset;
    let mut left = right - width;
    if left < monitor.left + 2 {
        left = monitor.left + 2;
    }
    Some(WidgetRect {
        left,
        top: taskbar.top,
        width,
        height: bar_h,
    })
}

/// 贪心分两行：按显示项宽度依次放入第一行，放不下则整项挪到第二行（最多两行）
pub(crate) fn plan_rows(widths: &[f32], pad: f32, sep_total: f32, max_row: f32) -> Vec<Vec<usize>> {
    let mut rows: Vec<Vec<usize>> = vec![Vec::new()];
    let mut used: Vec<f32> = vec![pad * 2.0];
    for (gi, w) in widths.iter().enumerate() {
        let empty = rows.last().map(|r| r.is_empty()).unwrap_or(true);
        let extra = if empty { 0.0 } else { sep_total };
        let current = used.last().copied().unwrap_or(0.0);
        if rows.len() < 2 && !empty && current + extra + w > max_row {
            rows.push(vec![gi]);
            used.push(pad * 2.0 + w);
        } else {
            rows.last_mut().unwrap().push(gi);
            *used.last_mut().unwrap() += extra + w;
        }
    }
    rows
}

/// 任务栏是否已滑出/隐藏（系统自带自动隐藏）
///
/// 底部任务栏隐藏时是向下滑出屏幕：窗口整体下移，屏幕内可见高度不断减小；
/// 可见部分不足一半（或完全滑出）即视为隐藏中，组件随之隐藏。
pub(crate) fn taskbar_slid_out(taskbar: RECT, monitor: RECT) -> bool {
    let bar_h = (taskbar.bottom - taskbar.top).max(1);
    let visible = monitor.bottom - taskbar.top;
    if visible <= 0 {
        return true;
    }
    visible * 2 < bar_h
}
// ---------------------------------------------------------------------------
// 渲染（Direct2D + DirectWrite + 分层窗口）
// ---------------------------------------------------------------------------

const WINDOW_CLASS: PCWSTR = w!("CloudSatchelPerfWidget");
const FONT_FAMILY: PCWSTR = w!("Microsoft YaHei UI");
const LOCALE: PCWSTR = w!("zh-CN");

/// 托盘锚点回退宽度（逻辑像素）：取不到 TrayNotifyWnd 时按任务栏右边界回退
const TRAY_FALLBACK_LOGICAL: f32 = 240.0;
/// 水平内边距（逻辑像素）
const PADDING_LOGICAL: f32 = 10.0;
/// 片段间距（逻辑像素）
const SEG_GAP_LOGICAL: f32 = 8.0;
/// 标签与其数值之间的小间距（逻辑像素；用户要求更紧凑 → 1.0）
const LABEL_VALUE_GAP_LOGICAL: f32 = 1.0;
/// 分隔符两侧间距（逻辑像素；用户要求更紧凑 → 3.5）
const SEP_GAP_LOGICAL: f32 = 3.5;
/// 单行内容最大宽度（逻辑像素）：超出则自动分为上下两行
const MAX_ROW_LOGICAL: f32 = 480.0;

struct RunPlan {
    text: Vec<u16>,
    x: f32,
    /// 该片段所在行的顶部 Y（单行时为 0；两行时为 0 / 半高）
    y: f32,
    width: f32,
    /// 该片段所在行的高度（用于垂直居中）
    height: f32,
    trailing: bool,
    color: Color,
}

struct Plan {
    width: i32,
    height: i32,
    runs: Vec<RunPlan>,
}

struct Renderer {
    factory: ID2D1Factory,
    dwrite: IDWriteFactory,
    fmt_left: IDWriteTextFormat,
    fmt_right: IDWriteTextFormat,
    rt: Option<ID2D1DCRenderTarget>,
    brush: Option<ID2D1SolidColorBrush>,
    dc: HDC,
    dib: HBITMAP,
    old_dib: HGDIOBJ,
    width: i32,
    height: i32,
    font_px: f32,
    /// 是否已记录首帧日志（供远程排查“组件不显示”）
    logged_first_frame: bool,
}

impl Renderer {
    fn new() -> windows::core::Result<Self> {
        unsafe {
            let factory: ID2D1Factory =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let dwrite: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let fmt_left = dwrite.CreateTextFormat(
                FONT_FAMILY,
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                14.0,
                LOCALE,
            )?;
            fmt_left.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING)?;
            fmt_left.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            let fmt_right = dwrite.CreateTextFormat(
                FONT_FAMILY,
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                14.0,
                LOCALE,
            )?;
            fmt_right.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_TRAILING)?;
            fmt_right.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            Ok(Self {
                factory,
                dwrite,
                fmt_left,
                fmt_right,
                rt: None,
                brush: None,
                dc: HDC::default(),
                dib: HBITMAP::default(),
                old_dib: HGDIOBJ::default(),
                width: 0,
                height: 0,
                font_px: 0.0,
                logged_first_frame: false,
            })
        }
    }

    fn utf16(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    /// 测量文本宽度（物理像素）
    fn measure(&self, text: &str, trailing: bool) -> f32 {
        let wide = Self::utf16(text);
        if wide.is_empty() {
            return 0.0;
        }
        let fmt = if trailing { &self.fmt_right } else { &self.fmt_left };
        unsafe {
            match self.dwrite.CreateTextLayout(&wide, fmt, 4096.0, 512.0) {
                Ok(layout) => {
                    let mut metrics =
                        windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_METRICS::default();
                    let _ = layout.GetMetrics(&mut metrics);
                    metrics.width
                }
                Err(_) => 0.0,
            }
        }
    }

    /// 按字号重建文本格式（IDWriteTextFormat 基础接口不提供 SetFontSize）
    fn ensure_font_size(&mut self, font_px: f32) -> bool {
        if self.font_px > 0.0 && (self.font_px - font_px).abs() < 0.01 {
            return true;
        }
        unsafe {
            let left = self.dwrite.CreateTextFormat(
                FONT_FAMILY,
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                font_px,
                LOCALE,
            );
            let right = self.dwrite.CreateTextFormat(
                FONT_FAMILY,
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                font_px,
                LOCALE,
            );
            let (Ok(left), Ok(right)) = (left, right) else {
                return false;
            };
            if left.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING).is_err()
                || left.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER).is_err()
                || right.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_TRAILING).is_err()
                || right.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER).is_err()
            {
                return false;
            }
            self.fmt_left = left;
            self.fmt_right = right;
            self.font_px = font_px;
        }
        true
    }

    /// 单个显示项（不含项间分隔符）的渲染宽度
    fn group_width(&self, group: &[&Segment], scale: f32, cols: &(f32, f32, f32)) -> f32 {
        let lv_gap = LABEL_VALUE_GAP_LOGICAL * scale;
        let seg_gap = SEG_GAP_LOGICAL * scale;
        let mut w = 0.0;
        for (i, seg) in group.iter().enumerate() {
            if i > 0 {
                let trailing = matches!(seg.kind, SegKind::Value(_));
                w += if trailing && matches!(group[i - 1].kind, SegKind::Label) {
                    lv_gap
                } else {
                    seg_gap
                };
            }
            w += self.segment_width(seg, cols);
        }
        w
    }

    /// 单个片段的渲染宽度（值列按参考串取固定宽度，保证数字变化不抖）
    fn segment_width(&self, seg: &Segment, cols: &(f32, f32, f32)) -> f32 {
        let trailing = matches!(seg.kind, SegKind::Value(_));
        let measured = self.measure(&seg.text, trailing);
        match seg.kind {
            SegKind::Value(ColKind::Percent) => measured.max(cols.0),
            SegKind::Value(ColKind::Rate) => measured.max(cols.1),
            SegKind::Value(ColKind::Temp) => measured.max(cols.2),
            _ => measured,
        }
    }

    /// 组装渲染计划：字号随任务栏高度；按项贪心换行（单行超宽 → 上下两行）；
    /// 值列固定宽度右对齐（整行不抖）
    fn build_plan(
        &mut self,
        segments: &[Segment],
        scale: f32,
        height: i32,
        max_row: f32,
        theme: Theme,
    ) -> Plan {
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        let font_min = (11.0 * scale).round();
        let font_max = (16.0 * scale).round();
        let font_px = ((height as f32 * 0.28).round())
            .clamp(font_min, font_max)
            .max(1.0);
        let _ = self.ensure_font_size(font_px);

        let pad = PADDING_LOGICAL * scale;
        let seg_gap = SEG_GAP_LOGICAL * scale;
        let lv_gap = LABEL_VALUE_GAP_LOGICAL * scale;
        let sep_gap = SEP_GAP_LOGICAL * scale;

        // 值列参考宽度（"100%" / "999.9M" / "100°C"）：保证数字变化时整行不抖
        let cols = (
            self.measure("100%", true),
            self.measure("999.9M", true),
            self.measure("100°C", true),
        );

        // 按「项」切分：分隔符只是项之间的连接符，不参与换行
        let mut groups: Vec<Vec<&Segment>> = Vec::new();
        let mut current: Vec<&Segment> = Vec::new();
        for seg in segments {
            if matches!(seg.kind, SegKind::Separator) {
                if !current.is_empty() {
                    groups.push(std::mem::take(&mut current));
                }
            } else {
                current.push(seg);
            }
        }
        if !current.is_empty() {
            groups.push(current);
        }

        let widths: Vec<f32> = groups
            .iter()
            .map(|g| self.group_width(g, scale, &cols))
            .collect();
        let sep_w = self.measure("·", false);

        // 贪心换行（最多两行）：第一行放不下就整项挪到第二行
        let rows = plan_rows(&widths, pad, sep_gap * 2.0 + sep_w, max_row);

        let row_count = rows.len().max(1) as f32;
        let row_h = height as f32 / row_count;
        let mut runs: Vec<RunPlan> = Vec::new();
        let mut block_w: f32 = 0.0;
        for (ri, group_ids) in rows.iter().enumerate() {
            let y = ri as f32 * row_h;
            let mut x = pad;
            for (k, gi) in group_ids.iter().enumerate() {
                if k > 0 {
                    // 项间分隔符（与 build_segments 输出一致的次级色圆点）
                    x += sep_gap;
                    runs.push(RunPlan {
                        text: Self::utf16("·"),
                        x,
                        y,
                        width: sep_w,
                        height: row_h,
                        trailing: false,
                        color: label_color(theme),
                    });
                    x += sep_w + sep_gap;
                }
                let group = &groups[*gi];
                for (si, seg) in group.iter().enumerate() {
                    let trailing = matches!(seg.kind, SegKind::Value(_));
                    let width = self.segment_width(seg, &cols);
                    let gap = if si == 0 {
                        0.0
                    } else if trailing && matches!(group[si - 1].kind, SegKind::Label) {
                        lv_gap
                    } else {
                        seg_gap
                    };
                    x += gap;
                    runs.push(RunPlan {
                        text: Self::utf16(&seg.text),
                        x,
                        y,
                        width,
                        height: row_h,
                        trailing,
                        color: seg.color,
                    });
                    x += width;
                }
            }
            block_w = block_w.max(x + pad);
        }

        Plan {
            width: block_w.round().max(1.0) as i32,
            height,
            runs,
        }
    }
}
impl Renderer {
    /// 重建 DIB / DC / 渲染目标（尺寸变化时）
    fn recreate_surface(&mut self, width: i32, height: i32) -> bool {
        unsafe {
            self.rt = None;
            self.brush = None;
            if !self.dib.is_invalid() {
                if !self.dc.is_invalid() {
                    SelectObject(self.dc, self.old_dib);
                }
                let _ = DeleteObject(HGDIOBJ(self.dib.0));
                self.dib = HBITMAP::default();
            }
            if !self.dc.is_invalid() {
                let _ = DeleteDC(self.dc);
                self.dc = HDC::default();
            }

            let dc = CreateCompatibleDC(None);
            if dc.is_invalid() {
                return false;
            }
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    // 负高度 = 自上而下位图
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits: *mut c_void = std::ptr::null_mut();
            let dib = match CreateDIBSection(Some(dc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
                Ok(b) => b,
                Err(e) => {
                    dlog::write(&format!("[perf-widget] CreateDIBSection 失败: {e}"));
                    let _ = DeleteDC(dc);
                    return false;
                }
            };
            let old = SelectObject(dc, HGDIOBJ(dib.0));

            let props = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };
            let rt = match self.factory.CreateDCRenderTarget(&props) {
                Ok(r) => r,
                Err(e) => {
                    dlog::write(&format!("[perf-widget] CreateDCRenderTarget 失败: {e}"));
                    SelectObject(dc, old);
                    let _ = DeleteObject(HGDIOBJ(dib.0));
                    let _ = DeleteDC(dc);
                    return false;
                }
            };
            let clip = RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            };
            if let Err(e) = rt.BindDC(dc, &clip) {
                dlog::write(&format!("[perf-widget] BindDC 失败: {e}"));
                SelectObject(dc, old);
                let _ = DeleteObject(HGDIOBJ(dib.0));
                let _ = DeleteDC(dc);
                return false;
            }
            self.brush = rt
                .CreateSolidColorBrush(
                    &D2D1_COLOR_F {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    },
                    None,
                )
                .ok();
            self.dc = dc;
            self.dib = dib;
            self.old_dib = old;
            self.rt = Some(rt);
            self.width = width;
            self.height = height;
            true
        }
    }

    /// 绘制 + 通过 UpdateLayeredWindow 提交（位置、尺寸、内容一并更新）
    fn present(&mut self, hwnd: HWND, plan: &Plan, x: i32, y: i32, theme: Theme) -> bool {
        if plan.width <= 0 || plan.height <= 0 {
            return false;
        }
        if self.width != plan.width || self.height != plan.height || self.rt.is_none() {
            if !self.recreate_surface(plan.width, plan.height) {
                return false;
            }
        }
        let (Some(rt), Some(brush)) = (self.rt.clone(), self.brush.clone()) else {
            return false;
        };
        let fmt_left = self.fmt_left.clone();
        let fmt_right = self.fmt_right.clone();

        unsafe {
            rt.BeginDraw();
            // 背景全透明：只有文字压在任务栏/壁纸上
            rt.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));
            rt.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);

            // 描边：先画一圈反相偏移，再画正文，保证任意壁纸上都可读
            let brush_base: ID2D1Brush = match brush.cast() {
                Ok(b) => b,
                Err(e) => {
                    dlog::write(&format!("[perf-widget] 画刷转换失败: {e}"));
                    return false;
                }
            };
            let halo = halo_color(theme);
            brush.SetColor(&D2D1_COLOR_F {
                r: halo[0],
                g: halo[1],
                b: halo[2],
                a: halo[3],
            });
            for run in &plan.runs {
                if run.text.is_empty() {
                    continue;
                }
                let fmt = if run.trailing { &fmt_right } else { &fmt_left };
                for (dx, dy) in [(-1.0f32, 0.0f32), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
                    let rect = D2D_RECT_F {
                        left: run.x + dx,
                        top: run.y + dy,
                        right: run.x + dx + run.width,
                        bottom: run.y + run.height,
                    };
                    rt.DrawText(
                        &run.text,
                        Some(fmt),
                        &rect,
                        Some(&brush_base),
                        D2D1_DRAW_TEXT_OPTIONS_NONE,
                        DWRITE_MEASURING_MODE_NATURAL,
                    );
                }
            }
            for run in &plan.runs {
                if run.text.is_empty() {
                    continue;
                }
                brush.SetColor(&D2D1_COLOR_F {
                    r: run.color[0],
                    g: run.color[1],
                    b: run.color[2],
                    a: run.color[3],
                });
                let fmt = if run.trailing { &fmt_right } else { &fmt_left };
                let rect = D2D_RECT_F {
                    left: run.x,
                    top: run.y,
                    right: run.x + run.width,
                    bottom: run.y + run.height,
                };
                rt.DrawText(
                    &run.text,
                    Some(fmt),
                    &rect,
                    Some(&brush_base),
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                    DWRITE_MEASURING_MODE_NATURAL,
                );
            }
            if let Err(e) = rt.EndDraw(None, None) {
                dlog::write(&format!("[perf-widget] EndDraw 失败: {e}"));
                return false;
            }



            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let dst = POINT { x, y };
            let size = SIZE {
                cx: plan.width,
                cy: plan.height,
            };
            let src = POINT { x: 0, y: 0 };
            let ulw = UpdateLayeredWindow(
                hwnd,
                None,
                Some(&dst),
                Some(&size),
                Some(self.dc),
                Some(&src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
            if !self.logged_first_frame {
                self.logged_first_frame = true;
                dlog::write(&format!(
                    "[perf-widget] 首帧渲染 ok={} rect=({x},{y}) size={}x{} runs={} font={:.0}",
                    ulw.is_ok(),
                    plan.width,
                    plan.height,
                    plan.runs.len(),
                    self.font_px
                ));
            }
            ulw.is_ok()
        }
    }
}
// ---------------------------------------------------------------------------
// 任务栏几何 / 可见性 / 窗口过程 / 线程主循环
// ---------------------------------------------------------------------------

/// 查询任务栏几何与锚点；不满足显示条件时返回 None
fn query_anchor() -> Option<(RECT, RECT, i32, f32)> {
    unsafe {
        let shell = match FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null()) {
            Ok(h) if !h.is_invalid() => h,
            _ => return None,
        };
        if !IsWindowVisible(shell).as_bool() {
            return None;
        }
        if crate::taskbar::is_autohide() || crate::taskbar::is_animating() {
            return None;
        }
        let mut bar = RECT::default();
        if GetWindowRect(shell, &mut bar).is_err() {
            return None;
        }
        let monitor = MonitorFromWindow(shell, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut mi).as_bool() {
            return None;
        }
        let screen = mi.rcMonitor;
        if taskbar_slid_out(bar, screen) {
            return None;
        }
        let dpi = GetDpiForWindow(shell);
        let scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };
        // 锚点：托盘区（输入法/时钟）左边界；取不到时按任务栏右边界回退
        let tray = FindWindowExW(Some(shell), None, w!("TrayNotifyWnd"), PCWSTR::null()).ok();
        let tray_left = match tray {
            Some(t) if !t.is_invalid() => {
                let mut tr = RECT::default();
                if GetWindowRect(t, &mut tr).is_ok() {
                    tr.left
                } else {
                    bar.right - (TRAY_FALLBACK_LOGICAL * scale).round() as i32
                }
            }
            _ => bar.right - (TRAY_FALLBACK_LOGICAL * scale).round() as i32,
        };
        Some((bar, screen, tray_left, scale))
    }
}

/// 组件是否应当隐藏（全屏 / 隐私触发时隐藏，与任务栏一致）
fn blocked_now() -> bool {
    fullscreen::is_any_fullscreen() || fullscreen::is_fullscreen_now() || privacy::is_triggered()
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        // 双保险：命中测试返回“穿透”，点击直接落到任务栏
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn thread_main() {
    unsafe {
        let hinstance = GetModuleHandleW(None)
            .map(|m| HINSTANCE(m.0))
            .unwrap_or(HINSTANCE(std::ptr::null_mut()));
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance,
            lpszClassName: WINDOW_CLASS,
            ..Default::default()
        };
        if RegisterClassW(&wc) == 0 {
            dlog::write("[perf-widget] RegisterClassW 失败");
            return;
        }
        let hwnd = match CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
            WINDOW_CLASS,
            w!("云笈性能"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(hinstance),
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                dlog::write(&format!("[perf-widget] CreateWindowExW 失败: {e}"));
                let _ = UnregisterClassW(WINDOW_CLASS, Some(hinstance));
                return;
            }
        };
        let mut renderer = match Renderer::new() {
            Ok(r) => r,
            Err(e) => {
                dlog::write(&format!("[perf-widget] Direct2D/DirectWrite 初始化失败: {e}"));
                let _ = DestroyWindow(hwnd);
                let _ = UnregisterClassW(WINDOW_CLASS, Some(hinstance));
                return;
            }
        };
        dlog::write("[perf-widget] 任务栏组件线程已启动");

        let mut visible = false;
        let mut logged_text = false;
        let mut last_sample = Instant::now() - Duration::from_secs(60);
        let mut last_signature = String::new();
        let mut last_theme = Theme::Dark;
        let mut snap: Option<perf::PerfSnapshot> = None;

        while !STOP.load(Ordering::SeqCst) {
            // 消息泵（窗口不接收输入，仅保持消息循环健康）
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    STOP.store(true, Ordering::SeqCst);
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let (enabled, monitor_on, items, offset_x) = config_snapshot();
            let active = enabled && monitor_on && !items.is_empty();

            // 采样节流：跟随采样间隔，最快 500ms 一次
            let interval = perf::interval_ms().max(500);
            if active && last_sample.elapsed() >= Duration::from_millis(interval) {
                last_sample = Instant::now();
                snap = perf::latest();
            }

            let hide = |hwnd: HWND, visible: &mut bool, sig: &mut String| {
                if *visible {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                    *visible = false;
                    sig.clear();
                }
            };

            if !active || blocked_now() {
                hide(hwnd, &mut visible, &mut last_signature);
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }

            let Some((bar, screen, tray_left, scale)) = query_anchor() else {
                hide(hwnd, &mut visible, &mut last_signature);
                std::thread::sleep(Duration::from_millis(200));
                continue;
            };

            let theme = read_theme();
            let segments = build_segments(&items, snap.as_ref(), theme);
            // 首次拿到采样数据后记录一次显示文本，便于远程确认内容
            if !logged_text && snap.is_some() {
                logged_text = true;
                let text: String = segments
                    .iter()
                    .map(|s| s.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                dlog::write(&format!("[perf-widget] 文本: {text}"));
            }
            let signature = segments
                .iter()
                .map(|s| format!("{}|{:?}", s.text, s.color))
                .collect::<String>();
            let height = bar.bottom - bar.top;
            // 单行最大宽度：固定上限与屏宽 45% 取小，超宽自动分两行
            let mon_w = (screen.right - screen.left) as f32;
            let max_row = (MAX_ROW_LOGICAL * scale).min(mon_w * 0.45);
            let plan = renderer.build_plan(&segments, scale, height, max_row, theme);
            let Some(rect) = compute_rect(bar, screen, tray_left, plan.width, scale, offset_x)
            else {
                hide(hwnd, &mut visible, &mut last_signature);
                std::thread::sleep(Duration::from_millis(200));
                continue;
            };

            let need_show = !visible;
            if signature != last_signature || theme != last_theme || need_show {
                if renderer.present(hwnd, &plan, rect.left, rect.top, theme) {
                    if need_show {
                        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                        // 置顶到 topmost 带最前，确保压在任务栏之上
                        let _ = SetWindowPos(
                            hwnd,
                            Some(HWND_TOPMOST),
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                        );
                        visible = true;
                    }
                    last_signature = signature;
                    last_theme = theme;
                }
            }

            std::thread::sleep(Duration::from_millis(60));
        }

        let _ = DestroyWindow(hwnd);
        let _ = UnregisterClassW(WINDOW_CLASS, Some(hinstance));
        dlog::write("[perf-widget] 任务栏组件线程已退出");
    }
}
// ---------------------------------------------------------------------------
// 单元测试（纯逻辑部分）
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perf::{CpuMetrics, MemoryMetrics, NetworkMetrics, PerfSnapshot};

    fn snapshot(cpu: f32, mem: f32, up: f64, down: f64) -> PerfSnapshot {
        PerfSnapshot {
            timestamp: 0,
            cpu: CpuMetrics {
                usage: cpu,
                ..Default::default()
            },
            gpu: Default::default(),
            memory: MemoryMetrics {
                usage: mem,
                ..Default::default()
            },
            network: NetworkMetrics {
                upload_bytes_per_sec: up,
                download_bytes_per_sec: down,
                ..Default::default()
            },
        }
    }

    #[test]
    fn rate_formatting_carries_units() {
        assert_eq!(format_rate(0.0), "0B");
        assert_eq!(format_rate(999.0), "999B");
        assert_eq!(format_rate(1228.8), "1.2K");
        assert_eq!(format_rate(8.8 * 1024.0 * 1024.0), "8.8M");
        assert_eq!(format_rate(105.0 * 1024.0 * 1024.0), "105M");
        assert_eq!(format_rate(1.05 * 1024.0 * 1024.0 * 1024.0), "1.1G");
        assert_eq!(format_rate(-5.0), "0B");
        assert_eq!(format_rate(f64::NAN), "0B");
    }

    #[test]
    fn items_respect_order_and_whitelist() {
        let parsed = items_from_strings(&[
            "memory".into(),
            "bogus".into(),
            "cpu".into(),
            "memory".into(),
            "net".into(),
        ]);
        assert_eq!(parsed, vec![ItemKind::Memory, ItemKind::Cpu, ItemKind::Net]);
        assert!(items_from_strings(&[]).is_empty());
        assert!(items_from_strings(&["gpu".into()]).is_empty());
    }

    #[test]
    fn segments_follow_item_order_and_separators() {
        let snap = snapshot(12.4, 45.6, 1228.8, 8.8 * 1024.0 * 1024.0);
        let segs = build_segments(
            &[ItemKind::Cpu, ItemKind::Memory, ItemKind::Net],
            Some(&snap),
            Theme::Dark,
        );
        let texts: Vec<&str> = segs.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(
            texts,
            vec!["CPU", "12%", "·", "内存", "46%", "·", "↑", "1.2K", "↓", "8.8M"]
        );
        assert_eq!(
            segs.iter()
                .filter(|s| s.kind == SegKind::Separator)
                .count(),
            2
        );

        // 用户改顺序：内存在前
        let reordered = build_segments(&[ItemKind::Memory, ItemKind::Cpu], Some(&snap), Theme::Dark);
        assert_eq!(reordered[0].text, "内存");
        assert_eq!(reordered[2].text, "·");
        assert_eq!(reordered[3].text, "CPU");

        // 空列表 → 无片段（组件隐藏）
        assert!(build_segments(&[], Some(&snap), Theme::Dark).is_empty());
    }

    #[test]
    fn temp_items_format_and_threshold_colors() {
        let mut snap = snapshot(10.0, 40.0, 0.0, 0.0);
        snap.gpu.temperature = Some(88.0);
        let segs = build_segments(&[ItemKind::GpuTemp], Some(&snap), Theme::Dark);
        let texts: Vec<&str> = segs.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(texts, vec!["GPU", "88°C"]);
        // 88°C → 红色；80°C → 橙色
        assert_eq!(segs[1].color, [1.0, 0.35, 0.33, 0.98]);
        assert_eq!(temp_color(Some(80.0), Theme::Dark), [1.0, 0.69, 0.13, 0.98]);
        assert_eq!(temp_color(Some(70.0), Theme::Dark), text_color(Theme::Dark));

        // 读不到温度（非 NVIDIA 显卡等）→ 显示 `--` 且用次级色
        let empty = PerfSnapshot::default();
        let segs = build_segments(&[ItemKind::GpuTemp], Some(&empty), Theme::Dark);
        assert_eq!(segs[1].text, "--");
        assert_eq!(segs[1].color, label_color(Theme::Dark));
        assert_eq!(format_temp(None), "--");
        assert_eq!(format_temp(Some(55.6)), "56°C");
    }

    #[test]
    fn rows_wrap_to_two_lines_when_too_wide() {
        // 单行放得下 → 一行
        let rows = plan_rows(&[100.0, 100.0], 8.0, 20.0, 480.0);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], vec![0, 1]);
        // 放不下 → 第二项整项换行
        let rows = plan_rows(&[200.0, 200.0, 200.0], 8.0, 20.0, 480.0);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec![0, 1]);
        assert_eq!(rows[1], vec![2]);
        // 最多两行：第三行不再拆
        let rows = plan_rows(&[400.0, 400.0, 400.0, 400.0], 8.0, 20.0, 480.0);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1], vec![1, 2, 3]);
        // 单项超宽也至少占一行
        let rows = plan_rows(&[900.0], 8.0, 20.0, 480.0);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], vec![0]);
    }

    #[test]
    fn colors_follow_thresholds_and_theme() {
        let dark_text = text_color(Theme::Dark);
        assert_eq!(load_color(50.0, Theme::Dark), dark_text);
        assert_eq!(load_color(85.0, Theme::Dark), [1.0, 0.69, 0.13, 0.98]);
        assert_eq!(load_color(95.0, Theme::Dark), [1.0, 0.35, 0.33, 0.98]);
        assert_ne!(text_color(Theme::Light), dark_text);
        assert_ne!(net_up_color(), net_down_color());
    }

    #[test]
    fn geometry_anchors_left_of_tray_on_bottom_taskbar() {
        let monitor = RECT {
            left: 0,
            top: 0,
            right: 2048,
            bottom: 1152,
        };
        let taskbar = RECT {
            left: 0,
            top: 1104,
            right: 2048,
            bottom: 1152,
        };
        let rect = compute_rect(taskbar, monitor, 1774, 240, 1.0, 0).expect("底部任务栏应可显示");
        assert_eq!(rect.height, 48);
        assert_eq!(rect.top, 1104);
        assert_eq!(rect.width, 240);
        assert_eq!(rect.left + rect.width, 1774 - 6);

        // 用户微调（逻辑像素）
        let shifted = compute_rect(taskbar, monitor, 1774, 240, 1.0, 20).expect("应可显示");
        assert_eq!(shifted.left, rect.left + 20);

        // 125% 缩放：间距与微调按比例放大
        let scaled = compute_rect(taskbar, monitor, 1774, 240, 1.25, 20).expect("应可显示");
        assert_eq!(scaled.left + scaled.width, 1774 - 8 + 25);

        // 贴到屏幕左边界时被钳制
        let clamped = compute_rect(taskbar, monitor, 100, 400, 1.0, -150).expect("应可显示");
        assert_eq!(clamped.left, 2);
    }

    #[test]
    fn geometry_rejects_non_bottom_and_vertical_taskbars() {
        let monitor = RECT {
            left: 0,
            top: 0,
            right: 2048,
            bottom: 1152,
        };
        // 顶部任务栏
        let top_bar = RECT {
            left: 0,
            top: 0,
            right: 2048,
            bottom: 48,
        };
        assert!(compute_rect(top_bar, monitor, 1774, 240, 1.0, 0).is_none());
        // 左侧竖向任务栏
        let left_bar = RECT {
            left: 0,
            top: 0,
            right: 48,
            bottom: 1152,
        };
        assert!(compute_rect(left_bar, monitor, 40, 240, 1.0, 0).is_none());
        // 宽度非法
        let bar = RECT {
            left: 0,
            top: 1104,
            right: 2048,
            bottom: 1152,
        };
        assert!(compute_rect(bar, monitor, 1774, 0, 1.0, 0).is_none());
        // 系统自动隐藏滑出中
        let slid = RECT {
            left: 0,
            top: 1130,
            right: 2048,
            bottom: 1178,
        };
        assert!(taskbar_slid_out(slid, monitor));
        assert!(!taskbar_slid_out(bar, monitor));
    }
}