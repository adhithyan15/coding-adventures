---
category: Rust
---

# Run strict lint before accepting new adversarial controls

The new subgraph title tests compiled and ran, but strict all-target Clippy
rejected an assignment written without space before unary negation. Keep the
failed strict receipt, correct the spacing, and rerun the complete required
lint without an allow. A new validator also initially used an older PATH Python
without tomllib; explicitly invoke the installed Python3.13 for that harness
and keep its failed launch separate from actual test execution.
