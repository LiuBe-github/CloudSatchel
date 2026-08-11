import { useEffect } from "react";
import { getState, setTheme as apiSetTheme } from "./bridge";
import type { AppState, ThemeMode } from "../vite-env";

export type { ThemeMode };

/** 将主题模式应用到 DOM（data-theme） */
export function applyTheme(mode: ThemeMode): void {
  const root = document.documentElement;
  if (mode === "system") {
    const dark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    root.dataset.theme = dark ? "dark" : "light";
    root.dataset.themeMode = "system";
  } else {
    root.dataset.theme = mode;
    root.dataset.themeMode = mode;
  }
}

/** 跟随系统主题变化（仅 system 模式） */
export function watchSystemTheme(mode: ThemeMode): () => void {
  if (mode !== "system") return () => {};
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  const onChange = (): void => {
    const root = document.documentElement;
    root.dataset.theme = mq.matches ? "dark" : "light";
  };
  mq.addEventListener("change", onChange);
  return () => mq.removeEventListener("change", onChange);
}

/** 首次加载时读取后端配置并应用主题 */
export function useThemeInit(): void {
  useEffect(() => {
    let cleanup = () => {};
    getState()
      .then((s: AppState) => {
        applyTheme(s.theme);
        cleanup = watchSystemTheme(s.theme);
      })
      .catch(() => {});
    return () => cleanup();
  }, []);
}

export async function changeTheme(mode: ThemeMode): Promise<void> {
  applyTheme(mode);
  try {
    await apiSetTheme(mode);
  } catch {
    /* 浏览器预览模式忽略 */
  }
}
