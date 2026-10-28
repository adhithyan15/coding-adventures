---
category: Cryptography & security review
---

# Prove optimized security paths from the CFG, not compiler enum encodings

An optimized-IR regression gate identified a successful Rust `Result` return
by matching the discriminant value emitted by one compiler. A newer hosted
stable compiler changed that private enum encoding without changing the
program's behavior, so the gate rejected correct code. Do not make security
proofs depend on compiler-owned layout choices. Start from the mandatory
success operation, follow every normal-return path in the optimized control-flow
graph, and require each path to cross the exact wipe being proved.
