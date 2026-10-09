import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue()],
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
    // 不要监听 src-tauri：cargo 重建时会锁住 target 下的 dll，
    // vite 的文件监听会因此抛 EBUSY 直接崩掉（开发服务器一挂，窗口就变成空白/无法访问）
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },

  // Vite options tailored for Tauri development
  clearScreen: false,
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2021",
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
