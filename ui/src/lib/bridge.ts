// 与 Tauri 后端（Rust command）通信的桥接层
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AppState, ThemeMode } from "../vite-env";

/** 应用是否运行在 Tauri 内（否则为浏览器预览模式） */
export const inTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function getState(): Promise<AppState> {
  if (!inTauri())
    return { enabled: true, iconsHidden: false, taskbarTransparent: false, theme: "system", animating: false, autostart: false, closeToTray: true };
  return (await invoke<AppState>("get_state")) as AppState;
}

export async function setEnabled(enabled: boolean): Promise<AppState> {
  if (!inTauri())
    return { enabled, iconsHidden: false, taskbarTransparent: false, theme: "system", animating: false, autostart: false, closeToTray: true };
  return (await invoke<AppState>("set_enabled", { enabled })) as AppState;
}

export async function setTaskbarTransparent(enabled: boolean): Promise<AppState> {
  if (!inTauri())
    return { enabled: true, iconsHidden: false, taskbarTransparent: enabled, theme: "system", animating: false, autostart: false, closeToTray: true };
  return (await invoke<AppState>("set_taskbar_transparent", { enabled })) as AppState;
}

export async function setTheme(mode: ThemeMode): Promise<AppState> {
  if (!inTauri())
    return { enabled: true, iconsHidden: false, taskbarTransparent: false, theme: mode, animating: false, autostart: false, closeToTray: true };
  return (await invoke<AppState>("set_theme", { mode })) as AppState;
}

export async function setAutostart(enabled: boolean): Promise<AppState> {
  if (!inTauri())
    return { enabled: true, iconsHidden: false, taskbarTransparent: false, theme: "system", animating: false, autostart: enabled, closeToTray: true };
  return (await invoke<AppState>("set_autostart", { enabled })) as AppState;
}

export async function setCloseToTray(enabled: boolean): Promise<AppState> {
  if (!inTauri())
    return { enabled: true, iconsHidden: false, taskbarTransparent: false, theme: "system", animating: false, autostart: false, closeToTray: enabled };
  return (await invoke<AppState>("set_close_to_tray", { enabled })) as AppState;
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
