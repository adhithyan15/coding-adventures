import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  // Default "/" works for local dev.
  // Set VITE_BASE=/coding-adventures/engram-docs/ for the GitHub Pages build
  // (the deploy workflow sets this automatically).
  base: process.env.VITE_BASE ?? "/",
});
