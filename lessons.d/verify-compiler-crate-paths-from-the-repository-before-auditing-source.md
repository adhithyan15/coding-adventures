---
category: Rust
---

# Verify compiler crate paths from the repository before auditing source

During the CV04 source-flow audit, a search included the nonexistent Rust crate path `closure-lexer/src`; `rg` reported a path error while other requested files still returned matches. A second guessed `closurec/src/simple.rs` path also did not exist; the driver logic is in `src/run.rs`. The actual JavaScript lexer is `javascript-lexer`. Verify package and source paths with the repository file inventory before searching. A partial search result must not be interpreted as evidence that the missing package has no relevant source.

The follow-up CV04b read-only audit repeated this mistake by guessing nonexistent `correlation-vector/src/chronology.rs` and `clock.rs` modules. The actual module is `journal.rs`, as the file inventory and exports show. Search the existing `src` directory or enumerate its modules before selecting a file; a subsystem concept does not imply a filename. The search returned partial matches plus a real path error, so it was not a complete module audit.
