import { defineConfig } from "vitest/config";

export default defineConfig({
  test: { coverage: {
    provider: "v8", include: ["src/**/*.ts"],
    thresholds: { statements: 95, lines: 95, functions: 95, branches: 85 },
  } },
});
