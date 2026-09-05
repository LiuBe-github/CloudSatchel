import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow, PhysicalPosition, primaryMonitor } from "@tauri-apps/api/window";
import type { MediaState } from "../vite-env";
import { useThemeInit } from "../lib/theme";
import {
  audioMediaControl,
  getMediaState,
  getState,
  getSystemMute,
  getSystemVolume,
  inTauri,
  onAudioWave,
  onMediaState,
  onStateUpdate,
  setSystemMute,
  setSystemVolume,
} from "../lib/bridge";
import appIcon from "../assets/app-icon.png";

/** 从封面图中提取主色（带饱和度的像素均值），稍作提亮作为强调色 */
function extractAccent(src: string): Promise<string> {
  return new Promise((resolve) => {
    const img = new Image();
    img.onload = () => {
      try {
        const size = 16;
        const canvas = document.createElement("canvas");
        canvas.width = size;
        canvas.height = size;
        const ctx = canvas.getContext("2d")!;
        ctx.drawImage(img, 0, 0, size, size);
        const data = ctx.getImageData(0, 0, size, size).data;
        let r = 0,
          g = 0,
          b = 0,
          n = 0;
        for (let i = 0; i < data.length; i += 4) {
          const R = data[i],
            G = data[i + 1],
            B = data[i + 2];
          const mx = Math.max(R, G, B),
            mn = Math.min(R, G, B);
          // 只统计有彩色饱和度的像素，避免被黑白/灰背景冲淡
          if ((mx - mn) / 255 > 0.16) {
            r += R;
            g += G;
            b += B;
            n++;
          }
        }
        let cr = 120,
          cg = 120,
          cb = 120;
        if (n > 0) {
          cr = r / n;
          cg = g / n;
          cb = b / n;
        }
        // 均值偏暗时稍微提亮，保证作为强调色在纸感背景上可读
        const lum = (0.299 * cr + 0.587 * cg + 0.114 * cb) / 255;
        if (lum < 0.35) {
          const boost = (0.35 - lum) * 255;
          cr = Math.min(255, cr + boost);
          cg = Math.min(255, cg + boost);
          cb = Math.min(255, cb + boost);
        }
        resolve(`rgb(${Math.round(cr)}, ${Math.round(cg)}, ${Math.round(cb)})`);
      } catch {
        resolve("");
      }
    };
    img.onerror = () => resolve("");
    img.src = src;
  });
}

/**
 * 音频识别面板（FR-18）：桌面右下角液态玻璃媒体面板。
 * - SMTC 音源信息 / 进度 / 上一首 · 暂停播放 · 下一首控制（网页音频自动置灰切歌按钮）
 * - WASAPI loopback + FFT 波形条（audio-wave 事件）
 * - 无播放时淡出隐藏；全屏（或云笈最大化）时隐藏；可拖拽，位置持久化
 */
