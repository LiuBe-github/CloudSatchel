import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { AiMessage } from "../vite-env";
import {
  aiSend,
  aiStop,
  getAiConfig,
  inTauri,
  onAiChunk,
  onAiDone,
  onAiError,
} from "../lib/bridge";
import { useThemeInit } from "../lib/theme";
import { Markdown } from "../lib/markdown";

/**
 * AI 小窗（FR-17）：全局快捷键（默认 Ctrl+Shift+Space）呼出的小型 AI 问答窗口。
 * - 配置复用 FR-15：API Key / 模型 / 接口地址，无需重复配置（未配置时提示去主界面）
 * - 上下文：小窗打开期间保留（多轮连续），关闭（隐藏）即清空，不落盘
 * - 关闭按钮 / Esc = 隐藏窗口（对话清空由隐藏事件触发）
 */
export default function AiPopup() {
  useThemeInit();
  const [messages, setMessages] = useState<AiMessage[]>([]);
  const [input, setInput] = useState("");
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState("");
  const [hasKey, setHasKey] = useState(false);
  const [model, setModel] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const messagesRef = useRef<HTMLDivElement>(null);

  // 新消息 / 流式增量时自动滚到底部
  useEffect(() => {
    const el = messagesRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [messages, generating]);

  // 初始读取 AI 配置（Key 是否存在、模型、接口地址）
  useEffect(() => {
    void getAiConfig().then((cfg) => {
      setHasKey(cfg.hasKey);
      setModel(cfg.model);
      setBaseUrl(cfg.baseUrl);
    });
  }, []);

  // 订阅 AI 流式事件
  useEffect(() => {
    const offChunk = onAiChunk((text) => {
      setMessages((prev) => {
        const last = prev[prev.length - 1];
        if (last && last.role === "assistant") {
          return [...prev.slice(0, -1), { role: "assistant", content: last.content + text }];
        }
        return [...prev, { role: "assistant", content: text }];
      });
    });
    const offDone = onAiDone(() => setGenerating(false));
    const offError = onAiError((msg) => {
      setGenerating(false);
      setError(msg);
    });
    return () => {
      offChunk();
      offDone();
      offError();
    };
  }, []);

  // 窗口隐藏 → 清空对话上下文（不落盘）：
  // 热键 / 关开关由后端隐藏时发 ai-popup-cleared 事件；本窗口自己的关闭按钮 / Esc 直接清空
  useEffect(() => {
    if (!inTauri()) return;
    const win = getCurrentWindow();
    const unlisten = win.listen("ai-popup-cleared", () => {
      setMessages([]);
      setError("");
      setGenerating(false);
      setInput("");
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  // 小窗打开时聚焦输入框（由后端 show 后的事件触发）
  useEffect(() => {
    if (!inTauri()) return;
    const win = getCurrentWindow();
    const unlisten = win.listen("ai-popup-shown", () => {
      window.setTimeout(() => inputRef.current?.focus(), 80);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  // Esc 隐藏小窗（隐藏即清空对话）
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && inTauri()) {
        clearAll();
        void getCurrentWindow().hide();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const clearAll = () => {
    setMessages([]);
    setError("");
    setGenerating(false);
    setInput("");
  };

  const hide = useCallback(() => {
    if (inTauri()) {
      clearAll();
      void getCurrentWindow().hide();
    }
  }, []);

  const send = async () => {
    const content = input.trim();
    if (!content || generating || !hasKey) return;
    setError("");
    const next: AiMessage[] = [...messages, { role: "user", content }];
    const history = next.slice(-20);
    setMessages(next);
    setInput("");
    setGenerating(true);
    try {
      await aiSend(baseUrl, model, history);
    } catch (err) {
      setGenerating(false);
      setError(String(err));
    }
  };

  const stop = () => {
    aiStop();
    setGenerating(false);
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void send();
    }
  };

  return (
    <div className="ai-popup-shell">
      {/* 小窗标题栏（可拖拽） */}
      <header className="ai-popup-titlebar" data-tauri-drag-region>
        <span className="ai-popup-title" data-tauri-drag-region>
          ✳ AI 小窗
        </span>
        <button className="icon-btn" onClick={hide} aria-label="隐藏小窗" title="隐藏（Esc）">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      </header>

      {/* 消息区 */}
      <div className="ai-popup-messages" ref={messagesRef}>
        {messages.length === 0 && (
          <div className="ai-popup-welcome">
            <div className="ai-popup-welcome-title">快速提问</div>
            <div className="ai-popup-welcome-desc">
              {hasKey
                ? `配置：${model || "默认模型"} · 关闭小窗即清空对话`
                : "尚未配置 API Key，请先在主界面「AI 助手」中配置后使用"}
            </div>
          </div>
        )}
        {messages.map((m, i) => (
          <div key={i} className={`ai-msg ${m.role === "user" ? "user" : "assistant"}`}>
            <div className="ai-msg-bubble">
              {m.role === "assistant" ? <Markdown content={m.content} /> : m.content}
            </div>
          </div>
        ))}
        {generating && <div className="ai-thinking">正在思考…</div>}
        {error && <div className="ai-popup-error">{error}</div>}
      </div>

      {/* 输入区 */}
      <div className="ai-popup-input-bar">
        <textarea
          ref={inputRef}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder="输入消息，Enter 发送"
          rows={2}
          disabled={!hasKey}
        />
        <div className="ai-popup-input-actions">
          {generating ? (
            <button className="seg-btn danger" onClick={stop}>
              停止
            </button>
          ) : (
            <button
              className="seg-btn primary"
              onClick={() => void send()}
              disabled={!input.trim() || !hasKey}
            >
              发送
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
