# Derive → IIR → VM interpreter

**Status:** Draft — 2026-08-18 (spec-first; sign-off = merge)
**Depends on:** [`macsyma-iir-vm.md`](macsyma-iir-vm.md) — this is Wave 5
item 2 of that spec's rollout; read it first for the full design (value
representation, the `/` exactness rule, the VM-sharing decision, PR
sequencing). This document only records Derive's own deltas.
**Unblocks:** the same IIR-bridge pattern for the rest of Wave 5 (Reduce,
Maple, Axiom, Wolfram).

## Summary

Bridges Derive onto `interpreter_ir` (IIR) the same way `macsyma-iir-vm.md`
bridged Macsyma: a new `derive-iir-compiler` frontend (a third retarget of
the `GrammarASTNode` CST `derive-runtime` and `derive-to-semantic-ir`
already walk) and a new `derive-vm` interpreter (its own dedicated VM,
per `macsyma-iir-vm.md` §6's explicit decision that every language in
this rollout gets one, not a shared crate).

```text
                                ┌─→ derive-runtime (+ repl)                [existing, unchanged]
derive source → GrammarASTNode ┤─→ derive-to-semantic-ir → any SIR backend [existing, unchanged]
                                └─→ derive-iir-compiler → IIRModule → derive-vm   [NEW]
```

## Deltas from Macsyma's design

Verified directly against `derive.grammar`/`derive.tokens`
(`code/grammars/derive/`) and `derive-to-semantic-ir/src/lower.rs`'s
module doc comment, not assumed from family resemblance:

1. **Assignment token is `:=` (`ASSIGN`), not Macsyma's bare `:`.** Derive
   has exactly one assignment token, used for both plain assignment and
   function definition (`F(x) := body`) — there is no `COLON`/`COLONEQ`
   split to dispatch on the way Macsyma's grammar has. Since v0 rejects
   all function calls/definitions anyway, `lower_assignment` disambiguates
   by checking whether the LHS is a bare `NAME` *before* lowering it as an
   expression, giving a definition-specific error message rather than
   falling through to the generic "function calls not supported" path.
2. **No unary-plus.** Derive's grammar is `unary = MINUS unary | power` —
   there is no `+5` production at all, unlike Macsyma's `(MINUS | PLUS)
   unary`. `Lowerer::lower_unary` has one fewer branch as a result.
3. **Multiplication token is `TIMES`, not `STAR`.**
4. **No `STRING` token, no boolean literal keywords.** `lower_token` only
   ever needs `NUMBER`/`NAME` arms.
5. Everything else — the accepted/rejected construct list, the `/`
   exactness rule, the inert-`cons`-chain representation for unevaluated
   `Apply`, the outright rejection of comparisons/`AND`/`OR`/`^` (Derive's
   real evaluator numerically folds concrete instances of these, exactly
   like Macsyma's) — is unchanged from `macsyma-iir-vm.md` §3/§4.

## Oracle ground truth

`derive-runtime`'s public API differs in shape from `macsyma-runtime`'s:
`DeriveSession::eval_to_outputs(source) -> Result<Vec<Output>, String>`,
each `Output { index, text }`, where `text` is already rendered by
`derive-runtime`'s own bespoke printer, `print_derive` (`src/printer.rs`)
— **not** `cas_pretty_printer` (Derive has no `Dialect` impl in that
shared crate; Macsyma's `MacsymaDialect` is Macsyma-specific). The oracle
test's `read_back` reader is otherwise identical in shape to
`macsyma-iir-compiler/tests/oracle.rs`'s, just rendering through
`print_derive` instead of `cas_pretty_printer::pretty`.

## Crates

- `code/packages/rust/derive-iir-compiler` — `compile`/`compile_source`,
  depends on `coding-adventures-derive-parser`/`interpreter-ir`/`parser`/
  `lexer`/`symbolic-ir`; dev-depends on `derive-vm`/`dynval-runtime`/
  `coding-adventures-derive-runtime`.
- `code/packages/rust/derive-vm` — `run`/`run_with_budget`, a direct
  structural port of `macsyma-vm`'s dispatch loop.

## Verification

- `cargo test -p derive-iir-compiler -p derive-vm` — unit tests (every
  accepted construct, every rejected construct's explicit-error path) +
  the oracle test.
- `cargo clippy -p derive-iir-compiler -p derive-vm --all-targets` /
  `cargo fmt --check` clean.
- Oracle corpus has an empty `known_bug` list, matching Macsyma's own v0
  corpus — the design guarantees exact agreement within v0's scope.

## References

[`macsyma-iir-vm.md`](macsyma-iir-vm.md) (the design this document
deltas from), [`MA07-derive-language.md`](MA07-derive-language.md) (the
original Derive language spec), `derive-to-semantic-ir/src/lower.rs`
(the rule-dispatch table retargeted a third time).
