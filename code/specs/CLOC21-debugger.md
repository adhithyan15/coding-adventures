# CLOC21 — `debugger;` end-to-end

> **Status:** Shipped (conservative v1: make representable, preserve the
> statement). AST node, parser bridge, emitter, scope analyzer, and every
> optimization pass handle the `debugger` statement. An end-to-end diff fixture
> (`simple-debugger`) pins the behaviour at the CLI.

## Why this spec exists

`debugger;` is a breakpoint hook: it pauses execution if a debugger is attached
and is otherwise a no-op. It was a Phase-2 statement gap. The grammar already
*parsed* it, but the typed AST had no node to represent it, so the
parser→typed-AST **bridge** declined (`UnsupportedSyntax`) and the CLI fell back
to **`WHITESPACE_ONLY`** — applying zero real optimization to *any* program that
contained a `debugger` statement (common in development builds). CLOC21 closes
that gap with the established playbook: make the statement representable, then
let every pass carry it through.

```text
source ──parse──▶ grammar AST ──bridge──▶ typed Program (DebuggerStatement)
       ──passes──▶ optimized Program ──emit──▶ JS text
```

## Scope of v1: make representable, preserve the statement

This change makes `debugger` representable and **preserves** it verbatim. The
value is that the *rest* of a program containing `debugger` now gets the full
SIMPLE/ADVANCED optimization pipeline instead of degrading to whitespace-only.

The upstream Closure Compiler **removes** `debugger` statements at SIMPLE and
ADVANCED. Stripping is intentionally deferred to a focused follow-up: it is a
behaviour change (it removes a debugging affordance) and is cleanly separable
from "make representable". Until then, v1 never regresses — a program that kept
its `debugger` under the whitespace fallback still keeps it, but now everything
around it is optimized.

## The AST node (`coding-adventures-javascript-ast`)

```rust
pub struct DebuggerStatement {
    pub cv: Option<CvId>,
}
```

A childless leaf, structurally identical to `EmptyStatement`. A new
`TaggedStatement::DebuggerStatement` variant and a `Statement::debugger_statement`
constructor expose it. ESTree wire format: `{ "type": "DebuggerStatement" }`.

## The bridge (`coding-adventures-javascript-parser`)

The grammar production is `debugger_statement = "debugger" SEMICOLON` — no node
children — so `convert_statement` maps it directly to a bare `DebuggerStatement`
marker (no child conversion). It is removed from the unsupported arm.

## The emitter (`coding-adventures-closure-emitter`)

`emit_debugger` writes `debugger;`. The keyword is followed only by its
terminator `;` (or, once that `;` is popped, a `}`/EOF), so no token-separation
handling is needed. The trailing `;` is a real statement terminator, so
`DebuggerStatement` is added to `last_stmt_uses_terminator_semi`: as the last
statement in a block its `;` is popped (`{debugger}`) — ASI re-supplies it —
exactly like `return`/`throw`/expression statements.

## Per-pass handling

A `debugger;` has no children, binds nothing, and references nothing, so every
pass treats it exactly like `EmptyStatement`: a no-op that is carried through
unchanged. It is grouped with the other childless leaf statements in each
pass's statement match (`constant-fold`, `fold-control-flow`, `dce`,
`inline-variables`, `inline`, `rename`, `rename-globals`, `rename-properties`)
and in the scope analyzer. These arms exist only to keep the matches exhaustive
over the new AST variant — the fail-closed property of the compiler-driven
design.

## End-to-end oracle (`closurec` diff fixture)

* **`simple-debugger`** — at SIMPLE: surrounding arithmetic folds and
  `function log` is KEPT — SIMPLE is open-world and never inlines or removes a
  top-level name (that inline is ADVANCED-only). The `debugger;` statement was
  preserved verbatim as of CLOC21; CLOC24 later made it **stripped** at
  SIMPLE/ADVANCED, which is the current fixture behavior. A companion assertion
  proves the output is NOT the whitespace fallback (the `1 + 2` ⇒ `3` fold, and
  post-CLOC24 the `debugger;` strip, can only come from the typed pipeline).

## Out of scope (future work)

* **Stripping `debugger`** at SIMPLE/ADVANCED to match upstream Closure — a
  focused follow-up (a behaviour change, cleanly separable from this PR).
  **Delivered in CLOC24.**
* `ForInStatement`, `ForOfStatement`, and `WithStatement` remain the last
  bridge-unsupported Phase-2 statements; they follow the same playbook (with
  more involved left-binding handling for the for-in/of forms).
