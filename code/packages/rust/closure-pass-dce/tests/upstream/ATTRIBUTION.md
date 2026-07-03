# Attribution

Tests in this directory are ported from the Google Closure Compiler
under the Apache License, Version 2.0:

    https://github.com/google/closure-compiler
    LICENSE: https://www.apache.org/licenses/LICENSE-2.0

## Files ported

- `peephole_remove_dead_code_test.rs`
    - upstream: `test/com/google/javascript/jscomp/PeepholeRemoveDeadCodeTest.java`
    - blob SHA at port time: `db5fe5af7e3dcba58560a0338c315d262dfbc04a`
    - tracked commit: see `UPSTREAM_SHA`

- `unreachable_code_elimination_test.rs`
    - upstream: `test/com/google/javascript/jscomp/UnreachableCodeEliminationTest.java`
    - tracked commit: see `UPSTREAM_SHA`
    - Pins the block-level reachability cleanup `DcePass` performs today
      (drop-after-`return`/`throw`, empty-statement removal, nested-block
      recursion, and the hoisting-soundness *decline* when a dead tail
      carries a `var`/`function`). Upstream's full CFG-based analysis —
      code after an `if`-both-branches-terminate (gap-151) and after
      `break`/`continue` in a general loop block (gap-152) — is pinned as
      `#[ignore]` placeholders.

## Translation notes

This is the **second** port under CLOC12 (after
`closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`).
Per CLOC12 §6, each upstream Java test file maps to one Rust file in the
matching pass crate's `tests/upstream/` directory.

- Upstream tests are written against a JS source-string surface
  (`fold("function f(){return 3;foo();}", "function f(){return 3;}")`).
  closurec doesn't yet expose a public source-string → typed `Program`
  bridge — `javascript-parser::parse_javascript` returns a generic
  `GrammarASTNode`. Until that bridge lands (a future CLOC11.* slice),
  ports here build typed `Program` values by hand using the same
  literal/statement constructors as `closure-pass-dce`'s own inline
  unit tests.
- **Coverage scope.** Our `DcePass` only handles two narrow
  categories per the crate-level docs:
    1. Dead-after-`ReturnStatement` removal inside `BlockStatement`s.
    2. `EmptyStatement` removal from `BlockStatement`s.
  Upstream `PeepholeRemoveDeadCode.java` is much broader — it
  collapses dead `if`-branches, simplifies useless loops, optimizes
  switches, removes useless labelled statements, normalises `let`/
  `const`/`var` lifting, prunes side-effect-free calls, etc. Most of
  those land in other passes in our setup (fold-control-flow,
  remove-unused-vars, etc.), or are simply not implemented yet.
- **Most ports will be `#[ignore]`-ed.** That's expected and is the
  same gap-tracking pattern as CLOC12.02. Every ignored test cites a
  `gap-NNN` entry in `code/specs/CLOC12-gaps.md`.

## Ignored tests

See `code/specs/CLOC12-gaps.md` for the current set of `gap-NNN`
entries that gate ignored ports.

## Skipped (intentionally not ported)

None yet.
