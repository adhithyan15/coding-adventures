import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  root: ".",
  // The loader lives in the Rust crate (mosaic-app-wasm/js), outside this
  // project; allow the dev server to serve it.
  server: { port: 5175, fs: { allow: ["../../../../../.."] } },
});
