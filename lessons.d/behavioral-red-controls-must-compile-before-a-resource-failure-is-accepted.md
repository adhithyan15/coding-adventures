---
category: Testing & coverage
---

# Behavioral red controls must compile before a resource failure is accepted

The first lane-ordering resource control reused `DiagramDirection` across
nested loops as if it implemented `Copy`. Rust returned E0382 before running
any tests. Cargo101 alone was therefore not the expected resource failure.

Clone the direction in the fixture, retain the original compile log and receipt,
then rerun the unchanged work bound. Accept a behavioral red only when the
specific counted assertion fails and the accompanying equivalence tests pass.
Never label a compile error, empty filter or stopped process as that red.

The helper's earlier anchor preflight also stopped before a source write: it
looked for `edge_gap`, while the existing helper is `layout_gap`. An immutable
reference snapshot had already been created. Validate every source anchor before
creating snapshots, or verify identical contents before reusing one; do not
overwrite it to make a retry succeed.

A fresh checkout regenerates an ignored Cargo.lock. Preserve that generated
file separately and copy the tested checkout's exact lock into the new checkout
before comparing execution, keeping the original tested inputs untouched.

The compiling retry measured17280 visits against3240, but its separate full-layout
oracle also failed because both old layouts contain NaN self-loop endpoints and
NaN is unequal to itself. Prove each corresponding endpoint's exact floating
bits first. Replace only already-proven NaNs in test-owned copies for the full
struct comparison; unexpected NaNs, changed payloads and other fields still fail.
Preserve the original one-pass/two-fail receipt rather than relabeling it.
