---
category: Rust
---

# A checked-index loop can use `while let` without weakening its overflow stop

Clippy rejected a sift-down loop that used `loop` and immediately matched a
checked child-index calculation, with `while_let_loop`. The checked index is
still the termination condition: bind it in `while let Some(child) = ...`, then
keep the separate `child >= len` bounds check inside the loop. This satisfies
Clippy without replacing checked arithmetic with an overflowing `2 * index + 1`.
