---
category: Rust
---

# A scripted insertion anchored on a fn line splits the item from its doc comment and attributes

**What went wrong.** LANG83 PR1b added helper functions and constants to
large backend files (`iir-to-llvm/src/lib.rs`, `iir-to-wasm/src/lower.rs`,
`iir-to-jvm-class-file/src/lower.rs`) with a scripted `replace(anchor, new +
anchor)`. The anchor was the target item's `fn name(` line. The `///` doc
comment and the `#[allow(...)]` attribute above that line stayed where they
were, so they now documented and decorated the new code. The original
function lost its docs. It happened three times in one session.
`cargo build` and `cargo test` accepted all three. `cargo clippy -- -D
warnings` rejected them as `empty_line_after_doc_comments`,
`doc_lazy_continuation` and `empty_line_after_outer_attr`. If an attribute
had applied cleanly to the inserted item, nothing would have failed.

**Next time.** To insert before an item, anchor on the first line of its
doc comment or attribute block, or append after the previous item's closing
brace. After any scripted insertion, run
`cargo clippy --all-targets -- -D warnings` on that crate before committing.
Then check `grep -n -B3 '^fn <name>'` to confirm the docs still sit on the
right item.
