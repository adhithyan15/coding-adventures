# Changelog — maple-iir-compiler

## v0.1.0 — 2026-08-18 — initial release (maple-iir-vm.md, Wave 5 item 4)

The fourth real-language frontend onto `interpreter_ir` (IIR) in this
rollout (after Macsyma, Derive, and Reduce) — a retarget of
`maple-runtime`'s/`maple-to-semantic-ir`'s existing CST dispatch.

* `compile(&GrammarASTNode, module_name) -> Result<IIRModule, MapleIirError>`
  and `compile_source(source, module_name)`.
* Accepted v0 grammar: integer literals; `+ - * /` (chains + unary `-`
  only); assignment (`x := expr`); free-symbol references; any other
  operator/head, or any operand involving a free symbol, lowers to an
  inert `cons`-chain.
* Unlike Derive/Reduce, Maple's grammar makes `f(x) := body` a genuine
  *parse error* (the LHS is always a bare `NAME`), so this crate needs no
  bare-name-vs-call disambiguation at all — only a check for an
  `arrow_def` RHS (`f := x -> body`), rejected as "no
  user-defined-function support".
* `true`/`false` boolean literal keywords are rejected outright (v0 stays
  boolean-free, matching Macsyma's identical scope) — the first language
  in this rollout where boolean literals are even syntactically possible
  to reject (Derive/Reduce have no boolean literal tokens at all).
* Comparisons, `and`/`or`/`not`, and `^` are rejected outright (no
  inert-data fallback), for the identical reason as every sibling
  frontend. `if`/`then`/`elif`/`else`/`end if`, `[...]` list literals,
  and `{...}` set literals are rejected outright too.
* `postfix`'s call suffix is syntactically non-chainable in Maple's own
  grammar (a single optional suffix, not a repetition) — `f(x)(y)` is a
  parse error, not a lowering error, confirmed by a dedicated regression
  test.
* 27 unit tests (every accepted construct round-tripped through
  `maple-vm`, every rejected construct's explicit-error path, plus the
  `;`-vs-`:` statement-terminator equivalence and the postfix
  non-chainability regression) + a 18-case oracle test cross-checking
  against `maple-runtime`, both rendered through the same `print_maple`
  function.
