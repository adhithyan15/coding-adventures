# Changelog — axiom-iir-compiler

## v0.1.0 — 2026-08-18 — initial release (axiom-iir-vm.md, Wave 5 item 5)

The fifth real-language frontend onto `interpreter_ir` (IIR) in this
rollout (after Macsyma, Derive, Reduce, and Maple) — a retarget of
`axiom-runtime`'s/`axiom-to-semantic-ir`'s existing CST dispatch.

* `compile(&GrammarASTNode, module_name) -> Result<IIRModule, AxiomIirError>`
  and `compile_source(source, module_name)`.
* **`program` is a SINGLE expression** (`axiom.grammar`'s own `program =
  expr`), unlike every sibling frontend's multi-statement worksheet
  loop — a real, disclosed structural difference, not an oversight. A
  consequence: this crate's `Lowerer` has no `env`/binding-threading at
  all, since a bound variable can never be referenced after its own `x
  := e` statement in the same compiled module.
* Accepted v0 grammar: integer literals; `+ - * /` (chains + unary `-`
  only); assignment (`x := expr`, always a bare NAME target by grammar
  construction); free-symbol references; any other operator/head, or any
  operand involving a free symbol, lowers to an inert `cons`-chain.
* `a : T` (declaration), `e :: T` (coercion), and `D has C`
  (category-membership query) — Axiom's own genuinely new territory
  relative to every sibling frontend, with no arithmetic analogue to
  fall back to — are rejected outright.
* `String` literals are rejected too (Axiom has real `STRING` tokens,
  unlike Derive/Reduce/Maple).
* **Genuine finding while building the oracle test:** `axiom-runtime`
  domain-tags essentially every result (`format_value` appends `" :
  <Domain>"`, e.g. `"42 : PositiveInteger"`), not only values that
  passed through an explicit `:` declaration. Domain inference is an
  entire fixed-table system this v0 slice does not implement, so
  `tests/oracle.rs` strips the suffix before comparing ground truth — a
  disclosed methodology choice, not an accidental first-try match (every
  case failed against the naive `expected` strings before this fix).
* 30 unit tests (every accepted construct round-tripped through
  `axiom-vm`, every rejected construct's explicit-error path) + a
  17-case oracle test cross-checking against `axiom-runtime`, both
  rendered through the same `print_axiom` function.
