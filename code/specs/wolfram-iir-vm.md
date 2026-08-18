# Wolfram → IIR → VM interpreter

**Status:** Draft — 2026-08-18 (spec-first; sign-off = merge)
**Depends on:** [`macsyma-iir-vm.md`](macsyma-iir-vm.md) — this is Wave 5
item 6, the LAST item, closing out that spec's rollout. Read
`macsyma-iir-vm.md` first for the full design.
**Unblocks:** nothing further in Wave 5 — this closes it. See
`macsyma-iir-vm.md` §6 for the next waves (Rational/Float substrate,
control flow, native codegen backends).

## Summary

Bridges Wolfram onto `interpreter_ir` (IIR), the sixth and final
real-language frontend in this rollout: a new `wolfram-iir-compiler`
frontend (a third retarget of the `GrammarASTNode` CST `wolfram-runtime`
and `wolfram-to-semantic-ir` already walk) and a new `wolfram-vm`
interpreter (its own dedicated VM, per `macsyma-iir-vm.md` §6).

```text
                                 ┌─→ wolfram-runtime (+ repl)                [existing, unchanged]
wolfram source → GrammarASTNode ┤─→ wolfram-to-semantic-ir → any SIR backend [existing, unchanged]
                                 └─→ wolfram-iir-compiler → IIRModule → wolfram-vm   [NEW]
```

## Deltas from the rest of Wave 5 — and from `wolfram-to-semantic-ir` itself

Verified directly against `wolfram.grammar`/`wolfram.tokens`
(`code/grammars/wolfram/`) and `wolfram-to-semantic-ir/src/lower.rs`'s
module doc comment. Wolfram's grammar is genuinely the richest of the
six (`wolfram-to-semantic-ir` covers ALL of it — pattern blanks,
rules, replacement, pure functions, `/@`/`@@` sugar — since SIR23's
"everything is data" design has no scope pressure narrowing it there).
**This crate deliberately does not follow that precedent.** v0's
arithmetic/assignment/unevaluated-Apply scope, set once in
`macsyma-iir-vm.md` and held constant across all six languages, is
narrower than what Wolfram's grammar could support — the cut here is a
choice, not a grammar-forced necessity the way Reduce's/Maple's/Axiom's
own genuinely-new constructs were.

1. **Both unary `-` AND `+` are real** (`unary = (MINUS|PLUS) unary |
   power`) — Wolfram is the only language in Wave 5 besides Macsyma
   itself with a real unary-plus no-op; Derive/Reduce/Maple/Axiom all
   lack one.
2. **Two distinct assignment tokens, `SET` (`=`) and `SETDELAYED`
   (`:=`)** — unlike Derive's/Reduce's single overloaded token. v0
   accepts `SET` only (bare-NAME target, same disambiguation as
   Derive's/Reduce's own `bare_name` check) and rejects `SETDELAYED`
   outright, mirroring `macsyma-iir-compiler`'s own `COLON`-vs-`COLONEQ`
   split.
3. **Genuinely new surface with no arithmetic analogue at all**: pattern
   blanks (`_`/`_h`), rules (`->`/`:>`), replacement (`/.`/`//.`),
   conditions (`/;`), alternatives (`|`), pattern tests (`?`), pure
   functions (`#`/`#n`/`##`/`expr &`), `/@`/`@@` sugar. All rejected
   outright — the same treatment Reduce's `if`/`<<...>>`/cons and Maple's
   `Set` literal got, just a longer list.
4. **`postfix` rejects unconditionally on ANY suffix**, not just a call —
   Wolfram's `postfix` has several distinct suffix shapes (`f[x]` calls,
   `x[[i]]` Part-indexing, and more), so unlike the narrower single-shape
   `postfix` in Derive/Reduce/Maple/Axiom, v0 doesn't attempt to
   distinguish which suffix kind is present.
5. Program structure (`{ statement_line }`, uniform, no optional trailing
   bare statement), `TIMES`/`SLASH`, the `/` exactness rule, and the
   inert-`cons`-chain representation are unchanged from Derive's shape.

## Oracle ground truth

`wolfram-runtime`'s API mirrors Derive's/Reduce's/Maple's exactly:
`WolframSession::eval_to_outputs(source) -> Result<Vec<Output>, String>`,
each rendered by `wolfram-runtime`'s own bespoke printer, `print_wolfram`
— not `cas_pretty_printer`. The oracle test's `read_back` reader is
identical in shape to the sibling frontends'.

## Crates

- `code/packages/rust/wolfram-iir-compiler` — `compile`/`compile_source`,
  depends on `coding-adventures-wolfram-parser`/`interpreter-ir`/
  `parser`/`lexer`/`symbolic-ir`; dev-depends on `wolfram-vm`/
  `dynval-runtime`/`coding-adventures-wolfram-runtime`.
- `code/packages/rust/wolfram-vm` — `run`/`run_with_budget`, a direct
  structural port of the sibling VMs' dispatch loop.

## Verification

- `cargo test -p wolfram-iir-compiler -p wolfram-vm` — unit tests
  (including a full sweep of Wolfram's rejected pattern/rule/replacement/
  pure-function surface) + the oracle test.
- `cargo clippy -p wolfram-iir-compiler -p wolfram-vm --all-targets` /
  `cargo fmt --check` clean.
- Oracle corpus has an empty `known_bug` list.

## Wave 5 close-out

This is the sixth and final Wave 5 item. All six SIR23 CAS-family
languages (Macsyma, Maxima, Derive, Reduce, Maple, Axiom, Wolfram — seven
counting Maxima's alias) now have an IIR bridge, matching what they
already had on the Semantic-IR side. See `macsyma-iir-vm.md` §6 for what
comes next: Wave 2 (Rational/Float, a `dynval-runtime` substrate change),
Wave 3 (control flow, a real evaluator loop per VM), Wave 4 (the other 6
IIR backends beyond the VM interpreter).

## References

[`macsyma-iir-vm.md`](macsyma-iir-vm.md),
[`derive-iir-vm.md`](derive-iir-vm.md) (the closer template for the
`bare_name` assignment-disambiguation pattern),
[`MA04-wolfram-language.md`](MA04-wolfram-language.md) (the original
Wolfram language spec), `wolfram-to-semantic-ir/src/lower.rs` (the
rule-dispatch table retargeted a third time, covering the full grammar
this crate deliberately narrows).
