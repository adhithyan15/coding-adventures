---
category: Testing & coverage
---

# A dependency-only security boundary change still must cover pre-existing public error paths to meet the library coverage gate

The HMAC dependency-boundary change added tests for the comparison behavior it
touched, and every functional gate passed, but a scoped Tarpaulin run still
reported only 93.65% line coverage. The uncovered executable lines were the
pre-existing public `HmacError::Display` implementation; a declaration line in
the already-exercised long-key branch was also reported as uncovered by LLVM's
line mapping.

Do not lower or omit the coverage gate because a patch is dependency-only.
Run scoped source coverage before review, inspect every uncovered executable
line, and add a focused assertion for stable public behavior when that behavior
is part of the package contract. Here, an exact error-display assertion raised
the package above the 95% library target while also proving that the failure
remains payload-free.
