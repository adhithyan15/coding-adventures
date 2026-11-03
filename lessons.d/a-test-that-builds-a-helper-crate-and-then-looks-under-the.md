---
category: Testing & coverage
---

# A test that builds a helper crate and then looks under the workspace target dir fails whenever CARGO_TARGET_DIR is set

**What happened (2026-11-02).** The five `*_effect_completion.rs` acceptances in
`mosaic-app-bindings` run `cargo build -p mosaic-app-conformance` and then load
the cdylib from `<workspace>/target/debug`. The child `cargo` inherits
`CARGO_TARGET_DIR`, so under a per-agent target directory (the rule for
background agents, to keep worktrees from sharing build output) the library was
built *there* and every acceptance failed with "missing conformance runtime".
The failure says nothing about the code under test, and CI never sees it because
CI does not set the variable.

**Fix.** Resolve the directory the same way cargo does: `CARGO_TARGET_DIR` when
set and non-empty (joined onto the build's working directory, so a relative
value works), else `<workspace>/target`.

**Do differently.** Any test or script that shells out to `cargo build` and then
opens an artifact must ask where cargo put it, not assume `target/`. `git grep
-n 'join("target")' -- '*.rs'` finds the pattern; `cargo metadata
--format-version 1` reports `target_directory` when a dependency-free answer is
needed.
