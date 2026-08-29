import { inTauri, translateOpen } from "../lib/bridge";
import { useThemeInit } from "../lib/theme";

/**
 * 翻译小按钮（translate-button 窗口）：选中文字松手后出现在选区下方，
 * 点击打开翻译弹窗；窗口 focusable:false（点击不抢焦点，保持类飞书体验）。
 */
export default function TranslateButton() {
  useThemeInit();
  if (!inTauri()) return null;
  return (
    <button
      type="button"
      className="translate-button"
      onClick={() => translateOpen()}
      title="翻译选中文本"
    >
      翻译
    </button>
  );
}
