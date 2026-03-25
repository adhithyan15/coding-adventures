import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  // Base path is configurable via VITE_BASE environment variable.
  //
  // Default "./" uses relative paths — works everywhere:
  //   - `npm run dev`  → Vite dev server (http://localhost:5173)
  //   - Electron       → file:// protocol, no web server needed
  //
  // Set VITE_BASE=/coding-adventures/engram/ for the GitHub Pages build
  // (the deploy workflow sets this automatically).
  base: process.env.VITE_BASE ?? "./",
});
