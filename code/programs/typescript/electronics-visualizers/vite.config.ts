import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  base: "/coding-adventures/electronics-visualizers/",
  resolve: {
    dedupe: ["react", "react-dom"],
  },
});
