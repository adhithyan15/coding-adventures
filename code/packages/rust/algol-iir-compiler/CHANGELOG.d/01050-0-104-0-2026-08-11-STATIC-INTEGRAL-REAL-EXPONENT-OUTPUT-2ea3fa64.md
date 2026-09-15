## 0.104.0 — 2026-08-11 — static integral-real exponent output

Finite real literal exponents whose values are exactly integral and within
`-64.0..=64.0` now share the deterministic repeated-arithmetic output path.
Fractional, oversized, non-finite, chained-real, and runtime exponents continue
to require runtime formatting and therefore fail closed.

