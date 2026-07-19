# CLOC24 — strip `debugger` at SIMPLE/ADVANCED

> **Status:** Shipped. The `closure-pass-dce` pass removes `debugger;`
> statements from statement lists at the SIMPLE and ADVANCED compilation
> levels, matching the upstream Closure Compiler. The `simple-debugger`
> end-to-end fixture (CLOC21) is repurposed as the strip oracle.

## Why this spec exists

CLOC21 made `debugger` *representable* in the typed AST so a program containing
one would route through the optimization pipeline instead of degrading to
`WHITESPACE_ONLY`. But CLOC21 deliberately deferred actually *removing* the
statement — it survived verbatim. The upstream Closure Compiler **strips**
`debugger` statements at its optimization levels, because a `debugger` is a
development-only breakpoint: it pauses execution only when a debugger is
attached and is a no-op otherwise. Removing it from a shipped program is
therefore semantics-preserving, and it is a (small) size win. CLOC24 closes
that gap.

## Where the strip lives

In **`closure-pass-dce`**, not a new pass. Dead-code elimination already owns
"remove statements that don't contribute to the program's observable
behaviour" (empty statements, dead-after-terminator tails). A `debugger`
statement is the same shape of removable noise, so it belongs here.

Crucially, the dce pass runs **only inside the typed (SIMPLE/ADVANCED)
pipeline**. `WHITESPACE_ONLY` does token-stripping only and never constructs a
typed AST, so it never runs dce. Putting the strip in dce therefore gives the
exact upstream contract **for free**:

| Level            | `debugger;` |
|------------------|-------------|
| `WHITESPACE_ONLY`| preserved   |
| `SIMPLE`         | stripped    |
| `ADVANCED`       | stripped    |

## What it does

Two sweeps, one per statement-list context:

1. **Block bodies** (`dce_block_statement`) — after the existing
   empty-statement sweep, `working.retain(|s| !is_debugger_statement(s))`.
   - `{ x; debugger; y; }` → `{ x; y; }`
   - `{ debugger; debugger; }` → `{}`
2. **Program top level** (`dce_program`) — the program body is a `Vec<ProgramItem>`,
   not a `BlockStatement`, so it needs its own sweep:
   `new_body.retain(|item| !is_debugger_program_item(item))`.

Each sweep records a `removed-debugger` contribution (which flips the pass's
`changed` flag) when it drops at least one statement.

## Soundness

Removing a `debugger` statement changes observable behaviour **only** under an
attached debugger — exactly the development-time artifact upstream Closure
strips intentionally. For a shipped program the transform is a no-op on output,
so it is unconditionally safe at optimization levels. (Unlike dead-code-tail
truncation, there is no hoisting concern: a `debugger` binds nothing.)

## Documented limitation

The sweep is **list-scoped**: it removes a `debugger` that appears directly in a
block body or the program body. A `debugger` reaching a *non-list* position —
e.g. a brace-less `if (c) debugger;` consequent — is preserved, because removing
it would leave a dangling single-statement slot that would have to be rewritten
to `;`/`{}`. This mirrors how the empty-statement sweep is also list-scoped, and
the bare-consequent shape is rare. Handling it (rewriting the slot to an empty
statement) is future work.

## End-to-end oracle

The CLOC21 **`simple-debugger`** fixture is repurposed. Input:

```js
function log(p) { report(p); }
log(1);
var x = 1 + 2;
debugger;
use(x);
```

At SIMPLE: `1 + 2` folds to `3`, the top-level `debugger;` is **stripped**, and
`function log` is **KEPT** — SIMPLE is open-world, so it never inlines or
deletes an observable top-level name (`log` could be called by another script
sharing the page); that inline runs only at closed-world ADVANCED:

```text
function log(p){report(p)};log(1);var x=3;use(x);
```

(ADVANCED still inlines `log` into `report(1)` and drops the declaration,
giving `report(1);var x=3;use(x);`.)

The whitespace-fallback regression guard now asserts `debugger` is **absent**:
a `WHITESPACE_ONLY` fallback would have preserved it, so its absence is a second
independent proof that the typed pipeline ran.

## Relationship to the Phase-2 statement campaign

CLOC20–23 made the four common bridge-unsupported statements (`do`/`while`,
`debugger`, `for`-`in`, `for`-`of`) representable so programs using them get
optimized instead of degrading. CLOC24 is the first *real optimization* built on
top of that representability work — turning `debugger` from "preserved so the
rest optimizes" into "actually removed, like upstream". It was the explicit
follow-up deferred from CLOC21.
