import { useEffect } from "react";
import type { BackgroundFit, BackgroundSettings, ThemeMode } from "../vite-env";
import { RangeRow } from "./RangeRow";
import { Switch } from "./Switch";

interface SettingsPanelProps {
  open: boolean;
  theme: ThemeMode;
  onThemeChange: (mode: ThemeMode) => void;
  autostart: boolean;
  onAutostartChange: (enabled: boolean) => void;
  closeToTray: boolean;
  onCloseToTrayChange: (enabled: boolean) => void;
  taskbarTransparent: boolean;
  onTaskbarTransparentChange: (enabled: boolean) => void;
  autohideEnabled: boolean;
  onAutohideChange: (enabled: boolean) => void;
  privacyIdleSecs: number;
  onPrivacyIdleChange: (secs: number) => void;
  background: BackgroundSettings;
  backgroundName: string;
  onBackgroundChange: (next: BackgroundSettings) => void;
  onChooseBackground: () => void;
  onClearBackground: () => void;
  onClose: () => void;
}

const THEME_OPTIONS: { value: ThemeMode; label: string; icon: string }[] = [
  { value: "light", label: "浅色", icon: "☀" },
  { value: "system", label: "跟随系统", icon: "◐" },
  { value: "dark", label: "深色", icon: "☾" },
];

const FIT_OPTIONS: { value: BackgroundFit; label: string }[] = [
  { value: "cover", label: "填充" },
  { value: "contain", label: "完整" },
  { value: "repeat", label: "平铺" },
];

/** 隐私操作空闲时间六档选项（秒） */
const PRIVACY_IDLE_OPTIONS: Array<{ value: number; label: string }> = [
  { value: 10, label: "10 秒" },
  { value: 30, label: "30 秒" },
  { value: 60, label: "1 分钟" },
  { value: 180, label: "3 分钟" },
  { value: 300, label: "5 分钟" },
  { value: 600, label: "10 分钟" },
];

/** 空闲时间显示：<60 秒显示「X 秒」，否则显示「X 分钟」 */
const formatIdle = (v: number): string =>
  v < 60 ? `${v} 秒` : `${Math.round(v / 60)} 分钟`;

