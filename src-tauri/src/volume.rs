//! 系统音量（v0.20.0）：音频面板音量调节条的数据源
//!
//! Core Audio（IAudioEndpointVolume）读写系统主音量（0.0 ~ 1.0）与静音状态；
//! 仅本地系统 API，不联网、不写注册表。每次操作独立初始化/释放 COM。

#![allow(non_snake_case)]

use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};

/// 获取默认渲染设备音量接口并执行操作（自动初始化 / 释放 COM）
fn with_volume<T>(
    f: impl FnOnce(&IAudioEndpointVolume) -> windows::core::Result<T>,
) -> Result<T, String> {
    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let need_uninit = hr.0 == 0;
        let result = (|| -> windows::core::Result<T> {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
            let volume: IAudioEndpointVolume = device.Activate(CLSCTX_INPROC_SERVER, None)?;
            f(&volume)
        })();
        if need_uninit {
            CoUninitialize();
        }
        result.map_err(|e| format!("系统音量操作失败: {e}"))
    }
}

/// 当前系统主音量（0.0 ~ 1.0）
pub fn get_level() -> Result<f32, String> {
    with_volume(|v| unsafe { v.GetMasterVolumeLevelScalar() })
}

/// 设置系统主音量（自动 clamp 到 0.0 ~ 1.0）
pub fn set_level(level: f32) -> Result<(), String> {
    with_volume(|v| unsafe { v.SetMasterVolumeLevelScalar(level.clamp(0.0, 1.0), std::ptr::null()) })
}

/// 当前是否静音
pub fn get_mute() -> Result<bool, String> {
    with_volume(|v| unsafe { v.GetMute() }.map(|muted| muted.as_bool()))
}

/// 设置静音
pub fn set_mute(mute: bool) -> Result<(), String> {
    with_volume(|v| unsafe { v.SetMute(mute, std::ptr::null()) })
}
