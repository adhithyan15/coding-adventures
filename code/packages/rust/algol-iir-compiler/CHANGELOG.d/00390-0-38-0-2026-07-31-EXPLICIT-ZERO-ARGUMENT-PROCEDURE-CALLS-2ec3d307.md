## 0.38.0 — 2026-07-31 — explicit zero-argument procedure calls

Typed and proper procedures with no formal parameters may now be called with
explicit empty parentheses. `answer()` works in an expression and `reset()` in
statement position, each lowering to an ordinary zero-argument IIR `call`.
Bare names retain their existing parsing: a statement may use a report-style
bare procedure name, while an expression bare name remains a variable read.

