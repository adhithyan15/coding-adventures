# Changelog

## [0.1.0] - 2026-07-11

### Added

- Initial `wolfram-to-semantic-ir` frontend crate (HML01 Stream B), the
  first to target SIR23 (symbolic-expression/pattern domain):
  `compile`/`compile_source` lowering `coding-adventures-wolfram-parser`'s
  `GrammarASTNode` CST into a `semantic_ir::Module`.
- Design decision ("everything is data", see `src/lower.rs`'s module doc
  comment): every Wolfram construct — arithmetic, comparisons, lists,
  function application, `=`/`:=` assignment — lowers to the SIR23 symbolic
  vocabulary (`SymSymbol`/`SymApply`/pattern/rule nodes), never to a
  host-language variable/binding. No environment, no evaluation happens at
  lowering time; that is deliberately left to a future backend runtime
  library (`sir-runtime-symbolic`, not yet built).
- Because every construct reduces to the same small vocabulary, this crate
  covers the full grammar surface `wolfram-parser` accepts: literals,
  arithmetic (`+ - * / ^`, unary, and explicit `Plus[…]`/`Times[…]`/…
  head-application forms with the same associative n-ary left-fold the
  native `wolfram-runtime` uses), comparisons, logic, lists, arbitrary
  function application (including a computed head, `f[x][y]`), `Set`/
  `SetDelayed`, pattern blanks (`_`/`_h`) and named patterns (`x_`/`x_h`),
  rules (`->`/`:>`) with the same pattern-name-to-reference RHS rewriting
  the native lowering performs, replacement (`/.`/`//.`) with rule-list
  flattening, and the W-6 (`/@`/`@@`/`[[ ]]`), W-11 (`#`/`#n`/`##`/`&`/
  `Function`), and W-21 (`|`/`/;`/`?`) operator sugar.
- Recursion-depth hardening applied from day one, not retrofitted: every
  flat operator-chain fold (`additive`/`multiplicative`/`logical_or`/
  `logical_and`/`alternatives`/`mapapply`/`patterntest`/`replaceall`) is
  capped at `MAX_EXPR_DEPTH` (256) operands before any tree is built
  (`check_chain_length`), and every `f[…]`/`{…}`/`[[…]]` argument list is
  capped at the same bound (`check_apply_arg_count`) as a modest
  allocation-size backstop. `compile_source` parses on an enlarged-stack
  (512 MiB) worker thread, reusing `wolfram-runtime`'s own validated-safe
  `EVAL_STACK_SIZE` deployment rather than inventing an unproven new
  configuration — see `wolfram-parser`'s own `MAX_RULE_DEPTH` doc comment
  for why no single cap can be both bare-stack-safe and support realistic
  nesting for this grammar.
- Verified the chain-length guard adversarially before shipping (not just
  asserted as correct): temporarily removed `check_chain_length`'s calls
  and re-ran the 60,000-term regression test, confirming a real `SIGABRT`
  native stack overflow without the guard, then restored it and confirmed
  the same test passes cleanly. Mirrors `matlab-to-semantic-ir`'s own
  post-hoc discovery of this exact bug class, applied here proactively.
- 55 tests: 44 unit tests over exact `Expr` shapes for every grammar
  production plus DoS-guard regressions (a 60,000-term flat chain, deeply
  nested parens, and an exact-boundary pair at `MAX_EXPR_DEPTH`/
  `MAX_EXPR_DEPTH + 1` operands), 9 validator/capability-rejection tests,
  1 doctest.
- Marks `wolfram-to-semantic-ir` done in `HML01-math-to-semantic-ir.md`'s
  Stream B rollout.

### Known limitation (disclosed, not a bug)

No module this crate produces currently executes end-to-end through any
backend: under the "everything is data" design, even bare literal
arithmetic emits at least one SIR23 node, and `semantic-ir-to-javascript`
does not implement SIR23 codegen yet (it depends on `sir-runtime-symbolic`,
a separate, not-yet-shipped runtime library — HML01 Stream B rollout item
6). This crate's tests therefore verify structural correctness and the
capability-rejection path only; there is no e2e `node`-execution test,
unlike `matlab-to-semantic-ir`'s purely-literal case.
