import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// "./" makes assets load via relative paths, so the built app works both from a
// web server and when opened under a subdirectory (e.g. GitHub Pages).
export default defineConfig({
  plugins: [react()],
  base: "./",
});
