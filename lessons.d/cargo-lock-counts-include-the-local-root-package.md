---
category: Rust
---

# Cargo lock counts include the local root package

The D1 red-run helper guessed that Cargo's "Locking 50 packages" meant a
50-entry lock and14 registry dependencies. Its startup assertion failed before
Cargo tests or a receipt were started. The actual standalone lock contains51
entries:36 local packages including closurec, and15 registry packages. Parse the
lock instead of subtracting a guessed root count from a progress message. Bind
full name/version/source/checksum tuples to already accepted dependency receipts
and keep this helper failure separate from any semantic test result.
