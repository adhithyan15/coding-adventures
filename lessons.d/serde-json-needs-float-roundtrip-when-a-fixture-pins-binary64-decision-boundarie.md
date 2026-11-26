---
category: Rust
---

# serde_json needs float_roundtrip when a fixture pins binary64 decision boundaries

The default `serde_json` float parser can read a decimal fixture value one ULP
away from the binary64 value that produced it. A microbenchmark fixture pins
inclusive verdict boundaries, so that small parse difference changed an
`equivalent` result into `inconclusive` even though the Rust ratio arithmetic
was correct.

When JSON decimal numbers are evidence for exact binary64 behavior, enable
`serde_json`'s `float_roundtrip` feature and test a value at the decision
boundary. Do not loosen the boundary comparison or fixture assertion to hide a
parser-rounding mismatch; the parsed observation is part of the input contract.
