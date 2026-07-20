# coding-adventures-maple-runtime

Evaluates Maple (a subset) by lowering the `maple-parser` (MP-3) CST into
[`symbolic-ir`](../symbolic-ir) and running it through
[`symbolic-vm`](../symbolic-vm)'s shared `SymbolicBackend` — the *same*
rewrite engine Wolfram, Macsyma, Derive, and Reduce already drive,
unchanged. See
[`code/specs/MA09-maple-language.md`](../../../specs/MA09-maple-language.md).

## Where this fits

`maple-runtime` is MP-4 of the Maple frontend/runtime pipeline:

```
maple.tokens + maple-lexer     (MP-2)
        │
maple.grammar + maple-parser   (MP-3)
        │
maple-runtime                  (MP-4, this crate) ── crate::lower ──► symbolic_ir::IRNode
        │                                              │
        │                                       symbolic_vm::VM (SymbolicBackend)
        │                                              │
maple-repl                     (MP-4)  ◄── crate::printer ─┘
```

## No custom `Backend` — verified against the real handler table

`SymbolicBackend` (built with `simplify: true`) already provides arithmetic
(`Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg`), comparison (`Equal`/`Less`/`Greater`/
`LessEqual`/`GreaterEqual`/`NotEqual`), logic (`And`/`Or`/`Not`), the held
`Assign`/`Define`/`If` forms, `List`, and (since `SymbolicBackend::new`
always builds with `simplify: true`) `D`/`Integrate` — so this crate adds
**no new evaluation code**, only:

- **`lower`** — Maple's surface `GrammarASTNode` (`statement`, `if_expr`,
  `assignment`, `arrow_def`, `logical_or`, `comparison`, `additive`,
  `postfix`, `atom`, …) → the canonical `IRNode` heads the VM dispatches,
  including the `diff`→`D`/`int`→`Integrate` calculus bridge, the arrow-
  operator (`f := (x, y) -> e`) → `Define` bridge, the `;`/`:` display-flag
  split (tracked *alongside* the IR, never folded into it — MA09 §3's own
  table calls this "a display flag on the surrounding session, not an IR
  node"), and the `elif`-chain → nested-`If` desugar.
- **`printer`** — the inverse: canonical `IRNode` → Maple surface notation
  (infix `+`/`-`/`*`/`/`/`^`, `and`/`or`/`not`, square-bracket `[a, b, c]`
  lists, curly-brace `{a, b, c}` sets, `if ... then ... [elif ...] [else
  ...] end if`, with a nested-`If` in the else-slot folded back into an
  `elif` rather than a second, redundant `if ... end if`).

**`Set` — a canonical head new to this repo, and a disclosed gap.** Maple is
the first language here with two distinct bracketed aggregate literals:
`[a, b, c]` (`List`, already fully handled) and `{a, b, c}` (MA09 §3/§5).
Grepping `symbolic_vm::handlers::build_handler_table` confirms **no**
handler exists for a `Set` head — unsurprising, no language before Maple has
asked for one. `Set` is defined *locally to this crate*
(`coding_adventures_maple_runtime::SET`), exactly the way `reduce-runtime`
defines its own new `CompoundExpression`/`Cons`/list-accessor heads rather
than adding them to the shared `symbolic-ir`/`symbolic-vm` crates. `Set` is
not a held head, so its elements *do* evaluate — but the call itself stays
structurally correct, unevaluated: real Maple's unordered/duplicate-
removing set semantics aren't enforced yet.

```rust
use coding_adventures_maple_runtime::MapleSession;

let mut s = MapleSession::new();
// Elements evaluate; the call stays a structurally-correct, unevaluated
// Set(...) -- real Maple's dedup/unordered semantics aren't enforced yet.
assert_eq!(s.feed("{1, 1, 2};\n").unwrap(), "{1, 1, 2}\n");
```

`diff`/`int` are thin calls into the already-shared `D`/`Integrate`
handlers — the same ones Derive's `DIF`/`INT` and Wolfram's own
`D`/`Integrate` call. This crate reimplements no calculus at all.

## Usage

```rust
use coding_adventures_maple_runtime::MapleSession;

let mut s = MapleSession::new();
assert_eq!(s.feed("x := 5;\n").unwrap(), "5\n");
assert_eq!(s.feed("x + 1;\n").unwrap(), "6\n");

// The arrow operator is Maple's general-purpose function definition (MA09
// §1/§3) -- NOT `f(x) := e` (that's real Maple's narrower remember-table
// spelling, deliberately excluded, and doesn't even parse in this subset).
s.feed("f := x -> x*x;\n").unwrap();
assert_eq!(s.feed("f(5);\n").unwrap(), "25\n");

// `:` suppresses the displayed line but the side effect still happens --
// MA09 §3's own statement-separator row.
assert_eq!(s.feed("y := 10:\n").unwrap(), "");
assert_eq!(s.feed("y;\n").unwrap(), "10\n");
```

`coding_adventures_maple_runtime::eval(src)` is a one-shot convenience for
callers that don't need a persistent session. Like `reduce-runtime`'s
`Output` (and unlike Derive's/Wolfram's `#n:`/`In[n]:=` numbered worksheet),
Maple's own session transcript has no numbered-input convention either
(MA09 §2/§5), so every result line here is unprefixed plain text.

## Why `Set` differs from `List` at the IR level

Both `[a, b, c]` and `{a, b, c}` are, syntactically, just "bracket +
comma-separated element list" — the same production shape every sibling
CAS-family grammar in this repo already implements for its own single
aggregate type. What makes them genuinely different is *evaluation*
semantics, not syntax: `List` preserves order and duplicates (`List`'s
handler is a pure passthrough once its elements are evaluated), while real
Maple's `Set` is unordered with duplicates silently removed (`{x, y, y}`
and `{y, x, y}` both produce `{x, y}`, per the Maple Help page's own worked
example). Lowering both to the *same* head (`List`) would erase that
distinction at the very first opportunity a future handler would need it —
so `Set` gets its own canonical head from day one, even though no handler
exists yet to enforce its semantics, so the *shape* is right for whenever
that handler lands.

