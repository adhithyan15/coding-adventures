---
category: Cross-platform & Windows BUILD_windows
---

# Merge writers must preserve bytes inspected by source anchor controls

The CV04a conflict resolver used Python's default Windows text writer on Chief
daemon source. That emitted CRLF bytes. Rust's `include_str!` retained them, so
the security control looking for the canonical `#[cfg(test)]\nmod tests {`
boundary refused the candidate even though Git showed no content diff.

Restore canonical LF bytes with an explicit `newline='\n'` writer. Keep the
original source-anchor assertion and exact eight anchored vault-openers unchanged;
the repaired full daemon suite must pass. Git blob equality alone does not prove
equality of files consumed as raw source data. Preserve original failed logs,
check both raw bytes and repository objects, and make conflict writers explicit
about newline policy before rerunning the affected control.
