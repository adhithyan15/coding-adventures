## 0.94.0 — 2026-08-11 — boolean standard output

Implementation-defined `print` and `output` procedure calls now render boolean
variables and expressions as `true` or `false`. Typed control flow selects the
corresponding shared string literal, avoiding a boolean-to-integer ABI coercion.

