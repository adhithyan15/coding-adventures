import { configDefaults, defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    exclude: ["dist/**", ...configDefaults.exclude],
    coverage: {
      provider: "v8",
      thresholds: {
        lines: 80,
      },
    },
  },
});
