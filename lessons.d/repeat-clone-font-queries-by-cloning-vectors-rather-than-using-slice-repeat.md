---
category: Rust
---

# Repeat Clone font queries by cloning vectors rather than using slice repeat

The Timeline typography test used queries.repeat(2) on Vec<FontQuery>. Rust's
slice repeat requires Copy, while FontQuery owns strings and implements Clone.
Cargo therefore failed with E0277 before any test harness ran. Concatenate two
cloned vectors instead: [queries.clone(), queries].concat(). Keep the exact
ordered queries and both complete Scene assertions unchanged.

The failed eabf36e3 run and raw compiler streams remain evidence of a build
failure, never a compiled semantic regression red. Rerun the unfiltered harness
at the fixed exact commit before treating the intended failures as exercised.
