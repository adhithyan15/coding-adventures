# Reduce → IIR → VM interpreter

**Status:** Draft — 2026-08-18 (spec-first; sign-off = merge)
**Depends on:** [`macsyma-iir-vm.md`](macsyma-iir-vm.md) — this is Wave 5
item 3 of that spec's rollout; read it first for the full design.
[`derive-iir-vm.md`](derive-iir-vm.md) is the closer template (same
`:=` disambiguation shape). This document only records Reduce's own
deltas.
**Unblocks:** the same IIR-bridge pattern for the rest of Wave 5 (Maple,
Axiom, Wolfram).

## Summary

Bridges Reduce onto `interpreter_ir` (IIR) the same way Macsyma and
Derive were bridged: a new `reduce-iir-compiler` frontend (a third
retarget of the `GrammarASTNode` CST `reduce-runtime` and
`reduce-to-semantic-ir` already walk) and a new `reduce-vm` interpreter
(its own dedicated VM, per `macsyma-iir-vm.md` §6).

```text
                                ┌─→ reduce-runtime (+ repl)                [existing, unchanged]
reduce source → GrammarASTNode ┤─→ reduce-to-semantic-ir → any SIR backend [existing, unchanged]
                                └─→ reduce-iir-compiler → IIRModule → reduce-vm   [NEW]
```

## Deltas from Derive's design

Verified directly against `reduce.grammar`/`reduce.tokens`
(`code/grammars/reduce/`) and `reduce-to-semantic-ir/src/lower.rs`'s
module doc comment:

1. **Three genuinely new grammar constructs, all outside v0 and all
   rejected outright**: an expression-shaped `if`/`then`/`else`
   (`if_expr`), a `<< s1; s2; ... >>` group statement (`group_expr`), and
   `a . b` cons (`cons`). Unlike arithmetic's inert-data fallback, none of
   these get one — v0 has no control-flow or list-cons runtime semantics
   to make either an evaluated or an inert-data representation
   verifiably correct.
2. **List literals use `{...}` (curly braces), not Derive's `[...]`.**
   Also rejected outright (v0 has no list support at all, unlike
   Macsyma's own `[...]` which is likewise rejected).
3. **`neq` (not `#`) is the not-equal comparison token**, a lowercase
   `KEYWORD`-typed token matched by value like `and`/`or`/`not`/`if`/
   `then`/`else` — moot for v0, since comparisons are rejected outright
   regardless of spelling.
4. **Program structure has an optional trailing bare statement**:
   `program = { statement_line } [ statement ]` (a source file need not
   end with a `;`/`$` terminator), unlike Derive's uniform
   `{ statement_line }`. `lower_file`'s loop handles both shapes.
5. **`assignment`'s RHS is the wider `expr` production**, not Derive's
   self-referential `assignment` — moot for v0 since none of the newly
   admitted RHS shapes (`if_expr`, `group_expr`) are in scope.
6. Everything else — the `:=` bare-name disambiguation, the `/`
   exactness rule, the inert-`cons`-chain representation, `TIMES`/
   `SLASH`/no-unary-plus, the outright rejection of comparisons/`and`/
   `or`/`^` — is unchanged from `derive-iir-vm.md`.

## Oracle ground truth

`reduce-runtime`'s API mirrors `derive-runtime`'s exactly:
`ReduceSession::eval_to_outputs(source) -> Result<Vec<Output>, String>`,
each rendered by `reduce-runtime`'s own bespoke printer, `print_reduce`
(`src/printer.rs`) — not `cas_pretty_printer`. The oracle test's
`read_back` reader is identical in shape to `derive-iir-compiler`'s.

## Crates

- `code/packages/rust/reduce-iir-compiler` — `compile`/`compile_source`,
  depends on `coding-adventures-reduce-parser`/`interpreter-ir`/`parser`/
  `lexer`/`symbolic-ir`; dev-depends on `reduce-vm`/`dynval-runtime`/
  `coding-adventures-reduce-runtime`.
- `code/packages/rust/reduce-vm` — `run`/`run_with_budget`, a direct
  structural port of `macsyma-vm`'s/`derive-vm`'s dispatch loop.

## Verification

- `cargo test -p reduce-iir-compiler -p reduce-vm` — unit tests + the
  oracle test.
- `cargo clippy -p reduce-iir-compiler -p reduce-vm --all-targets` /
  `cargo fmt --check` clean.
- Oracle corpus has an empty `known_bug` list.

## References

[`macsyma-iir-vm.md`](macsyma-iir-vm.md),
[`derive-iir-vm.md`](derive-iir-vm.md) (the closer template),
[`MA08-reduce-language.md`](MA08-reduce-language.md) (the original
Reduce language spec), `reduce-to-semantic-ir/src/lower.rs` (the
rule-dispatch table retargeted a third time).
