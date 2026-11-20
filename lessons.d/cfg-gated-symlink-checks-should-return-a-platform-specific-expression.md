---
category: Rust
---

# Cfg-gated symlink checks should return a platform-specific expression

The shared Rust test bootstrap failed strict Clippy with `clippy::needless_bool` in
`test-bootstrap/src/manifest.rs`. Its helper returned `true` early for a symbolic
link, then used a Windows-only reparse-point check and a non-Windows `false`
fallback. That shape made the non-Windows branch a needless Boolean return, while
the Windows branch still needed to recognize reparse points that are not
symbolic links.

Put the complete predicate in each cfg branch: on Windows, combine
`is_symlink()` with the reparse-point attribute; elsewhere, return `is_symlink()`
directly. This keeps the platform-specific behavior visible to Clippy and
preserves the Windows security check. Verify the exact package with strict
Clippy and its tests before treating an aggregate Rust build as clear.
