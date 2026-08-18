# Changelog — reduce-iir-compiler

## v0.1.0 — 2026-08-18 — initial release (reduce-iir-vm.md, Wave 5 item 3)

The third real-language frontend onto `interpreter_ir` (IIR) in this
rollout (after Macsyma and Derive) — a retarget of `reduce-runtime`'s/
`reduce-to-semantic-ir`'s existing CST dispatch, following
`macsyma-iir-compiler`'s/`derive-iir-compiler`'s precedent directly.

* `compile(&GrammarASTNode, module_name) -> Result<IIRModule, ReduceIirError>`
  and `compile_source(source, module_name)`.
* Accepted v0 grammar: integer literals; `+ - * /` (chains + unary `-`
  only); assignment (`x := expr`); free-symbol references; any other
  operator/head, or any operand involving a free symbol, lowers to an
  inert `cons`-chain.
* Concrete `+`/`-`/`*` always evaluate via a real `call_builtin`; `/`
  gets the identical narrower exactness rule the other two frontends
  established.
* Comparisons, `and`/`or`/`not`, and `^`/`**` are rejected outright (no
  inert-data fallback), for the identical reason as the other two
  frontends. `if`/`then`/`else`, `<< ... >>` group statements, `a . b`
  cons, and `{...}` list literals — all genuinely new surface Derive has
  no analogue of — are rejected outright too, since v0 has no
  control-flow or list-cons runtime semantics to give them any
  verifiably-correct meaning.
* `h(l, m) := body` (procedure definition) is rejected with a
  definition-specific error message, disambiguated purely by the LHS's
  shape — Reduce has only one `:=` token for both plain assignment and
  definition.
* 27 unit tests (every accepted construct round-tripped through
  `reduce-vm`, every rejected construct's explicit-error path, including
  a trailing-statement-without-terminator case exercising the grammar's
  optional final bare `statement`) + a 18-case oracle test cross-checking
  against `reduce-runtime`, both rendered through the same `print_reduce`
  function.
