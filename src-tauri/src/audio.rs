//! 音频识别（FR-18）：桌面右下角媒体面板的数据源
//!
//! - SMTC（`GlobalSystemMediaTransportControlsSessionManager`）读取当前播放会话
//!   （应用名、标题、播放状态、进度），并发送上一首 / 暂停 / 播放 / 下一首控制命令。
//!   仅本地系统 API，不联网、不写注册表。
//! - WASAPI loopback 采集系统音频输出 + FFT 生成 16 档波形数据（`audio-wave` 事件）；
//!   无播放时停止采集（慢轮询空转），降低开销。
//!
//! 事件：
//! - `media-state`（MediaState JSON）：媒体会话状态，约 1 秒轮询
//! - `audio-wave`（number[16]）：波形频段能量，约 100ms 一帧（仅播放时）
//!
//! 兼容性：SMTC 需要 Win10 1809+；不支持的版本 `supported=false`，前端隐藏面板。

#![allow(non_snake_case)]

use std::mem::size_of;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use base64::Engine;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use windows::ApplicationModel::AppInfo;
use windows::core::HSTRING;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionMediaProperties,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};
use windows::Storage::Streams::{DataReader, IInputStream, IRandomAccessStreamReference};
use windows::Win32::Media::Audio::{
    eMultimedia, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
    AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, WAVEFORMATEX,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};

use crate::dlog;

/// SMTC 轮询间隔（毫秒）
const SMTC_POLL_MS: u64 = 1000;
/// 波形采集帧间隔（毫秒，仅播放时）
const WAVE_POLL_MS: u64 = 100;
/// 波形频段数（前端渲染条数）
pub const WAVE_BANDS: usize = 16;
/// FFT 点数（radix-2）
const FFT_SIZE: usize = 512;

static RUNNING: AtomicBool = AtomicBool::new(true);
static ENABLED: AtomicBool = AtomicBool::new(false);
static THREAD: OnceLock<()> = OnceLock::new();

/// 封面缓存：键 = 曲目标识（标题|艺术家|专辑|应用），值 = data URL。
/// SMTC 缩略图几乎不随同一首歌变化，缓存避免每 1 秒重新解码 / base64 大图。
static THUMB_CACHE: Mutex<Option<(String, String)>> = Mutex::new(None);

/// 媒体会话状态（emit `media-state`）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    /// 是否有可用的媒体会话（且系统支持 SMTC）
    pub active: bool,
    /// 是否正在播放（暂停时为 false）
    pub playing: bool,
    /// 专辑 / 视频封面 data URL（`data:<mime>;base64,`，无封面时为空串；前端隐藏占位）
    pub thumbnail: String,
    /// 播放应用名（人类可读，如 "Apple Music"；未知时为空串）
    pub app_name: String,
    /// 媒体标题
    pub title: String,
    /// 艺术家 / 作者
    pub artist: String,
    /// 专辑名
    pub album: String,
    /// 当前进度（秒）
    pub position_secs: f64,
    /// 总时长（秒）
    pub duration_secs: f64,
    /// 上一首可用（网页音频等场景为 false，前端置灰）
    pub prev_enabled: bool,
    /// 下一首可用
    pub next_enabled: bool,
    /// 播放 / 暂停可用
    pub play_enabled: bool,
    /// 暂停可用
    pub pause_enabled: bool,
    /// 系统是否支持 SMTC（Win10 1809+）
    pub supported: bool,
}

impl MediaState {
    fn idle() -> Self {
        Self {
            active: false,
            playing: false,
            thumbnail: String::new(),
            app_name: String::new(),
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            position_secs: 0.0,
            duration_secs: 0.0,
            prev_enabled: false,
            next_enabled: false,
            play_enabled: false,
            pause_enabled: false,
            supported: true,
        }
    }
}

// ---------------------------------------------------------------------------
// 公共接口
// ---------------------------------------------------------------------------

