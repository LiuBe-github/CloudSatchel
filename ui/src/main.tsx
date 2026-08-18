import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import AiPopup from "./components/AiPopup";
import AudioPanel from "./components/AudioPanel";
import { inTauri } from "./lib/bridge";
import "./styles.css";

// 多窗口路由：主窗口渲染完整应用；辅助窗口渲染各自小组件
const label = inTauri() ? getCurrentWindow().label : "main";

createRoot(document.getElementById("root")!).render(
  label === "ai-popup" ? (
    <AiPopup />
  ) : label === "audio-panel" ? (
    <AudioPanel />
  ) : (
    <App />
  ),
);
