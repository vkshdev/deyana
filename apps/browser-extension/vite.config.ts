import { resolve } from "node:path";
import { defineConfig } from "vite";

export default defineConfig({
  publicDir: "public",
  build: {
    outDir: "dist",
    emptyOutDir: true,
    rollupOptions: {
      input: {
        "service-worker": resolve(__dirname, "src/background/serviceWorker.ts"),
        "content-bridge": resolve(__dirname, "src/content/contentBridge.ts"),
        popup: resolve(__dirname, "popup.html")
      },
      output: {
        entryFileNames: "[name].js",
        chunkFileNames: "chunks/[name]-[hash].js",
        assetFileNames: "assets/[name]-[hash][extname]"
      }
    }
  }
});
