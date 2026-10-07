---
category: Rust
---

# Deny unknown lints explicitly when verifying a strict Clippy command

Clippy exited successfully with `-D warnings` while emitting E0602 for the removed `doc_list_item_without_indentation` allowance; its unknown-lints diagnostic explicitly ignores that warnings escalation. Inspect the actual log, remove obsolete allowances after checking the installed driver lint list, and verify with both `-D warnings -D unknown-lints`. The remaining supported documentation allowances already covered this crate, so no broad documentation edits were needed.
