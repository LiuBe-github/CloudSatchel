import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  inTauri,
  getState,
  setEnabled,
  setTaskbarTransparent,
  setPerformanceMonitor,
  setPrivacyEnabled,
  setPrivacyIdleSecs,
  setAutohideEnabled,
  setPerfIntervalMs,
  setAutostart,
  setCloseToTray,
  setBackground,
  chooseBackgroundImage,
  copyBackgroundImage,
  close,
  minimize,
  toggleMaximize,
  onStateUpdate,
} from "./lib/bridge";
import { changeTheme, watchSystemTheme, useThemeInit } from "./lib/theme";
import type { AppState, BackgroundSettings, ThemeMode } from "./vite-env";
import { Switch } from "./components/Switch";
import { SettingsPanel } from "./components/SettingsPanel";
import { AboutPanel } from "./components/AboutPanel";
import { BackgroundLayer } from "./components/BackgroundLayer";
import { PerformancePanel } from "./components/PerformancePanel";
import { AiPanel } from "./components/AiPanel";
import { Toast, type ToastHandle } from "./components/Toast";
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
    id: "taskbar",
    icon: "▭",
    title: "任务栏",
    subtitle: "透明任务栏与自动隐藏，让任务栏更沉浸",
    detail:
      "透明任务栏让任务栏背景消失、与壁纸融为一体（全屏时自动恢复不透明）；自动隐藏开启后任务栏立即隐藏，鼠标移到屏幕下边界弹出。两个开关互不影响、各自持久化，退出应用自动恢复。",
  },
  {
    id: "performance-monitor",
    icon: "▥",
    title: "主机性能监控",
    subtitle: "实时查看 CPU、GPU、内存与网络状态",
    detail:
      "开启后以约 1 秒间隔在本机采集关键性能指标，参照 Windows 任务管理器性能页展示实时曲线与明细。关闭后立即停止采集。",
  },
  {
    id: "privacy",
    icon: "◉",
    title: "隐私操作",
    subtitle: "空闲时自动保护屏幕，防止窥屏",
    detail:
      "开启后，电脑空闲超过设定时间（默认 1 分钟），自动最小化所有窗口、隐藏桌面图标与任务栏并静音；您一操作鼠标或键盘，立即全部还原。",
  },
  {
    id: "ai",
    icon: "✳",
    title: "AI 助手",
    subtitle: "接入你自己的 OpenAI API Key 进行对话",
    detail:
      "配置你自己的接口地址、API Key 与模型名后即可使用（支持 OpenAI 及兼容服务）：流式回复、多轮上下文、可随时停止生成或清空对话。Key 经系统加密保存，仅在你发送消息时访问所配置的接口。",
  },
];

