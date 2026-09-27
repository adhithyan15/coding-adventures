import { defineConfig } from "vite";
import path from "node:path";

export default defineConfig({
  root: path.resolve(import.meta.dirname, "webcomponent"),
  base: "./",
  publicDir: "public",
  build: {
    outDir: path.resolve(import.meta.dirname, "dist-webcomponent"),
    emptyOutDir: true,
    rollupOptions: {
      input: {
        index: path.resolve(import.meta.dirname, "webcomponent/index.html"),
        light: path.resolve(import.meta.dirname, "webcomponent/light.html"),
        dark: path.resolve(import.meta.dirname, "webcomponent/dark.html"),
      },
    },
  },
});
