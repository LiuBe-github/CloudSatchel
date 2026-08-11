//! 开机自启动（纯净方案：启动文件夹快捷方式）
//!
//! 不写注册表、无需管理员权限：在用户启动文件夹
//! `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup`
//! 创建 / 删除 `.lnk` 快捷方式，Windows 登录后自动执行。

use std::path::{Path, PathBuf};

use windows::core::{Interface, HSTRING};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IPersistFile,
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{
    IShellLinkW, SHGetKnownFolderPath, ShellLink, KNOWN_FOLDER_FLAG, FOLDERID_Startup,
};

const STARTUP_FILE_NAME: &str = "如意工具箱.lnk";

/// 用户启动文件夹路径
fn startup_folder() -> Result<PathBuf, String> {
    unsafe {
        let path = SHGetKnownFolderPath(&FOLDERID_Startup, KNOWN_FOLDER_FLAG(0), None)
            .map_err(|e| format!("获取启动文件夹失败: {e}"))?;
        let s = path
            .to_string()
            .map_err(|e| format!("解析启动文件夹路径失败: {e}"))?;
        CoTaskMemFree(Some(path.as_ptr() as *const core::ffi::c_void));
        Ok(PathBuf::from(s))
    }
}

/// 自启动快捷方式路径
fn shortcut_path() -> Result<PathBuf, String> {
    Ok(startup_folder()?.join(STARTUP_FILE_NAME))
}

/// 当前是否已开启开机自启动（以快捷方式是否存在为准）
pub fn is_enabled() -> bool {
    shortcut_path().map(|p| p.exists()).unwrap_or(false)
}

/// 开启 / 关闭开机自启动
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    if enabled {
        create_shortcut()
    } else {
        remove_shortcut()
    }
}

fn create_shortcut() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("获取程序路径失败: {e}"))?;
    let folder = startup_folder()?;
    let lnk = folder.join(STARTUP_FILE_NAME);
    let exe_dir = exe.parent().unwrap_or(Path::new("")).to_path_buf();

    let exe_str = exe.to_string_lossy().to_string();
    let dir_str = exe_dir.to_string_lossy().to_string();
    let lnk_str = lnk.to_string_lossy().to_string();

    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        // 仅当本线程由我们初始化时负责反初始化；
        // 已是 STA / 其他模式则保持原样（S_FALSE / RPC_E_CHANGED_MODE）
        let need_uninit = hr.0 == 0;
        let result = (|| -> windows::core::Result<()> {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
            let exe_h = HSTRING::from(exe_str.as_str());
            let dir_h = HSTRING::from(dir_str.as_str());
            let desc_h = HSTRING::from("如意工具箱 · 开机自启动");
            link.SetPath(&exe_h)?;
            link.SetWorkingDirectory(&dir_h)?;
            link.SetDescription(&desc_h)?;
            link.SetIconLocation(&exe_h, 0)?;
            let persist: IPersistFile = link.cast()?;
            let lnk_h = HSTRING::from(lnk_str.as_str());
            persist.Save(&lnk_h, true)?;
            Ok(())
        })();
        if need_uninit {
            CoUninitialize();
        }
        result.map_err(|e| format!("创建开机自启动快捷方式失败: {e}"))
    }
}

fn remove_shortcut() -> Result<(), String> {
    let lnk = shortcut_path()?;
    if lnk.exists() {
        std::fs::remove_file(&lnk).map_err(|e| format!("删除开机自启动快捷方式失败: {e}"))?;
    }
    Ok(())
}
