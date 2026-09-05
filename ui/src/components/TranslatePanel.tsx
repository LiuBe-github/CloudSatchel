import { useEffect, useState } from "react";
import { Switch } from "./Switch";

interface TranslatePanelProps {
  enabled: boolean;
  engine: string;
  msRegion: string;
  targetLang: string;
  sourceLang: string;
  hasMsKey: boolean;
  onEnabledChange: (enabled: boolean) => Promise<void> | void;
  onEngineChange: (engine: string) => Promise<void> | void;
  onRegionChange: (region: string) => Promise<void> | void;
  onSaveMsKey: (apiKey: string) => Promise<void>;
  onTargetLangChange: (lang: string) => Promise<void> | void;
  onSourceLangChange: (lang: string) => Promise<void> | void;
}

const SOURCE_LANG_OPTIONS: Array<{ value: string; label: string }> = [
  { value: "auto", label: "自动检测" },
  { value: "zh-Hans", label: "简体中文" },
  { value: "zh-Hant", label: "繁體中文" },
  { value: "en", label: "英语 English" },
  { value: "ja", label: "日语 日本語" },
  { value: "ko", label: "韩语 한국어" },
  { value: "fr", label: "法语 Français" },
  { value: "de", label: "德语 Deutsch" },
  { value: "ru", label: "俄语 Русский" },
  { value: "es", label: "西班牙语 Español" },
];

const TARGET_LANG_OPTIONS: Array<{ value: string; label: string }> = [
  { value: "auto-zh-Hans", label: "自动识别 → 中文（简体）" },
  { value: "zh-Hans", label: "简体中文" },
  { value: "zh-Hant", label: "繁體中文" },
  { value: "en", label: "英语 English" },
  { value: "ja", label: "日语 日本語" },
  { value: "ko", label: "韩语 한국어" },
  { value: "fr", label: "法语 Français" },
  { value: "de", label: "德语 Deutsch" },
  { value: "ru", label: "俄语 Русский" },
  { value: "es", label: "西班牙语 Español" },
];

/** 鼠标选取翻译详情页（FR-19，v0.20.0）：开关 + 引擎 / 微软翻译配置 */
export function TranslatePanel({
  enabled,
  engine,
  msRegion,
  targetLang,
  sourceLang,
  hasMsKey,
  onEnabledChange,
  onEngineChange,
  onRegionChange,
  onSaveMsKey,
  onTargetLangChange,
  onSourceLangChange,
}: TranslatePanelProps) {
  const [msKeyInput, setMsKeyInput] = useState("");
  const [msKeyError, setMsKeyError] = useState("");
  const [msKeySaving, setMsKeySaving] = useState(false);
  const [regionInput, setRegionInput] = useState(msRegion);
  useEffect(() => setRegionInput(msRegion), [msRegion]);

  const saveMsKey = async () => {
    const key = msKeyInput.trim();
    if (!key) {
      setMsKeyError("请输入微软翻译 API Key");
      return;
    }
    setMsKeySaving(true);
    setMsKeyError("");
    try {
      await onSaveMsKey(key);
      setMsKeyInput("");
    } catch (err) {
      setMsKeyError(String(err));
    } finally {
      setMsKeySaving(false);
    }
  };

  return (
    <div className="detail-card noise-bg">
      <div className="detail-hero">
        <div className="detail-icon">译</div>
        <div className="detail-titles">
          <h1 className="detail-title">鼠标选取翻译</h1>
          <p className="detail-subtitle">选中文字松手即弹「翻译」按钮，点击出译文</p>
        </div>
      </div>
      <div className="detail-rule" />
      <div className="setting-row">
        <div className="setting-row-text">
          <div className="setting-row-title">启用选取翻译</div>
          <div className="setting-row-desc">
            任意应用选中文字后松手，文字下方出现「翻译」按钮；点击弹出翻译，点击其他位置或 Esc 关闭
          </div>
        </div>
        <Switch checked={enabled} onChange={() => void onEnabledChange(!enabled)} />
      </div>
      <div className="setting-row">
        <div className="setting-row-text">
          <div className="setting-row-title">翻译引擎</div>
          <div className="setting-row-desc">AI 助理复用「AI 助手」的 Key / 模型 / 接口配置</div>
        </div>
        <select
          className="select-box"
          value={engine}
          onChange={(e) => void onEngineChange(e.target.value)}
          disabled={!enabled}
        >
          <option value="ai">AI 助理</option>
          <option value="microsoft">微软翻译</option>
        </select>
      </div>
      <div className="setting-row">
        <div className="setting-row-text">
          <div className="setting-row-title">源语言</div>
          <div className="setting-row-desc">默认自动检测原文语言</div>
        </div>
        <select
          className="select-box"
          value={sourceLang}
          onChange={(e) => void onSourceLangChange(e.target.value)}
          disabled={!enabled}
        >
          {SOURCE_LANG_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </div>
      <div className="setting-row">
        <div className="setting-row-text">
          <div className="setting-row-title">目标语言</div>
          <div className="setting-row-desc">默认自动识别源语言并翻译为简体中文</div>
        </div>
        <select
          className="select-box"
          value={targetLang}
          onChange={(e) => void onTargetLangChange(e.target.value)}
          disabled={!enabled}
        >
          {TARGET_LANG_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </div>
      {engine === "microsoft" && (
        <>
          <div className="setting-row">
            <div className="setting-row-text">
              <div className="setting-row-title">微软翻译 API Key</div>
              <div className="setting-row-desc">
                {hasMsKey ? "已配置（可粘贴新 Key 更新）" : "未配置（Azure Translator Key）"}
              </div>
            </div>
            <div className="hotkey-editor">
              <input
                type="password"
                className="hotkey-input"
                value={msKeyInput}
                onChange={(e) => {
                  setMsKeyInput(e.target.value);
                  setMsKeyError("");
                }}
                placeholder="Azure Translator Key"
                spellCheck={false}
                disabled={!enabled}
              />
              <button
                type="button"
                className="seg-btn primary"
                onClick={() => void saveMsKey()}
                disabled={msKeySaving || !enabled}
              >
                {msKeySaving ? "保存中…" : "保存"}
              </button>
            </div>
          </div>
          {msKeyError && <div className="setting-row-desc error-text">{msKeyError}</div>}
          <div className="setting-row">
            <div className="setting-row-text">
              <div className="setting-row-title">区域（Region）</div>
              <div className="setting-row-desc">Azure 资源所在区域，如 eastasia / southeastasia；不确定可留空</div>
            </div>
            <input
              className="hotkey-input"
              style={{ maxWidth: 150 }}
              value={regionInput}
              onChange={(e) => setRegionInput(e.target.value)}
              onBlur={() => {
                if (regionInput !== msRegion) void onRegionChange(regionInput);
              }}
              placeholder="eastasia"
              spellCheck={false}
              disabled={!enabled}
            />
          </div>
        </>
      )}
      <p className="detail-note">
        仅点击「翻译」时联网：AI 助理走你配置的接口地址，微软翻译走 Azure Translator（Key 本地加密保存）。
      </p>
    </div>
  );
}
