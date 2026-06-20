# Changelog

All notable changes to the `coding-adventures-closure-pass-rename-globals` crate will be documented in this file.

## [0.5.0] - 2026-06-20

### Added — CLOC22: global renaming across `for`-`in`

`count_decl_names_stmt`, `collect_all_idents_stmt`, and `rename_apply_tagged`
recurse through `ForInStatement` (left / right / body), mirroring the
for-statement handling so the loop variable and uses inside the body rename
consistently.

## [0.4.0] - 2026-06-20

### Added — CLOC21: handle `DebuggerStatement`

Every phase of the pass now covers `DebuggerStatement` (grouped with the other
childless leaf statements) as a no-op. Added to keep the matches exhaustive over
the new AST variant.

## [0.3.0] - 2026-06-20

### Added — CLOC20: global renaming across `do`/`while`

`count_decl_names_stmt`, `collect_all_idents_stmt`, and `rename_apply_tagged`
recurse through `DoWhileStatement` (loop body and test), mirroring the existing
`while` handling.

## [0.2.0] - 2026-06-20

### Added — CLOC19: global renaming across `try`/`catch`/`finally` (catch-param soundness)

`count_decl_names_stmt`, `collect_all_idents_stmt`, and `rename_apply_tagged`
recurse through `TryStatement`. As in the local renamer, the catch `param` is
counted as a declared binding and added to the avoid set, so a global rename can
never produce a name that collides with a catch binding and the param itself is
never rewritten.

## [0.1.0] - 2026-06-18

### Added (CLOC13.I — aggressive top-level / global renaming)

New crate per CLOC06's canonical pass set — the **ADVANCED**-level complement to
`closure-pass-rename` (which only shortens the *locals* of leaf functions). It
renames program-private top-level names (`function` / `var` / `let` / `const`)
to short identifiers (`a`, `b`, …) at their declaration and every use site:

```js
function computeTotal(items) { return items; }
var result = computeTotal(list);
// => function a(items){return items} var b=a(list)
```

- **ADVANCED-only by construction.** In a script, a top-level name is part of
  the program's public surface, so renaming it is sound only under Closure's
  whole-program / `--externs` contract: everything externally visible is
  declared in the externs; anything else is private and may be shortened.
  `RenameGlobalsPass::new(do_not_rename: HashSet<String>)` takes that externs
  boundary; `with_no_externs()` is the pure whole-program form.
- **Soundness** (self-contained name-based analysis, same guard as `inline` /
  `inline-variables`): a top-level binding is renamed only when its name is
  **declared exactly once in the whole program** (so every use resolves to it —
  a sound α-conversion), is **not in the do-not-rename set**, and is longer than
  one character. Free globals (`console`, `window`, …) have no declaration here,
  so they are never candidates. The fresh name **avoids every identifier
  anywhere in the program** (declarations, uses, property names, free globals)
  and every externs name, so it can neither collide with another binding (incl.
  a function-local of the same letter) nor capture a free global. Property names
  (non-computed `.x` / object keys) are never rewritten; computed `o[x]` is.
- `name = "rename-globals"`, `depends_on = []`, `iteration_policy = OneShot`,
  `cost = 3`.

### Tests
- 16 tests: metadata contract + source → bridge → rename-globals → emit
  roundtrips covering top-level function/var renaming, use sites inside function
  bodies, the externs do-not-rename set, free-global and property-name
  preservation, computed-member use, single-char skip, shadowed-name skip, and
  fresh-name collision avoidance against a function-local.
