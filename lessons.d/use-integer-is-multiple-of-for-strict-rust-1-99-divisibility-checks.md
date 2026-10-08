---
category: Rust
---

# Use integer is_multiple_of for strict Rust 1.99 divisibility checks

The new bounded ACE walker passed native publication tests but failed strict
Rust 1.99 Clippy on `bytes % 4 != 0`. Use `!bytes.is_multiple_of(4)` for the
same aligned-length check. Run strict lint with the actual CI toolchain before
review and publication; do not suppress the lint or weaken alignment checks.
