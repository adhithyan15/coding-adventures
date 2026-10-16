---
category: Rust
---

# Retained tests belong beside original functions before separate test modules

The first parent-source retention helper assumed lib.rs ended with the original
unit-test module. Its guard refused before any source write: this parser has
separate cfg(test) modules after that module. A later shell command still
committed the new38-cell test file, so its title alone did not establish parent
retention. The corrected helper inserts each complete renamed parent function
beside its original function, preserving the entire file suffix and every other
byte. Full-function and full-file inverses now verify actual retention. Preserve
the original programs and assertions; do not infer implementation from a commit
title or a partially completed compound shell command.
