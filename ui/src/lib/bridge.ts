// 与 Tauri 后端（Rust command）通信的桥接层
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AiConfig,
  AiMessage,
  AppState,
  BackgroundSettings,
  PerfSnapshot,
  ThemeMode,
} from "../vite-env";

/** 应用是否运行在 Tauri 内（否则为浏览器预览模式） */
export const inTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const FALLBACK_STATE: AppState = {
  enabled: true,
  iconsHidden: false,
  taskbarTransparent: false,
  performanceMonitor: false,
  privacyEnabled: false,
  privacyIdleSecs: 60,
  privacyActive: false,
  autohideEnabled: false,
  perfIntervalMs: 1000,
  aiModel: "gpt-4o-mini",
  aiBaseUrl: "https://api.openai.com/v1",
  privacyBossKey: "Ctrl+`",
  bossKeyRegistered: false,
  aiPopupEnabled: true,
  aiPopupHotkey: "Ctrl+Shift+Space",
  aiPopupRegistered: false,
  theme: "system",
  animating: false,
  autostart: false,
  closeToTray: true,
  backgroundImagePath: "",
  backgroundFit: "cover",
  backgroundDim: 0.25,
  backgroundBlur: 0,
  backgroundScale: 1,
  backgroundPositionX: 50,
  backgroundPositionY: 50,
};

const fallback = (overrides: Partial<AppState> = {}): AppState => ({
  ...FALLBACK_STATE,
  ...overrides,
});

export async function getState(): Promise<AppState> {
  if (!inTauri()) return fallback();
  return (await invoke<AppState>("get_state")) as AppState;
}

export async function setEnabled(enabled: boolean): Promise<AppState> {
  if (!inTauri()) return fallback({ enabled });
  return (await invoke<AppState>("set_enabled", { enabled })) as AppState;
}

export async function setTaskbarTransparent(enabled: boolean): Promise<AppState> {
  if (!inTauri()) return fallback({ taskbarTransparent: enabled });
  return (await invoke<AppState>("set_taskbar_transparent", { enabled })) as AppState;
}

export async function setPerformanceMonitor(enabled: boolean): Promise<AppState> {
  if (!inTauri()) return fallback({ performanceMonitor: enabled });
  return (await invoke<AppState>("set_performance_monitor", { enabled })) as AppState;
}

export async function setPrivacyEnabled(enabled: boolean): Promise<AppState> {
  if (!inTauri()) return fallback({ privacyEnabled: enabled });
  return (await invoke<AppState>("set_privacy_enabled", { enabled })) as AppState;
}

export async function setPrivacyIdleSecs(secs: number): Promise<AppState> {
  if (!inTauri()) return fallback({ privacyIdleSecs: secs });
  return (await invoke<AppState>("set_privacy_idle_secs", { secs })) as AppState;
}

export async function setAutohideEnabled(enabled: boolean): Promise<AppState> {
  if (!inTauri()) return fallback({ autohideEnabled: enabled });
  return (await invoke<AppState>("set_autohide_enabled", { enabled })) as AppState;
}

/** 设置隐私老板键快捷键（FR-13 扩展）；格式无效或被占用时抛错 */
export async function setPrivacyBossKey(key: string): Promise<AppState> {
  if (!inTauri()) return fallback({ privacyBossKey: key });
  return (await invoke<AppState>("set_privacy_boss_key", { key })) as AppState;
}

/** 开关 AI 小窗（FR-17） */
export async function setAiPopupEnabled(enabled: boolean): Promise<AppState> {
  if (!inTauri()) return fallback({ aiPopupEnabled: enabled });
  return (await invoke<AppState>("set_ai_popup_enabled", { enabled })) as AppState;
}

/** 设置 AI 小窗呼出快捷键（FR-17）；格式无效或被占用时抛错 */
export async function setAiPopupHotkey(key: string): Promise<AppState> {
  if (!inTauri()) return fallback({ aiPopupHotkey: key });
  return (await invoke<AppState>("set_ai_popup_hotkey", { key })) as AppState;
}

export async function setPerfIntervalMs(ms: number): Promise<AppState> {
  if (!inTauri()) return fallback({ perfIntervalMs: ms });
  return (await invoke<AppState>("set_perf_interval_ms", { ms })) as AppState;
}