## Why the arrow operator bridges to `Define`

Maple's `f(x) := expr` — which is *exactly* Reduce's/Derive's own general
function-definition spelling — means something narrower in real Maple: a
remember-table patch onto an *already-existing* procedure (MA09 §1),
confirmed against the Maple `remember` Help page ("you will not be able to
substitute into it ... the way a real function normally would"). Real
Maple's actual general-purpose definition mechanism is the arrow/functional
operator, `f := (x, y) -> e` / `f := x -> e` (Help page
`operators/functional`) — so *that* spelling, not `f(x) := e`, is this
subset's bridge to the canonical `Define[name, List[params...], body]`
head every CAS-family sibling here already reuses for its own (differently
spelled) general-definition idiom. `maple-parser`'s own grammar enforces
this at the syntax level too: `f(x) := expr` fails to *parse* at all in
this subset (its `assignment` left-hand side is a bare `NAME` token, not a
general call-shaped expression), so no Maple program here can accidentally
mean the narrower remember-table thing.

## Robustness

`feed`/`eval_to_outputs` are the trust boundary for arbitrary Maple source.
Two independent deep-recursion vectors are closed (see the crate doc
comment for the full rationale):

1. **Deeply nested source** (parenthesised, list/set-literal nesting,
   `not`/unary-minus prefix chains, a flat `^` chain, or nested `if`/`end
   if`/`fi`) — already rejected by `maple-parser`'s own `MAX_RULE_DEPTH`.
2. **A long flat chain that folds into a deeply nested lowered tree** —
   two shapes: `additive`/`multiplicative` (the same vector
   `reduce-runtime`/`derive-runtime` already guard), and, genuinely new to
   this grammar, a long `elif` chain (`lower_if` folds it into nested
   `If`s). `MAX_STATEMENT_TOKENS` (measured against the real
   `maple-lexer` token stream, reset **unconditionally** on every
   `SEMI`/`COLON`) closes both. Unlike `reduce-runtime`'s own guard, no
   bracket-nesting-depth tracking is needed here — verified directly
   against `maple-parser`'s own compiled grammar that `SEMI`/`COLON` are
   referenced in exactly one place (`statement_line`'s own terminator), so
   every occurrence in a valid token stream is unambiguously a genuine
   top-level statement boundary (this subset has no bare compound-statement
   grouping construct the way REDUCE's `<< ... >>` is — MA09 §4 defers bare
   expression sequences entirely).

Evaluation itself runs on a worker thread with a large bounded stack inside
`catch_unwind`, so a reused-handler panic (e.g. a wrong-arity `diff(x)`
call) becomes a clean `Err` and the session is rebuilt rather than left
corrupted. Maple's grammar-enforced bare-`NAME` `Assign`/`Define`
left-hand side means, unlike Reduce's, there is no malformed-`Assign`-lhs
panic vector at all in this crate.

## Out-of-scope constructs are rejected at parse time (MA09 §4)

`proc(...) ... end proc` (block-structured procedures) and `for`/`while`
loops have no grammar production at all in `maple.grammar` — `proc`,
`for`, `while`, `do` are ordinary `NAME` tokens, not reserved words, so
`proc(x) x^2 end proc` parses only as far as the ordinary call `proc(x)`
before failing to find a statement terminator, and `for i from 1 to 10 do
...` parses only as far as the bare symbol `for`. Both surface as an
ordinary parse `Err`, forwarded as-is — no special-casing needed in this
crate at all, exactly like `reduce-runtime`'s identical "a parse error is
returned, not panicked" contract.

## Tests

```sh
cargo test -p coding-adventures-maple-runtime
```

104 tests: `lower`/`printer` unit tests covering every row of MA09 §3's
surface table (arithmetic, comparison, logic, `if`/`elif`/`else`/`end
if`/`fi`, lists, sets, the arrow-operator `Define` bridge, `diff`/`int`),
plus end-to-end session tests (arithmetic, persistent bindings/functions,
`if` with/without `else`, list/set elementwise evaluation and the
disclosed `Set` gap, the `;`/`:` display-flag split, both robustness
guards including the new elif-chain regression, panic recovery, and
explicit confirmation that `proc`/`for`/`while` and the remember-table
`f(x) := e` spelling are cleanly rejected at parse time), plus a doctest
on `MapleSession::feed`.
