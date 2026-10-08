---
category: CI & GitHub Actions
---

# A native-suite gate must also select its producer package and toolchain

The first Windows gate repair tested the real gate evaluator but omitted the
producer selection preceding it. Independent review reproduced a CV02
specification-only change with the gate enabled, no affected compiler package,
and no Rust toolchain. A passing Boolean test therefore could not demonstrate
that the native compiler command could execute.

Specify the exact external-specification-to-package relation, then test the
emitted production plan across Linux, macOS and Windows. Check the selected
compiler, required toolchain and native step gate together. Keep selection
bounded to the exact consumer, honor explicit language filters, and fail closed
when an expected consumer is missing rather than accepting an empty plan.
