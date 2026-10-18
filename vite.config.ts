import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(import.meta.dirname, "src"),
    },
  },
  // Tauri drives this dev server, so don't wipe its output, and don't let the
  // Rust build directory trigger frontend reloads.
  clearScreen: false,
  envPrefix: ["VITE_", "TAURI_ENV_"],
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  // Tauri will serve the built assets from a file:// style context later,
  // so keep asset paths relative rather than root-absolute.
  base: "./",
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});
