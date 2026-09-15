## 0.103.0 — 2026-08-11 — static real signed-power output

The bounded real output evaluator now accepts one explicitly signed integer
literal exponent in `-64..=64`. Negative exponents use repeated division, while
zero bases, non-finite intermediates, signed exponent chains, and runtime or
real exponents continue to fail closed without a runtime formatter.

