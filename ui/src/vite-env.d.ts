/// <reference types="vite/client" />

export type ThemeMode = "light" | "dark" | "system";

export type BackgroundFit = "cover" | "contain" | "repeat";

export interface BackgroundSettings {
  imagePath: string;
  fit: BackgroundFit;
  dim: number;
  blur: number;
  scale: number;
  positionX: number;
  positionY: number;
}

export interface AppState {
  enabled: boolean; // 功能是否激活
  iconsHidden: boolean; // 图标当前是否隐藏
  taskbarTransparent: boolean; // 任务栏是否透明
  performanceMonitor: boolean; // 主机性能监控是否激活
  privacyEnabled: boolean; // 隐私操作是否激活
  privacyIdleSecs: number; // 隐私操作空闲触发时间（秒）
  privacyActive: boolean; // 隐私操作当前是否已触发
  autohideEnabled: boolean; // 任务栏自动隐藏是否激活
  autohideIdleSecs: number; // 任务栏自动隐藏空闲时间（秒）
  theme: ThemeMode;
  animating: boolean;
  autostart: boolean; // 开机自启动（启动文件夹快捷方式）
  closeToTray: boolean; // 关闭到托盘：true=关闭最小化到托盘；false=关闭直接退出
  backgroundImagePath: string;
  backgroundFit: BackgroundFit;
  backgroundDim: number;
  backgroundBlur: number;
  backgroundScale: number;
  backgroundPositionX: number;
  backgroundPositionY: number;
}

export interface CpuMetrics {
  usage: number;
  temperature: number | null;
  currentFrequencyMhz: number | null;
  baseFrequencyMhz: number | null;
  coreCount: number | null;
  logicalProcessorCount: number;
  processCount: number;
  threadCount: number;
}

export interface GpuMetrics {
  name: string | null;
  utilization: number | null;
  temperature: number | null;
  memoryUsedMb: number | null;
  memoryTotalMb: number | null;
  sharedMemoryUsedMb: number | null;
  driverVersion: string | null;
}

export interface MemoryMetrics {
  usage: number;
  usedBytes: number;
  availableBytes: number;
  totalBytes: number;
  pagefileUsedBytes: number | null;
  pagefileTotalBytes: number | null;
}

export interface NetworkMetrics {
  uploadBytesPerSec: number;
  downloadBytesPerSec: number;
  adapterName: string | null;
  linkSpeedMbps: number | null;
}

export interface PerfSnapshot {
  timestamp: number;
  cpu: CpuMetrics;
  gpu: GpuMetrics;
  memory: MemoryMetrics;
  network: NetworkMetrics;
}
