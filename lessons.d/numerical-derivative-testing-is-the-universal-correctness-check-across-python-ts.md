---
category: Compiler / VM / language pipeline
---

# Numerical-derivative testing is the universal correctness check across Python/TS/Rust CAS ports

Instead of asserting exact IR shapes (which vary with the surrounding simplifier passes), substitute `x ← x_val` into the returned closed form, evaluate through the full backend (`vm.eval` / `SymbolicBackend.eval`), and central-difference at several sample points. Compare against the original integrand at the same samples. Tolerance of `1e-4` with step `h = 1e-5` is enough headroom for f64 round-trips through `Sin`/`Cos`/`Tan`/`Atan`/`Sqrt`. This single pattern carried Phases 26, 27, 28, and 34 across three languages with zero per-language correctness divergence.
