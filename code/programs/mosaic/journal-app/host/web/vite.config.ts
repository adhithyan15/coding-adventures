import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  root: ".",
  // Relative asset URLs, so the bundle works from any directory: GitHub Pages
  // serves it from /coding-adventures/journal/, not from the root (J5d; the
  // same fix #13832 made for Trestle).
  base: "./",
  // The loader lives in the Rust crate (mosaic-app-wasm/js), outside this
  // project; allow the dev server to serve it.
  server: { port: 5175, fs: { allow: ["../../../../../.."] } },
});
