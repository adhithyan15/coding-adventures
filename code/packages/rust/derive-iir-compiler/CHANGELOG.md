# Changelog — derive-iir-compiler

## v0.1.0 — 2026-08-18 — initial release (derive-iir-vm.md, Wave 5 item 2)

The second real-language frontend onto `interpreter_ir` (IIR) in this
rollout (after Macsyma) — a retarget of `derive-runtime`'s/
`derive-to-semantic-ir`'s existing CST dispatch, following
`macsyma-iir-compiler`'s precedent directly.

* `compile(&GrammarASTNode, module_name) -> Result<IIRModule, DeriveIirError>`
  and `compile_source(source, module_name)`.
* Accepted v0 grammar: integer literals; `+ - * /` (chains + unary `-`
  only — Derive has no unary-plus); assignment (`x := expr`); free-symbol
  references; any other operator/head, or any operand involving a free
  symbol, lowers to an inert `cons`-chain (an unevaluated symbolic
  `Apply` node).
* Concrete `+`/`-`/`*` always evaluate via a real `call_builtin`; `/`
  gets the identical narrower exactness rule `macsyma-iir-compiler`
  established (only evaluates literal/literal exact division; anything
  else involving `/` is an explicit compile error).
* Comparisons, `AND`/`OR`/`NOT`, and `^` are rejected outright (no
  inert-data fallback), for the identical reason Macsyma's `^`/
  comparisons/logic are.
* `F(x) := body` (function definition) is rejected with a
  definition-specific error message, disambiguated purely by the LHS's
  shape — Derive has only one `:=` token for both plain assignment and
  definition, unlike Macsyma's separate `:`/`:=`.
* Every other out-of-scope construct — `Float` literals, comparisons,
  `AND`/`OR`/`NOT`, `^`, `[...]`/`[...;...]` vector/matrix literals,
  postfix function calls — returns an explicit `DeriveIirError`, never a
  silent mis-lowering.
* 22 unit tests (every accepted construct round-tripped through
  `derive-vm`, every rejected construct's explicit-error path) + a
  19-case oracle test cross-checking against `derive-runtime`, both
  rendered through the same `print_derive` function.
