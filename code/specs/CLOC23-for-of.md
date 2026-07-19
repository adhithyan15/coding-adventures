# CLOC23 — `for`-`of` end-to-end

> **Status:** Shipped. AST node, parser bridge, emitter, scope analyzer, and
> every optimization pass handle the `for`-`of` loop. An end-to-end diff fixture
> (`simple-for-of`) pins the behaviour at the CLI. With this, every common
> Phase-2 statement is representable; only `with` remains bridge-unsupported.

## Why this spec exists

`for (left of right) body` iterates the values of an iterable. It was the last
common bridge-unsupported Phase-2 statement: the grammar parsed it, but the
typed AST had no node, so the bridge declined (`UnsupportedSyntax`) and the CLI
fell back to `WHITESPACE_ONLY` for *any* program using a for-of loop. CLOC23
closes that gap.

## Relationship to CLOC22 (`for`-`in`)

`for`-`of` is **structurally identical** to `for`-`in` — only the keyword (`of`
vs `in`) and the runtime iteration protocol differ. This spec therefore mirrors
[CLOC22](CLOC22-for-in.md) exactly; only the differences are called out here.
See CLOC22 for the full design of the shared shape (the `ForInit` left, the
loop-variable-as-binding soundness, the not-a-terminator property, and the
per-pass recursion).

## The AST node

```rust
pub struct ForOfStatement {
    pub cv: Option<CvId>,
    pub left: ForInit,        // VariableDeclaration | Expression
    pub right: Expression,
    pub body: Box<Statement>,
}
```

Identical fields to `ForInStatement`. (They are distinct Rust types — even though
the fields match, a single match arm cannot bind both via an or-pattern, so each
pass carries a separate, identical `ForOfStatement` arm.)

## Differences from `for`-`in`

1. **Bridge keyword + `using` decline.** `convert_for_of_statement` mirrors
   `convert_for_in_statement` but phase-splits on the `of` token. The for-of
   grammar additionally admits a `using` binding declaration
   (`for (using x of it)`); `using` is **not** modelled, so the bridge scans for
   a `using` token and declines (graceful WHITESPACE_ONLY fallback). As in
   for-in, destructuring lefts and any other unrepresentable binding decline
   gracefully rather than hard-erroring.
2. **Emitter keyword.** `emit_for_of` writes `for ( <left> of <right> ) <body>`
   — identical to `emit_for_in` with `of` in place of `in`, spaced on both sides
   for the same token-separation reason.
3. **`for await (… of …)`** is a *distinct* grammar production
   (`for_await_of_statement`) and remains bridge-unsupported (declines).

Everything else — the `ForInit` left handling, the loop-variable rename
consistency across `rename`/`rename-globals`, the not-a-terminator treatment in
`dce`/`fold-control-flow`, and the per-pass recursion into left/right/body — is
the same as CLOC22 and verified the same way.

## Soundness (loop variable)

For `for (var/let/const v of it)`, the rename passes treat the `left` binding
exactly like a for-loop init binding, so `v` renames consistently at its
declaration and every body use. Verified end-to-end:
`for (var entry of values)` with `sum + entry` ⟶ `for (var c of a)` with
`b + c` under ADVANCED. The `for (v of it)` expression-left form is a use only,
never a declaration.

## End-to-end oracle

* **`simple-for-of`** — at SIMPLE: arithmetic inside the for-of body folds, the
  loop survives verbatim, and the statement after the loop stays reachable.
  `function log` is KEPT — SIMPLE is open-world and never inlines or removes a
  top-level name (that inline is ADVANCED-only). A companion assertion proves
  the output is NOT the whitespace fallback (the `1 + 2` ⇒ `3` fold in the loop
  body can only come from the typed pipeline).

## Phase-2 statement campaign status

With for-of, the representable Phase-2 statements are complete:
`DoWhileStatement` (CLOC20), `DebuggerStatement` (CLOC21), `ForInStatement`
(CLOC22), and `ForOfStatement` (CLOC23) join the Phase-1 set. The only remaining
bridge-unsupported statement is **`with`**, which is intentionally *not* a target:
its dynamic scope makes renaming inside a `with` body unsound, so it is left to
decline to WHITESPACE_ONLY. `using`/`await using` declarations and
`for await (… of …)` also remain out of scope.
