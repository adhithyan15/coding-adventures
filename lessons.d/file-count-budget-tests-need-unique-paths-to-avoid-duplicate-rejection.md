---
category: Testing & coverage
---

# File-count budget tests need unique paths to avoid duplicate rejection

The discovery materializer's first count-limit regression used 65 copies of
one fixture path. Duplicate-path validation rejected that input even if the
64-file cap was removed, so the assertion could not prove the count boundary.
The read-only test review caught this before the tests were committed.
Use 65 unique safe paths for rejection and 64 unique safe paths as an accepted
control. Keep the separate duplicate-path regression: each refusal test must
leave the earlier validation rules satisfied.
