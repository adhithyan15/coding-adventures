# Lesson 92 — CI runners are ~25× slower for LZSS/compute-heavy tests; always set an explicit timeout

**Date:** 2026-04-26

**What happened:** The TypeScript ZStd TC-8 regression test (200 KB repetitive text → ≥ 128 sequences) ran in ~450 ms locally but took 12–15 seconds on CI runners. Vitest's default per-test timeout is 5 seconds. The CI job failed with a timeout error even though the test was functionally correct and passing locally.

**Rule:** Any test that triggers an LZSS/LZ77 pass over more than 50 KB should have an explicit timeout set to at least `30_000` ms (30 s) in vitest:

```ts
it("round-trips 200 KB ...", () => { ... }, 30_000);
```

CI runners (especially GitHub Actions free-tier) run at roughly 25× slower wall-clock for CPU-intensive loops. A test that takes < 1 s locally may take 25 s on CI. Default framework timeouts (5 s for vitest, 60 s for Go's `go test`) are often too tight for large compression round-trips. Always measure on CI before assuming the default is safe.

---
