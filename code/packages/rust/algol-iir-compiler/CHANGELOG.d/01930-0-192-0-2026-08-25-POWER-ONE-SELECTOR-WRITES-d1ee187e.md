## 0.192.0 — 2026-08-25 — power-one selector writes

Bounded static while analysis now recognizes integer and real exponentiation
whose complete literal exponent chain evaluates to one. This matches the
existing zero-multiply lowering exactly; dynamic, real-valued, zero, and
otherwise non-unit exponents continue to fail closed.

