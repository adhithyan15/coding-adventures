## 0.105.0 — 2026-08-11 — static integral exponent-chain output

The bounded real output path now evaluates right-associated exponent chains
whose integer- or real-literal members are all nonnegative and integral. Every
computed exponent must remain within the existing cap of 64; fractional,
negative-chain, oversized, and runtime shapes continue to fail closed.

