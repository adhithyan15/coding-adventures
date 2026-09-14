---
category: Compiler / VM / language pipeline
---

# TypeScript symbolic-ir uses `bigint` for IRInteger.value / IRRational.numer/denom

, not `number`. Fraction-arithmetic helpers in TS handlers (`fracGcd`, `fracMake`, `fracMod`) must take and return `bigint` throughout — mixing in `number` will silently truncate at the `Number.MAX_SAFE_INTEGER` boundary and corrupt the π-multiple lookup keys. Rust's analogous helpers use plain `i64` (sufficient for denominators ≤ 6).
