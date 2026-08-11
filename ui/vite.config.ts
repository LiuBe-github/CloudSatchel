import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Tauri 构建配置（参考花笺）：dev 端口 1420，产物在 dist/
export default defineConfig({
  plugins: [react()],
  base: "./",
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    assetsDir: "assets",
    target: "chrome110",
    cssCodeSplit: false,
  },
});
