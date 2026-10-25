---
category: Rust
---

# Cargo fmt selects packages before the rustfmt argument separator

`cargo fmt -- <paths>` passes the paths to rustfmt, but cargo-fmt does not
discover targets that way and exits with `Failed to find targets`. For focused
workspace formatting, select each crate with `-p` before `--`; reserve the
arguments after `--` for rustfmt options.
