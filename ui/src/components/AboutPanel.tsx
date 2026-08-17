import { useEffect } from "react";
import appIcon from "../assets/app-icon.png";

interface AboutPanelProps {
  open: boolean;
  onClose: () => void;
}

const APP_VERSION = "v0.11.3";

/** 关于面板：显示软件基本信息（花笺 Floral 式侧边面板） */
export function AboutPanel({ open, onClose }: AboutPanelProps) {
  // Esc 关闭关于面板
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
          <h2 className="settings-title">关于</h2>
          <button className="icon-btn" onClick={onClose} aria-label="关闭关于">
            <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <path d="M18 6 6 18M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div className="about-hero">
          <img className="about-logo" src={appIcon} alt="" draggable={false} />
          <div className="about-titles">
            <div className="about-name">云笈 · Cloud Satchel</div>
            <div className="about-version">{APP_VERSION}</div>
          </div>
        </div>

        <div className="settings-section">
          <div className="settings-label">简介</div>
          <p className="about-desc">
            纯净本地 Windows 桌面工具集：双击隐藏桌面图标、透明任务栏与主机性能监控。
            本地运行、不联网、不写注册表，退出即自动恢复。
          </p>
        </div>

        <div className="settings-section">
          <div className="settings-label">技术栈</div>
          <ul className="purity-list">
            <li><span className="purity-check">✓</span>React 19 + TypeScript + Vite（界面）</li>
            <li><span className="purity-check">✓</span>Tauri 2（Rust）+ Win32 API（桌面壳）</li>
            <li><span className="purity-check">✓</span>界面设计参照「花笺 Floral Notepaper」</li>
          </ul>
        </div>

        <div className="settings-section">
          <div className="settings-label">开源仓库</div>
          <div className="about-link">github.com/LiuBe-github/CloudSatchel</div>
          <p className="about-hint">任务栏引擎内嵌 TranslucentTB（GPL-3.0）</p>
        </div>

        <div className="settings-footer">
          <span className="settings-version">云笈 · {APP_VERSION}</span>
        </div>
      </div>
    </div>
  );
}
