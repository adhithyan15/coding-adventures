## 0.93.0 — 2026-08-11 — integer standard output

Implementation-defined `print` and `output` procedure calls now accept integer
variables and expressions, lowering them to the shared `print_i64` builtin.
String output retains its existing `print_str` paths; real and boolean output
remain explicit type errors.

