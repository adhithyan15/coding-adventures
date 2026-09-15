## 0.221.0 — 2026-08-27 — tracked integer function exponents

Static real-value metadata now carries exact tracked integer operands through
implemented standard functions such as `abs`, `sign`, and `entier` when they
form bounded integral power exponents. Runtime lowering remains unchanged.

