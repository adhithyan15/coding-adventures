## 0.100.0 — 2026-08-11 — static real multiplication output

Finite literal-only real multiplication now joins addition and subtraction in
the bounded compile-time output evaluator. Products, including parenthesized
and conditional leaves, print through the shared string path; division,
runtime operands, and non-finite results continue to fail closed.

