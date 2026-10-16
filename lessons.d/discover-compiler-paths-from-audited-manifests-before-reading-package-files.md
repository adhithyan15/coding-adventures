---
category: Rust
---

# Discover compiler paths from audited manifests before reading package files

A read-only preparation guessed compiler-local bridge/fold paths and later a
compiler lib.rs that do not exist. The actual bridge belongs to
javascript-parser/src/bridge.rs, folding to closure-pass-constant-fold/src/lib.rs,
and closurec is a binary with src/main.rs. Read the saved audit manifest and
rg --files before opening files; do not infer package ownership from the
compiler entrypoint. A root lessons.md is absent after sharding: inspect the
tracked lessons.d policy and use lessons.py instead. Failed reads produced no
source/test mutation or validation result. Preserve actual paths in the spec.
