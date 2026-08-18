# Changelog — wolfram-iir-compiler

## v0.1.0 — 2026-08-18 — initial release (wolfram-iir-vm.md, Wave 5 item 6)

The sixth and final real-language frontend onto `interpreter_ir` (IIR)
in this rollout (after Macsyma, Derive, Reduce, Maple, and Axiom) —
closing out Wave 5. A retarget of `wolfram-runtime`'s/
`wolfram-to-semantic-ir`'s existing CST dispatch.

* `compile(&GrammarASTNode, module_name) -> Result<IIRModule, WolframIirError>`
  and `compile_source(source, module_name)`.
* Deliberately narrower than `wolfram-to-semantic-ir`, which covers
  Wolfram's full grammar — v0 holds the same arithmetic/assignment/
  unevaluated-Apply scope constant across all six Wave 5 languages,
  rather than letting the richer grammar widen it.
* Accepted v0 grammar: integer literals; `+ - * /` (chains) and BOTH
  unary `-`/`+` (Wolfram is the only Wave 5 language with a real
  unary-plus, matching Macsyma's own grammar shape); assignment (`x =
  expr`, `SET` only, bare NAME target); free-symbol references; any
  other operator/head, or any operand involving a free symbol, lowers
  to an inert `cons`-chain.
* `SETDELAYED` (`:=`, function/pattern definition) is rejected outright,
  mirroring `macsyma-iir-compiler`'s own `COLON`-vs-`COLONEQ` split —
  Wolfram has two distinct assignment tokens, unlike Derive's/Reduce's
  single overloaded one.
* Pattern blanks, rules, replacement, conditions, alternatives, pattern
  tests, pure functions, and `/@`/`@@` sugar — Wolfram's own genuinely
  new territory relative to every sibling frontend — are all rejected
  outright, with no arithmetic analogue to fall back to.
* `postfix`'s rejection is unconditional on ANY suffix (not just calls)
  — Wolfram's `postfix` has several distinct suffix shapes (calls, Part
  indexing, …), so v0 makes no attempt to distinguish them, unlike
  `wolfram-to-semantic-ir`'s own richer handling.
* 34 unit tests (every accepted construct round-tripped through
  `wolfram-vm`, every rejected construct's explicit-error path,
  including the full pattern/rule/replacement/pure-function surface) +
  a 19-case oracle test cross-checking against `wolfram-runtime`, both
  rendered through the same `print_wolfram` function.
