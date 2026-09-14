---
category: TypeScript / JavaScript
---

# CI is ~25× slower than local for compute-heavy tests

Vitest's 5s default times out on 200KB+ LZSS round-trips. Set explicit `30_000` ms timeout for tests that exercise large compression/LZ77 passes.
