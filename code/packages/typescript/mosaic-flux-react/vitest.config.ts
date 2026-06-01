import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    coverage: {
      provider: "v8",
      include: ["src/**/*.ts"],
      // react.ts requires a jsdom + react-testing-library environment
      // to exercise meaningfully. v0.1.0 ships it untested at the
      // unit level and validates via integration in downstream
      // Mosaic-compiled apps. v0.2.0 will add the test setup.
      exclude: ["src/react.ts"],
      thresholds: {
        lines: 80,
        functions: 80,
        branches: 80,
        statements: 80,
      },
    },
  },
});
