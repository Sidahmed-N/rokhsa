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
  server: {
    port: 5173,
    strictPort: true,
  },
  // Tauri will serve the built assets from a file:// style context later,
  // so keep asset paths relative rather than root-absolute.
  base: "./",
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});
