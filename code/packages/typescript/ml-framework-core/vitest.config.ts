import { defineConfig } from "vitest/config";

// Vitest config for ml-framework-core.
//
// v0.1.0 (Tensor) is pure TypeScript — no native addons, no async work,
// nothing that benefits from cross-file parallelism but also nothing that
// breaks under it.  Keep the defaults for simplicity; future PRs that
// dispatch into the Rust matrix-rust-napi addon will need fileParallelism
// disabled (the .node file loads once globally) but PR #1 doesn't.
export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      thresholds: { lines: 80, functions: 80, branches: 80, statements: 80 },
    },
  },
});
