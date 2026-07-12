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
- 62 tests: 52 unit tests over exact `Expr` shapes for every grammar
  production plus DoS-guard regressions (flat-chain, chained
  bracket/part/pure-function-apply/`&`-run, multiplicative bracket/index
  combination, cross-`(...)`-boundary composition, deeply nested parens,
  and exact-boundary cases — see the "Fixed (CI)" entry below for why
  these run at a smaller scale than the incidents that originally
  motivated them), 9 validator/capability-rejection tests, 1 doctest.
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
- **HIGH — the round-2 fix's cumulative budget was scoped per grammar
  node, not globally, found in round 3 (final) of security review.**
  `add_chain_depth`'s running total correctly bounds nesting *within one*
  `postfix`/`amp` node, but resets fresh (`= 0`) on every call — it has no
  awareness that the node's own *base* (the "atom" the chain applies to)
  might already be an arbitrarily deep tree, built either by ordinary
  legitimate `(...)` nesting or by a *different* instance of the same
  chain-fold pattern one level up. Wrapping an already-at-cap chain in
  parentheses and appending another full chain — repeated across as many
  `(...)` boundaries as `wolfram-parser`'s own real-nesting limit allows
  (see that crate's `MAX_RULE_DEPTH` doc comment — around 98 levels on
  `compile_source`'s enlarged-stack thread) — composes each individually
  in-bounds chain multiplicatively, reaching tens of thousands of true
  nesting levels overall; confirmed via security review that this
  structurally bypasses every existing per-node guard (`add_chain_depth`,
  `check_chain_length`, and the ordinary CST-recursion `depth` parameter,
  which only costs ~2 units per `(...)` wrap — nowhere near enough to
  itself trip `MAX_EXPR_DEPTH`). The same composition gap applies in
  principle to every other flat-chain production (`additive`/
  `multiplicative`/`logical_or`/`logical_and`/`alternatives`/`mapapply`/
  `patterntest`/`replaceall`), not just `postfix`/`amp` — none of them
  account for their base operand's pre-existing depth either. Fixed with a
  different, more fundamental mechanism rather than another per-construct
  patch: constructing a deeply-nested `Box`-based tree costs only heap,
  not stack (each construction step is O(1) regardless of how the tree
  eventually gets used), so the risk is entirely in *walking* it
  recursively afterward — not in building it. Added
  `measure_depth_iterative`, an authoritative depth check that walks an
  already-built `Expr` using an explicit heap-allocated work stack (never
  native recursion, so it can never itself crash regardless of how deep
  the input already is, and bails out the moment depth is certain to
  exceed the cap rather than doing unbounded work). Called on `lhs`/`rhs`
  in `lower_rule` before either is handed to `collect_pattern_names`/
  `bind_pattern_refs` (the two functions in this crate that already
  recursed without their own cap, on the — now corrected — assumption that
  `MAX_EXPR_DEPTH` alone already bounded their input), and once per
  top-level statement in `lower_file` before it can reach the returned
  `Module` (protecting `semantic_ir::validate`, any backend, and — since
  `semantic_ir::Expr` has no custom `Drop` impl — the unconditional,
  unguarded recursive `Drop` glue that runs whenever a caller lets a
  `Module` go out of scope). This closes the gap regardless of how the
  oversized tree was composed, rather than requiring a bespoke guard for
  every new way constructs might chain together. Verified adversarially:
  temporarily disabled both new call sites, confirmed a composed chain
  that individually stays under every per-node cap (20 levels of
  `(...)`-wrapping around a 20-group bracket chain, 400 true nesting
  levels) is wrongly *accepted* — proving the structural bypass — then
  restored the fix and confirmed it is correctly rejected. (A larger,
  ~97-level/256-groups-per-level construction, matching the crate's other
  60,000-scale regressions, was also confirmed to be correctly rejected
  with the fix in place; it was not kept as a checked-in regression test
  because `wolfram-parser`'s own O(n) packrat-memo lookup — tracked
  separately as a performance follow-up, not a correctness issue — makes
  parsing it alone take minutes.)
- **HIGH — the round-3 fix's own rejection path recursively dropped the
  tree it had just detected as too deep, found in round 4 (final) of
  security review.** `measure_depth_iterative` correctly *detects* a
  pathologically deep tree and returns `None`, but detecting the problem
  doesn't dispose of it: the code then did `return Err(...)`, letting
  `lhs`/`rhs`/`expr` fall out of scope normally — since `semantic_ir::Expr`
  has no custom `Drop` impl, that invokes the compiler-derived *recursive*
  drop glue on the very tree just found to be too deep, exactly the same
  native-stack-overflow risk this whole fix history exists to eliminate,
  just relocated from "walking the tree forward" (validator/backend/the
  caller's own eventual `Drop` of an accepted `Module`) to "walking it
  backward" (this crate's own `Drop` of the value it's about to reject).
  This directly falsified `compile()`'s own documented safety claim ("safe
  to call directly on an ordinary thread"). Confirmed empirically via an
  isolated subprocess (not just reasoned about): calling `compile()`
  directly on a bare default-stack thread with a rejected, composed
  ~23,040-level-deep tree (90 levels of `(...)` wrapping around a 256-group
  bracket chain — well within `wolfram-parser`'s own real-nesting ceiling)
  produced a genuine `SIGABRT` stack overflow; the same construction at
  9,000 and 4,500 levels survived, bracketing the actual crash floor for
  ordinary recursive `Drop` somewhere in between. Fixed by adding
  `drop_iterative`, which takes ownership of a rejected tree and tears it
  down using an explicit heap-allocated work stack — moving each nested
  `Expr` field out via the match instead of leaving it to be dropped
  recursively as part of the outer value, the same technique a
  hand-written `impl Drop for List` uses to avoid overflowing on a long
  linked list, generalised from a list to a tree — called at both
  rejection sites (`lower_rule`, `lower_file`) before returning the error.
  Verified adversarially: re-ran the exact isolated-subprocess repro that
  crashed at ~23,040 levels with the fix in place and confirmed it now
  survives cleanly.
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

### Fixed (CI)

- **`PARSE_STACK_SIZE` reduced from 512 MiB to 64 MiB.** The initial value
  copied `wolfram-runtime`'s own `EVAL_STACK_SIZE` verbatim, but that crate
  spawns its enlarged-stack thread rarely (a long-running REPL/eval
  process), while `compile_source` may spawn one per call — many
  concurrent test threads each reserving 512 MiB of address space at once
  caused real CI resource pressure (the "Build and test affected packages"
  job was externally terminated, exit code 143, while building this
  crate). `wolfram-parser`'s own measured bare-stack crash floor (~276
  frames on ~2 MiB, ~7.4 KiB/frame) means supporting its full default
  `MAX_RULE_DEPTH` (2000 frames) with a comfortable ~4x margin needs only
  ~64 MiB, a fraction of 512 MiB's reservation with the same safety
  guarantee. Verified directly (not just reasoned about): a 90-level
  legitimate `(...)` nesting — comfortably inside `wolfram-parser`'s own
  measured ~98-level safe ceiling — still parses and lowers successfully
  through `compile_source` at the reduced stack size.

- **Real root cause of the CI exit-143 failures: several DoS-regression
  tests were slow to *parse*, not just slow to lower.** The stack-size
  reduction above was a reasonable, low-risk change but did not fix the
  failure — a fresh CI run at that commit failed identically (same exit
  code, same package, near-identical elapsed time on it). Confirmed by
  simulating a resource-constrained runner locally (`cargo test --
  --test-threads=2`): the whole suite took 47.85s (vs ~25s at full local
  parallelism), and four individual tests each took 15-16 seconds alone —
  the ones building a 60,000-element chained bracket/part/pure-function-
  apply construct, and the multiplicative 256×256 combination test.
  `wolfram-parser` must fully tokenize and parse a flat chain of that size
  *before* this crate's own lowering guards ever get a chance to reject
  it, and that parser's own packrat-memo lookup is a known, separately
  tracked O(n) performance concern (not a correctness issue) that scales
  badly at this size. Fixed by reducing these tests' scale (60,000 → 3,000
  for flat/chained constructs; 256×256 → 30×30 for the multiplicative
  combination) — comfortably past `MAX_EXPR_DEPTH` (256) still proves the
  guard rejects the input equally well, at a fraction of the parse cost;
  confirmed the reduced suite now completes in ~4s under the same
  `--test-threads=2` simulation. The original larger scales remain
  confirmed (via throwaway, not-checked-in adversarial repros during
  security review, per the entries above) to reproduce real crashes with
  each guard disabled — only the *permanent, always-run* regression tests
  were rescaled, not the adversarial-verification methodology itself.

### Known limitation (disclosed, not a bug)

No module this crate produces currently executes end-to-end through any
backend: under the "everything is data" design, even bare literal
arithmetic emits at least one SIR23 node, and `semantic-ir-to-javascript`
does not implement SIR23 codegen yet (it depends on `sir-runtime-symbolic`,
a separate, not-yet-shipped runtime library — HML01 Stream B rollout item
6). This crate's tests therefore verify structural correctness and the
capability-rejection path only; there is no e2e `node`-execution test,
unlike `matlab-to-semantic-ir`'s purely-literal case.
