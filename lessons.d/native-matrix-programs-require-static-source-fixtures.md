---
category: Rust
---

# Native matrix programs require static source fixtures

LANG's matrix `Prog.src` requires a static string. An owned formatted string
failed compilation after the compiler library passed854 tests; those library
results do not establish runtime execution. Use explicit immutable source
fixtures for both condition outcomes, preserving exact marker/output/exit
oracles without leaking allocations or changing the shared harness lifetime.
