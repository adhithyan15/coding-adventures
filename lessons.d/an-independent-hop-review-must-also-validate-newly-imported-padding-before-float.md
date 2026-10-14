---
category: Rust
---

# An independent hop review must also validate newly imported padding before floating clamp

The full diagram suite passed843 tests, but independent source review found a
new imported caller-owned padding field missing from the existing fallible
layout admission list. NaN can become a floating clamp bound and panic before
the later derived-geometry check. Add direct red controls before repairing the
finite inclusive spacing guard; retain producer behavior for accepted values.
Passing existing suites does not establish safety for a newly added IR field.

Strict Clippy also rejected the new legacy-oracle tuple annotation for type
complexity. Use a named test-only group type; do not suppress the lint or change
the exact directed occurrence/order assertions. Keep failed receipts distinct
from successful native execution, then rerun affected suites and strict lint.
