---
category: Compiler / VM / language pipeline
---

# Rust handler factories that recurse via `vm.eval` need `vm: &mut VM`, not `_vm: &mut VM`

The convention in `symbolic-vm/src/handlers.rs` was that pure-numeric handlers ignored the VM (`_vm`); Phase 31+ symmetry rules (`sin(-x) → -sin(x)`) need a recursive `vm.eval(...)` call to re-simplify the wrapped result, so the underscore must come off. Forgetting this gives "unused variable" warnings the first time you compile, but the bigger trap is leaving an early-return path that constructs a `NEG` wrapping an *unevaluated* `Sin(x)` — works in isolation but breaks composition (`sin(-(-x))` doesn't fold to `sin(x)` because the inner `-(-x)` never re-enters the simplifier).