export function SettingsPanel({
  open,
  theme,
  onThemeChange,
  autostart,
  onAutostartChange,
  closeToTray,
  onCloseToTrayChange,
  taskbarTransparent,
  onTaskbarTransparentChange,
  autohideEnabled,
  onAutohideChange,
  privacyIdleSecs,
  onPrivacyIdleChange,
  background,
  backgroundName,
  onBackgroundChange,
  onChooseBackground,
  onClearBackground,
  onClose,
}: SettingsPanelProps) {
  // Esc 关闭设置面板
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className={`side-panel${open ? " open" : ""}`}>
      <div className={`side-panel-inner${open ? " visible" : ""}`}>
        <div className="settings-header">
          <h2 className="settings-title">应用设置</h2>
          <button className="icon-btn" onClick={onClose} aria-label="关闭设置">
            <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <path d="M18 6 6 18M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div className="settings-section">
          <div className="settings-label">外观主题</div>
          <div className="theme-group" role="radiogroup" aria-label="外观主题">
            {THEME_OPTIONS.map((opt) => (
              <button
                key={opt.value}
                role="radio"
                aria-checked={theme === opt.value}
                className={`theme-option ${theme === opt.value ? "selected" : ""}`}
                onClick={() => onThemeChange(opt.value)}
              >
                <span className="theme-option-icon">{opt.icon}</span>
                {opt.label}
              </button>
            ))}
          </div>
        </div>

        <div className="settings-section">
          <div className="settings-label">背景图片</div>
          <div className="background-picker">
            <input
              type="text"
              readOnly
              value={backgroundName || "默认背景"}
              className="background-name-input"
            />
            <button type="button" className="seg-btn" onClick={onChooseBackground}>
              选择
            </button>
            {background.imagePath && (
              <button type="button" className="seg-btn danger" onClick={onClearBackground}>
                清除
              </button>
            )}
          </div>
          <div className="seg-group">
            {FIT_OPTIONS.map((opt) => (
              <button
                key={opt.value}
                className={`seg-option ${background.fit === opt.value ? "selected" : ""}`}
                onClick={() => onBackgroundChange({ ...background, fit: opt.value })}
              >
                {opt.label}
              </button>
            ))}
          </div>
          <RangeRow
            label="遮罩"
            value={background.dim}
            min={0}
            max={1}
            step={0.01}
            format={(v) => `${Math.round(v * 100)}%`}
            onChange={(v) => onBackgroundChange({ ...background, dim: v })}
          />
          <RangeRow
            label="缩放"
            value={background.scale}
            min={0.5}
            max={2}
            step={0.05}
            format={(v) => `${Math.round(v * 100)}%`}
            onChange={(v) => onBackgroundChange({ ...background, scale: v })}
          />
          <RangeRow
            label="横向"
            value={background.positionX}
            min={0}
            max={100}
            step={1}
            format={(v) => `${v}%`}
            onChange={(v) => onBackgroundChange({ ...background, positionX: v })}
          />
          <RangeRow
            label="纵向"
            value={background.positionY}
            min={0}
            max={100}
            step={1}
            format={(v) => `${v}%`}
            onChange={(v) => onBackgroundChange({ ...background, positionY: v })}
          />
          <RangeRow
            label="模糊"
            value={background.blur}
            min={0}
            max={20}
            step={1}
            format={(v) => `${v}px`}
            onChange={(v) => onBackgroundChange({ ...background, blur: v })}
          />
        </div>

        <div className="settings-section">
          <div className="settings-label">任务栏</div>
          <div className="setting-row">
            <div className="setting-row-text">
              <div className="setting-row-title">透明任务栏</div>
              <div className="setting-row-desc">让任务栏背景消失，与壁纸融为一体；退出应用自动恢复</div>
            </div>
            <Switch checked={taskbarTransparent} onChange={() => onTaskbarTransparentChange(!taskbarTransparent)} />
          </div>
          <div className="setting-row">
            <div className="setting-row-text">
              <div className="setting-row-title">任务栏自动隐藏</div>
              <div className="setting-row-desc">开启后任务栏立即隐藏；鼠标移到屏幕下边界弹出，移开再隐藏（不写注册表）</div>
            </div>
            <Switch checked={autohideEnabled} onChange={() => onAutohideChange(!autohideEnabled)} />
          </div>
        </div>

        <div className="settings-section">
          <div className="settings-label">隐私操作</div>
          <div className="setting-row">
            <div className="setting-row-text">
              <div className="setting-row-title">触发空闲时间</div>
              <div className="setting-row-desc">空闲超过该时长自动执行隐私保护（当前 {formatIdle(privacyIdleSecs)}）</div>
            </div>
            <select
              className="select-box"
              value={privacyIdleSecs}
              onChange={(e) => onPrivacyIdleChange(Number(e.target.value))}
            >
              {PRIVACY_IDLE_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
          </div>
          <div className="setting-row-desc" style={{ marginTop: 8 }}>
            空闲超时自动最小化所有窗口、隐藏桌面图标与任务栏并静音；操作鼠标或键盘立即还原
          </div>
        </div>

        <div className="settings-section">
          <div className="settings-label">启动与退出</div>
          <div className="setting-row">
            <div className="setting-row-text">
              <div className="setting-row-title">开机自启动</div>
              <div className="setting-row-desc">登录 Windows 后自动启动本应用（启动文件夹快捷方式，不写注册表）</div>
            </div>
            <Switch checked={autostart} onChange={() => onAutostartChange(!autostart)} />
          </div>
          <div className="setting-row">
            <div className="setting-row-text">
              <div className="setting-row-title">关闭到托盘</div>
              <div className="setting-row-desc">点击关闭窗口时最小化到系统托盘继续后台运行；关闭后点击关闭直接退出软件</div>
            </div>
            <Switch checked={closeToTray} onChange={() => onCloseToTrayChange(!closeToTray)} />
          </div>
        </div>

        <div className="settings-section">
          <div className="settings-label">纯净性</div>
          <ul className="purity-list">
            <li><span className="purity-check">✓</span>纯本地运行，不联网</li>
            <li><span className="purity-check">✓</span>不写注册表（自启动仅用启动文件夹快捷方式）</li>
            <li><span className="purity-check">✓</span>退出时自动恢复桌面图标</li>
            <li><span className="purity-check">✓</span>后台驻留由「关闭到托盘」开关控制（默认开）；开机自启动默认关闭</li>
          </ul>
        </div>

        <div className="settings-footer">
          <span className="settings-version">云笈 · v0.11.0</span>
        </div>
      </div>
    </div>
  );
}
