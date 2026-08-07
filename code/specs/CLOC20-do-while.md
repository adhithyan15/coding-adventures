# CLOC20 — `do` / `while` end-to-end

> **Status:** Shipped. AST node, parser bridge, emitter, scope analyzer, and
> every optimization pass handle the `do`-`while` loop. An end-to-end diff
> fixture (`simple-do-while`) pins the behaviour at the CLI.

## Why this spec exists

`do { … } while (test)` was the next Phase-2 statement gap after CLOC19's
try/catch. The grammar already *parsed* a do-while, but the typed AST had no node
to represent it, so the parser→typed-AST **bridge** declined
(`UnsupportedSyntax`) and the CLI fell back to **`WHITESPACE_ONLY`** — emitting
the source with only inter-token whitespace stripped, applying **zero** real
optimization. CLOC20 closes that gap with the exact playbook CLOC19 established:
make the statement representable, then recurse every pass into it.

```text
source ──parse──▶ grammar AST ──bridge──▶ typed Program (DoWhileStatement)
       ──passes──▶ optimized Program ──emit──▶ JS text
```

## The AST node (`coding-adventures-javascript-ast`)

```rust
pub struct DoWhileStatement {
    pub cv: Option<CvId>,
    pub body: Box<Statement>,
    pub test: Expression,
}
```

It is the mirror of `WhileStatement`, with one deliberate difference: the field
order is `body` before `test`, following **execution order**. A `do`-`while`
runs its body *at least once* before the test is first evaluated — the single
most important semantic distinction from `while`, and the one that drives the
soundness notes below. A new `TaggedStatement::DoWhileStatement` variant and a
`Statement::do_while_statement` constructor expose it.

## The bridge (`coding-adventures-javascript-parser`)

`do_while_statement` routes to `convert_do_while_statement`. The grammar
production is `do statement while ( expression )`, so the Node children (tokens
filtered out) are `[statement, expression]` — body first, test second (the
mirror of `while_statement`, which is `[expression, statement]`).

## The emitter (`coding-adventures-closure-emitter`)

`emit_do_while` writes `do <body> while ( <test> ) ;`:

* **`do`→body separation.** `do` is a keyword sitting directly before the body.
  `do{…}` lexes cleanly when the body is a block, but a bare-statement body
  would glue (`do foo()` must not become `dofoo()`), so a required space is
  inserted **only when the body is not a block**.
* **Trailing `;`.** The `while ( test )` tail does not end in `}` or `;`, so the
  statement emits an explicit terminator `;`. That `;` is a *real* terminator
  (ASI can supply it before a closing `}`), so `DoWhileStatement` is added to
  `last_stmt_uses_terminator_semi`: as the last statement in a block its `;` is
  popped (`{do{a}while(b)}`), exactly like `return`/`throw`/expression
  statements — but unlike plain `while`, whose trailing `;` is a body slot that
  must be kept.

## Soundness: a do-while is not eliminable and not a terminator

Two invariants, both flowing from "the body runs at least once":

1. **No dead-loop elimination.** `fold-control-flow` removes a `while (false){…}`
   because its body never runs. A `do {…} while (false)` runs the body exactly
   once, so it can **never** be eliminated that way — the do-while arm only
   recurses structurally (folds body + test), it never elides.
2. **Not a terminator.** Like `while`, control can fall out of a do-while
   (body runs, test fails, execution continues), so DCE keeps statements *after*
   the loop reachable and never adds a do-while to the dead-after-terminator
   set. In the dead-tail-truncation whitelist (`tail_is_safe_to_truncate`), a
   do-while — a compound statement that can wrap a hoisted `var` — defaults to
   **unsafe (preserved)**.

Beyond those, a do-while introduces no binding of its own (unlike a catch
parameter), so the renaming passes need no special reservation — they just
recurse into body and test.

### Per-pass handling

| Pass | What it does for `DoWhileStatement` |
|------|--------------------------------------|
| `constant-fold` | recurse fold into body + test |
| `fold-control-flow` | recurse fold into body + test; **never** elide (body runs once) |
| `dce` | recurse DCE into body + test; not a terminator |
| `inline-variables` | recurse count/use/propagate into body + test |
| `inline` | recurse all phases (count/tally/inline/splice/collect) into body + test |
| `rename` | recurse process/collect/rewrite into body + test |
| `rename-globals` | recurse count/collect/apply into body + test |
| `rename-properties` | recurse classify/rewrite into body + test |

The scope analyzer (`closure-scope-analyzer`) walks the body and test in the
current scope (a do-while introduces no new scope, same as `while`).

## End-to-end oracle (`closurec` diff fixture)

* **`simple-do-while`** — at SIMPLE: arithmetic inside the do-while body folds,
  the loop survives verbatim, and the statement after the loop stays reachable.
  `function log` is KEPT — SIMPLE is open-world and never inlines or removes a
  top-level name (that inline is ADVANCED-only). A companion assertion proves
  the output is NOT the whitespace fallback (the `1 + 2` ⇒ `3` fold in the loop
  body can only come from the typed pipeline).

## Out of scope (future Phase-2 work)

`ForInStatement`, `ForOfStatement`, `WithStatement`, and `DebuggerStatement`
remain bridge-unsupported (they decline to `WHITESPACE_ONLY`). They follow the
same playbook and are natural standalone follow-ups.
