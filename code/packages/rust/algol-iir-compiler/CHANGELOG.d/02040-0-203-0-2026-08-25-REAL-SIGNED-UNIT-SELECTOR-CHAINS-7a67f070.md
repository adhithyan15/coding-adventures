## 0.203.0 — 2026-08-25 — real signed-unit selector chains

Bounded static while analysis now recognizes finite-real multiplication and
division chains whose exact signed-unit factors have an even number of negative
ones. These chains preserve the selector's value and zero sign bit; odd-sign
chains and division by a selector remain conservative.

