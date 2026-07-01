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

Twelfth port under CLOC12 (after constant-fold ×2, dce, fold-control-flow,
inline, remove-unused-vars, rename-globals, rename-properties, and the
emitter CodePrinter ×4 ports). Per CLOC12 §6, each upstream Java test file
maps to one Rust file in the matching pass crate's `tests/upstream/`
directory. This is the **first** port into `closure-pass-rename`.

Unlike the hand-built-AST ports (`dce`, `remove-unused-vars`), the rename
crate already carries `javascript-parser` + `closure-emitter` as
dev-dependencies, so this port drives the **real** source →
`grammar_to_program` bridge → `RenamePass` → `emit` roundtrip — the exact
chain closurec's SIMPLE level uses. Each case is `assert_eq!(rename(src),
expected)` on the emitted string, mirroring upstream's
`test("var x=1; x", "var a=1; a")` shape (modulo the emitter's
minified-but-spaced output: binary operators keep spaces and function
declarations get a trailing `;`).

### What our pass supports (active `#[test]`s)

Upstream `RenameVars` is a single whole-program renamer that shortens
**every** non-externed binding — globals, all nested function scopes, and
function *names* — using a frequency-biased name generator so the most
common identifiers get the shortest names. closurec deliberately **splits**
that responsibility: `RenamePass` (this crate) is the conservative,
provably-sound **local** renamer — it shortens parameters and uniquely-bound
`var`/`let`/`const` locals of **leaf** functions (no nested functions),
while global renaming lives in the separate `rename-globals` pass (ADVANCED
only). The active cases pin the local-renaming behaviors that correspond to
upstream `RenameVars` intent restricted to a single leaf scope:

- leaf parameter and local `var`/`let`/`const` renaming, at declaration and
  every use site (upstream `testRenameSimple` / `testRenameLocals`);
- reserved-name avoidance — a fresh short name never captures a referenced
  free global (upstream `testRenameLocalsWithNamesReservedForGlobals`);
- property names and non-computed object keys are never renamed (they are
  not variable references);
- soundness guards that upstream gets "for free" from full scope analysis
  but which we enforce explicitly: catch bindings are reserved, a name
  declared twice is skipped, single-char names are left alone.

### What we do NOT do yet (`#[ignore = "blocked on gap-NNN"]`)

- **gap-144** — rename **global** variables in this pass. closurec routes
  globals through `rename-globals`; upstream `RenameVars` renames them in
  one sweep (`testRenameGlobals`).
- **gap-145** — rename parameters/locals of **non-leaf** (nesting)
  functions (`testRenameNested`-style whole-program renaming).
- **gap-146** — rename **function declaration names** themselves
  (upstream shortens `function longName(){}` → `function a(){}`).
- **gap-147** — **frequency-biased** name allocation: upstream orders the
  generated names so the most-referenced variable gets the 1-char name
  (`testBias*`); ours allocates in declaration order.

Each ignored placeholder is pinned to a `gap-NNN` entry in
`code/specs/CLOC12-gaps.md`; running with `--include-ignored` measures
progress as those gaps close.
