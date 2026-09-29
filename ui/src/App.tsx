import { useCallback, useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  inTauri,
  getState,
  setEnabled,
  setTaskbarTransparent,
  setPerformanceMonitor,
  setPrivacyEnabled,
  setPrivacyIdleSecs,
  setPrivacyBossKey,
  setPrivacyPassword,
  clearPrivacyPassword,
  onPrivacyPasswordRequired,
  onPrivacyPasswordReset,
  setAiPopupEnabled,
  setAiPopupHotkey,
  setAudioPanelEnabled,
  setAudioPanelOpacity,
  setAudioPanelClickThrough,
  setTranslateEnabled,
  setTranslateEngine,
  setTranslateMsRegion,
  setTranslateTargetLang,
  setTranslateSourceLang,
  saveTranslateMsKey,
  setAutohideEnabled,
  setPerfIntervalMs,
  setPerfTaskbarEnabled,
  setPerfTaskbarItems,
  setPerfTaskbarOffsetX,
  setAutostart,
  setCloseToTray,
  setBackground,
  chooseBackgroundImage,
  copyBackgroundImage,
  close,
  minimize,
  toggleMaximize,
  onStateUpdate,
  onTaskbarTransparentFailed,
} from "./lib/bridge";
import { changeTheme, watchSystemTheme, useThemeInit } from "./lib/theme";
import type { AppState, BackgroundSettings, ThemeMode } from "./vite-env";
import { Switch } from "./components/Switch";
import { SettingsPanel } from "./components/SettingsPanel";
import { AboutPanel } from "./components/AboutPanel";
import { BackgroundLayer } from "./components/BackgroundLayer";
import { PerformancePanel } from "./components/PerformancePanel";
import { AiPanel } from "./components/AiPanel";
import { TranslatePanel } from "./components/TranslatePanel";
import { Toast, type ToastHandle } from "./components/Toast";
import { Icon, type IconName } from "./components/Icon";
import appIcon from "./assets/app-icon.png";

interface Feature {
  id: string;
  icon: IconName;
  title: string;
  subtitle: string;
  detail: string;
}

