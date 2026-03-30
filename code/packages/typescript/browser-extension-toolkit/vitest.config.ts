import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      exclude: [
        "src/index.ts",
        "src/scaffold/cli.ts",
      ],
      thresholds: {
        lines: 80,
      },
    },
  },
});
