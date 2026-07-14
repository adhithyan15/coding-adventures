# coding-adventures-derive-runtime

Evaluates Derive (a subset) by lowering the `derive-parser` (D-3) CST into
[`symbolic-ir`](../symbolic-ir) and running it through
[`symbolic-vm`](../symbolic-vm)'s shared `SymbolicBackend` — the *same*
rewrite engine Wolfram and Macsyma already drive, unchanged. See
[`code/specs/MA07-derive-language.md`](../../../specs/MA07-derive-language.md).

## Where this fits

`derive-runtime` is D-4 of the Derive frontend/runtime pipeline:

```
derive.tokens + derive-lexer   (D-2)
        │
derive.grammar + derive-parser (D-3)
        │
derive-runtime                 (D-4, this crate) ── crate::lower ──► symbolic_ir::IRNode
        │                                              │
        │                                       symbolic_vm::VM (SymbolicBackend)
        │                                              │
derive-repl                    (D-4)  ◄── crate::printer ─┘
```

## No custom `Backend`

`SymbolicBackend` (built with `simplify: true`) already provides every
operation MA07 §3/§4 scopes for D-4 — arithmetic, comparison, logic, the
held `Assign`/`Define`/`If` forms, and the base `D`/`Integrate` calculus
handlers — so this crate adds **no new evaluation code**, only:

- **`lower`** — Derive's surface `GrammarASTNode` (assignment, additive,
  multiplicative, power, postfix, atom, …) → the canonical `IRNode` heads
  the VM dispatches, including the uppercase-surface→canonical head bridge
  (`SIN`→`Sin`, `DIF`→`D`, `INT`→`Integrate`, `IF`→`If`, …) and the `:=`
  assignment-vs-definition disambiguation by LHS shape (see the module doc
  comment — Derive, unlike Wolfram/Macsyma, has only one assignment
  operator token, so there is no operator to branch on).
- **`printer`** — the inverse: canonical `IRNode` → Derive surface notation
  (infix `+`/`-`/`*`/`/`/`^`, `AND`/`OR`/`NOT`, `F(…)` calls), bridging
  builtin heads back to their uppercase spelling.

`LIM`/`SOLVE`/`SUM`/`PRODUCT`/`TAYLOR` are **not** wired — MA07 §4 ("Honest
scope") defers them to their own follow-on items, since the shared VM has
no existing handler for any of them (unlike `D`/`Integrate`, which are
already fully implemented and used unchanged).

## Usage

```rust
use coding_adventures_derive_runtime::DeriveSession;

let mut s = DeriveSession::new();
assert_eq!(s.feed("x := 5\n").unwrap(), "#1: 5\n");
assert_eq!(s.feed("x + 1\n").unwrap(), "#2: 6\n");
assert_eq!(s.feed("DIF(x^2, x)\n").unwrap(), "#3: 2*x\n");
```

`coding_adventures_derive_runtime::eval(src)` is a one-shot convenience for
callers that don't need a persistent session.

## Robustness

`feed`/`eval_to_outputs` are the trust boundary for arbitrary Derive source.
Two independent deep-recursion vectors are closed (see the crate doc
comment for the full rationale):

1. **Deeply nested source** (`((((…))))`) — already rejected by
   `derive-parser`'s own `MAX_RULE_DEPTH`.
2. **A long flat chain** (`1+1+1+…`) that folds into a deeply *nested*
   lowered tree — grammar repetitions aren't bounded by `MAX_RULE_DEPTH`, so
   `MAX_STATEMENT_TOKENS` (measured against the real lexer token stream)
   closes this separately.

Evaluation itself runs on a worker thread with a large bounded stack inside
`catch_unwind`, so a reused-handler panic (e.g. a malformed `Assign` LHS)
becomes a clean `Err` and the session is rebuilt rather than left corrupted.

## Tests

```sh
cargo test -p coding-adventures-derive-runtime
```

Unit tests for every `lower`/`printer` construct, plus end-to-end session
tests: arithmetic, persistent assignment/user-defined functions, `DIF`/`INT`/`IF`
wiring through the shared handler table, the two robustness guards, and
panic recovery.