const FEATURES: Feature[] = [
  {
    id: "hide-icons",
    icon: "desktop",
    title: "双击隐藏桌面图标",
    subtitle: "在桌面空白处双击，可快速隐藏 / 显示桌面图标",
    detail: "开启后，双击桌面空白区域即可隐藏所有桌面图标；再次双击恢复显示。双击图标本身仍会正常打开应用，不会误触发。",
  },
  {
    id: "taskbar",
    icon: "taskbar",
    title: "任务栏",
    subtitle: "透明任务栏与自动隐藏，让任务栏更沉浸",
    detail:
      "透明任务栏让任务栏背景消失、与壁纸融为一体（全屏时自动恢复不透明）；自动隐藏开启后任务栏立即隐藏，鼠标移到屏幕下边界弹出。两个开关互不影响、各自持久化，退出应用自动恢复。",
  },
  {
    id: "performance-monitor",
    icon: "performance",
    title: "主机性能监控",
    subtitle: "实时查看 CPU、GPU、内存与网络状态",
    detail:
      "开启后以约 1 秒间隔在本机采集关键性能指标，参照 Windows 任务管理器性能页展示实时曲线与明细。关闭后立即停止采集。",
  },
  {
    id: "privacy",
    icon: "shield",
    title: "隐私操作",
    subtitle: "空闲时自动保护屏幕，防止窥屏",
    detail:
      "开启后，电脑空闲超过设定时间（默认 1 分钟），自动最小化所有窗口、隐藏桌面图标与任务栏并静音；恢复时需输入解锁密码（可在设置中设置），老板键可一键直接恢复。",
  },
  {
    id: "ai",
    icon: "sparkles",
    title: "AI 助手",
    subtitle: "接入你自己的 OpenAI API Key 进行对话",
    detail:
      "配置你自己的接口地址、API Key 与模型名后即可使用（支持 OpenAI 及兼容服务）：流式回复、多轮上下文、可随时停止生成或清空对话。Key 经系统加密保存，仅在你发送消息时访问所配置的接口。",
  },
  {
    id: "audio",
    icon: "audio",
    title: "音频识别",
    subtitle: "桌面右下角媒体面板：音源、进度与波形",
    detail:
      "开启后桌面右下角显示当前播放的音源、标题与进度，支持上一首 / 暂停播放 / 下一首控制与波形可视化；无播放自动隐藏，全屏时隐藏。音源信息通过系统媒体会话（SMTC）本地读取，不联网。",
  },
  {
    id: "translate",
    icon: "translate",
    title: "鼠标选取翻译",
    subtitle: "选中文字松手即弹「翻译」按钮，点击出译文",
    detail:
      "开启后，在任意应用选中一段文字并松开鼠标，文字下方会出现「翻译」按钮；点击弹出翻译界面（引擎可在详情页选择 AI 助理或微软翻译），点击其他位置或按 Esc 即关闭，不影响原应用。",
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
      privacyHasPassword: false,
      privacyBossKey: "Ctrl+`",
      bossKeyRegistered: false,
      aiPopupEnabled: true,
      aiPopupHotkey: "Ctrl+Shift+Space",
      aiPopupRegistered: false,
      audioPanelEnabled: true,
      audioPanelX: -1,
      audioPanelY: -1,
      audioPanelOpacity: 75,
      audioPanelClickThrough: false,
      translateEnabled: true,
      translateEngine: "ai",
      translateMsRegion: "",
      translateTargetLang: "auto-zh-Hans",
      translateSourceLang: "auto",
      translateHasMsKey: false,
      elevated: false,
      fullscreenActive: false,
      autohideEnabled: false,
      perfIntervalMs: 1000,
      perfTaskbarEnabled: false,
      perfTaskbarItems: ["cpu", "memory", "gpu", "gpu_temp", "net"],
      perfTaskbarOffsetX: 0,
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
  const featureRefs = useRef<Array<HTMLButtonElement | null>>([]);
  const settingsTriggerRef = useRef<HTMLButtonElement>(null);
  const aboutTriggerRef = useRef<HTMLButtonElement>(null);
  const lastSidePanelTrigger = useRef<HTMLButtonElement | null>(null);

  // 首次进入：读取后端状态
  useEffect(() => {
    getState()
      .then((s) => {
        setState(s);
        if (s.elevated) {
          toastRef.current?.show(
            "检测到以管理员身份运行：鼠标选取翻译与双击隐藏桌面图标可能失效，建议从开始菜单或桌面快捷方式正常启动",
          );
        }
      })
      .catch(() => {});
  }, []);

  // 监听 Tauri 后端推送的状态更新（桌面双击/动画进行时）
  useEffect(() => {
    const unlisten = onStateUpdate((s) => setState(s));
    const offFailed = onTaskbarTransparentFailed(() => {
      toastRef.current?.show("任务栏透明开启失败：系统阻止了透明引擎", "error");
    });
    const offPwdRequired = onPrivacyPasswordRequired(() => {
      toastRef.current?.show("请先在设置中设置解锁密码，再开启隐私操作", "warning");
    });
    const offPwdReset = onPrivacyPasswordReset(() => {
      toastRef.current?.show("已用开机 PIN 解锁，旧密码已清除，请在设置中重新设置");
    });
    return () => {
      unlisten();
      offFailed();
      offPwdRequired();
      offPwdReset();
    };
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
      const isAudio = activeId === FEATURES[5].id;
      const isTranslate = activeId === FEATURES[6].id;
      // 隐私开关可能被后端拒绝（未设置解锁密码），先记下用户意图
      const wantPrivacyOn = isPrivacy && !state.privacyEnabled;
      const next = isPerformance
        ? await setPerformanceMonitor(!state.performanceMonitor)
        : isPrivacy
          ? await setPrivacyEnabled(!state.privacyEnabled)
          : isAudio
            ? await setAudioPanelEnabled(!state.audioPanelEnabled)
            : isTranslate
              ? await setTranslateEnabled(!state.translateEnabled)
              : await setEnabled(!state.enabled);
      setState(next);
      if (isPerformance) {
        toastRef.current?.show(next.performanceMonitor ? "性能监控已开启" : "性能监控已关闭");
      } else if (isPrivacy && wantPrivacyOn && !next.privacyEnabled) {
        // 后端拒绝开启：privacy-password-required 事件 toast 已提示，
        // 这里不再弹「已关闭」以免覆盖真正的提示
      } else if (isPrivacy) {
        toastRef.current?.show(
          next.privacyEnabled
            ? "隐私操作已开启，空闲时将自动保护屏幕"
            : "隐私操作已关闭",
        );
      } else if (isAudio) {
        toastRef.current?.show(next.audioPanelEnabled ? "音频识别已开启" : "音频识别已关闭");
      } else if (isTranslate) {
        toastRef.current?.show(next.translateEnabled ? "鼠标选取翻译已开启" : "鼠标选取翻译已关闭");
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
  }, [busyToggle, state.enabled, state.performanceMonitor, state.privacyEnabled, state.audioPanelEnabled, state.translateEnabled, activeId]);

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

  const handleBossKeyChange = useCallback(async (key: string) => {
    const next = await setPrivacyBossKey(key);
    setState(next);
    toastRef.current?.show("老板键已更新");
  }, []);

  const handlePrivacyPasswordSave = useCallback(async (password: string) => {
    const next = await setPrivacyPassword(password);
    setState(next);
    toastRef.current?.show("解锁密码已设置");
  }, []);

  const handlePrivacyPasswordClear = useCallback(async () => {
    const next = await clearPrivacyPassword();
    setState(next);
    toastRef.current?.show("解锁密码已清除，隐私操作已关闭");
  }, []);

  const handleAiPopupEnabledChange = useCallback(async (enabled: boolean) => {
    try {
      const next = await setAiPopupEnabled(enabled);
      setState(next);
      toastRef.current?.show(enabled ? "AI 小窗已开启" : "AI 小窗已关闭");
    } catch (err) {
      console.error("切换 AI 小窗失败", err);
      toastRef.current?.show("操作失败，请稍后重试");
    }
  }, []);

  const handleAiPopupHotkeyChange = useCallback(async (key: string) => {
    const next = await setAiPopupHotkey(key);
    setState(next);
    toastRef.current?.show("AI 小窗快捷键已更新");
  }, []);

  const handleAudioOpacityChange = useCallback(async (opacity: number) => {
    try {
      const next = await setAudioPanelOpacity(opacity);
      setState(next);
    } catch (err) {
      console.error("更新音频面板透明度失败", err);
      toastRef.current?.show("操作失败，请稍后重试");
    }
  }, []);

  const handleAudioClickThroughChange = useCallback(async (enabled: boolean) => {
    try {
      const next = await setAudioPanelClickThrough(enabled);
      setState(next);
      toastRef.current?.show(
        enabled
          ? "音频面板已设为鼠标穿透（仅展示，可点击其下方的窗口）"
          : "音频面板已取消鼠标穿透（可拖动/操作）",
      );
    } catch (err) {
      console.error("切换音频面板鼠标穿透失败", err);
      toastRef.current?.show("操作失败，请稍后重试");
    }
  }, []);

  const handleTranslateEnabledChange = useCallback(async (enabled: boolean) => {
    try {
      const next = await setTranslateEnabled(enabled);
      setState(next);
      toastRef.current?.show(enabled ? "鼠标选取翻译已开启" : "鼠标选取翻译已关闭");
    } catch (err) {
      console.error("切换鼠标选取翻译失败", err);
      toastRef.current?.show("操作失败，请稍后重试");
    }
  }, []);

  const handleTranslateEngineChange = useCallback(async (engine: string) => {
    try {
      const next = await setTranslateEngine(engine);
      setState(next);
    } catch (err) {
      console.error("更新翻译引擎失败", err);
      toastRef.current?.show("操作失败，请稍后重试");
    }
  }, []);

  const handleTranslateMsRegionChange = useCallback(async (region: string) => {
    try {
      const next = await setTranslateMsRegion(region);
      setState(next);
    } catch (err) {
      console.error("更新微软翻译区域失败", err);
      toastRef.current?.show("操作失败，请稍后重试");
    }
  }, []);

  const handleSaveTranslateMsKey = useCallback(async (apiKey: string) => {
    await saveTranslateMsKey(apiKey);
    const s = await getState();
    setState(s);
  }, []);

  const handleTranslateTargetLangChange = useCallback(async (lang: string) => {
    try {
      const next = await setTranslateTargetLang(lang);
      setState(next);
    } catch (err) {
      console.error("更新翻译目标语言失败", err);
      toastRef.current?.show("操作失败，请稍后重试");
    }
  }, []);

  const handleTranslateSourceLangChange = useCallback(async (lang: string) => {
    try {
      const next = await setTranslateSourceLang(lang);
      setState(next);
    } catch (err) {
      console.error("更新翻译源语言失败", err);
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

  const handlePerfTaskbarEnabled = useCallback(async (enabled: boolean) => {
    try {
      const next = await setPerfTaskbarEnabled(enabled);
      setState(next);
      toastRef.current?.show(enabled ? "任务栏性能组件已开启" : "任务栏性能组件已关闭");
    } catch (err) {
      console.error("切换任务栏性能组件失败", err);
      toastRef.current?.show("操作失败，请稍后重试");
    }
  }, []);

  const handlePerfTaskbarItems = useCallback(async (items: string[]) => {
    try {
      const next = await setPerfTaskbarItems(items);
      setState(next);
    } catch (err) {
      console.error("更新任务栏组件显示项失败", err);
      toastRef.current?.show("操作失败，请稍后重试");
    }
  }, []);

  const handlePerfTaskbarOffsetX = useCallback(async (offset: number) => {
    try {
      const next = await setPerfTaskbarOffsetX(offset);
      setState(next);
    } catch (err) {
      console.error("更新任务栏组件位置失败", err);
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

  // 侧边面板（花笺 Floral 式）：打开后交给标题聚焦，关闭时回到触发按钮。
  const openSidePanel = useCallback((panel: "settings" | "about", trigger: HTMLButtonElement | null) => {
    lastSidePanelTrigger.current = trigger;
    setSidePanel(panel);
    window.requestAnimationFrame(() => {
      document.querySelector<HTMLElement>(`.side-panel.open [data-panel-heading="${panel}"]`)?.focus();
    });
  }, []);
  const closeSidePanel = useCallback(() => {
    setSidePanel(null);
    window.requestAnimationFrame(() => lastSidePanelTrigger.current?.focus());
  }, []);
  const onFeatureKeyDown = useCallback((event: ReactKeyboardEvent<HTMLButtonElement>, index: number) => {
    const last = FEATURES.length - 1;
    let next: number | null = null;
    if (event.key === "ArrowDown") next = index === last ? 0 : index + 1;
    if (event.key === "ArrowUp") next = index === 0 ? last : index - 1;
    if (event.key === "Home") next = 0;
    if (event.key === "End") next = last;
    if (next === null) return;
    event.preventDefault();
    setActiveId(FEATURES[next].id);
    featureRefs.current[next]?.focus();
  }, []);

  const feature = FEATURES.find((f) => f.id === activeId) ?? FEATURES[0];
  const isTaskbar = activeId === FEATURES[1].id;
  const isPerformance = activeId === FEATURES[2].id;
  const isPrivacy = activeId === FEATURES[3].id;
  const isAi = activeId === FEATURES[4].id;
  const isAudio = activeId === FEATURES[5].id;
  const isTranslate = activeId === FEATURES[6].id;
  const featureOn = isPerformance
    ? state.performanceMonitor
    : isPrivacy
      ? state.privacyEnabled
      : isAudio
        ? state.audioPanelEnabled
        : isTranslate
          ? state.translateEnabled
          : state.enabled;
  const stateHint = isPerformance
    ? state.performanceMonitor
      ? "性能监控 · 当前已开启"
      : "性能监控 · 当前已关闭"
    : isPrivacy
      ? state.privacyActive
        ? "隐私操作 · 已触发保护，输入密码后还原"
        : "隐私操作 · 空闲超过设定时间自动触发"
      : isAudio
        ? state.audioPanelEnabled
          ? "音频识别 · 播放时右下角显示媒体面板"
          : "音频识别 · 当前已关闭"
        : isTranslate
          ? state.translateEnabled
            ? "选取翻译 · 选中文字松手即出现「翻译」按钮"
            : "选取翻译 · 当前已关闭"
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
            ref={settingsTriggerRef}
            className={`icon-btn ${sidePanel === "settings" ? "active" : ""}`}
            onClick={() => (sidePanel === "settings" ? closeSidePanel() : openSidePanel("settings", settingsTriggerRef.current))}
            aria-label="设置"
            aria-expanded={sidePanel === "settings"}
            aria-controls="settings-panel"
            title="设置"
          >
            <Icon name="settings" size={17} />
          </button>
          <button
            ref={aboutTriggerRef}
            className={`icon-btn ${sidePanel === "about" ? "active" : ""}`}
            onClick={() => (sidePanel === "about" ? closeSidePanel() : openSidePanel("about", aboutTriggerRef.current))}
            aria-label="关于"
            aria-expanded={sidePanel === "about"}
            aria-controls="about-panel"
            title="关于"
          >
            <Icon name="info" size={17} />
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
            {FEATURES.map((f, index) => (
              <button
                key={f.id}
                ref={(node) => { featureRefs.current[index] = node; }}
                className={`feature-item ${f.id === activeId ? "active" : ""}`}
                onClick={() => setActiveId(f.id)}
                onKeyDown={(event) => onFeatureKeyDown(event, index)}
                aria-current={f.id === activeId ? "page" : undefined}
              >
                <span className="feature-icon"><Icon name={f.icon} size={18} /></span>
                <span className="feature-name">{f.title}</span>
                {f.id === activeId && <span className="feature-active-dot" />}
              </button>
            ))}
          </nav>
          <div className="sidebar-footer">
          <div className="sidebar-meta">本地纯净工具</div>
          <div className="sidebar-version">v1.4.0</div>
          </div>
        </aside>

        <section className="detail-pane">
          <div className="detail-content" key={activeId}>
          {isPerformance ? (
            <PerformancePanel
              enabled={state.performanceMonitor}
              busy={busyToggle}
              onChange={handleToggle}
              intervalMs={state.perfIntervalMs}
              onIntervalChange={handlePerfInterval}
              taskbarEnabled={state.perfTaskbarEnabled}
              taskbarItems={state.perfTaskbarItems}
              taskbarOffsetX={state.perfTaskbarOffsetX}
              onTaskbarEnabledChange={handlePerfTaskbarEnabled}
              onTaskbarItemsChange={handlePerfTaskbarItems}
              onTaskbarOffsetXChange={handlePerfTaskbarOffsetX}
            />
          ) : isAi ? (
            <AiPanel
              model={state.aiModel}
              baseUrl={state.aiBaseUrl}
              onModelChange={handleAiModelChange}
              onBaseUrlChange={handleAiBaseUrlChange}
            />
          ) : isTaskbar ? (
            <div className="detail-card noise-bg">
              <div className="detail-hero">
                <div className="detail-icon"><Icon name={feature.icon} size={28} /></div>
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
                  label="透明任务栏"
                  busy={busyToggle}
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
                  label="任务栏自动隐藏"
                  busy={busyToggle}
                />
              </div>

              <p className="detail-note">{feature.detail}</p>
            </div>
          ) : isTranslate ? (
            <TranslatePanel
              enabled={state.translateEnabled}
              engine={state.translateEngine}
              msRegion={state.translateMsRegion}
              hasMsKey={state.translateHasMsKey}
              onEnabledChange={handleTranslateEnabledChange}
              onEngineChange={handleTranslateEngineChange}
              onRegionChange={handleTranslateMsRegionChange}
              onSaveMsKey={handleSaveTranslateMsKey}
              targetLang={state.translateTargetLang}
              onTargetLangChange={handleTranslateTargetLangChange}
              sourceLang={state.translateSourceLang}
              onSourceLangChange={handleTranslateSourceLangChange}
            />
          ) : (
            <div className="detail-card noise-bg">
              <div className="detail-hero">
                <div className="detail-icon"><Icon name={feature.icon} size={28} /></div>
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
                <Switch checked={featureOn} onChange={handleToggle} disabled={busyToggle} busy={busyToggle} label={`${feature.title}开关`} />
              </div>

              <p className="detail-note">{feature.detail}</p>
            </div>
          )}
          </div>

          <div className="detail-footer">
            <span className="hint-icon"><Icon name="eye" size={14} /></span>
            {isTaskbar
              ? "透明与自动隐藏互不影响 · 均不写注册表 · 退出应用自动恢复"
              : isPerformance
                ? "仅本机采集 · 不联网 · 关闭后立即停止采样"
                : isPrivacy
                  ? "空闲超时自动保护 · 需密码还原 · 老板键一键恢复 · 退出应用自动还原"
                  : isAi
                    ? "仅在你发送消息时访问你配置的接口地址 · Key 本地加密保存 · 对话不落盘"
                    : isAudio
                      ? "SMTC 本地读取音源 · WASAPI 波形 · 不联网 · 无播放自动隐藏"
                      : isTranslate
                        ? "仅点击「翻译」时联网 · Key 本地加密保存 · 不记录对话内容"
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
          privacyHasPassword={state.privacyHasPassword}
          onPrivacyPasswordSave={handlePrivacyPasswordSave}
          onPrivacyPasswordClear={handlePrivacyPasswordClear}
          privacyBossKey={state.privacyBossKey}
          bossKeyRegistered={state.bossKeyRegistered}
          onBossKeyChange={handleBossKeyChange}
          aiPopupEnabled={state.aiPopupEnabled}
          aiPopupHotkey={state.aiPopupHotkey}
          aiPopupRegistered={state.aiPopupRegistered}
          onAiPopupEnabledChange={handleAiPopupEnabledChange}
          onAiPopupHotkeyChange={handleAiPopupHotkeyChange}
          audioPanelOpacity={state.audioPanelOpacity}
          audioPanelClickThrough={state.audioPanelClickThrough}
          onAudioOpacityChange={handleAudioOpacityChange}
          onAudioClickThroughChange={handleAudioClickThroughChange}

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
