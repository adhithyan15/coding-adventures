## 0.117.0 — 2026-08-11 — static integer power snapshots

Integer snapshot tracking now evaluates nonnegative literal exponent chains
within the existing exponent cap using checked `i64` powers. Overflow,
negative or dynamic exponents, and oversized chains invalidate metadata.

