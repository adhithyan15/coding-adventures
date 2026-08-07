# Changelog

## [0.1.0] - 2026-07-27

### Added

- Initial grammar-driven Rust Axiom parser (MA13 §6, task MA-13c), following
  MA-13a's design-only kickoff spec
  ([`MA13-axiom-language.md`](../../../specs/MA13-axiom-language.md)) and
  MA-13b's `axiom-lexer` (merged, PR #8997).
- `code/grammars/axiom/axiom.grammar`, implementing MA13 §4's full
  consumer-view surface:
  - The ordinary infix arithmetic/comparison cascade (loosest to tightest):
    `comparison` (`= ~= < <= > >=`, non-chaining) → `coercion` (`::`,
    non-chaining) → `additive` (`+ -`) → `multiplicative` (`* /`) → `unary`
    (prefix `-`) → `power` (`^`/`**`, the same operator, right-associative)
    → `postfix` (function application) → `atom`.
  - `postfix`/`call_args`: `f(a, b)` and the paren-optional single-argument
    form `f a` (`factorial 7`, `ff z`), unified into one production, with
    the paren-optional branch restricted to a single bare `atom` — never a
    further operator-led expression — so `f -1`/`f +1` unambiguously parse
    as subtraction/addition, never a call with a signed-literal argument
    (no lookahead predicate needed; a structural consequence of `atom`
    having no unary-minus/-plus alternative).
  - `assignment` (`x := e`, bare-`NAME` left-hand side) and `define`
    (`f(x: T, ...): T == e` / `f x == e`, held-body function definition) as
    two entirely separate productions — a genuine departure from
    `derive-parser`'s/`reduce-parser`'s single shared `:=` production, since
    Axiom's declared-function-definition form needs a typed parameter list
    no ordinary call `arglist` can express. Both the declared form's
    per-parameter type annotations and its return-type annotation are
    REQUIRED (the narrower, spec-literal reading of MA13 §4's own single
    confirmed row — no untyped-parenthesised or optional-return-type
    variant is accepted).
  - `if_expr` (`if p then e1 else e2`) — `else` MANDATORY in this cut (MA13
    §4: "missing else — deferred"), unlike `reduce.grammar`'s/
    `idl.grammar`'s own optional `else`.
  - `group` (`( e1; e2; ...; eN )`): unifies plain grouping and the
    parenthesised, `;`-separated block into ONE rule (distinguished by
    child count at a later lowering layer), mirroring `derive.grammar`'s
    own vector/matrix-row-counting convention.
  - `declaration` (`a : T`, `(a, b, c) : T`) — left-hand side narrowed to a
    bare `NAME` or parenthesised name-list, unlike `coercion`'s fully
    general expression left-hand side (`e :: T`) — a real, disclosed
    asymmetry: `has`'s and `declaration`'s operands are always
    domain/category-shaped per MA13 §3's fixed lookup-table design, while
    `coercion`'s left-hand side genuinely can be an arbitrary computed
    value (MA13's own `3 :: Fraction Integer` example).
  - `has_query` (`D has C`) — deliberately its OWN top-level `expr`
    alternative, not folded into the arithmetic cascade the way `coercion`
    is (both operands are always `type_expr`-shaped, never arithmetic
    values, per every one of MA13's own worked examples).
  - `type_expr`/`type_ctor_args` — a domain/category-shaped expression,
    structurally close to `postfix`/`atom` (both are "a NAME, optionally
    applied to further arguments" — Axiom's own domain construction reuses
    ordinary call syntax, MA13 §3) but kept as its own named rule for CST
    clarity, mirroring `idl-parser`'s `index_suffix`/`call_suffix` naming
    discipline. The paren-optional constructor-argument shorthand
    (`Fraction Integer`) is deliberately restricted to a single BARE `NAME`
    (never a further nested `type_expr` with its own args) — a conservative
    narrowing avoiding both unconfirmed syntax (MA13's only paren-optional
    example is the simple two-level case; every richer nesting example uses
    fully explicit parens) and an adversarial "long chain of bare
    identifiers, one recursion frame per identifier, no delimiter to pay
    for it" shape that every other recursive production in this grammar
    avoids by costing at least one real input character per level.
  - `program = expr` — parses exactly ONE top-level expression, not a
    repeated multi-statement worksheet. `axiom.tokens` gives top-level
    inputs no separator at all (no significant newline, unlike Derive; `;`
    reserved exclusively for the parenthesised block, unlike Reduce's
    `;`/`$`) — a direct, disclosed consequence of MA13 §5 framing Axiom as
    a numbered, per-line interactive session (mirrored by a future
    `axiom-repl`'s own step-counted prompt) rather than a batch worksheet.
- `code/packages/rust/axiom-parser/`, wrapping the shared `GrammarParser`
  (no hand-written parsing logic, per `lessons.md`'s
  `feedback_no_handwritten_lexers_parsers`):
  - `examples/regenerate_grammar.rs` — regenerates `src/_grammar.rs` from
    `axiom.grammar`, mirroring `prolog-parser`'s own regen example.
  - `MAX_RULE_DEPTH = 140`, independently measured (not assumed from a
    sibling or from `parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`)
    against four structurally distinct recursive shapes — parenthesised
    grouping/block nesting, nested function-call arguments, a unary prefix
    chain, and a power chain — via binary search on an uncapped parser, one
    subprocess per data point, on a 2 MiB-stack worker thread, in a debug
    build. Nesting-count floors: parens 27/28, calls 24/25, unary 201/202,
    power 100/101. Rule-frame floors (converted via binary search over
    `with_max_depth` against a fixed 5000-level input per shape): parens
    282/283, calls **211/212** (the binding constraint — notably NOT
    parenthesised grouping, unlike `derive-parser`/`reduce-parser`'s own
    dominant shape), unary 212/213, power 213/214. `140` sits about 33.6%
    below the binding 211 floor (comparable margin to `derive-parser`'s own
    ~33%). Measured real-input headroom at 140 (capped parser, no crash
    risk): parens/calls parse cleanly to 13 levels (14 trips), unary to 130
    levels (131 trips), power to 65 levels (66 trips) — all well beyond any
    hand-written Axiom expression. The measurement harness itself (a
    throwaway `examples/depth_probe.rs` plus a shell binary-search script)
    was not committed, matching `derive-parser`/`reduce-parser`/
    `idl-parser`'s own precedent of not keeping a permanent measurement
    tool.
- `code/packages/rust/Cargo.toml` workspace registration (`axiom-parser`
  added directly after `axiom-lexer`; the sibling `axiom-runtime`/
  `axiom-repl`/`axiom-to-semantic-ir` crates do not exist yet — MA-13d/e,
  separate follow-on tasks).
- 58 unit tests + 1 doc test covering every grammar rule in MA13 §4's scope
  table: every literal shape; both call forms (explicit-parens and
  paren-optional) including the `f -1`/`f +1` disambiguation regression;
  empty and non-empty list literals; arithmetic precedence/associativity
  (including the `^`/`**` same-operator and right-associativity checks);
  every comparison operator; `:=` including right-associative chaining;
  both `==` definition forms including the "requires every parameter typed"
  and "requires a return type" narrowing regressions and the
  "`f x` alone is a call, `f x == e` is a definition" disambiguation;
  `if`-`then`-`else` including the mandatory-`else` regression, dangling-else
  resolution, and "usable as an expression"; the parenthesised block/group
  unification (verified by counting `expr` children); plain and tuple
  declarations, including a parameterized-domain type; coercion, including
  the paren-optional type shorthand and both precedence-boundary checks
  (binds tighter than comparison, looser than additive); `has` queries
  (both of MA13's own true/false worked examples), including the
  "unreachable as a bare arithmetic operand, but reachable through explicit
  parens" pair; `type_expr`'s own paren-optional-argument-must-be-bare-name
  restriction; `--` comments; syntax-error reporting including a panicking
  and a `Result`-returning entry point; and a full depth-guard regression
  suite (deep-input-returns-error-not-crash and cap-trips-before-native-
  overflow on a default ~2 MiB stack thread, for all four measured shapes,
  plus exact real-input headroom boundary tests pinned to the measured
  values above).
