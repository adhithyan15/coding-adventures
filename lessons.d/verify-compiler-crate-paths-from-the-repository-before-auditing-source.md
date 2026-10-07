---
category: Rust
---

# Verify compiler crate paths from the repository before auditing source

During the CV04 source-flow audit, a search included the nonexistent Rust crate path `closure-lexer/src`; `rg` reported a path error while other requested files still returned matches. A second guessed `closurec/src/simple.rs` path also did not exist; the driver logic is in `src/run.rs`. The actual JavaScript lexer is `javascript-lexer`. Verify package and source paths with the repository file inventory before searching. A partial search result must not be interpreted as evidence that the missing package has no relevant source.
