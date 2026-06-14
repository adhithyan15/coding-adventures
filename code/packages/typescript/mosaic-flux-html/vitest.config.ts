import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // The DOM helpers exercise real DOM operations; jsdom gives us
    // those without a browser. The core types (store, selector,
    // middleware, devtools) work in any environment but they're
    // tested in this same jsdom context anyway.
    environment: "jsdom",
    coverage: {
      provider: "v8",
      include: ["src/**/*.ts"],
      thresholds: {
        lines: 80,
        functions: 80,
        branches: 80,
        statements: 80,
      },
    },
  },
});
