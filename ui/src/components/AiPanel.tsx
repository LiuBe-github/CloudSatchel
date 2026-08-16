import { useEffect, useState } from "react";
import type { AiMessage } from "../vite-env";
import {
  aiSend,
  aiStop,
  getAiConfig,
  onAiChunk,
  onAiDone,
  onAiError,
  saveAiKey,
  setAiModel,
} from "../lib/bridge";

interface AiPanelProps {
  /** 当前模型名（来自 AppState，持久化） */
  model: string;
  /** 保存模型名后回调更新全局状态 */
  onModelChange: (model: string) => void;
}

/** AI 助手对话页面（FR-15）：配置区 + 消息区 + 输入区 */
export function AiPanel({ model, onModelChange }: AiPanelProps) {
  const [hasKey, setHasKey] = useState(false);
  const [configOpen, setConfigOpen] = useState(false);
  const [apiKey, setApiKey] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [modelInput, setModelInput] = useState(model);
  const [messages, setMessages] = useState<AiMessage[]>([]);
  const [input, setInput] = useState("");
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);

  // 初始加载配置状态并订阅流式事件
  useEffect(() => {
    void getAiConfig().then((cfg) => {
      setHasKey(cfg.hasKey);
      setModelInput(cfg.model);
    });
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

  const send = async () => {
    const content = input.trim();
    if (!content || generating || !hasKey) return;
    setError("");
    const next: AiMessage[] = [...messages, { role: "user", content }];
    // 历史过长时仅保留最近 20 条，避免超出模型上下文窗口
    const history = next.slice(-20);
    setMessages(next);
    setInput("");
    setGenerating(true);
    try {
      await aiSend(modelInput.trim() || "gpt-4o-mini", history);
    } catch (err) {
      setGenerating(false);
      setError(String(err));
    }
  };

  const stop = () => {
    aiStop();
    setGenerating(false);
  };

  const clear = () => {
    setMessages([]);
    setError("");
  };

  const saveConfig = async () => {
    try {
      if (apiKey.trim()) {
        await saveAiKey(apiKey);
        setHasKey(true);
        setApiKey("");
      }
      if (modelInput.trim()) {
        const next = await setAiModel(modelInput.trim());
        onModelChange(next.aiModel);
      }
      setSaved(true);
      window.setTimeout(() => setSaved(false), 2000);
      setConfigOpen(false);
      setError("");
    } catch (err) {
      setError(String(err));
    }
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void send();
    }
  };

  return (
    <div className="ai-panel animate-scale-in">
      {/* 配置区 */}
      <div className="ai-config-bar">
        {hasKey ? (
          <span className="ai-config-status">✓ 已配置 API Key{modelInput ? ` · ${modelInput}` : ""}</span>
        ) : (
          <span className="ai-config-status warn">未配置 API Key，请先配置后使用</span>
        )}
        <button className="seg-btn" onClick={() => setConfigOpen((v) => !v)}>
          {configOpen ? "收起配置" : "配置"}
        </button>
      </div>

      {configOpen && (
        <div className="ai-config-box">
          <div className="ai-config-row">
            <span className="ai-config-label">API Key</span>
            <input
              type={showKey ? "text" : "password"}
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={hasKey ? "已加密保存（留空保持不变）" : "sk-…"}
              spellCheck={false}
            />
            <button className="seg-btn" onClick={() => setShowKey((v) => !v)}>
              {showKey ? "隐藏" : "显示"}
            </button>
          </div>
          <div className="ai-config-row">
            <span className="ai-config-label">模型名</span>
            <input
              value={modelInput}
              onChange={(e) => setModelInput(e.target.value)}
              placeholder="gpt-4o-mini"
              spellCheck={false}
            />
            <button className="seg-btn primary" onClick={() => void saveConfig()}>
              保存
            </button>
          </div>
          <div className="ai-config-note">
            Key 经 Windows DPAPI 加密后保存在本机，仅请求 api.openai.com；对话内容不落盘
            {saved && <span className="ai-config-saved"> · 已保存 ✓</span>}
          </div>
        </div>
      )}

      {/* 错误提示条 */}
      {error && (
        <div className="ai-error-bar">
          <span>{error}</span>
          <button className="ai-error-close" onClick={() => setError("")} aria-label="关闭">
            ×
          </button>
        </div>
      )}

      {/* 消息区 */}
      <div className="ai-messages">
        {messages.length === 0 && (
          <div className="ai-welcome">
            <div className="ai-welcome-title">AI 助手</div>
            <div className="ai-welcome-desc">
              配置你自己的 OpenAI API Key 后即可开始对话；支持流式回复与多轮上下文，可随时停止生成或清空对话
            </div>
          </div>
        )}
        {messages.map((m, i) => (
          <div key={i} className={`ai-msg ${m.role === "user" ? "user" : "assistant"}`}>
            <div className="ai-msg-bubble">{m.content}</div>
          </div>
        ))}
        {generating && <div className="ai-thinking">正在思考…</div>}
      </div>

      {/* 输入区 */}
      <div className="ai-input-bar">
        <textarea
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder="输入消息，Enter 发送，Shift+Enter 换行"
          rows={2}
        />
        <div className="ai-input-actions">
          <button className="seg-btn" onClick={clear} disabled={messages.length === 0}>
            清空对话
          </button>
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
