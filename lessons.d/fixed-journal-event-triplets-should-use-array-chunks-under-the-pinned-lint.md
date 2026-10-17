---
category: Testing & coverage
---

# Fixed journal event triplets should use array chunks under the pinned lint toolchain

CLOC39's focused tests passed twelve new native/resource functions and five old
array-length functions, but the pinned Rust 1.99 strict check rejected the new
test's chunks_exact(3) with chunks_exact_to_as_chunks. The focused receipt stays
unaccepted; successful test execution does not imply strict lint acceptance.

Use as_chunks::<3>(), assert the remainder is empty, then iterate the fixed
arrays while retaining the existing exact journal length and event assertions.
Keep production code and prior tests unchanged, add no lint allowance, and run
the affected full suites and strict checks on the actual successor head.
