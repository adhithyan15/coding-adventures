## 0.116.0 — 2026-08-11 — static integer arithmetic snapshots

Straight-line integer snapshot tracking now evaluates checked `+`, `-`, `*`,
`div`, and `mod` expressions. Overflow, zero division, unsupported operators,
and dynamic operands invalidate the metadata rather than introducing host
arithmetic or an unsound runtime formatting shortcut.

