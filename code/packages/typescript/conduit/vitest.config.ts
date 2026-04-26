/**
 * vitest.config.ts — Vitest configuration for coding-adventures-conduit.
 *
 * We run all tests in a single "node" environment so the built-in Node.js
 * `fetch` and the native `.node` addon are both available.
 *
 * The TypeScript source is compiled on-the-fly by Vitest's built-in esbuild
 * pipeline — no separate `tsc` step needed for running tests.
 */

import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // Run tests serially (not in parallel worker threads) so that the
    // E2E test suites that start real TCP servers don't fight over ports.
    // Each describe block starts its own server on port 0 (OS-assigned).
    pool: "forks",
    poolOptions: {
      forks: {
        singleFork: true,
      },
    },
    environment: "node",
    include: ["tests/**/*.test.ts"],
    // Increase timeout for E2E tests that start real TCP servers.
    testTimeout: 15000,
    hookTimeout: 10000,
  },
});