/// 启动音频识别轮询线程（幂等）
pub fn start(app: AppHandle) {
    RUNNING.store(true, Ordering::SeqCst);
    THREAD.get_or_init(|| {
        let app_smtc = app.clone();
        let app_wave = app.clone();
        std::thread::spawn(move || smtc_loop(app_smtc));
        std::thread::spawn(move || wave_loop(app_wave));
    });
}

/// 停止轮询并清理（退出应用时调用）
pub fn stop() {
    RUNNING.store(false, Ordering::SeqCst);
}

/// 开关：false 时停止采集并推送空闲状态（面板隐藏）
pub fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::SeqCst);
}

/// 发送媒体控制命令（前端按钮点击）：
/// action: "play" / "pause" / "next" / "prev"
pub fn control(app: AppHandle, action: &str) {
    let action = action.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let result = execute_control(&action);
        if let Err(e) = result {
            dlog::write(&format!("[audio] control {action} failed: {e}"));
        }
        let _ = app.emit("media-control-done", ());
    });
}

// ---------------------------------------------------------------------------
// SMTC 轮询
// ---------------------------------------------------------------------------

/// 安全地初始化 COM（MTA）
unsafe fn co_init() -> bool {
    CoInitializeEx(None, COINIT_MULTITHREADED).is_ok()
}

fn smtc_loop(app: AppHandle) {
    dlog::write("[audio] smtc loop started");
    let need_uninit = unsafe { co_init() };
    let mut last_state: Option<String> = None;

    loop {
        std::thread::sleep(Duration::from_millis(SMTC_POLL_MS));
        if !RUNNING.load(Ordering::SeqCst) {
            break;
        }
        if !ENABLED.load(Ordering::SeqCst) {
            if last_state.take().is_some() {
                let _ = app.emit("media-state", MediaState::idle());
            }
            continue;
        }

        let state = read_current_session();
        // 仅状态变化时推送，减少事件量（进度每 5 秒强制刷新一次亦可；简单起见状态变化才推）
        let key = serde_json::to_string(&state).unwrap_or_default();
        if last_state.as_deref() != Some(key.as_str()) {
            last_state = Some(key);
            let _ = app.emit("media-state", state);
        }
    }
    if need_uninit {
        unsafe { CoUninitialize() };
    }
    dlog::write("[audio] smtc loop stopped");
}

/// 读取当前 SMTC 会话状态（失败 / 无会话 → idle）
fn read_current_session() -> MediaState {
    let result = (|| -> windows::core::Result<MediaState> {
        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.get()?;
        let session = manager.GetCurrentSession()?;
        let playback = session.GetPlaybackInfo()?;
        let controls = playback.Controls()?;
        let timeline = session.GetTimelineProperties()?;
        let props = session.TryGetMediaPropertiesAsync()?.get()?;

        let status = playback.PlaybackStatus()?;
        let playing = status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;
        // TimeSpan.Duration 单位为 100 纳秒
        let position = timeline.Position()?.Duration.max(0) as f64 / 10_000_000.0;
        let end = timeline.EndTime()?.Duration.max(0) as f64 / 10_000_000.0;

        let title = props.Title()?.to_string();
        let artist = props.Artist()?.to_string();
        let album = props.AlbumTitle()?.to_string();
        // 应用名：AUMID → 人类可读显示名（如 AppleInc.AppleMusicWin_xxx!App → "Apple Music"）
        let app_name = app_display_name(
            &session.SourceAppUserModelId().unwrap_or_default().to_string(),
        );

        // 封面缩略图（带缓存：同一首歌不反复读取）
        let thumb_key = format!("{title}|{artist}|{album}|{app_name}");
        let thumbnail = cached_thumbnail(&thumb_key, || read_thumbnail(&props));

        Ok(MediaState {
            active: true,
            playing,
            thumbnail,
            app_name,
            title,
            artist,
            album,
            position_secs: position,
            duration_secs: end,
            prev_enabled: controls.IsPreviousEnabled().unwrap_or(false),
            next_enabled: controls.IsNextEnabled().unwrap_or(false),
            play_enabled: controls.IsPlayEnabled().unwrap_or(true),
            pause_enabled: controls.IsPauseEnabled().unwrap_or(true),
            supported: true,
        })
    })();

    match result {
        Ok(state) => state,
        Err(_) => {
            // 无会话（GetCurrentSession 返回 null → Err）或系统不支持 SMTC
            let mut idle = MediaState::idle();
            idle.supported = smtc_available();
            idle
        }
    }
}

