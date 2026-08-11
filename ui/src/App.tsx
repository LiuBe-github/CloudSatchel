import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  inTauri,
  getState,
  setEnabled,
  setTaskbarTransparent,
  setAutostart,
  close,
  minimize,
  toggleMaximize,
  onStateUpdate,
  onCloseRequested,
  hideToTray,
  quitApp,
} from "./lib/bridge";
import { changeTheme, watchSystemTheme, useThemeInit } from "./lib/theme";
import type { AppState, ThemeMode } from "./vite-env";
import { Switch } from "./components/Switch";
import { SettingsPanel } from "./components/SettingsPanel";
import { Toast, type ToastHandle } from "./components/Toast";
import { CloseDialog } from "./components/CloseDialog";
import appIcon from "./assets/app-icon.png";

const FEATURES = [
  {
    id: "hide-icons",
    icon: "◵",
    title: "双击隐藏桌面图标",
    subtitle: "在桌面空白处双击，可快速隐藏 / 显示桌面图标",
    detail: "开启后，双击桌面空白区域即可隐藏所有桌面图标；再次双击恢复显示。双击图标本身仍会正常打开应用，不会误触发。",
  },
  {
    id: "transparent-taskbar",
    icon: "▭",
    title: "透明任务栏",
    subtitle: "让任务栏背景消失，只保留任务按钮",
    detail:
      "开启后任务栏背景消失，只保留任务按钮，与壁纸融为一体（约 1~2 秒生效）；关闭后恢复系统默认外观。",
  },
];

interface AppProps {
  initial?: AppState;
}

