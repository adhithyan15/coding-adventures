# coding-adventures-closure-pass-inline-variables

Constant-propagation pass for the Closure Compiler clone — Closure
Compiler's `InlineVariables` in miniature. Replaces references to a
top-level `const` bound to a literal with the literal itself, so the
binding can be deleted and downstream folding sees a concrete value.
Per [CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s
canonical pass set.

## What it does

```js
// before
const RATE = 2;
total = base * RATE;

// after inline-variables (the const is now unreferenced; remove-unused-vars
// deletes it, after which constant-fold can fold base*2 if base is known)
total = base * 2;
```

## Why only `const`, only literals

Propagating `X`'s value to a use site is sound only when that value is
the same at every use as at the declaration:

1. **`const`, never `let`/`var`.** A `const` can't be reassigned; a
   `let`/`var` could be written between declaration and use, so its
   initializer isn't a safe substitute.
2. **A literal, never an expression.** A literal is immutable.
   `const X = y;` isn't propagated (`y` could change); `const X = o.p;`
   isn't either (`o.p` may be a getter).

Plus the self-contained **shadow guard** the inline pass uses — the name
must be declared exactly once in the whole program — so every occurrence
of the identifier provably refers to this one `const`.

## The temporal dead zone

Scope resolution isn't enough: a `const` read *before* its declaration
line runs throws `ReferenceError` (even from a function called early), so
replacing that read with the inert literal would erase the throw. We
guard conservatively — a `const` is a candidate only when every top-level
item before it is **inert** (a function declaration, which only hoists,
or a variable declaration with literal initializers, which runs nothing),
and only single-declarator `const`s are taken. A block of constants at the
top of the file is fully covered; a `const` after any executable
statement is left alone.

## Single-use vs. multi-use

- **One use** → always propagate (the whole `const` declaration is
  overhead once its single use is gone).
- **N > 1 uses** → propagate only when the literal is short
  (`<= MAX_MULTIUSE_LITERAL_LEN`), so duplicating it doesn't outweigh
  deleting the declaration. A long string constant used in twenty places
  stays a `const`.

The pass only propagates; it leaves the emptied `const` for
`remove-unused-vars` to delete (mirroring how `inline` leaves dead
functions for `treeshake`).

## Where it sits

`depends_on = ["constant-fold"]` so a folded initializer
(`const X = 1 + 2` → `const X = 3`) is a literal when this pass looks.
It runs before `remove-unused-vars` (which clears the emptied
declaration) and feeds `constant-fold` on the next fixed-point sweep.

## Dependency whitelist

- `coding-adventures-closure-pass-pipeline` — `Pass` trait + types.
- `coding-adventures-javascript-ast` — `Program` and the typed AST.

Dev-deps: `coding-adventures-javascript-tokens`,
`coding-adventures-javascript-parser`, `coding-adventures-closure-emitter`
(source → bridge → pass → emit roundtrip tests),
`coding-adventures-closure-pass-constant-fold` (pipeline-ordering test),
`coding-adventures-type-sidecar`, `coding_adventures_correlation_vector`.
