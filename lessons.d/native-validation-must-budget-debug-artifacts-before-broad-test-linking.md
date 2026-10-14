---
category: Rust
---

# Native validation must budget debug artifacts before broad test linking

The nine-package native ABI validation exhausted disk while linking LANG tests.
Its four terminal stages exited101 before any tests executed; preserve those
logs as build failures. The isolated delivery target contained10.35GB of
regenerable debug artifacts. After confirming the validator was terminal and
the resolved directory was exactly its own target/debug, remove only that
directory. Rerun the complete test set with debug symbols and incremental
compilation disabled; keep all inputs, assertions and limits. Record those
build settings in the new receipt. Never touch frozen Miri output or unrelated
worktrees to make room for a delivery build.
