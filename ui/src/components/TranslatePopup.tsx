import { useEffect, useState } from "react";
import {
  inTauri,
  onTranslateCleared,
  onTranslatePending,
  onTranslateResult,
  translateHide,
} from "../lib/bridge";
import { useThemeInit } from "../lib/theme";

interface PopupState {
  source: string;
  engine: string;
  target: string;
  ok: boolean;
  error: string;
  loading: boolean;
}

const IDLE: PopupState = {
  source: "",
  engine: "",
  target: "",
  ok: true,
  error: "",
  loading: false,
};

/**
 * 翻译弹窗（translate-popup 窗口）：显示源文本与译文；
 * 弹窗失焦（点击其他位置）或按 Esc → 隐藏按钮与弹窗。
 */
export default function TranslatePopup() {
  useThemeInit();
  const [data, setData] = useState<PopupState>(IDLE);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!inTauri()) return;
    const offPending = onTranslatePending((p) =>
      setData({
        source: p.source,
        engine: p.engine,
        target: "",
        ok: true,
        error: "",
        loading: true,
      }),
    );
    const offResult = onTranslateResult((r) =>
      setData({
        source: r.source,
        engine: r.engine,
        target: r.target,
        ok: r.ok,
        error: r.error,
        loading: false,
      }),
    );
    const offCleared = onTranslateCleared(() => setData(IDLE));
    const onBlur = () => translateHide();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") translateHide();
    };
    window.addEventListener("blur", onBlur);
    window.addEventListener("keydown", onKey);
    return () => {
      offPending();
      offResult();
      offCleared();
      window.removeEventListener("blur", onBlur);
      window.removeEventListener("keydown", onKey);
    };
  }, []);

  const copy = async () => {
    if (!data.target) return;
    try {
      await navigator.clipboard.writeText(data.target);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      /* 剪贴板不可用时静默 */
    }
  };

  return (
    <div className="translate-popup">
      <div className="translate-popup-head">
        <span className="translate-popup-title">
          翻译{data.engine ? ` · ${data.engine}` : ""}
        </span>
        <button
          type="button"
          className="icon-btn"
          onClick={() => translateHide()}
          aria-label="关闭翻译"
        >
          <svg
            viewBox="0 0 24 24"
            width="14"
            height="14"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
          >
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      </div>
      {data.source && (
        <div className="translate-popup-source" title={data.source}>
          {data.source}
        </div>
      )}
      <div className="translate-popup-body">
        {data.loading ? (
          <div className="translate-popup-loading">翻译中…</div>
        ) : data.target ? (
          <>
            <div className="translate-popup-result">{data.target}</div>
            <button
              type="button"
              className="seg-btn primary translate-popup-copy"
              onClick={() => void copy()}
            >
              {copied ? "已复制" : "复制译文"}
            </button>
          </>
        ) : data.error ? (
          <div className="translate-popup-err">{data.error}</div>
        ) : (
          <div className="translate-popup-hint">选中文字后点击「翻译」按钮</div>
        )}
      </div>
      <div className="translate-popup-foot">点击其他位置或按 Esc 关闭</div>
    </div>
  );
}
