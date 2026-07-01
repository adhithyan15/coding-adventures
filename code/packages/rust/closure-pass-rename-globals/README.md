# coding-adventures-closure-pass-rename-globals

Aggressive top-level (global) renaming pass for the Closure Compiler
clone — the **ADVANCED**-level complement to
[`closure-pass-rename`](../closure-pass-rename), which only shortens the
*locals* of leaf functions. This pass shortens program-private *top-level*
names. Per [CLOC06](../../../specs/CLOC06-pass-interface-contract.md).

## What it does

```js
// before  (SIMPLE leaves these alone — they might be external)
function computeTotal(items) { return items; }
var result = computeTotal(list);

// after rename-globals  (ADVANCED, nothing external)
function a(items) { return items; }
var b = a(list);
```

## Why ADVANCED-only

In a script, a top-level `function`/`var`/`let`/`const` is part of the
program's public surface (another script can read it off the global
object / shared top-level scope). Renaming it is sound only under
Closure's whole-program / `--externs` contract: *everything externally
visible is declared in the externs*; anything else is private and may be
shortened. SIMPLE makes no such assumption and never touches top-level
names — which is exactly why ADVANCED output is smaller. The pass takes
the externs boundary as a **do-not-rename set**:

```rust
RenameGlobalsPass::new(externs_names)   // protect these
RenameGlobalsPass::with_no_externs()    // pure whole-program
```

## Soundness

A top-level binding is renamed only when:

1. it is declared at the top level (free globals have no declaration → never touched);
2. its name is **declared exactly once in the whole program** — so every use resolves to it, a sound α-conversion (the self-contained guard `inline` / `inline-variables` also use);
3. it is **not in the do-not-rename set**; and
4. its name is longer than one character.

The fresh short name **avoids every identifier anywhere in the program**
plus every externs name, so it can't collide with another binding (incl.
a function-local of the same letter) or capture a free global. Property
names (non-computed `.x` / object keys) are never rewritten; computed
`o[x]` is. Names also bound elsewhere (a parameter, a local) are declared
more than once → skipped.

## Composition with `closure-pass-rename`

The two renamers touch disjoint names: `rename` shortens leaf-function
parameters/locals; this pass shortens top-level names. (A name bound both
top-level and locally is declared more than once, so neither renames it.)

## Dependency whitelist

- `coding-adventures-closure-pass-pipeline` — `Pass` trait + types.
- `coding-adventures-javascript-ast` — `Program` and the typed AST.
- `coding_adventures_correlation_vector` — the `Contribution` type for
  rename provenance (see below).
- `serde_json` — `Contribution.meta` JSON values.

Dev-deps: `coding-adventures-javascript-tokens`,
`coding-adventures-javascript-parser`, `coding-adventures-closure-emitter`,
`coding-adventures-type-sidecar` (source → bridge → pass → emit roundtrip
tests).

## Rename provenance (correlation vector)

Renaming is a transformation, not a deletion, so this pass records each
global rename as a `renamed` **contribution** carrying `{from, to}`
(rather than tombstoning, as the DCE / fold-control-flow / treeshake
passes do for what they delete). The pipeline attaches these to the
program-root CV entry, so a `--correlation_vector` consumer can map a
minified global (`a`) back to its original source name (`longName`) —
the rename *table* as queryable provenance. Program output is
byte-for-byte unchanged. (Follow-up: per-output-span provenance —
contributing to each renamed identifier's own CV id — needs the log
threaded through the `rename_apply_*` recursion.)

## Upstream conformance tests

`tests/upstream/rename_vars_test.rs` ports Google Closure Compiler's
`RenameVarsTest` (Apache-2.0; see `ATTRIBUTION.md` and `UPSTREAM_SHA`), per the
CLOC12 test-port convention. Because the pass exposes a source-string surface
through public crate APIs, the port drives the real `source → bridge → rename →
emit` chain and asserts on the emitted string — the same `test(js, expected)`
surface upstream uses. It pins the 8 global-renaming behaviors this pass
supports today and records 4 upstream behaviors it does not — function-local
renaming, parameter renaming, short-name reuse across disjoint scopes, and
pseudo-name mode — as `#[ignore = "blocked on gap-NNN"]` placeholders tied to
`code/specs/CLOC12-gaps.md` (gap-134 … gap-137). Run with
`cargo test --test upstream_rename_vars` (add `-- --include-ignored` to list the
pending gaps).
