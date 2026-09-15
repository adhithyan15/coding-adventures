## 0.107.0 — 2026-08-11 — composed static real standard-function output

Formatter-free `abs` and exact-`sqrt` values may now nest and participate in
finite literal-only real arithmetic. Evaluation remains aware of lexical
standard-function overrides, and runtime, inexact, invalid, or non-finite
subexpressions continue to fail closed.

