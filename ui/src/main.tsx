import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import AiPopup from "./components/AiPopup";
import { inTauri } from "./lib/bridge";
import "./styles.css";

// 多窗口路由：主窗口渲染完整应用；ai-popup 窗口渲染 AI 小窗（FR-17）
const label = inTauri() ? getCurrentWindow().label : "main";

createRoot(document.getElementById("root")!).render(
  label === "ai-popup" ? <AiPopup /> : <App />,
);
