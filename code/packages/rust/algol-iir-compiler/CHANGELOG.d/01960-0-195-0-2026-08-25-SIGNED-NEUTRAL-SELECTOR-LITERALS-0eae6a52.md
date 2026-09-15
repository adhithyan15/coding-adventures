## 0.195.0 — 2026-08-25 — signed neutral selector literals

Bounded static while analysis now recognizes exact unary-plus neutral literals
and signed integer zero inside selector identity expressions. Negative one and
other signed changing expressions remain conservative.