export default function AudioPanel() {
  useThemeInit();
  const [media, setMedia] = useState<MediaState | null>(null);
  const [wave, setWave] = useState<number[]>(Array(16).fill(0));
  const [fullscreen, setFullscreen] = useState(false);
  const [positioned, setPositioned] = useState(false);
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [opacity, setOpacity] = useState(75);
  const [accent, setAccent] = useState("");
  const [volume, setVolume] = useState(50);
  const [muted, setMuted] = useState(false);
  const volTimerRef = useRef<number | null>(null);
  const waveRef = useRef<number[]>([]);
  const mediaRef = useRef<{ positionSecs: number; at: number }>({ positionSecs: 0, at: 0 });
  const [, setProgressTick] = useState(0);

  // 初始化：应用持久化位置或计算右下角默认位置；只定位、不显示窗口。
  // 是否显示完全交给下方 visible effect——启动时若没有媒体会话（media 为
  // null，visible=false），窗口保持隐藏，避免在右下角残留透明虚框。
  useEffect(() => {
    if (!inTauri()) return;
    const win = getCurrentWindow();
    void getState().then(async (s) => {
      setEnabled(s.audioPanelEnabled);
      setOpacity(s.audioPanelOpacity);
      if (!s.audioPanelEnabled) return; // 开关关闭：保持隐藏
      try {
        const primary = await primaryMonitor();
        if (primary) {
          const work = primary.workArea;
          const size = await win.outerSize();
          // 右下角安全距离：距右缘/下缘 26px（原 21px，整体左移/上移 5px），不贴着屏幕边缘和任务栏
          const MARGIN = 26;
          // 兜底安全距离：持久化坐标即使贴了最右/最下缘，也至少留 5px
          const SAFE = 5;
          // 优先用持久化位置，否则放主显示器工作区右下角（MARGIN 边距）
          let x =
            s.audioPanelX >= 0
              ? s.audioPanelX
              : work.position.x + work.size.width - size.width - MARGIN;
          let y =
            s.audioPanelY >= 0
              ? s.audioPanelY
              : work.position.y + work.size.height - size.height - MARGIN;
          // clamp 进工作区并保底安全距离：窗口变宽/变高后，残留的贴边坐标
          // 不再合适，强制回拉到距右/下缘至少 SAFE，避免溢出屏边界或盖住任务栏
          x = Math.min(Math.max(x, work.position.x), work.position.x + work.size.width - size.width - SAFE);
          y = Math.min(Math.max(y, work.position.y), work.position.y + work.size.height - size.height - SAFE);
          await win.setPosition(new PhysicalPosition(x, y));
        }
      } catch {
        /* 忽略定位失败（默认窗口位置） */
      }
      setPositioned(true);
    });
  }, []);

  // 读取系统音量与静音状态（音量调节条，v0.20.0）
  useEffect(() => {
    if (!inTauri()) return;
    void getSystemVolume()
      .then((v) => setVolume(Math.round(v * 100)))
      .catch(() => {});
    void getSystemMute().then(setMuted).catch(() => {});
    return () => {
      if (volTimerRef.current) window.clearTimeout(volTimerRef.current);
    };
  }, []);

  const onVolumeChange = (v: number) => {
    setVolume(v);
    if (volTimerRef.current) window.clearTimeout(volTimerRef.current);
    // 滑块拖动高频触发：80ms 节流后写系统音量
    volTimerRef.current = window.setTimeout(() => {
      void setSystemVolume(v / 100).catch(() => {});
    }, 80);
  };

  const toggleMute = () => {
    const next = !muted;
    setMuted(next);
    void setSystemMute(next).catch(() => setMuted(!next));
  };

  // 订阅 SMTC 状态 / 波形 / 全屏状态
  useEffect(() => {
    const offMedia = onMediaState((s) => {
      mediaRef.current = { positionSecs: s.positionSecs, at: Date.now() };
      setMedia(s);
    });
    const offWave = onAudioWave((w) => {
      waveRef.current = w;
      setWave(w);
    });
    const offState = onStateUpdate((s) => {
      setFullscreen(s.fullscreenActive);
      setEnabled(s.audioPanelEnabled);
      setOpacity(s.audioPanelOpacity);
    });
    // 慢机器上 WebView 冷启动可能错过首次 media-state 边沿事件（启动即播放 /
    // WASAPI 兜底只在 arm 瞬间发一次）→ 挂载后主动查询当前状态补救
    void getMediaState()
      .then((s) => {
        if (s) {
          mediaRef.current = { positionSecs: s.positionSecs, at: Date.now() };
          setMedia(s);
        }
      })
      .catch(() => {});
    return () => {
      offMedia();
      offWave();
      offState();
    };
  }, []);

  // 面板可见性：开关开启 + 有媒体会话 + 非全屏才显示；无播放/关闭/全屏隐藏。
  // 注意：acrylic 毛玻璃是窗口级属性（不随内容 opacity 淡出），
  // 因此「隐藏」必须是窗口级 hide/show，而非仅 CSS 淡出。
  const visible =
    enabled !== false &&
    positioned &&
    media !== null &&
    media.supported &&
    media.active &&
    !fullscreen;

  // 窗口级显示/隐藏（acrylic 无法随内容淡出）
  useEffect(() => {
    if (!inTauri()) return;
    const win = getCurrentWindow();
    if (visible) {
      void win.show();
    } else {
      void win.hide();
    }
  }, [visible]);

  // 封面换曲时提取主题强调色（空封面/提取失败则回落默认竹青）
  useEffect(() => {
    if (media?.thumbnail) {
      void extractAccent(media.thumbnail).then((c) => setAccent(c));
    } else {
      setAccent("");
    }
  }, [media?.thumbnail]);

  const control = useCallback((action: "prev" | "play" | "pause" | "next") => {
    audioMediaControl(action);
  }, []);

  // 播放中本地推进进度：事件驱动后 SMTC 不再每秒推送，进度条由本地 1 秒递增保持平滑
  useEffect(() => {
    if (!visible || !media?.playing) return;
    const timer = window.setInterval(() => setProgressTick((t) => t + 1), 1000);
    return () => window.clearInterval(timer);
  }, [visible, media?.playing]);

  const progressAt = mediaRef.current.at || Date.now();
  const elapsedSecs = media?.playing ? (Date.now() - progressAt) / 1000 : 0;
  const progress =
    media && media.durationSecs > 0
      ? Math.min(100, ((mediaRef.current.positionSecs + elapsedSecs) / media.durationSecs) * 100)
      : 0;

  // 副标题：歌手 · 专辑优先；无歌手信息时显示应用名
  const subText = media?.fallback
    ? "系统音频 · 未识别到媒体信息"
    : media?.artist
      ? media.album
        ? `${media.artist} · ${media.album}`
        : media.artist
      : media?.appName || (media?.active ? "媒体会话" : "");

  const styleVars = {
    ...(accent ? ({ "--audio-accent": accent } as React.CSSProperties) : {}),
    "--audio-bg-top": `${opacity}%`,
    "--audio-bg-bottom": `${Math.min(100, opacity + 4)}%`,
  } as React.CSSProperties;

  return (
    <div
      className={`audio-panel${visible ? " visible" : ""}`}
      style={styleVars}
    >
      <div className="audio-panel-body">
        <div className={`audio-panel-art${media?.thumbnail ? "" : " placeholder"}`}>
          <img src={media?.thumbnail || appIcon} alt="" draggable={false} />
        </div>
        <div className="audio-panel-info">
          <div className="audio-panel-title" title={media?.title || ""}>
            {media?.title || "未在播放"}
          </div>
          <div className="audio-panel-sub" title={subText}>
            {subText}
          </div>
        </div>
        <div className="audio-panel-controls">
          <button
            className="audio-btn"
            onClick={() => control("prev")}
            disabled={!media?.prevEnabled}
            title="上一首"
          >
            <svg viewBox="0 0 24 24" width="13" height="13" fill="currentColor">
              <path d="M6 6h2v12H6zM20 6l-10 6 10 6z" />
            </svg>
          </button>
          {media?.playing ? (
            <button
              className="audio-btn audio-btn-main"
              onClick={() => control("pause")}
              disabled={!media?.pauseEnabled}
              title="暂停"
            >
              <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor">
                <path d="M7 5h4v14H7zM13 5h4v14h-4z" />
              </svg>
            </button>
          ) : (
            <button
              className="audio-btn audio-btn-main"
              onClick={() => control("play")}
              disabled={!media?.playEnabled}
              title="播放"
            >
              <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor">
                <path d="M8 5v14l11-7z" />
              </svg>
            </button>
          )}
          <button
            className="audio-btn"
            onClick={() => control("next")}
            disabled={!media?.nextEnabled}
            title="下一首"
          >
            <svg viewBox="0 0 24 24" width="13" height="13" fill="currentColor">
              <path d="M16 6h2v12h-2zM4 6l10 6-10 6z" />
            </svg>
          </button>
        </div>
      </div>
      <div className="audio-panel-volume">
        <button
          type="button"
          className={`audio-vol-btn${muted ? " muted" : ""}`}
          onClick={toggleMute}
          title={muted ? "取消静音" : "静音"}
        >
          <svg viewBox="0 0 24 24" width="13" height="13" fill="currentColor">
            <path d="M3 9v6h4l5 4V5L7 9H3z" />
            {muted && (
              <path
                d="M16 8l6 8M22 8l-6 8"
                stroke="currentColor"
                strokeWidth="1.8"
                fill="none"
              />
            )}
          </svg>
        </button>
        <input
          type="range"
          className="audio-vol-slider"
          min={0}
          max={100}
          step={1}
          value={volume}
          onChange={(e) => onVolumeChange(Number(e.target.value))}
          title="系统音量"
        />
        <span className="audio-vol-text">{volume}%</span>
      </div>
      <div className="audio-panel-progress">
        <div className="audio-panel-progress-bar" style={{ width: `${progress}%` }} />
      </div>
      <div className="audio-panel-wave">
        {wave.map((v, i) => (
          <span
            key={i}
            className="audio-wave-bar"
            style={{
              // 非线性放大：pow 提升低能量让震动更明显，底座抬到 10% 避免静止；
              // v0.20.1 系数 135→175、指数 0.75→0.7，低能量更活跃（震动幅度更大）
              height: `${Math.min(
                100,
                Math.max(10, Math.round(Math.pow(Math.max(0, v), 0.7) * 175)),
              )}%`,
            }}
          />
        ))}
      </div>
    </div>
  );
}
