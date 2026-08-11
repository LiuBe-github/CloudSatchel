import { forwardRef, useImperativeHandle, useRef } from "react";

export interface ToastHandle {
  show: (message: string) => void;
}

export const Toast = forwardRef<ToastHandle>(function Toast(_, ref) {
  const timer = useRef<number | undefined>(undefined);
  const el = useRef<HTMLDivElement>(null);

  useImperativeHandle(ref, () => ({
    show(message: string) {
      const node = el.current;
      if (!node) return;
      const textNode = node.querySelector(".toast-text");
      if (textNode) textNode.textContent = message;
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
    <div className="toast" ref={el} role="status" aria-live="polite">
      <span className="toast-check">✓</span>
      <span className="toast-text" />
    </div>
  );
});
