---
category: Rust
---

# Anchor Rust transaction edits on the complete method before placing the success return

While adding fallible CV02 deletion, a short patch anchor placed `Ok(())`
before the tombstone-recording block. Compilation rejected the resulting
method. The fix moved the return after the recorded deletion and usage commit.

Read the entire method after a transaction edit. Its order must remain
preflight, mutation, ledger commit, then success. Anchor changes on the method
signature and relevant final block, rather than a repeated `if !enabled` shape.
Run the focused test immediately so a misplaced return cannot survive into a
larger implementation diff.
