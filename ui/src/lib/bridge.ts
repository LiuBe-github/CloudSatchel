// 与 Tauri 后端（Rust command）通信的桥接层
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AppState, ThemeMode } from "../vite-env";

/** 应用是否运行在 Tauri 内（否则为浏览器预览模式） */
export const inTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function getState(): Promise<AppState> {
  if (!inTauri())
    return { enabled: true, iconsHidden: false, taskbarTransparent: false, theme: "system", animating: false, autostart: false };
  return (await invoke<AppState>("get_state")) as AppState;
}

export async function setEnabled(enabled: boolean): Promise<AppState> {
  if (!inTauri())
    return { enabled, iconsHidden: false, taskbarTransparent: false, theme: "system", animating: false, autostart: false };
  return (await invoke<AppState>("set_enabled", { enabled })) as AppState;
}

export async function setTaskbarTransparent(enabled: boolean): Promise<AppState> {
  if (!inTauri())
    return { enabled: true, iconsHidden: false, taskbarTransparent: enabled, theme: "system", animating: false, autostart: false };
  return (await invoke<AppState>("set_taskbar_transparent", { enabled })) as AppState;
}

export async function setTheme(mode: ThemeMode): Promise<AppState> {
  if (!inTauri())
    return { enabled: true, iconsHidden: false, taskbarTransparent: false, theme: mode, animating: false, autostart: false };
  return (await invoke<AppState>("set_theme", { mode })) as AppState;
}

export async function setAutostart(enabled: boolean): Promise<AppState> {
  if (!inTauri())
    return { enabled: true, iconsHidden: false, taskbarTransparent: false, theme: "system", animating: false, autostart: enabled };
  return (await invoke<AppState>("set_autostart", { enabled })) as AppState;
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

/** 主窗口请求关闭（标题栏关闭按钮 / Alt+F4）——由前端弹出「最小化到托盘 / 退出」询问 */
export function onCloseRequested(cb: () => void): () => void {
  if (!inTauri()) return () => {};
  const unlisten = listen("close-requested", () => cb());
  return () => {
    void unlisten.then((fn) => fn());
  };
}

/** 最小化到系统托盘，继续后台运行 */
export function hideToTray(): void {
  if (!inTauri()) return;
  void invoke("hide_to_tray");
}

/** 直接退出软件（后端会先恢复桌面图标 / 任务栏） */
export function quitApp(): void {
  if (!inTauri()) return;
  void invoke("quit_app");
}

/** 订阅后端推送的状态更新 */
export function onStateUpdate(cb: (state: AppState) => void): () => void {
  if (!inTauri()) return () => {};
  const unlisten = listen<AppState>("state-updated", (event) => cb(event.payload));
  return () => {
    void unlisten.then((fn) => fn());
  };
}
