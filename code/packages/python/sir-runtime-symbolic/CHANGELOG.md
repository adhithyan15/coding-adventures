# Changelog

## 0.1.0 — initial release (SIR23 Tier A pattern matcher, Phase A Slice 4)

First release of the SIR23 Tier A symbolic-expression + pattern/rewrite
runtime for Python, part of the SIR22/SIR23 second-wave backend-expansion
initiative (`code/specs/SIR23-symbolic-pattern-semantic-ir.md`'s "Backend
impact" section). Python port of the published
`@coding-adventures/sir-runtime-symbolic` TypeScript package, following the
TypeScript backend's *imported-package* model rather than this stack's
usual inlined-runtime convention — the same model this repo's own
`coding-adventures-sir-runtime-array` (SIR22) package already established
for Python.

**Tier A only.** Implements exactly the seven `Expr` variants the SIR23
spec designates Tier A: `SymSymbol`, `SymRational`, `SymApply`,
`SymPatternBlank`, `SymPatternNamed`, `SymRule`, `SymReplaceAll`. Tier B (a
general expression evaluator — `Add`/`Sin`/`D`/user-function-dispatch
folding) is explicitly out of scope; a `SymApply` builds an inert term
tree, nothing more.

### Added

- `sym` / `int_` (re-exported as `int`) / `number_node` / `rational` /
  `string_node` / `apply` — leaf/compound term constructors, thin wrappers
  over `coding-adventures-symbolic-ir`'s `IRSymbol`/`IRInteger`/`IRFloat`/
  `IRRational`/`IRString`/`IRApply` dataclasses.
- `BLANK` / `PATTERN` / `RULE` / `RULE_DELAYED` / `is_blank` / `is_pattern`
  / `is_rule` — re-exported unchanged from
  `coding-adventures-cas-pattern-matching`.
- `blank` / `blank_typed` — wrap `cas_pattern_matching.Blank`'s single
  `head: str | None` parameter as two separate functions, matching the
  published TypeScript sibling package's own `blank`/`blankTyped` split.
- `named` / `rule` / `rule_delayed` — wrap `cas_pattern_matching`'s
  `Pattern`/`Rule`/`RuleDelayed` under SIR23's own vocabulary names.
- `match_pattern` — thin wrapper over `cas_pattern_matching.match`, renamed
  to match the published TypeScript sibling package's own `matchPattern`.
- `substitute` — a small, self-contained reimplementation of
  `cas_pattern_matching.rewriter`'s private `_substitute` helper (that
  package does not export `substitute` publicly, unlike its TypeScript
  counterpart), built only on `cas_pattern_matching`'s public
  `is_pattern`/`pattern_name` surface.
- `apply_rule` — unmodified re-export of `cas_pattern_matching.apply_rule`.
- `Bindings` — re-exported unchanged from `cas_pattern_matching`.
- `replace_all(expr, rules) -> IRNode | DepthLimitError` — Wolfram `/.`:
  one bottom-up pass, first-match-wins per subtree, no retry. Genuinely new
  code — neither the Rust, TypeScript, nor Python `cas-pattern-matching`
  reference exposes this single-pass operator.
- `replace_repeated(expr, rules, max_iterations=100) -> IRNode |
  RewriteCycleError | DepthLimitError` — Wolfram `//.`: bottom-up traversal
  to a fixed point, reimplementing `cas_pattern_matching.rewrite`'s
  algorithm rather than calling it directly (that function has no
  recursion-depth parameter to hook a cap into).
- `MAX_TERM_DEPTH = 512` — the tree-walk recursion-depth cap, fixed at the
  SAME value as the TypeScript/JavaScript/Ruby sibling backends' own
  constant (deliberate cross-backend consistency, not independently
  re-derived).
- `DepthLimitError` / `RewriteCycleError` — dataclass error sentinels
  `replace_all`/`replace_repeated` return instead of raising directly.
- `unwrap(result) -> IRNode` — raises a plain `ValueError` on either
  sentinel; passes a real `IRNode` through unchanged.

### Security

- **CWE-674 (uncontrolled recursion) — found by this release's own
  `/security-review`: a rule's OWN `lhs`/`rhs` depth needed the SAME cap
  as the target tree, and initially did not have it.** An earlier draft of
  this package assumed `match_pattern`/`substitute`/`apply_rule` needed no
  cap because a rule's pattern/RHS is "authored by a compiler frontend...
  not by runtime data" — but nothing in this runtime enforces that: a
  compiled SIR23 program can build an arbitrarily deep `lhs`/`rhs` via an
  ordinary loop calling `apply`/`named` (the exact same constructors used
  to build the *target*), with zero dependency on the target's own depth.
  `replace_all(shallow_target, [rule(blank(), huge_rhs)])` raised an
  uncaught Python `RecursionError` from inside `substitute`/`match`'s own
  recursion (which the target-tree walk's cap never reaches, since that
  recursion is driven by the *rule's* structure) instead of the documented
  `DepthLimitError`. **Fixed** by validating every rule's `lhs`/`rhs`
  against `MAX_TERM_DEPTH` up front, via a new `_rules_exceed_depth`
  helper — critically, an ITERATIVE (explicit-stack, non-recursive) check,
  so checking a maliciously deep rule cannot itself overflow the stack —
  before `replace_all`/`replace_repeated` ever start walking the target.
  Regression-tested by `test_deep_rule_rhs_reports_depth_limit_error_not_
  a_crash`/`test_deep_rule_lhs_pattern_chain_reports_depth_limit_error_
  not_a_crash` (package-level) and, in `semantic-ir-to-python`'s own
  `tests/sir23_symbolic.rs`,
  `deep_rule_rhs_reports_depth_limit_error_not_a_crash_even_with_a_
  shallow_target` (a REAL compiled `for`-loop building the deep `rhs`, not
  a hand-built static AST). `match_pattern`/`substitute`/`apply_rule`
  themselves remain uncapped as raw, lower-level primitives (matching
  `cas_pattern_matching`'s own uncapped design) — SIR23 Tier A codegen
  never calls them directly, only through the now-guarded
  `replace_all`/`replace_repeated`.
- **`replace_repeated`'s retry-on-fire step is an iterative `while` loop at
  the SAME call frame, not a recursive call per firing** — the exact fix
  the published TypeScript sibling package's own `/security-review` found
  necessary: `cas_pattern_matching.rewrite()`'s naive port recurses once
  per rule firing at a single tree position, so a caller passing a large
  `max_iterations` could exhaust the native stack through that path alone,
  independent of `MAX_TERM_DEPTH` and independent of how deep or shallow
  the expression itself is. `test_survives_huge_max_iterations_without_a_
  stack_overflow` regression-tests this directly: a two-rule cycle (`a ->
  b`, `b -> a`) that never grows the tree, retried 50,000 times, resolves
  cleanly to `RewriteCycleError` instead of a stack overflow.
- **`max_iterations` (CPU-time bound) and `MAX_TERM_DEPTH` (stack-depth
  bound) are independent guards**, each regression-tested on its own axis
  (a deep-but-non-cycling tree hits only the depth cap; a cycling-but-
  shallow rule set hits only the iteration cap) — proving neither guard
  is silently doing the other's job.
- No `eval`/`exec`/dynamic code execution anywhere in this package.

### Full standard layout

`pyproject.toml` (src layout, depends on `coding-adventures-symbolic-ir`
and `coding-adventures-cas-pattern-matching`), `BUILD`, `BUILD_windows`,
`required_capabilities.json` (no capabilities), `py.typed`, README. pytest
suite well over the 95% coverage target; `mypy --strict` + `ruff` clean.
