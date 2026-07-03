# Attribution

Tests in this directory are ported from the Google Closure Compiler
under the Apache License, Version 2.0:

    https://github.com/google/closure-compiler
    LICENSE: https://www.apache.org/licenses/LICENSE-2.0

## Files ported

- `remove_unused_vars_test.rs`
    - upstream: `test/com/google/javascript/jscomp/RemoveUnusedCodeTest.java`
      (the descendant of the historical `RemoveUnusedVarsTest.java`; the
      unused-binding removal logic moved into `RemoveUnusedCode`)
    - tracked commit: see `UPSTREAM_SHA`

## Translation notes

Fourth port under CLOC12 (after constant-fold, dce, and the emitter /
source-map ports). Per CLOC12 §6, each upstream Java test file maps to
one Rust file in the matching pass crate's `tests/upstream/` directory.

- Upstream tests are written against a JS source-string surface
  (`test("var a = 1;", "")`). closurec does not yet expose a public
  source-string → typed `Program` entry point, so — exactly as the
  `closure-pass-dce` port does — each case is built directly on the
  typed AST with small helpers (`var_decl`, `use_stmt`, `call`, …) and
  asserts on the surviving declarator names after running **only**
  `RemoveUnusedVarsPass`.

- Our `RemoveUnusedVarsPass` implements the provably-sound core of the
  upstream pass: it drops **GLOBAL-scope** `var` / `let` / `const`
  bindings that are **unreferenced** and have a **pure** initializer
  (literal, bare identifier, or none). Upstream `RemoveUnusedCode` does
  much more — function-local removal, unused parameters, unused function
  declarations, self-referential dead cycles, and side-effect extraction
  (`var a = f();` → `f();`). Those behaviors are ported as
  `#[ignore = "blocked on gap-NNN"]` placeholders pinned to
  `code/specs/CLOC12-gaps.md` (gap-121 … gap-126) so they become live
  the moment each gap closes.
