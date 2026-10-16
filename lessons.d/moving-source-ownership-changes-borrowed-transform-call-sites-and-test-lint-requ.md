---
category: Rust
---

# Moving source ownership changes borrowed transform call sites and test lint requirements

D3b focused execution passed all 15 tests, but strict all-target Clippy rejected
eleven warnings before the full suite ran. Moving String ownership leaves a
borrowed str at the transform boundary, so remove the old redundant ampersands.
Use values iteration, fixed-size slice chunks, borrowed one-element slices and
direct limit initializers in the new tests. Preserve the failed receipt and run
strict lint on the successor source; focused test success cannot replace it.
