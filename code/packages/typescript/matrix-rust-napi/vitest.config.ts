import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // Smoke tests load the .node addon, which is a real OS-level
    // dynamic library — disable parallelism to keep the test logs
    // deterministic and avoid double-load surprises on slower CI
    // runners.
    fileParallelism: false,
    coverage: {
      provider: "v8",
      // The TS wrapper is intentionally thin (one re-export module);
      // most lines are types.  Lower the threshold so a 30-line file
      // doesn't trigger false coverage warnings.
      thresholds: { lines: 70 },
    },
  },
});
