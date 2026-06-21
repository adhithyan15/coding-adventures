# CLOC25 — drop a redundant `else` after a terminating `if` consequent

> **Status:** Shipped. The `fold-control-flow` pass removes the `else` from an
> `if` whose consequent unconditionally terminates, hoisting the `else` body
> into the enclosing block (when scope-safe). This is upstream Closure's
> `MinimizeExitPoints`. The `simple-else-hoist` fixture pins it at the CLI.

## Why this spec exists

A frequent pattern is an `if` whose consequent always exits, with an `else`
holding the fall-through case:

```js
function classify(n) {
  if (n < 0) { return negative(n); }
  else       { record(n); return positive(n); }
}
```

The `else` is redundant: when `n < 0` the function `return`s, so control reaches
the `else` body **only** when the test was false. Removing the `else` and
splicing its body in after the `if` is semantics-preserving and deletes the
`else` keyword and its braces:

```js
function classify(n) {
  if (n < 0) { return negative(n); }
  record(n);
  return positive(n);
}
```

Upstream Closure does exactly this (`MinimizeExitPoints`). CLOC25 ports a
conservative slice.

## Where it lives

In **`fold-control-flow`**, inside `fold_block_statement` — the pass that
already processes a `BlockStatement.body` as a statement list (dropping
dead-code-after-`return`, folding constant `if`s, etc.). The `else`-hoist needs
to splice statements into that enclosing list, so it belongs in exactly this
list-processing loop. (DCE owns *removing* code; this is control-flow
*minimization*, fold-control-flow's job.)

## The transform

For each folded statement `if (C) T else E` in a block body where:

1. **`T` definitely terminates** — `consequent_definitely_terminates(T)`: `T` is
   a `return`/`throw`, or a block whose **last** statement is. (We do not look
   inside nested `if`/loops/`try` — a conservative check; declining merely
   forgoes the optimization.)
2. **`E` is scope-safe to hoist** — `alternate_is_hoistable(E)`:
   * a block `E` is gated on `block_is_scope_safe_to_hoist` (no block-scoped
     `let`/`const`/`function` at its top level — those would leak or collide if
     moved out; plain `var` is function-scoped and hoists harmlessly);
   * a bare (non-block) tagged statement is safe (a bare lexical/`function`
     declaration can't be an unbraced `else` body in valid JS);
   * a bare `Declaration` `else` body is declined.

the pass rewrites it to `if (C) T` followed by the statements of `E`, and
records a `hoisted-else-after-terminator` contribution.

The trimmed `if (C) T` is **not** itself a terminator (a false test falls
through), so following statements stay reachable — except when the hoisted tail
itself ends in a `return`, in which case the existing
dead-code-after-terminator drop removes anything that followed the original
`if`.

## Soundness

* **Control flow.** `T` exits unconditionally on the true path, so the true
  path never reaches `E` or the code after the `if`; the false path runs `E`
  then the following code. That is identical before and after the rewrite.
* **Scope.** The only hazard is moving a block-scoped binding out of the `else`
  block. The `block_is_scope_safe_to_hoist` gate forbids exactly that, mirroring
  the same guard `closure-pass-dce` uses for block flattening.

## Interaction with the existing if-else→ternary fold

`fold_if_statement` already folds `if (x) return E1; else return E2;` to
`return x ? E1 : E2` (gap-017) — but only when **both** branches are a single
`return <expr>`. That fold runs first (via `fold_statement`), so when it
applies, the block-level `else`-hoist never sees an `if`-with-`else`. The
`else`-hoist covers the cases the ternary fold cannot: a `throw` consequent, a
multi-statement `else`, a non-`return` `else`.

## End-to-end oracle

* **`simple-else-hoist`** — `classify` (above) at SIMPLE emits
  `function classify(n){if(n < 0){return negative(n)}record(n);return positive(n)}`.
  Under `WHITESPACE_ONLY` the `else` survives verbatim, so the guard asserting
  the optimized output contains **no** `else` doubly proves the typed pipeline
  ran.

## Note: the grammar parser has no ASI

The grammar parser requires explicit statement terminators — it does **not**
implement automatic semicolon insertion. A block body like `{ a() }` (no `;`
before `}`) fails to parse and closurec degrades the whole program to
`WHITESPACE_ONLY`. Fixture inputs therefore use explicit semicolons. (ASI
support is a separate, larger frontend item.)
