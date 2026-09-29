import { forwardRef, useImperativeHandle, useRef } from "react";

export interface ToastHandle {
  show: (message: string, tone?: ToastTone) => void;
}

export type ToastTone = "success" | "warning" | "error";

const TONE_ICON: Record<ToastTone, string> = {
  success: "✓",
  warning: "!",
  error: "×",
};

export const Toast = forwardRef<ToastHandle>(function Toast(_, ref) {
  const timer = useRef<number | undefined>(undefined);
  const el = useRef<HTMLDivElement>(null);

  useImperativeHandle(ref, () => ({
    show(message: string, tone?: ToastTone) {
      const node = el.current;
      if (!node) return;
      const resolvedTone = tone ?? (message.includes("失败") ? "error" : message.startsWith("请先") ? "warning" : "success");
      const textNode = node.querySelector(".toast-text");
      const iconNode = node.querySelector(".toast-icon");
      if (textNode) textNode.textContent = message;
      if (iconNode) iconNode.textContent = TONE_ICON[resolvedTone];
      node.dataset.tone = resolvedTone;
      node.classList.remove("toast-exit");
      node.classList.add("toast-entry");
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => {
        node.classList.remove("toast-entry");
        node.classList.add("toast-exit");
      }, 2200);
    },
  }));

  return (
    <div className="toast" ref={el} role="status" aria-live="polite" aria-atomic="true">
      <span className="toast-icon" aria-hidden="true">✓</span>
      <span className="toast-text" />
    </div>
  );
});
