import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { resolve } from "path";

// @tauri-apps/cli sets TAURI_DEV_HOST when using a physical device / remote dev
const host = process.env.TAURI_DEV_HOST;

// Two entry points: the docked strip and the detail popover window.
export default defineConfig({
  plugins: [svelte()],
  // Tauri expects a fixed port and manages its own console output.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    rollupOptions: {
      input: {
        strip: resolve(__dirname, "index.html"),
        detail: resolve(__dirname, "detail.html"),
      },
    },
  },
});
