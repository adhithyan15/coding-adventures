---
category: Rust
---

# `cargo fmt -p moslayout-compiler` is especially destructive: it rewrites the GENERATED `src/_grammar.rs` (600+ lines) AND explodes the hand-formatted compact `PRIMITIVES` array (several entries per line) into one-per-line

Adding two UI35 primitives — a genuinely +15-line change — produced an 837-line diff across a generated file and a roster nobody asked me to reformat. Verify with `git diff --stat`: a purely additive registration should show insertions only. Fix: `git checkout --` both files and re-apply the addition by hand in the file's existing compact style, skipping `cargo fmt` for this crate entirely. Same family as the `adj-lang` generated-grammar lesson below and the `cargo fmt -p <pkg>` scope lesson.
