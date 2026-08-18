# Maple → IIR → VM interpreter

**Status:** Draft — 2026-08-18 (spec-first; sign-off = merge)
**Depends on:** [`macsyma-iir-vm.md`](macsyma-iir-vm.md) — this is Wave 5
item 4 of that spec's rollout; read it first for the full design.
[`reduce-iir-vm.md`](reduce-iir-vm.md) is the closer template (statement/
`program` loop shape). This document only records Maple's own deltas.
**Unblocks:** the same IIR-bridge pattern for the rest of Wave 5 (Axiom,
Wolfram).

## Summary

Bridges Maple onto `interpreter_ir` (IIR) the same way the prior three
languages were bridged: a new `maple-iir-compiler` frontend (a third
retarget of the `GrammarASTNode` CST `maple-runtime` and
`maple-to-semantic-ir` already walk) and a new `maple-vm` interpreter
(its own dedicated VM, per `macsyma-iir-vm.md` §6).

```text
                               ┌─→ maple-runtime (+ repl)                [existing, unchanged]
maple source → GrammarASTNode ┤─→ maple-to-semantic-ir → any SIR backend [existing, unchanged]
                               └─→ maple-iir-compiler → IIRModule → maple-vm   [NEW]
```

## Deltas from Reduce's design

Verified directly against `maple.grammar`/`maple.tokens`
(`code/grammars/maple/`) and `maple-to-semantic-ir/src/lower.rs`'s
module doc comment:

1. **`f(x) := body` is a genuine PARSE ERROR in Maple, not just an
   out-of-scope construct.** `assignment = NAME ASSIGN ( arrow_def |
   expr ) | expr` — the LHS is always a bare `NAME` by grammar
   construction (real Maple's identical-looking spelling means a
   narrower remember-table patch this repo doesn't implement — MA09
   §1/§4). So `lower_assignment` needs **no** bare-name-vs-call
   disambiguation at all, unlike Derive's/Reduce's own check — it only
   has to reject an `arrow_def` RHS (`f := x -> body`, Maple's separate
   general-function-definition production).
2. **`true`/`false` are real boolean literal TOKENS**, unlike Derive's/
   Reduce's grammars (which have no boolean literal syntax at all —
   booleans only arise from comparison/logic results there). v0 rejects
   them outright, matching Macsyma's own boolean-free v0 scope.
3. **`postfix`'s call suffix is syntactically non-chainable**:
   `postfix = atom [ LPAREN [ arglist ] RPAREN ]` — a single OPTIONAL
   suffix, not Derive's/Reduce's repeated `{ ... }`. `f(x)(y)` is a
   *parse* error in this grammar, confirmed by a dedicated regression
   test — there is no chain-length guard to write because the axis it
   would guard is structurally impossible here.
4. **No `if_expr` counterpart to Reduce's simple 2-or-3-child form** —
   Maple's `if`/`elif`/`else`/`end if` right-folds like Macsyma's own
   elif chain (out of v0 scope regardless, rejected outright).
5. **List/set literals**: `[...]` (`List`, shared head) and `{...}`
   (`Set`, a new head only Maple has) — both rejected outright in v0.
6. Everything else — `TIMES`/`SLASH`/no-unary-plus, the `/` exactness
   rule, the inert-`cons`-chain representation, the outright rejection
   of comparisons/`and`/`or`/`^`, the `{ statement_line } [ statement ]`
   program-loop shape — is unchanged from `reduce-iir-vm.md`.

## Oracle ground truth

`maple-runtime`'s API mirrors `reduce-runtime`'s exactly:
`MapleSession::eval_to_outputs(source) -> Result<Vec<Output>, String>`,
each rendered by `maple-runtime`'s own bespoke printer, `print_maple`
(`src/printer.rs`) — not `cas_pretty_printer`. The oracle test's
`read_back` reader is identical in shape to the sibling frontends'.

## Crates

- `code/packages/rust/maple-iir-compiler` — `compile`/`compile_source`,
  depends on `coding-adventures-maple-parser`/`interpreter-ir`/`parser`/
  `lexer`/`symbolic-ir`; dev-depends on `maple-vm`/`dynval-runtime`/
  `coding-adventures-maple-runtime`.
- `code/packages/rust/maple-vm` — `run`/`run_with_budget`, a direct
  structural port of the sibling VMs' dispatch loop.

## Verification

- `cargo test -p maple-iir-compiler -p maple-vm` — unit tests + the
  oracle test.
- `cargo clippy -p maple-iir-compiler -p maple-vm --all-targets` /
  `cargo fmt --check` clean.
- Oracle corpus has an empty `known_bug` list.

## References

[`macsyma-iir-vm.md`](macsyma-iir-vm.md),
[`reduce-iir-vm.md`](reduce-iir-vm.md) (the closer template),
[`MA09-maple-language.md`](MA09-maple-language.md) (the original Maple
language spec), `maple-to-semantic-ir/src/lower.rs` (the rule-dispatch
table retargeted a third time).