/// 按缓存键返回缩略图；未命中时调用 `load` 生成并写入缓存。
/// SMTC 缩略图解码 / base64 较耗，同一首歌只读取一次。
fn cached_thumbnail(key: &str, load: impl FnOnce() -> String) -> String {
    if let Ok(guard) = THUMB_CACHE.lock() {
        if let Some((k, v)) = guard.as_ref() {
            if k == key {
                return v.clone();
            }
        }
    }
    let value = load();
    if let Ok(mut guard) = THUMB_CACHE.lock() {
        *guard = Some((key.to_string(), value.clone()));
    }
    value
}

/// 读取 SMTC 缩略图流 → data URL（`data:<mime>;base64,`）。
/// 失败 / 无封面 / 无法识别的图片格式 → 返回空串（前端隐藏封面占位）。
fn read_thumbnail(props: &GlobalSystemMediaTransportControlsSessionMediaProperties) -> String {
    const MAX_BYTES: u32 = 1_500_000; // 缩略图上限约 1.5MB，防异常大图拖垮轮询
    let result = (|| -> windows::core::Result<String> {
        let thumb: IRandomAccessStreamReference = props.Thumbnail()?;
        let stream = thumb.OpenReadAsync()?.get()?;
        let size = stream.Size()?.min(MAX_BYTES as u64) as u32;
        if size < 4 {
            return Ok(String::new()); // 空缩略图
        }
        let input: IInputStream = stream.GetInputStreamAt(0)?;
        // GetCurrentInputStream 更省事；这里显式 GetInputStreamAt(0) 保证从头读
        let reader = DataReader::CreateDataReader(&input)?;
        let loaded = reader.LoadAsync(size)?.get()?;
        if loaded == 0 {
            return Ok(String::new());
        }
        let mut bytes = vec![0u8; loaded as usize];
        reader.ReadBytes(&mut bytes)?;

        // 用文件头嗅探真实图片格式（SMTC ContentType 常不准/为空）
        let mime = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            "image/png"
        } else if bytes.starts_with(b"\xff\xd8\xff") {
            "image/jpeg"
        } else if bytes.starts_with(b"GIF8") {
            "image/gif"
        } else if bytes.starts_with(b"BM") {
            "image/bmp"
        } else {
            return Ok(String::new());
        };
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        Ok(format!("data:{mime};base64,{b64}"))
    })();
    result.unwrap_or_default()
}

/// 系统是否支持 SMTC（尝试创建 manager，成功即支持）
fn smtc_available() -> bool {
    GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .map(|op| op.get().is_ok())
        .unwrap_or(false)
}

/// 把 AUMID 转成人类可读的应用显示名。
/// 打包应用（MSIX）可通过 AppInfo 查询 DisplayName（如 "Apple Music"）；
/// 非打包应用（如 wmplayer）无 AUMID 显示名，回退为去掉 !App 后缀的原始串。
fn app_display_name(aumid: &str) -> String {
    if aumid.is_empty() {
        return String::new();
    }
    if let Ok(info) = AppInfo::GetFromAppUserModelId(&HSTRING::from(aumid)) {
        if let Ok(display) = info.DisplayInfo() {
            if let Ok(name) = display.DisplayName() {
                let s = name.to_string();
                if !s.is_empty() {
                    return s;
                }
            }
        }
    }
    aumid.split('!').next().unwrap_or(aumid).to_string()
}

