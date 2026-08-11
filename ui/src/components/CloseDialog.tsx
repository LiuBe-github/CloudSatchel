import { useEffect } from "react";

interface CloseDialogProps {
  onCancel: () => void;
  onMinimizeToTray: () => void;
  onQuit: () => void;
}

/** 关闭窗口时的询问弹窗：最小化到托盘（后台运行）或直接退出 */
export function CloseDialog({ onCancel, onMinimizeToTray, onQuit }: CloseDialogProps) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel]);

  return (
    <div
      className="dialog-overlay"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onCancel();
      }}
    >
      <div
        className="dialog-card noise-bg animate-scale-in"
        role="dialog"
        aria-modal="true"
        aria-label="关闭如意工具箱"
      >
        <div className="dialog-hero">
          <div className="dialog-icon">
            <svg
              viewBox="0 0 24 24"
              width="22"
              height="22"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.8"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
            >
              <path d="M12 3v10" />
              <path d="m7 9 5 5 5-5" />
              <path d="M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" />
            </svg>
          </div>
          <div>
            <h2 className="dialog-title">关闭如意工具箱？</h2>
            <p className="dialog-subtitle">最小化到托盘后，软件继续在后台运行</p>
          </div>
        </div>

        <p className="dialog-desc">
          最小化到托盘后，已开启的双击隐藏图标、透明任务栏等功能保持生效；
          点击系统托盘图标即可随时恢复窗口。
        </p>

        <div className="dialog-actions">
          <button className="dialog-btn ghost" onClick={onCancel}>
            取消
          </button>
          <button className="dialog-btn primary" onClick={onMinimizeToTray}>
            最小化到托盘
          </button>
          <button className="dialog-btn danger" onClick={onQuit}>
            直接退出
          </button>
        </div>
      </div>
    </div>
  );
}
