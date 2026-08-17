//! AI 助手（FR-15）：OpenAI 兼容对话代理
//!
//! - API Key 用 Windows DPAPI（CryptProtectData / CryptUnprotectData）加密后落盘
//!   （`%LOCALAPPDATA%\CloudSatchel\ai-key.bin`），磁盘上不出现明文 Key。
//! - OpenAI API 不允许浏览器跨域直连（无 CORS），对话请求由 Rust 后端代理：
//!   `reqwest` 请求 Chat Completions（`stream: true`），SSE 分块解析后经 Tauri
//!   事件（ai-chunk / ai-done / ai-error）推送前端逐字渲染。
//! - 联网边界：仅在用户主动发送消息时请求 api.openai.com；对话历史不落盘；
//!   日志与错误信息不包含 Key 或对话内容。

use std::path::PathBuf;
use std::sync::Mutex;

use futures_util::future::{AbortHandle, Abortable};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

const KEY_FILE_NAME: &str = "ai-key.bin";
const API_URL: &str = "https://api.openai.com/v1/chat/completions";
/// 请求总超时（秒）：超过即中止并提示
const TIMEOUT_SECS: u64 = 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiMessage {
    pub role: String, // "user" | "assistant" | "system"
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConfig {
    pub has_key: bool,
    pub model: String,
}

/// 当前进行中的对话任务（「停止生成」时 abort）
static ABORT_HANDLE: Mutex<Option<AbortHandle>> = Mutex::new(None);

fn key_path() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("CloudSatchel")
        .join(KEY_FILE_NAME)
}

// ---------------------------------------------------------------------------
// DPAPI 加解密
// ---------------------------------------------------------------------------

fn dpapi_encrypt(plain: &str) -> Result<Vec<u8>, String> {
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };
    unsafe {
        let bytes = plain.as_bytes();
        let in_blob = CRYPT_INTEGER_BLOB {
            cbData: bytes.len() as u32,
            pbData: bytes.as_ptr() as *mut u8,
        };
        let mut out_blob = CRYPT_INTEGER_BLOB::default();
        CryptProtectData(
            &in_blob,
            windows::core::PWSTR::null(),
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out_blob,
        )
        .map_err(|e| format!("DPAPI 加密失败: {e}"))?;
        let out = std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(out_blob.pbData as *mut core::ffi::c_void)));
        Ok(out)
    }
}

fn dpapi_decrypt(data: &[u8]) -> Result<String, String> {
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};
    unsafe {
        let in_blob = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut out_blob = CRYPT_INTEGER_BLOB::default();
        CryptUnprotectData(
            &in_blob,
            None,
            None,
            None,
            None,
            0,
            &mut out_blob,
        )
        .map_err(|e| format!("DPAPI 解密失败（可能非本机/本用户加密）: {e}"))?;
        let out = std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(out_blob.pbData as *mut core::ffi::c_void)));
        String::from_utf8(out).map_err(|_| "API Key 解码失败".to_string())
    }
}

// ---------------------------------------------------------------------------
// Key 存取
// ---------------------------------------------------------------------------

/// 是否已保存 API Key（加密文件存在）
pub fn has_key() -> bool {
    key_path().is_file()
}

/// 清洗 API Key：去掉所有空白字符（防复制粘贴带入空格/换行破坏 Key）
fn clean_key(api_key: &str) -> String {
    api_key.chars().filter(|c| !c.is_whitespace()).collect()
}

