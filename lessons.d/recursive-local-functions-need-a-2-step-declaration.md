---
category: Rust
---

# Recursive local functions need a 2-step declaration

Short assignment `addConstant := func(...)` can't reference itself. Use `var addConstant func(...)` then `addConstant = func(...)`. (Same pattern in Go.)
