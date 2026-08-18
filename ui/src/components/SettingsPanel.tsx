import { useEffect, useState } from "react";
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
  privacyIdleSecs: number;
  onPrivacyIdleChange: (secs: number) => void;
  privacyBossKey: string;
  bossKeyRegistered: boolean;
  onBossKeyChange: (key: string) => Promise<void>;
  aiPopupEnabled: boolean;
  aiPopupHotkey: string;
  aiPopupRegistered: boolean;
  onAiPopupEnabledChange: (enabled: boolean) => Promise<void>;
  onAiPopupHotkeyChange: (key: string) => Promise<void>;
  audioPanelEnabled: boolean;
  onAudioPanelEnabledChange: (enabled: boolean) => Promise<void>;
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
  privacyIdleSecs,
  onPrivacyIdleChange,
  privacyBossKey,
  bossKeyRegistered,
  onBossKeyChange,
  aiPopupEnabled,
  aiPopupHotkey,
  aiPopupRegistered,
  onAiPopupEnabledChange,
  onAiPopupHotkeyChange,
  audioPanelEnabled,
  onAudioPanelEnabledChange,
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

  // 老板键输入框：跟随持久化值；保存失败时回退显示当前生效值
  const [bossKeyInput, setBossKeyInput] = useState(privacyBossKey);
  const [bossKeyError, setBossKeyError] = useState("");
  const [bossKeySaving, setBossKeySaving] = useState(false);
  useEffect(() => setBossKeyInput(privacyBossKey), [privacyBossKey]);

  const saveBossKey = async () => {
    const key = bossKeyInput.trim();
    if (!key || key === privacyBossKey) {
      setBossKeyInput(privacyBossKey);
      return;
    }
    setBossKeySaving(true);
    setBossKeyError("");
    try {
      await onBossKeyChange(key);
      setBossKeyError("");
    } catch (err) {
      setBossKeyError(String(err));
      setBossKeyInput(privacyBossKey);
    } finally {
      setBossKeySaving(false);
    }
  };

  // AI 小窗热键输入框（复用同一套编辑/保存/错误处理模式）
  const [popupKeyInput, setPopupKeyInput] = useState(aiPopupHotkey);
  const [popupKeyError, setPopupKeyError] = useState("");
  const [popupKeySaving, setPopupKeySaving] = useState(false);
  useEffect(() => setPopupKeyInput(aiPopupHotkey), [aiPopupHotkey]);

  const savePopupHotkey = async () => {
    const key = popupKeyInput.trim();
    if (!key || key === aiPopupHotkey) {
      setPopupKeyInput(aiPopupHotkey);
      return;
    }
    setPopupKeySaving(true);
    setPopupKeyError("");
    try {
      await onAiPopupHotkeyChange(key);
      setPopupKeyError("");
    } catch (err) {
      setPopupKeyError(String(err));
      setPopupKeyInput(aiPopupHotkey);
    } finally {
      setPopupKeySaving(false);
    }
  };

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
          <div className="setting-row">
            <div className="setting-row-text">
              <div className="setting-row-title">隐私老板键</div>
              <div className="setting-row-desc">
                按下立即触发隐私保护（无需等待空闲），再按恢复；老板键触发后鼠标/键盘操作不会恢复（默认 Ctrl+`）
              </div>
            </div>
            <div className="hotkey-editor">
              <input
                className="hotkey-input"
                value={bossKeyInput}
                onChange={(e) => {
                  setBossKeyInput(e.target.value);
                  setBossKeyError("");
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    void saveBossKey();
                  }
                }}
                placeholder="Ctrl+Shift+Space"
                spellCheck={false}
              />
              <button
                type="button"
                className="seg-btn primary"
                onClick={() => void saveBossKey()}
                disabled={bossKeySaving}
              >
                {bossKeySaving ? "保存中…" : "保存"}
              </button>
            </div>
          </div>
          {bossKeyError && <div className="setting-row-desc error-text">{bossKeyError}</div>}
          {!bossKeyError && !bossKeyRegistered && (
            <div className="setting-row-desc error-text">
              老板键注册失败（可能被其他程序占用），已降级为仅空闲触发
            </div>
          )}
          <div className="setting-row-desc" style={{ marginTop: 8 }}>
            空闲超时自动最小化所有窗口、隐藏桌面图标与任务栏并静音；操作鼠标或键盘立即还原
          </div>
        </div>

        <div className="settings-section">
          <div className="settings-label">AI 小窗</div>
          <div className="setting-row">
            <div className="setting-row-text">
              <div className="setting-row-title">启用 AI 小窗</div>
              <div className="setting-row-desc">开启后可用快捷键随时呼出小型 AI 问答窗；关闭后快捷键失效且不影响主界面 AI 助手</div>
            </div>
            <Switch
              checked={aiPopupEnabled}
              onChange={() => void onAiPopupEnabledChange(!aiPopupEnabled)}
            />
          </div>
          <div className="setting-row">
            <div className="setting-row-text">
              <div className="setting-row-title">呼出快捷键</div>
              <div className="setting-row-desc">按下切换呼出 / 隐藏小窗（默认 Ctrl+Shift+Space）</div>
            </div>
            <div className="hotkey-editor">
              <input
                className="hotkey-input"
                value={popupKeyInput}
                onChange={(e) => {
                  setPopupKeyInput(e.target.value);
                  setPopupKeyError("");
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    void savePopupHotkey();
                  }
                }}
                placeholder="Ctrl+Shift+Space"
                spellCheck={false}
                disabled={!aiPopupEnabled}
              />
              <button
                type="button"
                className="seg-btn primary"
                onClick={() => void savePopupHotkey()}
                disabled={popupKeySaving || !aiPopupEnabled}
              >
                {popupKeySaving ? "保存中…" : "保存"}
              </button>
            </div>
          </div>
          {popupKeyError && <div className="setting-row-desc error-text">{popupKeyError}</div>}
          {!popupKeyError && !aiPopupRegistered && aiPopupEnabled && (
            <div className="setting-row-desc error-text">
              AI 小窗快捷键注册失败（可能被其他程序占用），已降级为不可呼出
            </div>
          )}
          <div className="setting-row-desc" style={{ marginTop: 8 }}>
            小窗对话复用主界面 AI 助手的 Key / 模型配置；关闭小窗即清空对话，不落盘
          </div>
        </div>

        <div className="settings-section">
          <div className="settings-label">音频识别</div>
          <div className="setting-row">
            <div className="setting-row-text">
              <div className="setting-row-title">启用音频面板</div>
              <div className="setting-row-desc">桌面右下角显示当前播放的音源、进度与媒体控制；有播放时显示，无播放自动隐藏</div>
            </div>
            <Switch
              checked={audioPanelEnabled}
              onChange={() => void onAudioPanelEnabledChange(!audioPanelEnabled)}
            />
          </div>
          <div className="setting-row-desc" style={{ marginTop: 8 }}>
            音源信息与控制在本地通过系统媒体会话（SMTC）读取；波形由 WASAPI 采集 + FFT 生成，不联网、不写注册表
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
          <span className="settings-version">云笈 · v0.16.7</span>
        </div>
      </div>
    </div>
  );
}