function backgroundOf(state: AppState): BackgroundSettings {
  return {
    imagePath: state.backgroundImagePath,
    fit: state.backgroundFit,
    dim: state.backgroundDim,
    blur: state.backgroundBlur,
    scale: state.backgroundScale,
    positionX: state.backgroundPositionX,
    positionY: state.backgroundPositionY,
  };
}

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
      performanceMonitor: false,
      privacyEnabled: false,
      privacyIdleSecs: 60,
      privacyActive: false,
      autohideEnabled: false,
      perfIntervalMs: 1000,
      aiModel: "gpt-4o-mini",
      aiBaseUrl: "https://api.openai.com/v1",
      theme: "system",
      animating: false,
      autostart: false,
      closeToTray: true,
      backgroundImagePath: "",
      backgroundFit: "cover",
      backgroundDim: 0.25,
      backgroundBlur: 0,
      backgroundScale: 1,
      backgroundPositionX: 50,
      backgroundPositionY: 50,
    },
  );
  const [sidePanel, setSidePanel] = useState<"settings" | "about" | null>(null);
  const [activeId, setActiveId] = useState(FEATURES[0].id);
  const [busyToggle, setBusyToggle] = useState(false);
  const [maximized, setMaximized] = useState(false);
  const [backgroundName, setBackgroundName] = useState(
    () => localStorage.getItem("backgroundImageName") ?? "",
  );
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
      const isPerformance = activeId === FEATURES[2].id;
      const isPrivacy = activeId === FEATURES[3].id;
      const next = isPerformance
        ? await setPerformanceMonitor(!state.performanceMonitor)
        : isPrivacy
          ? await setPrivacyEnabled(!state.privacyEnabled)
          : await setEnabled(!state.enabled);
      setState(next);
      if (isPerformance) {
        toastRef.current?.show(next.performanceMonitor ? "性能监控已开启" : "性能监控已关闭");
      } else if (isPrivacy) {
        toastRef.current?.show(
          next.privacyEnabled
            ? "隐私操作已开启，空闲时将自动保护屏幕"
            : "隐私操作已关闭",
        );
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
  }, [busyToggle, state.enabled, state.performanceMonitor, state.privacyEnabled, activeId]);

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

  const handleCloseToTray = useCallback(async (enabled: boolean) => {
    try {
      const next = await setCloseToTray(enabled);
      setState(next);
      toastRef.current?.show(enabled ? "已开启：关闭窗口时最小化到托盘" : "已关闭：关闭窗口时直接退出");
    } catch (err) {
      console.error("切换关闭到托盘失败", err);
      toastRef.current?.show("操作失败，请稍后重试");
    }
  }, []);

  const handleTaskbarTransparent = useCallback(async (enabled: boolean) => {
    try {
      const next = await setTaskbarTransparent(enabled);
      setState(next);
      toastRef.current?.show(enabled ? "任务栏已透明化" : "已恢复系统默认任务栏");
    } catch (err) {
      console.error("切换透明任务栏失败", err);
      toastRef.current?.show("操作失败，请稍后重试");
    }
  }, []);

  const handlePrivacyIdle = useCallback(async (secs: number) => {
    try {
      const next = await setPrivacyIdleSecs(secs);
      setState(next);
    } catch (err) {
      console.error("更新隐私操作空闲时间失败", err);
      toastRef.current?.show("操作失败，请稍后重试");
    }
  }, []);

  const handleAutohideEnabled = useCallback(async (enabled: boolean) => {
    try {
      const next = await setAutohideEnabled(enabled);
      setState(next);
      toastRef.current?.show(enabled ? "任务栏自动隐藏已开启（立即隐藏）" : "任务栏自动隐藏已关闭");
    } catch (err) {
      console.error("切换任务栏自动隐藏失败", err);
      toastRef.current?.show("操作失败，请稍后重试");
    }
  }, []);

  const handlePerfInterval = useCallback(async (ms: number) => {
    try {
      const next = await setPerfIntervalMs(ms);
      setState(next);
    } catch (err) {
      console.error("更新性能监控刷新间隔失败", err);
      toastRef.current?.show("操作失败，请稍后重试");
    }
  }, []);

  const handleAiModelChange = useCallback((model: string) => {
    setState((s) => ({ ...s, aiModel: model }));
  }, []);

  const handleAiBaseUrlChange = useCallback((baseUrl: string) => {
    setState((s) => ({ ...s, aiBaseUrl: baseUrl }));
  }, []);

  const handleBackgroundChange = useCallback(async (next: BackgroundSettings) => {
    try {
      const s = await setBackground(next);
      setState(s);
    } catch (err) {
      console.error("更新背景图片设置失败", err);
      toastRef.current?.show("操作失败，请稍后重试");
    }
  }, []);

  const handleChooseBackground = useCallback(async () => {
    try {
      const path = await chooseBackgroundImage();
      if (!path) return;
      const name = path.split(/[\\/]/).pop() ?? "";
      const saved = await copyBackgroundImage(path);
      localStorage.setItem("backgroundImageName", name);
      setBackgroundName(name);
      const s = await setBackground({ ...backgroundOf(state), imagePath: saved });
      setState(s);
      toastRef.current?.show("背景图片已更新");
    } catch (err) {
      console.error("选择背景图片失败", err);
      toastRef.current?.show("操作失败，请稍后重试");
    }
  }, [state]);

  const handleClearBackground = useCallback(async () => {
    try {
      localStorage.removeItem("backgroundImageName");
      setBackgroundName("");
      const s = await setBackground({ ...backgroundOf(state), imagePath: "" });
      setState(s);
      toastRef.current?.show("已恢复默认背景");
    } catch (err) {
      console.error("清除背景图片失败", err);
      toastRef.current?.show("操作失败，请稍后重试");
    }
  }, [state]);

  // 侧边面板（花笺 Floral 式）：设置 / 关于 共用主内容区右侧推开面板
  const openSettings = useCallback(() => setSidePanel("settings"), []);
  const openAbout = useCallback(() => setSidePanel("about"), []);
  const closeSidePanel = useCallback(() => setSidePanel(null), []);

  const feature = FEATURES.find((f) => f.id === activeId) ?? FEATURES[0];
  const isTaskbar = activeId === FEATURES[1].id;
  const isPerformance = activeId === FEATURES[2].id;
  const isPrivacy = activeId === FEATURES[3].id;
  const isAi = activeId === FEATURES[4].id;
  const featureOn = isPerformance
    ? state.performanceMonitor
    : isPrivacy
      ? state.privacyEnabled
      : state.enabled;
  const stateHint = isPerformance
    ? state.performanceMonitor
      ? "性能监控 · 当前已开启"
      : "性能监控 · 当前已关闭"
    : isPrivacy
      ? state.privacyActive
        ? "隐私操作 · 已触发保护，操作鼠标或键盘后自动还原"
        : "隐私操作 · 空闲超过设定时间自动触发"
      : state.iconsHidden
        ? "桌面图标 · 当前已隐藏"
        : "桌面图标 · 当前已显示";

  return (
    <div className={`app-shell${maximized ? " maximized" : ""}`}>
      <BackgroundLayer state={state} />
      {/* 标题栏（data-tauri-drag-region 实现无边框拖拽） */}
      <header className="titlebar" data-tauri-drag-region>
        <div className="titlebar-title" data-tauri-drag-region>
          <img className="brand-img" src={appIcon} alt="" draggable={false} />
          云笈
        </div>
        <div className="titlebar-actions">
          <button
            className={`icon-btn ${sidePanel === "settings" ? "active" : ""}`}
            onClick={() => (sidePanel === "settings" ? closeSidePanel() : openSettings())}
            aria-label="设置"
            title="设置"
          >
            <svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
              <circle cx="12" cy="12" r="3.2" />
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h.01a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51h.01a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v.01a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
            </svg>
          </button>
          <button
            className={`icon-btn ${sidePanel === "about" ? "active" : ""}`}
            onClick={() => (sidePanel === "about" ? closeSidePanel() : openAbout())}
            aria-label="关于"
            title="关于"
          >
            <svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round">
              <circle cx="12" cy="12" r="9" />
              <path d="M12 11v5" />
              <path d="M12 7.5h.01" />
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
          <div className="sidebar-version">v0.12.1</div>
          </div>
        </aside>

        <section className="detail-pane">
          {isPerformance ? (
            <PerformancePanel
              enabled={state.performanceMonitor}
              busy={busyToggle}
              onChange={handleToggle}
              intervalMs={state.perfIntervalMs}
              onIntervalChange={handlePerfInterval}
            />
          ) : isAi ? (
            <AiPanel
              model={state.aiModel}
              baseUrl={state.aiBaseUrl}
              onModelChange={handleAiModelChange}
              onBaseUrlChange={handleAiBaseUrlChange}
            />
          ) : isTaskbar ? (
            <div className="detail-card noise-bg animate-scale-in">
              <div className="detail-hero">
                <div className="detail-icon">{feature.icon}</div>
                <div className="detail-titles">
                  <h1 className="detail-title">{feature.title}</h1>
                  <p className="detail-subtitle">{feature.subtitle}</p>
                </div>
              </div>

              <div className="detail-rule" />

              <div className="setting-row">
                <div className="setting-row-text">
                  <div className="setting-row-title">透明任务栏</div>
                  <div className="setting-row-desc">
                    任务栏背景消失，与壁纸融为一体；全屏或云笈最大化时自动恢复不透明
                  </div>
                </div>
                <Switch
                  checked={state.taskbarTransparent}
                  onChange={() => handleTaskbarTransparent(!state.taskbarTransparent)}
                  disabled={busyToggle}
                />
              </div>
              <div className="setting-row">
                <div className="setting-row-text">
                  <div className="setting-row-title">任务栏自动隐藏</div>
                  <div className="setting-row-desc">
                    开启后立即隐藏；鼠标移到屏幕下边界弹出，移开再隐藏
                  </div>
                </div>
                <Switch
                  checked={state.autohideEnabled}
                  onChange={() => handleAutohideEnabled(!state.autohideEnabled)}
                  disabled={busyToggle}
                />
              </div>

              <p className="detail-note">{feature.detail}</p>
            </div>
          ) : (
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
          )}

          <div className="detail-footer">
            <span className="hint-icon">␣</span>
            {isTaskbar
              ? "透明与自动隐藏互不影响 · 均不写注册表 · 退出应用自动恢复"
              : isPerformance
                ? "仅本机采集 · 不联网 · 关闭后立即停止采样"
                : isPrivacy
                  ? "空闲超时自动保护 · 操作鼠标或键盘立即还原 · 退出应用自动恢复"
                  : isAi
                    ? "仅在你发送消息时访问你配置的接口地址 · Key 本地加密保存 · 对话不落盘"
                    : "桌面空白处双击可快速切换 · 仅当功能激活时生效"}
          </div>
        </section>

        <SettingsPanel
          open={sidePanel === "settings"}
          theme={state.theme}
          onThemeChange={handleTheme}
          autostart={state.autostart}
          onAutostartChange={handleAutostart}
          closeToTray={state.closeToTray}
          onCloseToTrayChange={handleCloseToTray}
          privacyIdleSecs={state.privacyIdleSecs}
          onPrivacyIdleChange={handlePrivacyIdle}
          background={backgroundOf(state)}
          backgroundName={backgroundName}
          onBackgroundChange={handleBackgroundChange}
          onChooseBackground={handleChooseBackground}
          onClearBackground={handleClearBackground}
          onClose={closeSidePanel}
        />
        <AboutPanel open={sidePanel === "about"} onClose={closeSidePanel} />
      </main>

      <Toast ref={toastRef} />
    </div>
  );
}

export default App;