fn execute_control(action: &str) -> windows::core::Result<()> {
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.get()?;
    let session = manager.GetCurrentSession()?;
    match action {
        "play" => {
            session.TryPlayAsync()?.get()?;
        }
        "pause" => {
            session.TryPauseAsync()?.get()?;
        }
        "next" => {
            session.TrySkipNextAsync()?.get()?;
        }
        "prev" => {
            session.TrySkipPreviousAsync()?.get()?;
        }
        _ => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// WASAPI loopback 波形采集
// ---------------------------------------------------------------------------

fn wave_loop(app: AppHandle) {
    dlog::write("[audio] wave loop started");
    let need_uninit = unsafe { co_init() };
    let mut client: Option<IAudioClient> = None;
    let mut capture: Option<IAudioCaptureClient> = None;
    let mut fmt: Option<AudioFormat> = None;
    // 连续静音计数（每 100ms 一次）；超过阈值释放采集客户端（停止采集，降低开销）
    let mut silent_ticks: u32 = 0;

    loop {
        std::thread::sleep(Duration::from_millis(if capture.is_none() {
            // 已释放（静音中）：慢轮询等待重新播放
            500
        } else {
            WAVE_POLL_MS
        }));
        if !RUNNING.load(Ordering::SeqCst) {
            break;
        }
        if !ENABLED.load(Ordering::SeqCst) {
            // 开关关闭：释放客户端，空转等待
            if capture.is_some() {
                let _ = client.as_ref().map(|c| unsafe { c.Stop() });
                client = None;
                capture = None;
                fmt = None;
            }
            continue;
        }

        // 懒初始化 loopback 客户端（首次播放时创建）
        if capture.is_none() {
            match unsafe { init_loopback() } {
                Ok((c, cap, f)) => {
                    client = Some(c);
                    capture = Some(cap);
                    fmt = Some(f);
                }
                Err(e) => {
                    dlog::write(&format!("[audio] loopback init failed: {e}"));
                    continue;
                }
            }
        }

        let cap = capture.as_ref().unwrap();
        let f = fmt.as_ref().unwrap();
        unsafe {
            let mut data: *mut u8 = std::ptr::null_mut();
            let mut frames: u32 = 0;
            let mut flags: u32 = 0;
            if cap.GetBuffer(&mut data, &mut frames, &mut flags, None, None).is_ok()
                && frames > 0
                && !data.is_null()
            {
                let wave = compute_wave(data, frames as usize, f);
                let _ = cap.ReleaseBuffer(frames);
                let has_energy = wave.iter().any(|v| *v > 0.0);
                if has_energy {
                    silent_ticks = 0;
                    let _ = app.emit("audio-wave", wave);
                } else {
                    silent_ticks += 1;
                }
            }
        }
        // 连续静音约 2 秒：释放采集客户端（停止采集），等有声音再重建
        if silent_ticks >= 20 && capture.is_some() {
            let _ = client.as_ref().map(|c| unsafe { c.Stop() });
            client = None;
            capture = None;
            fmt = None;
            silent_ticks = 0;
        }
    }
    if need_uninit {
        unsafe { CoUninitialize() };
    }
    dlog::write("[audio] wave loop stopped");
}

/// 解析后的音频格式（供样本解码）
#[derive(Clone, Copy)]
struct AudioFormat {
    sample_rate: u32,
    channels: usize,
    bits: usize,
    is_float: bool,
}

/// 初始化 WASAPI loopback 捕获客户端（默认渲染设备）
unsafe fn init_loopback() -> windows::core::Result<(IAudioClient, IAudioCaptureClient, AudioFormat)> {
    let enumerator: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER)?;
    let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
    let client: IAudioClient = device.Activate(CLSCTX_INPROC_SERVER, None)?;

    let mix_ptr = client.GetMixFormat()?;
    let fmt = *mix_ptr;
    // GetMixFormat 通常返回 WAVEFORMATEXTENSIBLE（wFormatTag=0xFFFE，cbSize=22）；
    // 必须完整复制头部 + cbSize 字节，截断会导致 Initialize 报 E_INVALIDARG
    let total = size_of::<WAVEFORMATEX>() + fmt.cbSize as usize;
    let mut format_buf = vec![0u8; total];
    std::ptr::copy_nonoverlapping(mix_ptr as *const u8, format_buf.as_mut_ptr(), total);
    windows::Win32::System::Com::CoTaskMemFree(Some(mix_ptr as *const core::ffi::c_void));

    let af = parse_format(&fmt);
    client.Initialize(
        AUDCLNT_SHAREMODE_SHARED,
        AUDCLNT_STREAMFLAGS_LOOPBACK,
        0,
        0,
        format_buf.as_ptr() as *const WAVEFORMATEX,
        None,
    )?;
    let capture: IAudioCaptureClient = client.GetService()?;
    client.Start()?;
    Ok((client, capture, af))
}

/// 从 WAVEFORMATEX 头部解析样本格式（支持 WAVEFORMATEXTENSIBLE SubFormat GUID）
fn parse_format(fmt: &WAVEFORMATEX) -> AudioFormat {
    const WAVE_FORMAT_IEEE_FLOAT: u16 = 0x0003;
    const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;
    const KSDATAFORMAT_SUBTYPE_IEEE_FLOAT: windows::core::GUID = windows::core::GUID::from_u128(
        0x00000003_0000_0010_8000_00aa00389b71,
    );
    let is_float = if fmt.wFormatTag == WAVE_FORMAT_IEEE_FLOAT {
        true
    } else if fmt.wFormatTag == WAVE_FORMAT_EXTENSIBLE && fmt.cbSize as usize >= 22 {
        // WAVEFORMATEXTENSIBLE 布局：WAVEFORMATEX（18 字节）+ Samples(2) + dwChannelMask(4) + SubFormat(16)
        let ptr = fmt as *const WAVEFORMATEX as *const u8;
        let sub = unsafe { *(ptr.add(24) as *const windows::core::GUID) };
        sub == KSDATAFORMAT_SUBTYPE_IEEE_FLOAT
    } else {
        false
    };
    AudioFormat {
        sample_rate: fmt.nSamplesPerSec.max(44100),
        channels: fmt.nChannels.max(1) as usize,
        bits: fmt.wBitsPerSample.max(16) as usize,
        is_float,
    }
}

/// 判断样本是否为 IEEE float32（支持 WAVEFORMATEXTENSIBLE 的 SubFormat GUID）——已并入 parse_format
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_format_detects_float() {
        // 标准 PCM WAVEFORMATEX（16bit）
        let pcm = WAVEFORMATEX {
            wFormatTag: 1, // WAVE_FORMAT_PCM
            nChannels: 2,
            nSamplesPerSec: 44100,
            nAvgBytesPerSec: 176400,
            nBlockAlign: 4,
            wBitsPerSample: 16,
            cbSize: 0,
        };
        let f = parse_format(&pcm);
        assert!(!f.is_float);
        assert_eq!(f.sample_rate, 44100);
        assert_eq!(f.channels, 2);
        assert_eq!(f.bits, 16);
    }
}

