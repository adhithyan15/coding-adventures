---
category: Rust
---

# A script that inserts a struct field after a line must not copy that line's prefix when several fields share it

**What went wrong.** LANG84 phase 4b added `release: Option<String>` to
`interpreter_ir::IIRModule`, so all 161 struct literals in the repository
needed a `release: None`. A Python script found each literal's `imports`
line and inserted a new line after it. For the indentation it copied
everything on that line before `imports`. On a doc-comment line that held
two fields (`//!     exports: vec![], imports: vec![],`), that copied
`exports: vec![], ` into the new line too, so the literal named `exports`
twice. On a plain code line with three fields it did the same thing. A
`git diff -U0 | grep '^+.*exports.*release'` caught all three. `cargo check
--all-targets` would have caught the code line, but only `cargo test --doc`
compiles the doc examples.

**Fix.** Rewrite the inserted line to the leading whitespace (plus any
`//!`/`///` marker) followed by `release: None,`.

**Next time.** When inserting a field mechanically, compute the new line's
prefix from the leading whitespace and comment marker only. Then grep the
added lines for any other field name, and run `cargo test --doc` on every
crate whose doc examples were edited, as well as `cargo check
--all-targets`.
