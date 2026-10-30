import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath, URL } from "node:url";

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  // Tauri 需要固定端口 + 不清屏（否则 Rust 侧连不上）
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
    headers: {
      "Access-Control-Allow-Origin": "*",
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    target: "chrome105",
    // Vite 8 用 rolldown/oxc，esbuild 不再内置；用默认压缩器
    minify: true,
    sourcemap: false,
  },
});
