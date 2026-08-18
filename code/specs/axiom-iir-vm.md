# Axiom → IIR → VM interpreter

**Status:** Draft — 2026-08-18 (spec-first; sign-off = merge)
**Depends on:** [`macsyma-iir-vm.md`](macsyma-iir-vm.md) — this is Wave 5
item 5 of that spec's rollout; read it first for the full design.
This document records Axiom's own deltas, the largest of the rollout so
far (a genuinely different `program` shape, not just grammar-surface
differences).
**Unblocks:** the same IIR-bridge pattern for the last Wave 5 item
(Wolfram).

## Summary

Bridges Axiom onto `interpreter_ir` (IIR): a new `axiom-iir-compiler`
frontend (a third retarget of the `GrammarASTNode` CST `axiom-runtime`
and `axiom-to-semantic-ir` already walk) and a new `axiom-vm`
interpreter (its own dedicated VM, per `macsyma-iir-vm.md` §6).

```text
                               ┌─→ axiom-runtime (+ repl)                [existing, unchanged]
axiom source → GrammarASTNode ┤─→ axiom-to-semantic-ir → any SIR backend [existing, unchanged]
                               └─→ axiom-iir-compiler → IIRModule → axiom-vm   [NEW]
```

## Deltas from the rest of Wave 5

Verified directly against `axiom.grammar`/`axiom.tokens`
(`code/grammars/axiom/`), `axiom-to-semantic-ir/src/lower.rs`'s module
doc comment, and `axiom-runtime/src/lib.rs`:

1. **`program` is a SINGLE expression, not a multi-statement worksheet.**
   `axiom.grammar`'s own `program = expr` — Axiom is modeled as a
   numbered, per-line interactive session (matching `axiom-repl`'s own
   step counter), not a batch file. Every prior Wave 5 item's `lower_file`
   loops over `{ statement_line } [ statement ]`; this crate's
   `lower_file` lowers exactly one top-level expression.
2. **No `env`/binding-threading at all — a real simplification, not a
   missing feature.** Since a compiled module can never contain more than
   one statement, a bound variable can never be referenced after its own
   `x := e` — there is no second statement to read it back in. So
   `Lowerer` carries no `env: HashMap` (every sibling crate does); a bare
   `NAME` is *always* a free symbol. `x := e` still lowers and returns
   `e`'s value (matching every sibling's "assignment's value is the RHS's
   value" convention) — there is simply nothing to bind for.
3. **`a : T` (declaration), `e :: T` (coercion), `D has C`
   (category-membership query) — genuinely new territory with no
   arithmetic analogue.** Unlike `if`/`while`/list-literals in the other
   four languages (which at least *could* theoretically fall back to
   inert data the way `Add`/`Sub` do), these three have no sibling
   precedent at all and are rejected outright, matching how
   `axiom-to-semantic-ir` itself needed a wholly new design decision
   (three new reserved `SymApply` head names) to represent them — v0
   doesn't need to represent them at all, just reject them cleanly.
4. **`assignment = NAME ASSIGN expr` guarantees a bare-NAME target**,
   exactly like Maple's (not Derive's/Reduce's own bare-name-vs-call
   check) — but Axiom has no `arrow_def`-equivalent RHS to check for
   either, since function definitions are a wholly separate grammar
   production (`declared_define`/`undeclared_define`), never reachable
   through `assignment` at all.
5. **Real `STRING` tokens** (unlike Derive/Reduce/Maple) — rejected in
   v0, matching every other out-of-scope literal kind.
6. Arithmetic (`TIMES`/`SLASH`/no-unary-plus), the `/` exactness rule,
   the inert-`cons`-chain representation, and the outright rejection of
   comparisons/`^` are unchanged from the rest of Wave 5.

## Oracle ground truth — a genuine finding, not assumed

`axiom-runtime::AxiomSession::eval_to_output` returns a single `Output`
(not `Vec<Output>`, matching `program`'s single-expression shape), whose
`text` is `format_value`'s output. **Confirmed empirically** (this
crate's own first oracle draft assumed otherwise and every case failed):
`format_value` appends `" : <Domain>"` to `print_axiom`'s plain rendering
whenever a domain was inferred — and Axiom's evaluator infers a domain
for essentially *every* value (`42` → `"42 : PositiveInteger"`, `x + y`
→ `"x + y : Polynomial(Integer)"`), not only ones that passed through an
explicit `:` declaration. Domain inference (`axiom-runtime::domains`) is
an entire fixed-table system v0 does not implement at all (declarations
are rejected outright), so the oracle test's ground truth strips the `"
: <Domain>"` suffix before comparing — a disclosed methodology choice
(compare values only, not domain tags), not an accidental match.

## Crates

- `code/packages/rust/axiom-iir-compiler` — `compile`/`compile_source`,
  depends on `coding-adventures-axiom-parser`/`interpreter-ir`/`parser`/
  `lexer`/`symbolic-ir`; dev-depends on `axiom-vm`/`dynval-runtime`/
  `coding-adventures-axiom-runtime`.
- `code/packages/rust/axiom-vm` — `run`/`run_with_budget`, a direct
  structural port of the sibling VMs' dispatch loop.

## Verification

- `cargo test -p axiom-iir-compiler -p axiom-vm` — unit tests + the
  oracle test.
- `cargo clippy -p axiom-iir-compiler -p axiom-vm --all-targets` /
  `cargo fmt --check` clean.
- Oracle corpus has an empty `known_bug` list (domain-suffix stripping
  handles the one systematic ground-truth difference; no per-case gaps).

## References

[`macsyma-iir-vm.md`](macsyma-iir-vm.md),
[`MA13-axiom-language.md`](MA13-axiom-language.md) (the original Axiom
language spec), `axiom-to-semantic-ir/src/lower.rs` (the rule-dispatch
table retargeted a third time, and the source of the `:`/`::`/`has`
design-decision precedent this crate deliberately does NOT need to
replicate, since v0 rejects all three outright).
