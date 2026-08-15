// 与 Tauri 后端（Rust command）通信的桥接层
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AppState, BackgroundSettings, PerfSnapshot, ThemeMode } from "../vite-env";

/** 应用是否运行在 Tauri 内（否则为浏览器预览模式） */
export const inTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const FALLBACK_STATE: AppState = {
  enabled: true,
  iconsHidden: false,
  taskbarTransparent: false,
  performanceMonitor: false,
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

export async function getPerfSnapshot(): Promise<PerfSnapshot | null> {
  if (!inTauri()) return null;
  return (await invoke<PerfSnapshot | null>("get_perf_snapshot")) ?? null;
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
