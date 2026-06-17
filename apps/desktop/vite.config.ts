import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const cacheDir = process.env.DEYANA_VITE_CACHE_DIR ?? "../../tmp/vite-cache/desktop";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  cacheDir,
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true
  },
  envPrefix: ["VITE_", "TAURI_"]
});
