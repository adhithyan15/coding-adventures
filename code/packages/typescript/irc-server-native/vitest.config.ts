import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // Run serially in one fork so real TCP servers don't fight over ports.
    pool: "forks",
    singleFork: true,
    environment: "node",
    include: ["tests/**/*.test.ts"],
    testTimeout: 15000,
    hookTimeout: 10000,
  },
});
