# coding-adventures-tree-set-native

Rust-backed tree set for Python via the repo's zero-dependency `python-bridge`.

## What It Provides

- Native `TreeSet` backed by the Rust [tree-set](../../rust/tree-set/) crate
- Sorted iteration, rank, range, and set-algebra helpers
- Numeric operations routed through Rust for the fast path
