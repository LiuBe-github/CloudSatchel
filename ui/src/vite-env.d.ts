/// <reference types="vite/client" />

export type ThemeMode = "light" | "dark" | "system";

export interface AppState {
  enabled: boolean; // 功能是否激活
  iconsHidden: boolean; // 图标当前是否隐藏
  taskbarTransparent: boolean; // 任务栏是否透明
  theme: ThemeMode;
  animating: boolean;
  autostart: boolean; // 开机自启动（启动文件夹快捷方式）
}
