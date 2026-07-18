# coding-adventures-reduce-runtime

Evaluates Reduce (a subset) by lowering the `reduce-parser` (R-3) CST into
[`symbolic-ir`](../symbolic-ir) and running it through
[`symbolic-vm`](../symbolic-vm)'s shared `SymbolicBackend` — the *same*
rewrite engine Wolfram, Macsyma, and Derive already drive, unchanged. See
[`code/specs/MA08-reduce-language.md`](../../../specs/MA08-reduce-language.md).

## Where this fits

`reduce-runtime` is R-4 of the Reduce frontend/runtime pipeline:

```
reduce.tokens + reduce-lexer   (R-2)
        │
reduce.grammar + reduce-parser (R-3)
        │
reduce-runtime                 (R-4, this crate) ── crate::lower ──► symbolic_ir::IRNode
        │                                              │
        │                                       symbolic_vm::VM (SymbolicBackend)
        │                                              │
reduce-repl                     (R-4)  ◄── crate::printer ─┘
```

## No custom `Backend` — and what that costs

`SymbolicBackend` (built with `simplify: true`) already provides arithmetic
(`Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg`), comparison (`Equal`/`Less`/`Greater`/
`LessEqual`/`GreaterEqual`/`NotEqual`), logic (`And`/`Or`/`Not`), the held
`Assign`/`Define`/`If` forms, and `List` — so this crate adds **no new
evaluation code**, only:

- **`lower`** — Reduce's surface `GrammarASTNode` (`assignment`, `if_expr`,
  `group_expr`, `additive`, `cons`, `postfix`, `atom`, …) → the canonical
  `IRNode` heads the VM dispatches, including the lowercase-surface→canonical
  head bridge (`list`→`List`, `first`→`First`, `rest`→`Rest`, …), the `:=`
  assignment-vs-definition disambiguation by LHS shape (mirroring
  `derive-runtime`'s identical trick — Reduce, like Derive, has only one
  assignment token), and the `cons` (`.`)-onto-a-literal-list fold (MA08
  §3: `a . {b, c}` → `List[a, b, c]`, not a standalone `Cons` node).
- **`printer`** — the inverse: canonical `IRNode` → Reduce surface notation
  (infix `+`/`-`/`*`/`/`/`^`, `and`/`or`/`not`, curly-brace `{a, b, c}`
  lists, `if ... then ... else ...`, `<< s1; s2; ... >>`).

