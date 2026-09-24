# CLOC21 — `debugger;` end-to-end

> **Status:** Shipped (conservative v1: make representable, preserve the
> statement). AST node, parser bridge, emitter, scope analyzer, and every
> optimization pass handle the `debugger` statement. An end-to-end diff fixture
> (`simple-debugger`) pins the behaviour at the CLI.

## Why this spec exists

`debugger;` is a breakpoint hook: it pauses execution if a debugger is attached,
and does nothing if one is not. (That is **not** the same as "is a no-op" —
whether a debugger is attached is not known at compile time, so the statement is
observable and a pass may not drop it for size. An earlier revision of this line
used the "otherwise a no-op" phrasing, and CLOC24 was built on it.) It was a Phase-2 statement gap. The grammar already
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

> **Retracted 2026-09-23 (CCR-053).** The paragraph below claimed upstream
> Closure removes `debugger` at SIMPLE and ADVANCED. Measured against the
> pinned oracle (`closure-compiler-v20260915`), that is false: upstream
> **keeps** a reachable `debugger` at SIMPLE, and removes one only as
> collateral when the statement enclosing it goes — after a `return` or a
> `throw`, or inside a folded-away `if (false)`. At ADVANCED the rule is
> narrower rather than absent: upstream additionally eliminates a call whose
> body is *only* a `debugger` (`function f(){debugger}f();` → nothing), which
> is call-elimination treating the body as pure rather than a `debugger` sweep.
> We do not do that, so do not read "upstream keeps it" as covering ADVANCED
> unconditionally. The follow-up it anticipated
> (CLOC24) was therefore built on a false premise, and has been reverted; see
> `CLOC24-strip-debugger.md`, which carries the full rebuttal. Preserving
> `debugger` — what this spec actually shipped — was the correct behaviour all
> along.

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

A `debugger;` has no children, binds nothing, and references nothing, so most
passes carry it through unchanged. **It is not interchangeable with
`EmptyStatement`, and an earlier revision of this paragraph said it was**: `dce`
sweeps a stray `EmptyStatement` out of a statement list and deliberately does
*not* sweep a `DebuggerStatement`, because an empty statement is a genuine
no-op and a `debugger` is observable (CCR-053). It is grouped with the other childless leaf statements in each
pass's statement match (`constant-fold`, `fold-control-flow`, `dce`,
`inline-variables`, `inline`, `rename`, `rename-globals`, `rename-properties`)
and in the scope analyzer. These arms exist only to keep the matches exhaustive
over the new AST variant — the fail-closed property of the compiler-driven
design.

## End-to-end oracle (`closurec` diff fixture)

* **`simple-debugger`** — at SIMPLE: surrounding arithmetic folds and
  `function log` is KEPT — SIMPLE is open-world and never inlines or removes a
  top-level name (that inline is ADVANCED-only). The `debugger;` statement is
  **preserved verbatim**, as it has been since CLOC21. CLOC24 briefly made it
  stripped; CCR-053 reverted that on 2026-09-23 after measuring the oracle, so
  the fixture is back to the CLOC21 behaviour described here. A companion
  assertion proves the output is NOT the whitespace fallback — the `1 + 2` ⇒ `3`
  fold is now the only signal, since the `debugger;` no longer distinguishes the
  two paths (both keep it).

## Out of scope (future work)

* ~~**Stripping `debugger`** at SIMPLE/ADVANCED to match upstream Closure — a
  focused follow-up (a behaviour change, cleanly separable from this PR).
  **Delivered in CLOC24.**~~ **Withdrawn (CCR-053, 2026-09-23):** upstream does
  not strip `debugger`, so there is nothing here to match. CLOC24 shipped and
  has been reverted.
* `ForInStatement`, `ForOfStatement`, and `WithStatement` remain the last
  bridge-unsupported Phase-2 statements; they follow the same playbook (with
  more involved left-binding handling for the for-in/of forms).