export async function getPerfSnapshot(): Promise<PerfSnapshot | null> {
  if (!inTauri()) return null;
  return (await invoke<PerfSnapshot | null>("get_perf_snapshot")) ?? null;
}

// ---------------------------------------------------------------------------
// AI 助手（FR-15）
// ---------------------------------------------------------------------------

export async function getAiConfig(): Promise<AiConfig> {
  if (!inTauri()) {
    return { hasKey: false, model: "gpt-4o-mini", baseUrl: "https://api.openai.com/v1" };
  }
  return (await invoke<AiConfig>("get_ai_config")) as AiConfig;
}

export async function saveAiKey(apiKey: string): Promise<void> {
  if (!inTauri()) return;
  await invoke("save_ai_key", { apiKey });
}

export async function setAiModel(model: string): Promise<AppState> {
  if (!inTauri()) return fallback({ aiModel: model });
  return (await invoke<AppState>("set_ai_model", { model })) as AppState;
}

export async function setAiBaseUrl(baseUrl: string): Promise<AppState> {
  if (!inTauri()) return fallback({ aiBaseUrl: baseUrl });
  return (await invoke<AppState>("set_ai_base_url", { baseUrl })) as AppState;
}

export async function aiSend(
  baseUrl: string,
  model: string,
  messages: AiMessage[],
): Promise<void> {
  if (!inTauri()) return;
  await invoke("ai_send", { baseUrl, model, messages });
}

export function aiStop(): void {
  if (!inTauri()) return;
  void invoke("ai_stop");
}

/** 订阅 AI 流式输出增量 */
export function onAiChunk(cb: (text: string) => void): () => void {
  if (!inTauri()) return () => {};
  const unlisten = listen<string>("ai-chunk", (event) => cb(event.payload));
  return () => {
    void unlisten.then((fn) => fn());
  };
}

/** 订阅 AI 回复结束 */
export function onAiDone(cb: () => void): () => void {
  if (!inTauri()) return () => {};
  const unlisten = listen("ai-done", () => cb());
  return () => {
    void unlisten.then((fn) => fn());
  };
}

/** 订阅 AI 错误 */
export function onAiError(cb: (message: string) => void): () => void {
  if (!inTauri()) return () => {};
  const unlisten = listen<string>("ai-error", (event) => cb(event.payload));
  return () => {
    void unlisten.then((fn) => fn());
  };
}

export async function setTheme(mode: ThemeMode): Promise<AppState> {
  if (!inTauri()) return fallback({ theme: mode });
  return (await invoke<AppState>("set_theme", { mode })) as AppState;
}

export async function setAutostart(enabled: boolean): Promise<AppState> {
  if (!inTauri()) return fallback({ autostart: enabled });
  return (await invoke<AppState>("set_autostart", { enabled })) as AppState;
}

export async function setCloseToTray(enabled: boolean): Promise<AppState> {
  if (!inTauri()) return fallback({ closeToTray: enabled });
  return (await invoke<AppState>("set_close_to_tray", { enabled })) as AppState;
}

export async function setBackground(settings: BackgroundSettings): Promise<AppState> {
  if (!inTauri()) return fallback();
  return (await invoke<AppState>("set_background", { settings })) as AppState;
}

export async function chooseBackgroundImage(): Promise<string | null> {
  if (!inTauri()) return null;
  return (await invoke<string | null>("choose_background_image")) ?? null;
}

export async function copyBackgroundImage(sourcePath: string): Promise<string> {
  if (!inTauri()) return sourcePath;
  return (await invoke<string>("copy_background_image", { sourcePath })) as string;
}

export async function readBackgroundImage(path: string): Promise<string | null> {
  if (!inTauri()) return null;
  return (await invoke<string | null>("read_background_image", { path })) ?? null;
}

export function minimize(): void {
  if (!inTauri()) return;
  void invoke("minimize_window");
}

export function toggleMaximize(): void {
  if (!inTauri()) return;
  void invoke("toggle_maximize_window");
}

export function close(): void {
  if (!inTauri()) return;
  void invoke("close_window");
}

/** 订阅后端推送的状态更新 */
export function onStateUpdate(cb: (state: AppState) => void): () => void {
  if (!inTauri()) return () => {};
  const unlisten = listen<AppState>("state-updated", (event) => cb(event.payload));
  return () => {
    void unlisten.then((fn) => fn());
  };
}