**A real gap, disclosed rather than papered over:** grepping
`symbolic-vm::handlers::build_handler_table` turns up **no** handler for
`CompoundExpression` (Reduce's `<< ... >>`), `First`/`Second`/`Third`/
`Rest`/`Part`/`Append`/`Reverse` (the list accessors/constructors), or
`Cons` — MA08 §5's claim that these are "already implemented for Macsyma/
Wolfram/Derive" does not hold for the *shared*, unchanged `SymbolicBackend`
this crate is required to reuse: Macsyma's list functions and Wolfram's
`CompoundExpression` are each wired through a **bespoke `Backend`**
specific to that language, which is exactly what R-4 is not supposed to
build. This crate still lowers to the structurally-correct heads MA08 §3
documents, so a future item adding real handlers needs no lowering change
— but evaluating one of these calls today does not perform the operation:
arguments evaluate (so `Assign`/`Define` side effects inside a
`<< ... >>` genuinely happen, in order), the call itself just stays
unevaluated, exactly like calling an undefined user function:

```rust
use coding_adventures_reduce_runtime::ReduceSession;

let mut s = ReduceSession::new();
// The Assign inside really fires (and persists!) -- but the group's own
// result is the unevaluated CompoundExpression(1, 2), not just `2`.
assert_eq!(s.feed("<< a := 1; a + 1 >>;\n").unwrap(), "<< 1; 2 >>\n");
assert_eq!(s.feed("a;\n").unwrap(), "1\n");

// first(...) evaluates its argument but the call itself stays unevaluated.
assert_eq!(
    coding_adventures_reduce_runtime::eval("first({1, 2, 3});\n").unwrap(),
    "first({1, 2, 3})\n"
);
```

Also disclosed: MA08 §3's own prose describes the arithmetic "Lowers to"
column as `Plus`/`Subtract`/`Times`/`Power` (and even expands `a / b` to
`Times[a, Power[b, -1]]` and `-a` to `Times[-1, a]`) — none of those
spellings exist in `symbolic-ir` (`grep -n '"Plus"\|"Subtract"\|"Times"'
symbolic-ir/src/lib.rs` returns nothing). The real, already-reused heads
are `Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg` — what `derive-runtime`/
`macsyma-compiler` already lower `+`/`-`/`*`/`/`/`^`/unary-`-` to, and what
this crate lowers to as well, so all four CAS-family languages keep
agreeing on every arithmetic result. The spec has been corrected to match
(see MA08's own changelog note); this README and `crate::lower`'s module
doc comment carry the full reasoning.

## Usage

```rust
use coding_adventures_reduce_runtime::ReduceSession;

let mut s = ReduceSession::new();
assert_eq!(s.feed("x := 5;\n").unwrap(), "5\n");
assert_eq!(s.feed("x + 1;\n").unwrap(), "6\n");
// A procedure definition's own result is just the defined name (`h`) --
// like `derive-runtime`'s `Define`, the body is stored, not evaluated yet.
assert_eq!(s.feed("h(x) := x*x;\n").unwrap(), "h\n");
assert_eq!(s.feed("h(5);\n").unwrap(), "25\n");
```

`coding_adventures_reduce_runtime::eval(src)` is a one-shot convenience for
callers that don't need a persistent session. Unlike `derive-runtime`'s
`#n:`-numbered `Output`, Reduce's own session transcript has no numbered-
input convention (MA08 §2/§5), so every result line here is unprefixed
plain text.

## Robustness

`feed`/`eval_to_outputs` are the trust boundary for arbitrary Reduce source.
Two independent deep-recursion vectors are closed (see the crate doc
comment for the full rationale):

1. **Deeply nested source** (parenthesised, or a right-recursive
   `:=`/`if`-`else`/cons/power chain) — already rejected by
   `reduce-parser`'s own `MAX_RULE_DEPTH`.
2. **A long flat chain** (`1+1+1+…`, or a chained call `f(x)(x)(x)…`) that
   folds into a deeply *nested* lowered tree — grammar repetitions aren't
   bounded by `MAX_RULE_DEPTH`, so `MAX_STATEMENT_TOKENS` (measured against
   the real lexer token stream, reset on Reduce's own `SEMI`/`DOLLAR`
   statement separators — but only at bracket depth 0, i.e. genuine
   top-level statement boundaries) closes this separately. A `/security-
   review` finding caught a bypass in an earlier version that reset
   unconditionally, including on a `;`/`$` lexically inside a `<< ... >>`
   group statement embedded as one operand of a much larger enclosing
   chain (`1 + 1 + (<<0;0>>) + 1 + 1 + ...`) — see `check_statement_token_
   counts`'s doc comment for the full accounting.

Evaluation itself runs on a worker thread with a large bounded stack inside
`catch_unwind`, so a reused-handler panic (e.g. a malformed `Assign` LHS)
becomes a clean `Err` and the session is rebuilt rather than left corrupted.
A thread-spawn failure itself (OS thread-count/memory pressure) is also
handled as an ordinary `Err`, not a caller-thread panic — a second
`/security-review` finding against an earlier `.expect()`-based version.

## Tests

```sh
cargo test -p coding-adventures-reduce-runtime
```

Unit tests for every `lower`/`printer` construct in MA08 §3's table
(arithmetic, comparison, logic, `if`/`<< ... >>`, lists, cons-folding,
assignment/definition), plus end-to-end session tests: arithmetic,
persistent assignment/user-defined operators, `if` with and without
`else`, both robustness guards (including the chained-call-postfix and
inside-a-group-statement variants), panic recovery, and the disclosed
list-accessor/`CompoundExpression` gap (evaluates without crashing, stays
structurally unevaluated).
