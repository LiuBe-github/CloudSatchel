import { useEffect } from "react";
import type { ThemeMode } from "../vite-env";
import { Switch } from "./Switch";

interface SettingsPanelProps {
  open: boolean;
  theme: ThemeMode;
  onThemeChange: (mode: ThemeMode) => void;
  autostart: boolean;
  onAutostartChange: (enabled: boolean) => void;
  closeToTray: boolean;
  onCloseToTrayChange: (enabled: boolean) => void;
  onClose: () => void;
}

const THEME_OPTIONS: { value: ThemeMode; label: string; icon: string }[] = [
  { value: "light", label: "浅色", icon: "☀" },
  { value: "system", label: "跟随系统", icon: "◐" },
  { value: "dark", label: "深色", icon: "☾" },
];

export function SettingsPanel({
  open,
  theme,
  onThemeChange,
  autostart,
  onAutostartChange,
  closeToTray,
  onCloseToTrayChange,
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
          <span className="settings-version">云笈 · v0.6.6</span>
        </div>
      </div>
    </div>
  );
}
