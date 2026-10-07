---
category: Rust
---

# Windows automatic DACL inheritance depends on the file handle's opened parent

An initial native publication repair used `SetSecurityInfo` through a reopened
destination handle for every captured DACL. Protected policies survived, but
legacy unprotected ACLs gained inherited parent ACEs and failed exact readback.
Using the retained stage handle preserved explicit legacy entries, while that
same handle removed inherited entries from a new file's intended parent policy.

Use a verified copy-or-reject protocol: route explicit policies through the
private stage parent and inherited policies through the actual destination
parent. First round-trip the policy on a distinct permanently empty inode;
reject unsupported policies before originals change. Verify final policy again.
Never widen a future data-bearing inode during preflight: tightening its DACL
does not revoke an already-open reader. Restore candidate privacy only on
pre-commit failure, since stage and successful output links share one policy.
