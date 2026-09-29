import { useEffect, useRef, useState } from "react";
import { Icon } from "./Icon";
import { dismissPrivacyUnlock, unlockPrivacy, unlockPrivacyWithHello } from "../lib/bridge";
import { useThemeInit } from "../lib/theme";

/**
 * 隐私解锁卡片（privacy-unlock 窗口，v1.3.0）：
 * 隐私保护触发后，鼠标/键盘操作会弹出本卡片；输入正确密码才还原全部状态。
 * 输错提示重试；Esc 暂时收起（保护不解除，下次键鼠操作再次弹出）。
 */
export default function PrivacyUnlock() {
  useThemeInit();
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [helloBusy, setHelloBusy] = useState(false);
  const [shake, setShake] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        void dismissPrivacyUnlock();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const submit = async () => {
    if (!password || busy || helloBusy) return;
    setBusy(true);
    setError("");
    try {
      // 验证通过：后端隐藏本窗口并还原全部状态
      await unlockPrivacy(password);
    } catch (err) {
      setError(typeof err === "string" ? err : "密码错误，请重试");
      setPassword("");
      setShake(true);
      window.setTimeout(() => setShake(false), 450);
      inputRef.current?.focus();
    } finally {
      setBusy(false);
    }
  };

  // 忘记密码：弹 Windows Hello（开机 PIN）系统验证，通过后后端清除旧密码并还原
  const forgotPassword = async () => {
    if (busy || helloBusy) return;
    setHelloBusy(true);
    setError("");
    try {
      await unlockPrivacyWithHello();
    } catch (err) {
      setError(typeof err === "string" ? err : "系统验证未通过，请重试");
      setShake(true);
      window.setTimeout(() => setShake(false), 450);
      inputRef.current?.focus();
    } finally {
      setHelloBusy(false);
    }
  };

  return (
    <div className={`privacy-unlock${shake ? " shake" : ""}`}>
      <div className="privacy-unlock-icon"><Icon name="shield" size={30} /></div>
      <div className="privacy-unlock-title">隐私保护已启用</div>
      <input
        ref={inputRef}
        type="password"
        className="privacy-unlock-input"
        placeholder="输入密码解锁"
        value={password}
        onChange={(e) => {
          setPassword(e.target.value);
          setError("");
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            void submit();
          }
        }}
        spellCheck={false}
        autoComplete="off"
      />
      {error && <div className="privacy-unlock-error">{error}</div>}
      <button
        type="button"
        className="seg-btn primary privacy-unlock-btn"
        onClick={() => void submit()}
        disabled={busy || helloBusy || !password}
      >
        {helloBusy ? "等待系统验证…" : busy ? "验证中…" : "解锁"}
      </button>
      <button
        type="button"
        className="privacy-unlock-forgot"
        onClick={() => void forgotPassword()}
        disabled={busy || helloBusy}
      >
        忘记密码？用开机 PIN 解锁
      </button>
    </div>
  );
}
