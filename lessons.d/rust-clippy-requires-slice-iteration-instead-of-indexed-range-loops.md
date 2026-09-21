---
category: Rust
---

# Rust Clippy requires slice iteration instead of indexed range loops

With warnings denied, Clippy's `needless_range_loop` rejects a numeric range
whose only purpose is indexing the same slice. Use
`slice.iter().copied().enumerate().skip(start).take(length)` when both the byte
and its absolute offset are needed; this also avoids a direct indexing panic
site in hostile-input parsers.
