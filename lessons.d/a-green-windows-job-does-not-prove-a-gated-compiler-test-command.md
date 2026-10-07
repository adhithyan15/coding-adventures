---
category: CI & GitHub Actions
---

# A green Windows job does not prove a gated compiler test command ran

The CV02 repaired-head CI plan selected `rust/programs/closurec` for Windows,
but its `build-windows-os-suites` verdict was false. The workflow skips ordinary
Windows package tests unless that gate or another native toolchain requirement
is enabled. A green job could therefore prove unrelated lint without running
the compiler's new permission regressions.

Add the compiler's real affected-package identity and its native acceptance
specification to the step gate, with real-evaluator positive and negative tests.
Inspect the emitted plan, actual step conclusion and successful package result
before claiming native acceptance; a job label is not execution evidence.
