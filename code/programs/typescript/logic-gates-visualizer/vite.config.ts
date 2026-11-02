import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  base: "/coding-adventures/logic-gates/",
  resolve: {
    preserveSymlinks: true,
    dedupe: ["react", "react-dom"],
  },
});
