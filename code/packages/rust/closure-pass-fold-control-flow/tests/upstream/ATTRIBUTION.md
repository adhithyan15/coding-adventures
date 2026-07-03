# Attribution

Tests in this directory are ported from the Google Closure Compiler
under the Apache License, Version 2.0:

    https://github.com/google/closure-compiler
    LICENSE: https://www.apache.org/licenses/LICENSE-2.0

## Files ported

- `peephole_minimize_conditions_test.rs`
    - upstream: `test/com/google/javascript/jscomp/PeepholeMinimizeConditionsTest.java`
    - blob SHA at port time: `732eaa9532f1af8cd05a58677f5f966695087492`
    - tracked commit: see `UPSTREAM_SHA`

## Translation notes

Third port under CLOC12 (after `closure-pass-constant-fold` in CLOC12.02
and `closure-pass-dce` in CLOC12.04). Same per-crate `tests/upstream/`
layout per CLOC12.01 §3.

- Upstream tests are written against a JS source-string surface
  (`fold("if(x){foo()}", "x&&foo()")`). closurec doesn't yet expose a
  source-string → typed `Program` bridge — `javascript-parser::parse_javascript`
  returns a generic `GrammarASTNode`. Until that bridge lands, ports
  here build typed `Program` values by hand using the same `IfStatement`
  / `WhileStatement` constructors as `closure-pass-fold-control-flow`'s
  own inline tests.
- **Coverage scope.** Our `FoldControlFlowPass` today only does:
    1. `if (truthy literal) C else A` → `C`.
    2. `if (falsy literal) C else A` → `A`.
    3. `if (falsy literal) C` (no alternate) → `EmptyStatement`.
    4. `while (falsy literal) …` → empty.
  Upstream `PeepholeMinimizeConditions.java` is much broader — it
  converts `if (x) foo()` into `x && foo()`, hoists ternaries out of
  if-else, collapses De Morgan equivalents, etc. Those compactions
  belong in this crate eventually but aren't implemented yet.
- **Most ports will be `#[ignore]`-ed.** Each ignored test cites a
  `gap-NNN` entry in `code/specs/CLOC12-gaps.md`. Tests we *can*
  cover today are literal-condition folds — the same shape this
  crate's own inline tests already check, but with names and
  semantics that match upstream.

## Ignored tests

See `code/specs/CLOC12-gaps.md` for the current set of `gap-NNN`
entries that gate ignored ports.

## Skipped (intentionally not ported)

None yet.
