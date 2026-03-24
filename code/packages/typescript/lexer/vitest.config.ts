import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      include: [
        "src/**/*.ts",
      ],
      exclude: [
        "src/index.ts",
        "src/token.ts",
      ],
      provider: "v8",
      thresholds: {
        lines: 80,
      },
    },
  },
});
