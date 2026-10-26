---
category: Rust
---

# Clippy requires adjacent state-validation conditions to be collapsed

With warnings denied, Clippy rejects an outer `if` whose body contains only a
second `if`. Combine the predicates with `&&`, retaining parentheses around a
compound inner predicate. This is especially useful in snapshot validators:
the resulting guard presents the complete rejection condition in one place
and avoids discovering a style-only failure after the behavioral tests pass.
