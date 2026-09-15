## 0.118.0 — 2026-08-11 — static integer-function snapshots

Integer snapshot tracking now evaluates non-overridden `abs`, `sign`, and
`entier` calls over bounded static operands. Absolute-value overflow and
non-finite, inexact-range, dynamic, or lexically overridden calls fail closed.

