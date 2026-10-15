---
category: Rust
---

# Space a negative assignment so strict Clippy can distinguish it from subtraction

Railroad labels source2bbd129c passed focused layout16 and paint13 tests, but full affected14-package strict lint stopped on the new test-only assignment x=-1.0 (suspicious_assignment_formatting). Use x= -1.0 with identical tokens and value. Preserve the failed receipt; no full-suite test acceptance is inferred because strict lint ran first. Rerun strict checks and full suites on the corrected exact source without suppressing warnings or changing assertions.
