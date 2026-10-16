---
category: Rust
---

# Verify compiler crate paths from the repository before auditing source

During the CV04 source-flow audit, a search included the nonexistent Rust crate path `closure-lexer/src`; `rg` reported a path error while other requested files still returned matches. A second guessed `closurec/src/simple.rs` path also did not exist; the driver logic is in `src/run.rs`. The actual JavaScript lexer is `javascript-lexer`. Verify package and source paths with the repository file inventory before searching. A partial search result must not be interpreted as evidence that the missing package has no relevant source.

The follow-up CV04b read-only audit repeated this mistake by guessing nonexistent `correlation-vector/src/chronology.rs` and `clock.rs` modules. The actual module is `journal.rs`, as the file inventory and exports show. Search the existing `src` directory or enumerate its modules before selecting a file; a subsystem concept does not imply a filename. The search returned partial matches plus a real path error, so it was not a complete module audit.

The D2 audit also guessed `tests/cv_fold_lineage.rs`; the actual existing graph
tests are in `tests/cv_fold_trace.rs`. Run `rg --files` with a filename glob
first, then open its exact returned path. A familiar test concept is not evidence
of a filename, and a failed read does not count as auditing those assertions.



An idle D2 follow-up search repeated this error with the positional path

`code/specs/CLOC27*` on Windows. PowerShell did not expand it and rg returned

os error123 alongside partial matches. Use `rg --files code/specs -g 'CLOC27*'`

to discover exact files, or use `-g` while searching the existing directory.

Treat the failed partial search as incomplete evidence, never a source audit.

D3a again guessed src/cli.rs for flag mapping; the actual mapping is wire.rs.
Enumerate the existing src directory, then search it with a Rust filename glob.