/// 加密保存 API Key（原子写入，磁盘无明文）
pub fn save_key(api_key: &str) -> Result<(), String> {
    let cleaned = clean_key(api_key);
    if cleaned.is_empty() {
        return Err("API Key 不能为空".to_string());
    }
    let data = dpapi_encrypt(&cleaned)?;
    let path = key_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("bin.tmp");
    std::fs::write(&tmp, &data).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

/// 读取并解密 API Key
pub fn load_key() -> Result<String, String> {
    let data = std::fs::read(key_path()).map_err(|e| format!("读取 Key 文件失败: {e}"))?;
    let key = dpapi_decrypt(&data)?;
    crate::dlog::write(&format!("[ai] key loaded: len={}", key.len()));
    Ok(key)
}

// ---------------------------------------------------------------------------
// Tauri Commands
// ---------------------------------------------------------------------------

/// 查询 AI 配置状态（不返回 Key 明文）
#[tauri::command]
pub fn get_ai_config(model: String) -> AiConfig {
    AiConfig {
        has_key: has_key(),
        model,
    }
}

/// 保存 API Key（DPAPI 加密落盘）
#[tauri::command]
pub fn save_ai_key(api_key: String) -> Result<(), String> {
    save_key(&api_key)
}

/// 发送对话并流式推送回复。
///
/// 事件：`ai-chunk`(String 增量) / `ai-done`(null) / `ai-error`(String 错误信息)。
/// 停止生成：`ai_stop` 中止进行中的任务（Abortable 取消后请求一并断开）。
#[tauri::command]
pub async fn ai_send(app: AppHandle, model: String, messages: Vec<AiMessage>) -> Result<(), String> {
    let (handle, registration) = AbortHandle::new_pair();
    *ABORT_HANDLE.lock().unwrap() = Some(handle);

    let fut = stream_chat(app.clone(), model, messages);
    let result = Abortable::new(fut, registration).await;
    *ABORT_HANDLE.lock().unwrap() = None;

    match result {
        Ok(inner) => inner,
        Err(_aborted) => {
            // 用户主动「停止」：正常结束，不当作错误
            let _ = app.emit("ai-done", ());
            Ok(())
        }
    }
}

/// 停止当前生成（中止进行中的请求与事件推送）
#[tauri::command]
pub fn ai_stop() {
    if let Some(handle) = ABORT_HANDLE.lock().unwrap().take() {
        handle.abort();
    }
}

/// 实际执行 OpenAI 流式对话（被 Abortable 包裹，可被 ai_stop 中止）
async fn stream_chat(
    app: AppHandle,
    model: String,
    messages: Vec<AiMessage>,
) -> Result<(), String> {
    let key = load_key()?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
    });

    let resp = client
        .post(API_URL)
        .bearer_auth(&key)
        .json(&body)
        .send()
        .await
        .map_err(|e| friendly_net_error(&e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        // 读取 OpenAI 返回的错误详情（如 "Incorrect API key provided: sk-xxx...xxxx"），
        // 便于用户对比掩码确认是否填错 Key；不含完整 Key 与对话内容
        let detail = resp
            .text()
            .await
            .ok()
            .and_then(|body| {
                serde_json::from_str::<serde_json::Value>(&body)
                    .ok()
                    .and_then(|v| v["error"]["message"].as_str().map(|s| s.to_string()))
            })
            .filter(|s| !s.is_empty());
        let msg = if status.as_u16() == 401 {
            match detail {
                Some(d) => format!("API Key 无效（401）：{d}"),
                None => "API Key 无效（401 Unauthorized）".to_string(),
            }
        } else if status.as_u16() == 429 {
            "请求过于频繁（429），请稍后再试".to_string()
        } else {
            format!("OpenAI 接口返回错误（HTTP {}）", status.as_u16())
        };
        return Err(msg);
    }

    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();
    let mut finished = false;
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit("ai-error", format!("网络读取中断: {e}"));
                break;
            }
        };
        buf.extend_from_slice(&chunk);
        // 按行解析 SSE
        while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line);
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(data) = line.strip_prefix("data:") {
                let data = data.trim();
                if data == "[DONE]" {
                    finished = true;
                    break;
                }
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(content) = value["choices"][0]["delta"]["content"].as_str() {
                        let _ = app.emit("ai-chunk", content.to_string());
                    }
                }
            }
        }
        if finished {
            break;
        }
    }

    if !finished {
        let _ = app.emit("ai-error", "连接意外中断，回复不完整".to_string());
        return Err("连接意外中断".to_string());
    }
    let _ = app.emit("ai-done", ());
    Ok(())
}

/// 网络错误的用户可读提示（不包含 URL / Key / 内容）
fn friendly_net_error(e: &reqwest::Error) -> String {
    if e.is_timeout() {
        format!("请求超时（超过 {TIMEOUT_SECS} 秒无响应）")
    } else if e.is_connect() {
        "无法连接到 api.openai.com，请检查网络".to_string()
    } else {
        format!("网络请求失败: {e}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DPAPI 加密/解密往返 + 密文不含明文
    #[test]
    fn dpapi_roundtrip_keeps_key_private() {
        let key = "sk-test-abcdef-123456";
        let enc = dpapi_encrypt(key).expect("encrypt");
        let as_text = String::from_utf8_lossy(&enc);
        assert!(
            !as_text.contains("sk-test"),
            "密文不应包含明文 Key"
        );
        let dec = dpapi_decrypt(&enc).expect("decrypt");
        assert_eq!(dec, key);
    }

    /// Key 清洗：去掉所有空白字符（粘贴带入的换行/空格不会破坏 Key）
    #[test]
    fn clean_key_removes_all_whitespace() {
        assert_eq!(
            clean_key(" sk-abc \ndef\tghi \n"),
            "sk-abcdefghi"
        );
        assert_eq!(clean_key("sk-abc"), "sk-abc");
        assert!(clean_key("  \n\t ").is_empty());
    }
}
