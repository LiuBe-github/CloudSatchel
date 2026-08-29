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
  autohideEnabled: boolean; // 任务栏自动隐藏是否激活（开启即隐藏）
  perfIntervalMs: number; // 性能监控采样间隔（毫秒）
  aiModel: string; // AI 助手模型名
  aiBaseUrl: string; // AI 助手接口地址（OpenAI 兼容）
  privacyBossKey: string; // 隐私老板键（默认 Ctrl+`）
  bossKeyRegistered: boolean; // 老板键热键是否注册成功（被占用时为 false）
  aiPopupEnabled: boolean; // AI 小窗开关（默认开）
  aiPopupHotkey: string; // AI 小窗呼出快捷键（默认 Ctrl+Shift+Space）
  aiPopupRegistered: boolean; // AI 小窗热键是否注册成功
  audioPanelEnabled: boolean; // 音频识别面板开关（默认开）
  audioPanelX: number; // 音频面板位置 X（物理像素，-1 = 未设置）
  audioPanelY: number; // 音频面板位置 Y
  audioPanelOpacity: number; // 音频面板背景不透明度（0~100，越高越不透明）
  audioPanelClickThrough: boolean; // 音频面板鼠标穿透（开启=仅展示，关闭=可拖动/操作）
  translateEnabled: boolean; // 鼠标选取翻译开关（默认开）
  translateEngine: string; // 翻译引擎："ai" | "microsoft"
  translateMsRegion: string; // 微软翻译区域（Region）
  translateHasMsKey: boolean; // 是否已配置微软翻译 Key（DPAPI 加密文件存在）
  fullscreenActive: boolean; // 当前是否有全屏应用（面板/任务栏叠加用）
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

export interface AiMessage {
  role: "user" | "assistant" | "system";
  content: string;
}

export interface AiConfig {
  hasKey: boolean;
  model: string;
  baseUrl: string;
}

/** SMTC 媒体会话状态（FR-18） */
export interface MediaState {
  active: boolean;
  playing: boolean;
  thumbnail: string;
  appName: string;
  title: string;
  artist: string;
  album: string;
  positionSecs: number;
  durationSecs: number;
  prevEnabled: boolean;
  nextEnabled: boolean;
  playEnabled: boolean;
  pauseEnabled: boolean;
  supported: boolean;
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

/** 翻译弹窗：开始翻译（源文本与引擎） */
export interface TranslatePending {
  source: string;
  engine: string;
}

/** 翻译弹窗：翻译结果 */
export interface TranslateResult {
  source: string;
  target: string;
  engine: string;
  ok: boolean;
  error: string;
}
