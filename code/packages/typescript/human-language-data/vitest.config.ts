import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      include: ["src/**/*.ts"],
      // The direct-invoke guard in cli.ts only runs as a standalone process.
      exclude: ["src/index.ts"],
      thresholds: {
        lines: 85,
      },
    },
  },
});
