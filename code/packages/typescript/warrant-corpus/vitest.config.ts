import { defineConfig } from "vitest/config";

// The whole-corpus test enumerates 572,109 cases and is several times slower
// under coverage tracing, so the timeout is generous and set per test there.
export default defineConfig({
  test: {
    include: ["tests/**/*.test.ts"],
    testTimeout: 60_000,
    coverage: {
      provider: "v8",
      include: ["src/**/*.ts"],
      exclude: ["src/bin.ts"],
      thresholds: { lines: 95, statements: 95, functions: 95, branches: 90 },
    },
  },
});
