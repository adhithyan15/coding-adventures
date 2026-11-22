---
category: Rust
---

# Run cargo fmt on the target crate manifest because cargo fmt --all rewrites every workspace member

Running `cargo fmt --all` from a package directory still formats every Rust
workspace member. In a large moving monorepo, that can rewrite thousands of
unrelated files even when the intended change touches one crate, and formatter
version pinning does not make that broad scope appropriate.

Format the target crate with its manifest path instead:

```bash
cargo fmt --manifest-path code/packages/rust/<crate>/Cargo.toml
```

For a formatting check, add `-- --check` to that same manifest-scoped command.
After formatting, inspect `git diff --name-only` and require every changed path
to belong to the intended slice before continuing.
