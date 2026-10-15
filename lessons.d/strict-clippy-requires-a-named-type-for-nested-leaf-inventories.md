---
category: Rust
---

# Strict Clippy requires a named type for nested leaf inventories

The independent AVL leaf oracle compiled and passed with the complete618-test
core/parser suite, then strict Clippy rejected its nested tuple/vector return
type. Factor the test entry into a local type alias; retain the independent
height/order/extrema assertions and warnings-denied gate. Preserve the native
pass and actual lint failure at their original head; the full runner was not
successful. Do not add a lint suppression for a simple declaration repair.
