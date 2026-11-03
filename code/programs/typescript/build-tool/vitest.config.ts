import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // Several suites start real processes: git (tests/gitdiff.test.ts builds
    // a repository with about ten git calls) and the CLI itself through
    // node + tsx (tests/resolver.test.ts gives that child a 30 s budget).
    // On a hosted Windows runner each process start costs hundreds of
    // milliseconds, so those tests ran 5.2 s and failed vitest's 5 s default
    // with no assertion failing. The per-test limit matches the 30 s budget
    // the CLI tests already grant their child process; a test that really
    // hangs still fails, just after 30 s instead of 5 s.
    testTimeout: 30_000,
    coverage: {
      provider: "v8",
      include: ["src/**/*.ts"],
      exclude: ["src/index.ts"],
      thresholds: {
        statements: 80,
        branches: 80,
        functions: 80,
        lines: 80,
      },
    },
  },
});
