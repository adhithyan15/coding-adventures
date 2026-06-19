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

Dev-deps: `coding-adventures-javascript-tokens`,
`coding-adventures-javascript-parser`, `coding-adventures-closure-emitter`,
`coding-adventures-type-sidecar`, `coding_adventures_correlation_vector`
(source → bridge → pass → emit roundtrip tests).
