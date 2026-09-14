---
category: Rust
---

# Don't run `cargo fmt --all` for package-scoped work

— it reformats hundreds of unrelated crates and buries the feature diff. Use `cargo fmt -p <pkg>`.
