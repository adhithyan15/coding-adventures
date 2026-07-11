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
- 61 tests: 51 unit tests over exact `Expr` shapes for every grammar
  production plus DoS-guard regressions (a 60,000-term flat chain, a
  60,000-deep chained bracket/part/pure-function-apply/`&`-run, a
  256×256-multiplicative bracket/index combination, deeply nested parens,
  and exact-boundary pairs), 9 validator/capability-rejection tests, 1
  doctest.
- Marks `wolfram-to-semantic-ir` done in `HML01-math-to-semantic-ir.md`'s
  Stream B rollout.

### Fixed (security review, before first push)

- **HIGH — DoS: chained postfix application/part/pure-function-apply
  groups bypassed the chain-length guard entirely.** The initial
  `check_chain_length` audit covered every flat operator-chain production
  the module doc comment enumerates (`additive`/`multiplicative`/
  `logical_or`/`logical_and`/`alternatives`/`mapapply`/`patterntest`/
  `replaceall`) but missed that `postfix` (`f[…][…][…]…`, `x[[…]][[…]]…`)
  and `amp`'s `&`-run/trailing-application suffixes (`expr & & &…`,
  `f&[…][…]…`) have the *identical* flat-repetition grammar shape — a long
  chain collapses into one CST node with many bracket-group/`&` children
  rather than nesting through parens, so it never recurses through the
  depth-capped `lower_node` at all. `lower_postfix`'s `while` loop and
  `lower_amp`'s wrapping loops iteratively rebuild `result` as the head (or
  `Part` target, or `Function` wrapper) of a brand-new node every
  iteration, producing an unboundedly deep tree with **no check ever
  firing** — confirmed via security review to crash (`SIGABRT`, real stack
  overflow) on a symbol followed by 60,000 chained `[0]` groups. Fixed by
  adding two new guards mirroring `check_chain_length`'s shape but counting
  tokens/child-node-kinds specific to each production
  (`check_postfix_chain_length` counts `LBRACKET`/`LDBRACKET` tokens;
  `check_amp_chain_length` counts both the `&`-run length and the trailing
  `amp_apply` suffix count), called before their respective loops run.
  Verified adversarially the same way the flat-chain guards were: disabled
  the two new checks, reproduced the same `SIGABRT` on the exact same input
  that had crashed pre-fix, then restored them and confirmed the dedicated
  regression tests pass.
- **HIGH — the fix above's own two new guards were independently
  bypassable, found in round 2 of security review.** `check_postfix_chain_length`
  capped the number of bracket/part *groups* to `MAX_EXPR_DEPTH`;
  `check_apply_arg_count` separately capped the number of indices/args
  *within* one group to `MAX_EXPR_DEPTH`. Those two axes multiply, not add —
  an `LDBRACKET` group folds one `Part` per index, so N chained groups each
  carrying M indices builds N×M levels of real nesting. Both per-axis caps
  individually passing (N ≤ 256 and M ≤ 256) still permitted up to
  256×256 = 65,536 levels, confirmed via security review to reproduce the
  same `SIGABRT` stack overflow at that scale. Fixed by replacing both
  per-production guards (`check_postfix_chain_length`/
  `check_amp_chain_length`) with a single cumulative `add_chain_depth`
  budget threaded through the whole `postfix`/`amp`/`amp_apply` chain,
  charging each group's own contribution (`args.len()`/`indices.len()`,
  floored at 1 — a safe conservative upper bound, since a plain
  non-associative call only adds one real level regardless of its
  argument count) against a single running total capped at
  `MAX_EXPR_DEPTH` for the *entire* chain, not per group. Verified
  adversarially: reproduced the 256×256 crash with the new guard disabled,
  then confirmed the fix rejects it cleanly and the dedicated regression
  test passes.
- **MEDIUM — `compile_source`'s panic handling defeated its own "fails
  cleanly" hardening guarantee.** `compile_source` is documented as the
  hardened entry point specifically so pathological input fails with a
  `Result::Err` instead of crashing the caller. But `.spawn(...).expect(...)`
  panicked the calling thread on OS thread-creation failure, and
  `.join().expect(...)` re-panicked the calling thread on *any* worker
  panic — silently defeating the guarantee for every failure mode short of
  an actual stack overflow (which aborts the whole process before `.join()`
  ever runs, and so was never masked by this bug). Fixed by converting both
  failure paths into a `WolframLowerError` instead of `.expect()`-ing.

### Known limitation (disclosed, not a bug)

No module this crate produces currently executes end-to-end through any
backend: under the "everything is data" design, even bare literal
arithmetic emits at least one SIR23 node, and `semantic-ir-to-javascript`
does not implement SIR23 codegen yet (it depends on `sir-runtime-symbolic`,
a separate, not-yet-shipped runtime library — HML01 Stream B rollout item
6). This crate's tests therefore verify structural correctness and the
capability-rejection path only; there is no e2e `node`-execution test,
unlike `matlab-to-semantic-ir`'s purely-literal case.
