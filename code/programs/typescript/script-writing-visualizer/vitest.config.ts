import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    globals: true,
    coverage: {
      provider: "v8",
      // The pure logic (core.ts + drill.ts) is what we hold to a high bar;
      // main.ts is the thin DOM shell and data.ts is just JSON imports.
      include: ["src/core.ts", "src/drill.ts", "src/scheduler.ts", "src/interleave.ts"],
    },
  },
});
