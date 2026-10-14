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

The next review found that finite accepted padding can also make an ordinary
node taller than the fixed horizontal lane. Check exact node/lane/canvas
enclosure, not just finite dimensions, and size the cross extent from the
resolved node height while preserving default geometry.

A root-relative git add was accidentally issued from the Cargo package
directory before the red run. The add/commit failed, while Cargo genuinely
executed the working-tree tests. Commit from the repository root and record
that exact source blob and pre-commit execution honestly; do not attribute the
run to an earlier committed source that lacks those tests.

The first enclosure control did not compile because a laid-out node has no
lane_id field. Resolve its stable node ID through the caller-owned diagram,
then find that source lane in the layout. Preserve the E0609 log as a harness
failure with zero tests executed, distinct from the subsequent geometric red.
