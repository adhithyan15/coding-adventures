# Attribution

Tests in this directory are ported from the Google Closure Compiler
under the Apache License, Version 2.0:

    https://github.com/google/closure-compiler
    LICENSE: https://www.apache.org/licenses/LICENSE-2.0

## Files ported

- `rename_vars_test.rs`
    - upstream: `test/com/google/javascript/jscomp/RenameVarsTest.java`
    - tracked commit: see `UPSTREAM_SHA`

## Translation notes

CLOC12 port for the `rename-globals` pass (after constant-fold, dce, the
emitter / source-map ports, remove-unused-vars, inline, and
fold-control-flow). Per CLOC12 §6, each upstream Java test file maps to one
Rust file in the matching pass crate's `tests/upstream/` directory.

- Unlike the AST-builder ports (dce, remove-unused-vars), `rename-globals`
  exposes everything a source-string surface needs through public crate
  APIs, so this port drives the real `source → bridge → RenameGlobalsPass →
  emit` chain and asserts on the emitted string — the same surface upstream
  `RenameVarsTest` uses (`test(js, expected)`).

- **What our pass does today:** it renames **GLOBAL-scope** bindings
  (top-level `function` names and `var` / `let` / `const` targets) to the
  shortest fresh names `a`, `b`, `c`, … in first-appearance order. It leaves
  untouched: names already 1 character long (no shrink available), free /
  undeclared globals (`window`, `console`, a called-but-undeclared
  `helper()`), dotted property keys (`obj.total`), and any name in the
  do-not-rename (externs) set.

- **What upstream `RenameVars` additionally does** — renaming **local**
  variables and function parameters, the pseudo-name / stable-name and
  `_GLOBAL_`-prefix reservation modes, and re-using freed short names across
  disjoint local scopes — our pass does not cover yet. Those cases are
  ported as `#[ignore = "blocked on gap-NNN"]` placeholders pinned to
  `code/specs/CLOC12-gaps.md` (gap-134 … gap-137) so they go live the moment
  each gap closes.

Every active test that *disagrees* with our pass is a real closurec defect,
not a translation artifact — that is the entire point of the port.
