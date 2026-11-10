---
category: Rust
---

# Making a tactic accept a previously refused operation breaks dependents' tests that used it as the example of an unsupported input

constraint-engine 0.5.0 started lowering `bvudiv`. constraint-vm's `tests/qfbv.rs` used `(bvudiv x x)` as its example of an input the tactic must refuse, so its test failed. The constraint-engine tests alone passed. It was only caught by running the BUILD file of every dependent (`cargo tree -i <crate>` lists them), not just the crate under change.

When widening what a solver or parser accepts, `grep -rn` the operation name across the other crates' tests for "refused / unsupported / Unknown" expectations before running their BUILDs. Pick refusal examples that are structurally outside the fragment (an unknown function name), not merely not-yet-implemented.
