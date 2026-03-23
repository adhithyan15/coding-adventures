import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  // "./" makes assets load via relative paths, which works in both:
  //   - Electron (file:// protocol, no web server)
  //   - GitHub Pages (deployed under a subdirectory)
  base: "./",
});
