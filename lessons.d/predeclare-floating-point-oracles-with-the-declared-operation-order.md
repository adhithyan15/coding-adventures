---
category: Testing & coverage
---

# Predeclare floating-point oracles with the declared operation order

The independent Python-only Venn preflight initially asserted the rounded
titled coordinate484.16. That assertion failed before it wrote an oracle
model: the declared400+(526*0.16) evaluates to484.15999999999997, bits
0x407e428f5c28f5c2, while the decimal literal has bits0x407e428f5c28f5c3.
This was an oracle-preparation failure, not a Rust test or source result.

Preserve that failed preflight separately. Compute and record the independent
operation-order bits before changing production; use exact-bit constants for
the authored witness instead of rounded display coordinates. Keep platform
math limitations explicit and consume genuine Rust results later. Do not
weaken assertions with a tolerance or derive expected positions from returned
production layouts. Miri remains DEFERRED/PENDING and untouched.
