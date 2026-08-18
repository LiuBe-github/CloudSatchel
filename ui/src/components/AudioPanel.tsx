import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow, PhysicalPosition, primaryMonitor } from "@tauri-apps/api/window";
import { useThemeInit } from "../lib/theme";
import {
  audioMediaControl,
  getState,
  inTauri,
  onAudioWave,
  onMediaState,
  onStateUpdate,
  setAudioPanelPosition,
} from "../lib/bridge";

interface MediaState {
  active: boolean;
  playing: boolean;
  appName: string;
  title: string;
  artist: string;
  positionSecs: number;
  durationSecs: number;
  prevEnabled: boolean;
  nextEnabled: boolean;
  playEnabled: boolean;
  pauseEnabled: boolean;
  supported: boolean;
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
  const waveRef = useRef<number[]>([]);

  // 初始化：应用持久化位置或计算右下角默认位置；按开关决定是否显示
  useEffect(() => {
    if (!inTauri()) return;
    const win = getCurrentWindow();
    void getState().then(async (s) => {
      if (!s.audioPanelEnabled) return; // 开关关闭：保持隐藏
      if (s.audioPanelX >= 0 && s.audioPanelY >= 0) {
        await win.setPosition(new PhysicalPosition(s.audioPanelX, s.audioPanelY));
      } else {
        // 右下角（主显示器工作区右下角 - 面板尺寸 - 16px 边距）
        try {
          const primary = await primaryMonitor();
          if (primary) {
            const size = await win.outerSize();
            const x = primary.workArea.position.x + primary.workArea.size.width - size.width - 16;
            const y = primary.workArea.position.y + primary.workArea.size.height - size.height - 16;
            await win.setPosition(new PhysicalPosition(Math.max(0, x), Math.max(0, y)));
          }
        } catch {
          /* 忽略定位失败（默认窗口位置） */
        }
      }
      await win.show();
      setPositioned(true);
    });
  }, []);

  // 拖拽结束后持久化位置
  useEffect(() => {
    if (!inTauri()) return;
    const win = getCurrentWindow();
    const unlisten = win.onMoved(({ payload }) => {
      void setAudioPanelPosition(payload.x, payload.y);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  // 订阅 SMTC 状态 / 波形 / 全屏状态
  useEffect(() => {
    const offMedia = onMediaState((s) => setMedia(s));
    const offWave = onAudioWave((w) => {
      waveRef.current = w;
      setWave(w);
    });
    const offState = onStateUpdate((s) => setFullscreen(s.fullscreenActive));
    return () => {
      offMedia();
      offWave();
      offState();
    };
  }, []);

  // 面板可见性：无播放 / 不支持 SMTC / 全屏时淡出隐藏
  const visible =
    positioned &&
    media !== null &&
    media.supported &&
    media.active &&
    !fullscreen;

  const show = visible;

  const control = useCallback((action: "prev" | "play" | "pause" | "next") => {
    audioMediaControl(action);
  }, []);

  const progress =
    media && media.durationSecs > 0
      ? Math.min(100, (media.positionSecs / media.durationSecs) * 100)
      : 0;

  return (
    <div className={`audio-panel${show ? " visible" : ""}`}>
      <div className="audio-panel-drag" data-tauri-drag-region />
      <div className="audio-panel-body">
        <div className="audio-panel-info">
          <div className="audio-panel-title" title={media?.title || ""}>
            {media?.title || "未在播放"}
          </div>
          <div className="audio-panel-sub">
            {media?.appName || (media?.active ? "媒体会话" : "")}
            {media?.artist ? ` · ${media.artist}` : ""}
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
      <div className="audio-panel-progress">
        <div className="audio-panel-progress-bar" style={{ width: `${progress}%` }} />
      </div>
      <div className="audio-panel-wave">
        {wave.map((v, i) => (
          <span
            key={i}
            className="audio-wave-bar"
            style={{ height: `${Math.max(8, Math.round(v * 100))}%` }}
          />
        ))}
      </div>
    </div>
  );
}
