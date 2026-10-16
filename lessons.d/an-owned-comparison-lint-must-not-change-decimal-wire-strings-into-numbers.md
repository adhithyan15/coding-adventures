---
category: Testing & coverage
---

# An owned-comparison lint must not change decimal wire strings into numbers

CLOC33 full0d passed2,881 tests but CLI strict Clippy rejected constructing
start/end.to_string() for each metadata lookup comparison. Its suggested direct
Value==usize comparison would change the expected JSON representation: source
coordinates are precision-safe canonical decimal strings, not JSON numbers.

Hoist the two expected Strings outside the predicate and compare borrowed
as_str values. Preserve every exact coordinate/parent/ancestry/output assertion.
Keep the original full receipt overall unaccepted for its strict failure, rerun
the changed target and strict gate, and explicitly bind unchanged inputs before
retaining successful tests from the earlier execution head. Never call a
suggested lint rewrite semantically neutral without checking the wire schema.