function App({ initial }: AppProps) {
  useThemeInit();
  const [state, setState] = useState<AppState>(
    initial ?? {
      enabled: true,
      iconsHidden: false,
      taskbarTransparent: false,
      theme: "system",
      animating: false,
      autostart: false,
    },
  );
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [activeId, setActiveId] = useState(FEATURES[0].id);
  const [busyToggle, setBusyToggle] = useState(false);
  const [closePromptOpen, setClosePromptOpen] = useState(false);
  const [maximized, setMaximized] = useState(false);
  const toastRef = useRef<ToastHandle>(null);

  // 首次进入：读取后端状态
  useEffect(() => {
    getState()
      .then((s) => setState(s))
      .catch(() => {});
  }, []);

  // 监听 Tauri 后端推送的状态更新（桌面双击/动画进行时）
  useEffect(() => {
    const unlisten = onStateUpdate((s) => setState(s));
    return unlisten;
  }, []);

  // 关闭窗口请求：弹出「最小化到托盘 / 直接退出」询问
  useEffect(() => {
    const unlisten = onCloseRequested(() => setClosePromptOpen(true));
    return unlisten;
  }, []);

  // 主题跟随系统
  useEffect(() => {
    const cleanup = watchSystemTheme(state.theme);
    return cleanup;
  }, [state.theme]);

  // 最大化时圆角归零：Windows 最大化窗口是直角，
  // 若内容仍带圆角，四个角会透出桌面形成“虚框”
  useEffect(() => {
    if (!inTauri()) return;
    let cancelled = false;
    const win = getCurrentWindow();
    const update = async () => {
      const m = await win.isMaximized();
      if (!cancelled) setMaximized(m);
    };
    void update();
    const unlisten = win.onResized(() => void update());
    return () => {
      cancelled = true;
      void unlisten.then((fn) => fn());
    };
  }, []);

  const handleToggle = useCallback(async () => {
    if (busyToggle) return;
    setBusyToggle(true);
    try {
      const isTaskbar = activeId === FEATURES[1].id;
      const next = isTaskbar
        ? await setTaskbarTransparent(!state.taskbarTransparent)
        : await setEnabled(!state.enabled);
      setState(next);
      if (isTaskbar) {
        toastRef.current?.show(next.taskbarTransparent ? "任务栏已透明化" : "已恢复系统默认任务栏");
      } else if (next.enabled) {
        toastRef.current?.show("功能已激活，现在可以双击桌面空白处切换图标");
      } else {
        toastRef.current?.show("功能已停用，双击桌面不再生效");
      }
    } catch (err) {
      console.error("切换功能状态失败", err);
      toastRef.current?.show("操作失败，请稍后重试");
    } finally {
      setBusyToggle(false);
    }
  }, [busyToggle, state.enabled, state.taskbarTransparent, activeId]);

  const handleTheme = useCallback(
    async (mode: ThemeMode) => {
      await changeTheme(mode);
      setState((s) => ({ ...s, theme: mode }));
    },
    [],
  );

  const handleAutostart = useCallback(async (enabled: boolean) => {
    try {
      const next = await setAutostart(enabled);
      setState(next);
      toastRef.current?.show(enabled ? "已开启开机自启动" : "已关闭开机自启动");
    } catch (err) {
      console.error("切换开机自启动失败", err);
      toastRef.current?.show("操作失败，请稍后重试");
    }
  }, []);

  const handleMinimizeToTray = useCallback(() => {
    setClosePromptOpen(false);
    hideToTray();
  }, []);

  const handleQuit = useCallback(() => {
    setClosePromptOpen(false);
    quitApp();
  }, []);

  const feature = FEATURES.find((f) => f.id === activeId) ?? FEATURES[0];
  const isTaskbar = activeId === FEATURES[1].id;
  const featureOn = isTaskbar ? state.taskbarTransparent : state.enabled;
  const stateHint = isTaskbar
    ? state.taskbarTransparent
      ? "任务栏 · 当前已透明"
      : "任务栏 · 当前为系统默认"
    : state.iconsHidden
      ? "桌面图标 · 当前已隐藏"
      : "桌面图标 · 当前已显示";

  return (
    <div className={`app-shell${maximized ? " maximized" : ""}`}>
      {/* 标题栏（data-tauri-drag-region 实现无边框拖拽） */}
      <header className="titlebar" data-tauri-drag-region>
        <div className="titlebar-title" data-tauri-drag-region>
          <img className="brand-img" src={appIcon} alt="" draggable={false} />
          云笈
        </div>
        <div className="titlebar-actions">
          <button
            className={`icon-btn ${settingsOpen ? "active" : ""}`}
            onClick={() => setSettingsOpen((v) => !v)}
            aria-label="设置"
            title="设置"
          >
            <svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
              <circle cx="12" cy="12" r="3.2" />
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h.01a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51h.01a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v.01a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
            </svg>
          </button>
          <button className="win-btn" onClick={minimize} aria-label="最小化" title="最小化">
            <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true">
              <path d="M0 5.2h10" stroke="currentColor" strokeWidth="1" fill="none" />
            </svg>
          </button>
          <button className="win-btn" onClick={toggleMaximize} aria-label="最大化" title="最大化">
            <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true">
              <rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" strokeWidth="1" />
            </svg>
          </button>
          <button className="win-btn win-close" onClick={close} aria-label="关闭" title="关闭">
            <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true">
              <path d="M0 0l10 10M10 0L0 10" stroke="currentColor" strokeWidth="1.1" fill="none" />
            </svg>
          </button>
        </div>
      </header>

      {/* 主体：左侧功能列表 + 右侧功能详情 */}
      <main className="app-body">
        <aside className="sidebar noise-bg">
          <div className="sidebar-heading">功能</div>
          <nav className="feature-list">
            {FEATURES.map((f) => (
              <button
                key={f.id}
                className={`feature-item ${f.id === activeId ? "active" : ""}`}
                onClick={() => setActiveId(f.id)}
              >
                <span className="feature-icon">{f.icon}</span>
                <span className="feature-name">{f.title}</span>
                {f.id === activeId && <span className="feature-active-dot" />}
              </button>
            ))}
          </nav>
          <div className="sidebar-footer">
          <div className="sidebar-meta">本地纯净工具</div>
          <div className="sidebar-version">v0.5.0</div>
          </div>
        </aside>

        <section className="detail-pane">
          <div className="detail-card noise-bg animate-scale-in" key={feature.id}>
            <div className="detail-hero">
              <div className="detail-icon">{feature.icon}</div>
              <div className="detail-titles">
                <h1 className="detail-title">{feature.title}</h1>
                <p className="detail-subtitle">{feature.subtitle}</p>
              </div>
            </div>

            <div className="detail-rule" />

            <div className="detail-row">
              <div className="detail-state">
                <span
                  className={`state-dot ${featureOn ? "on" : "off"}`}
                  style={{ background: state.animating ? "var(--color-bamboo-light)" : undefined }}
                />
                <div>
                  <div className="state-label">{featureOn ? "功能已激活" : "功能已停用"}</div>
                  <div className="state-hint">{stateHint}</div>
                </div>
              </div>
              <Switch checked={featureOn} onChange={handleToggle} disabled={busyToggle} />
            </div>

            <p className="detail-note">{feature.detail}</p>
          </div>

          <div className="detail-footer">
            <span className="hint-icon">␣</span>
            {isTaskbar ? "任务栏背景消失 · 退出应用自动恢复" : "桌面空白处双击可快速切换 · 仅当功能激活时生效"}
          </div>
        </section>
      </main>

      {settingsOpen && (
        <SettingsPanel
          theme={state.theme}
          onThemeChange={handleTheme}
          autostart={state.autostart}
          onAutostartChange={handleAutostart}
          onClose={() => setSettingsOpen(false)}
        />
      )}

      {closePromptOpen && (
        <CloseDialog
          onCancel={() => setClosePromptOpen(false)}
          onMinimizeToTray={handleMinimizeToTray}
          onQuit={handleQuit}
        />
      )}

      <Toast ref={toastRef} />
    </div>
  );
}

export default App;
