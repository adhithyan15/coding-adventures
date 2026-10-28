---
category: Rust
---

# Gate companions need private transition engines

Making a checked functional simulator the production backend of its gate
partner creates a convincing differential test while proving nothing about the
gate implementation. Keep the functional crate in `dev-dependencies` only.
The gate package should own a private strict transition engine whose decoded
control lines drive gate arithmetic and whose result commits through stable-Q
DFF storage. Differential tests can then compare two genuinely independent
production paths, while source locks and compile-valid hostile mutations guard
the boundary against later oracle substitution or direct-Q shortcuts.
