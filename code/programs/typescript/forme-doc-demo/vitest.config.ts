import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // 60s for hooks, not vitest's 10s default. Two `beforeAll` hooks do the
    // expensive work for their whole suite: `writeBundle` in build.test.ts
    // builds and writes the demo site, and the JSDOM suite in
    // search-bundle.test.ts also esbuild-bundles the search client before
    // building the site. Together the two files take ~5 s on an idle laptop.
    // On a 3-core `macos-latest` runner building ~490 packages at once (#17972)
    // both hooks overran 10 s and every test behind them failed unrun. The
    // work is real and fixed-size (one build of the checked-in corpus), so a
    // stated budget is the fix. 60 s still fails a hang quickly.
    hookTimeout: 60_000,
    coverage: {
      provider: "v8",
      reporter: ["text", "lcov"],
      include: ["src/**/*.ts"],
    },
  },
});
