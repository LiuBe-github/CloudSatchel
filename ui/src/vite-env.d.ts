/// <reference types="vite/client" />

export type ThemeMode = "light" | "dark" | "system";

export interface AppState {
  enabled: boolean; // 功能是否激活
  iconsHidden: boolean; // 图标当前是否隐藏
  taskbarTransparent: boolean; // 任务栏是否透明
  theme: ThemeMode;
  animating: boolean;
  autostart: boolean; // 开机自启动（启动文件夹快捷方式）
  closeToTray: boolean; // 关闭到托盘：true=关闭最小化到托盘；false=关闭直接退出
}
