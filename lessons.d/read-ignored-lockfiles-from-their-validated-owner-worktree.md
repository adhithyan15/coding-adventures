---
category: Rust
---

# Read ignored lockfiles from their validated owner worktree

The D1 source-transfer helper proved tracked maps, then failed because it guessed
the native ignored Cargo.lock exists in R4. R4 owns only its separate standalone
compiler lock. Preserve helper exit1 and its unaccepted partial proof; compare
registry tuples to the existing accepted R2 native lock instead of copying or
generating another lock. Verify ignored-file existence and ownership first.

A first correction script itself failed to parse a Windows path nested inside a
Python string. Preserve it, use Path.as_posix plus repr for emitted Python paths,
and compile the corrected helper before execution. Neither failure ran Cargo or
changed source, and the active producer/frozen Miri inputs stayed unchanged.
Both source producers are now consumed; the filled lesson is installed.
