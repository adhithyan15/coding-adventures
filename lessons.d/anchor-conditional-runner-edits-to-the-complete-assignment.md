---
category: Python
---

# Anchor conditional runner edits to the complete assignment

The minima runner v3 deriver targeted a conditional suffix also present in a
different assignment. Its unique-occurrence guard correctly refused before
writing the runner or executing Rust. The failed deriver remains preserved.

Select the complete expected-failure assignment, assert exactly one matching
line and its complete ending, then inspect the bounded successor diff. Keep
the earlier successful runner immutable. A preparatory guard refusal is not
a compiled semantic red or successful validation.
