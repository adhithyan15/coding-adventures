import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    globals: true,
    // Vitest's 5 s / 10 s defaults assume the suite owns the machine. On
    // main's sharded full build ~970 packages test on one runner, and a
    // stalled worker failed trivial synchronous tests here after 5-18 s of
    // wall clock (ci.yml run 38509620113). 30 s keeps an order of magnitude
    // of headroom; nothing in this suite does network I/O that a bigger
    // budget could hide. See
    // lessons.d/vitest-s-default-timeouts-are-too-tight-for-a-suite-that-shares.md
    testTimeout: 30000,
    hookTimeout: 30000,
  },
});
