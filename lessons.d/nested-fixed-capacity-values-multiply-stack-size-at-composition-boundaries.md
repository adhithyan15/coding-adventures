---
category: Rust
---

# Nested fixed-capacity values multiply stack size at composition boundaries

An allocation-free outer array sized only by the protocol's plausible item
count can still overflow a default test-thread stack when every element embeds
its own fixed-capacity arrays. The multiplication is easy to miss because each
package's bound is reasonable in isolation.

Before choosing a fixed outer capacity, inspect `size_of` or exercise the
complete composed value on the default stack. Keep the smallest defensible
fail-closed outer ceiling, document that it bounds the composed stack footprint,
and test one value beyond the limit. Do not mask the design problem by raising
the test stack or moving an otherwise allocation-free parser onto the heap.