/// 对采集到的音频帧做混音 + FFT，输出 WAVE_BANDS 个频段能量（0..1）
fn compute_wave(data: *const u8, frames: usize, fmt: &AudioFormat) -> Vec<f32> {
    let channels = fmt.channels;
    let is_float = fmt.is_float;
    let bytes_per_sample = fmt.bits / 8;

    // 混音为单声道 float32，采样到 FFT_SIZE
    let mut samples = vec![0f32; FFT_SIZE];
    let step = channels * bytes_per_sample;
    let mut total = 0usize;
    let mut idx = 0usize;
    while idx < frames * step && total < FFT_SIZE {
        let base = idx;
        let mut sum = 0f32;
        for ch in 0..channels {
            let off = base + ch * bytes_per_sample;
            if is_float && bytes_per_sample >= 4 {
                let v = unsafe { *(data.add(off) as *const f32) };
                sum += v;
            } else if bytes_per_sample >= 2 {
                let v = unsafe { *(data.add(off) as *const i16) } as f32 / 32768.0;
                sum += v;
            } else {
                // 8 位无符号
                let v = unsafe { *data.add(off) } as f32 / 128.0 - 1.0;
                sum += v;
            }
        }
        samples[total] = sum / channels as f32;
        total += 1;
        idx += step;
    }
    if total == 0 {
        return vec![0.0; WAVE_BANDS];
    }

    // 能量过低 → 视为静音
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / total as f32).sqrt();
    if rms < 0.003 {
        return vec![0.0; WAVE_BANDS];
    }

    // radix-2 迭代 FFT（只算幅度谱）
    let mut re = samples.clone();
    let mut im = vec![0f32; FFT_SIZE];
    let n = FFT_SIZE;
    // 位反转置换
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2usize;
    while len <= n {
        let ang = -2.0 * std::f32::consts::PI / len as f32;
        let wlen_re = ang.cos();
        let wlen_im = ang.sin();
        let half = len / 2;
        let mut i = 0usize;
        while i < n {
            let mut w_re = 1f32;
            let mut w_im = 0f32;
            for k in 0..half {
                let u_re = re[i + k];
                let u_im = im[i + k];
                let v_re = re[i + k + half] * w_re - im[i + k + half] * w_im;
                let v_im = re[i + k + half] * w_im + im[i + k + half] * w_re;
                re[i + k] = u_re + v_re;
                im[i + k] = u_im + v_im;
                re[i + k + half] = u_re - v_re;
                im[i + k + half] = u_im - v_im;
                let nw_re = w_re * wlen_re - w_im * wlen_im;
                let nw_im = w_re * wlen_im + w_im * wlen_re;
                w_re = nw_re;
                w_im = nw_im;
            }
            i += len;
        }
        len <<= 1;
    }

    // 幅度谱（前 N/2 个 bin，对称）
    let bins = n / 2;
    let mut mag = vec![0f32; bins];
    let norm = n as f32 / 2.0;
    for i in 0..bins {
        mag[i] = (re[i] * re[i] + im[i] * im[i]).sqrt() / norm;
    }

    // 对数分频段：从 ~2 号 bin 到 bins-1，每段能量取 RMS，动态增益后截断到 0..1
    let sr = fmt.sample_rate as f32;
    let hz_per_bin = sr / n as f32;
    let lo_bin = (60.0 / hz_per_bin).max(1.0) as usize;
    let hi_bin = bins - 1;
    let mut wave = vec![0f32; WAVE_BANDS];
    for b in 0..WAVE_BANDS {
        let t0 = b as f32 / WAVE_BANDS as f32;
        let t1 = (b + 1) as f32 / WAVE_BANDS as f32;
        // 对数分布：bin 范围 [lo, hi]
        let start = lo_bin + ((hi_bin - lo_bin) as f32 * t0) as usize;
        let end = lo_bin + ((hi_bin - lo_bin) as f32 * t1) as usize;
        let end = end.max(start + 1);
        let mut sum = 0f32;
        let mut cnt = 0usize;
        for i in start..end.min(bins) {
            sum += mag[i] * mag[i];
            cnt += 1;
        }
        if cnt > 0 {
            let energy = (sum / cnt as f32).sqrt();
            wave[b] = (energy * 4.0).min(1.0); // 增益 4x 让波形明显
        }
    }
    wave
}
