# Changelog — macsyma-iir-compiler

## v0.1.0 — 2026-08-14 — initial release (macsyma-iir-vm.md, v0)

The first frontend to bridge a math language onto `interpreter_ir` (IIR),
the shared AOT/lang-vm chain — a third, independent retarget of the
`GrammarASTNode` CST `macsyma-compiler` and `macsyma-to-semantic-ir`
already walk.

* `compile(&GrammarASTNode, module_name) -> Result<IIRModule, MacsymaIirError>`
  and `compile_source(source, module_name)`.
* Accepted v0 grammar: integer literals; `+ - * /` (chains + unary);
  assignment (`x: expr`); free-symbol references; any other
  operator/head, or any operand involving a free symbol, lowers to an
  inert `cons`-chain (an unevaluated symbolic `Apply` node), mirroring
  `mccarthy-lisp-iir-compiler::lower_quote`'s `QUOTE` shape.
* Concrete `+`/`-`/`*` always evaluate via a real `call_builtin` (executed
  by the VM, never frontend-folded) since integer arithmetic is always
  exact. `/` gets a narrower rule: only evaluates when both direct
  operands are literal integer tokens whose quotient is exact; anything
  else involving `/` (a non-exact literal division, or a concrete-but-
  non-literal operand) is an explicit compile error rather than a silent
  wrong answer — see `src/lower.rs`'s module doc comment for the full
  rationale (`dynval-runtime`'s `/` builtin truncates, and Macsyma's own
  exact-`Rational` result for a non-exact division isn't representable in
  v0).
* Comparisons, `and`/`or`/`not`, and `^`/`**` are rejected outright (no
  inert-data fallback) — `macsyma-runtime`'s real evaluator numerically
  evaluates concrete instances of these (`2^3` → `8`, `3<5` → `true`),
  unlike `+`/`-`/`*`, so building inert data for them would silently
  disagree with ground truth.
* Every other out-of-scope construct — `Rational`/`Float`/`Str` literals,
  `:=` (function definition), `if`/`while`/`for`/`block`/`return`, list
  literals, postfix function calls — returns an explicit
  `MacsymaIirError` citing `macsyma-iir-vm.md` §4, never a silent
  mis-lowering.
* 25 unit tests: every accepted construct (round-tripped through
  `macsyma-vm` as a dev-dependency) and every rejected construct's
  explicit-error path, including the `/` exactness boundary cases
  (`20/4` accepted, `-4/2` accepted via literal-negation propagation,
  `7/2` and `x:6$ x/2` both rejected, division by literal zero rejected).
* A cross-checked oracle test against `macsyma-runtime` is a follow-up PR
  (macsyma-iir-vm.md §8, PR 4) — not included here.
