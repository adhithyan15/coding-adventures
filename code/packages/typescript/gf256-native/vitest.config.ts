// vitest.config.ts -- Test configuration with v8 coverage
// ========================================================
//
// We use v8 coverage because the code under test is a native addon --
// the JavaScript surface is thin (just the index.js loader), but we
// want to ensure all exported symbols are exercised and the JS glue
// is covered.

import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      include: ["index.js"],
    },
  },
});
