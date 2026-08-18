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
* **Security-review fix (before this crate's first push, not a
  post-release patch): `compile_source` now parses on an enlarged-stack
  worker thread, mirroring `wolfram-to-semantic-ir::compile_source`
  exactly.** The first draft incorrectly reasoned by analogy to
  `macsyma-iir-compiler::compile_source` (no worker thread needed) —
  invalid for Wolfram specifically: `wolfram-parser`'s own
  `MAX_RULE_DEPTH` doc comment documents a bare-stack crash floor of only
  ~11 real nesting levels (its 20-rule-per-level precedence cascade costs
  ~20 `parse_rule` frames per level of ordinary `(...)` nesting), so
  trivially small syntactically-valid input (e.g. 300 levels of
  parenthesization) crashed the process natively instead of returning a
  clean `Err`. Fixed with the identical `PARSE_STACK_SIZE` (64 MiB)
  worker-thread pattern `wolfram-to-semantic-ir` already established;
  `deeply_nested_parens_fail_cleanly_not_natively` is the regression
  test. The other five Wave 5 frontends' own no-worker-thread
  `compile_source` shape is correct for THEM (their parsers' shallower
  precedence cascades really are bare-stack-safe) — this was a
  Wolfram-specific gap, not a rollout-wide pattern error.
* 36 unit tests (every accepted construct round-tripped through
  `wolfram-vm`, every rejected construct's explicit-error path,
  including the full pattern/rule/replacement/pure-function surface, and
  the deep-nesting stack-safety regression) + a 19-case oracle test
  cross-checking against `wolfram-runtime`, both rendered through the
  same `print_wolfram` function.
