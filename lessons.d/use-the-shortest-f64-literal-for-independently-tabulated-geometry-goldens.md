---
category: Rust
---

# Use the shortest f64 literal for independently tabulated geometry goldens

Circular Railroad source3e2bbad4 passed its focused25 parser,14 layout and12
paint tests, but strict all-target Clippy rejected six decimal golden literals
with excessive_precision. The full14-package test stage did not run; retain
that actual failed strict receipt instead of claiming aggregate acceptance.

Use Clippy's representable decimal spelling for the six occurrences. Record
the old/new f64 differences (at most one ULP) and keep the fixed1e-12 comparison
tolerance, full command/control/end-point assertions and radial-error controls.
Do not suppress the lint or loosen assertions. Production source is unchanged;
rerun focused tests and the full affected suite/strict lint on the corrected
source and obtain independent review for that exact head.
