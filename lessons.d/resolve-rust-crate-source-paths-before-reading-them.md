---
category: Rust
---

# Resolve Rust crate source paths before reading them

Rust type names and dependency aliases do not guarantee a matching package
directory. Guessing a source path from a type name wastes a command and can
hide the actual ownership boundary.

Before opening a dependency's source, resolve its package from the consuming
`Cargo.toml` or search exact manifest `name` fields, then read the discovered
path. If the type is re-exported or supplied by another dependency, search the
workspace for the exact declaration rather than inventing a directory name.
