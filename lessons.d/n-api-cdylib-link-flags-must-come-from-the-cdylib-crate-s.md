---
category: Native extensions & FFI
---

# N-API cdylib link flags must come from the cdylib crate's own `build.rs`,

NOT from a bridge dep — `cargo:rustc-cdylib-link-arg` does not propagate. On macOS, every `.node`-producing crate needs its own `build.rs` emitting `-undefined dynamic_lookup`.
