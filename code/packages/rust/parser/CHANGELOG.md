# Changelog

All notable changes to the `parser` crate will be documented in this file.

## [0.4.2] - 2026-07-13

### Fixed — packrat memo / left-recursion-guard hot path no longer allocates a `String` per lookup

`GrammarParser::parse_rule_inner` looked up and inserted into its packrat
`memo` cache and its `in_progress` left-recursion guard via a
`format!("{},{}", rule_idx, pos)`-allocated `String` key — on *every* rule
attempted at *every* token position, for every grammar built on this crate
(flagged as follow-up work in `wolfram-parser`'s own `MAX_RULE_DEPTH` doc
comment, since Wolfram's dense rule-chain grammar makes the hot-path cost
most visible there, but the cost applied to all ~130 downstream consumers
equally). Changed both `memo: HashMap<String, MemoEntry>` and
`in_progress: HashSet<String>` to key on a plain `(usize, usize)` tuple
instead — no allocation, and hashing/equality on two `usize`s rather than a
formatted string.

Also fixed `record_failure`'s furthest-expected-position tracking, which
allocated `expected.to_string()` on *every* call just to check
`!v.contains(&expected.to_string())` — including the overwhelmingly common
case where the expectation was already recorded and nothing new needed to
be pushed. Changed to `!v.iter().any(|s| s.as_str() == expected)`, which
compares against the existing `&str`s directly and only allocates once a
push is actually needed. Added `test_furthest_failure_expectations_are_deduplicated`
to lock in that the dedup behavior itself (not just its allocation cost)
stayed exactly the same.

Purely an internal-state change — `memo`, `in_progress`, and
`record_failure` are all private; no public API changed, and every
existing test (this crate's own 41, plus a downstream sample across
`wolfram-parser`/`macsyma-parser`/`apl-parser`/`j-parser`/`matlab-parser`/
`ruby-parser`/`python-parser`, ~400 tests total) passes unchanged.

## [0.4.1] - 2026-06-30

### Fixed — recursion-depth guard is now OPT-IN (default is unlimited)

0.4.0 turned the guard ON for **every** caller by defaulting `new()` to
`DEFAULT_MAX_RULE_DEPTH` (128). That global default cap is unsound: **rule-chain
depth ≠ source-nesting depth**. A rich grammar spends many rule-frames per
source-nesting level, so any single cap low enough to sit below the native-stack
overflow point on the default stack (~200 frames) rejects legitimate *moderate*
nesting on richer grammars — and it also preempts frontends that already guard
themselves on an enlarged stack.

Two downstream consumers broke under the 0.4.0 default cap:

- **wolfram-runtime** — `moderate_nesting_still_evaluates` parses 40 legitimate
  nested parens; the Wolfram grammar spends ~30 rule-frames per paren, so 40
  parens ≈ 1280 frames tripped the 128 cap → a real regression.
- **python-to-semantic-ir** — its deep-nesting tests deliberately run the parse
  on a 64 MiB worker stack so the *lowerer's* own 256-level depth check is what
  fires; the parser's 128 cap preempted it with a different error.

**Fix:** `new()` now defaults `max_depth` to `usize::MAX` (unlimited),
restoring 0.4.0-pre behaviour for every existing frontend. The guard is opt-in:
callers that parse untrusted input on the default stack dial it in with
`.with_max_depth(DEFAULT_MAX_RULE_DEPTH)`. (closurec opts in at its ASI parse
sites — see `coding-adventures-javascript-parser`.) `DEFAULT_MAX_RULE_DEPTH`
stays as the recommended value for opt-in callers; its doc no longer claims to
be "far above any real program's nesting" for *all* grammars (only for the
JS-shaped grammars that opt in).

## [0.4.0] - 2026-06-30

### Fixed — recursion-depth guard against native stack overflow (DoS)

`GrammarParser`'s recursive descent (`parse_rule` → `match_element` →
`parse_rule` for nested rule references) previously had **no bound on nesting
depth**. The existing left-recursion guard (`in_progress`) only breaks *left*
recursion; it does nothing for deep *right* recursion / nesting such as
`((((…))))` or `[[[…]]]`, where every extra layer is a fresh `(rule, pos)`
pair the memo never short-circuits. Sufficiently deep input therefore recursed
once per layer and **overflowed the native thread stack** — an *uncatchable*
process abort, not a recoverable error. Because every SIR frontend
(twig / ruby / python / javascript) reaches this parser through its public
entry, a few-hundred-deep nested literal could crash the host process *before*
the frontend's own source-level depth checks could fire.

The parser now tracks recursion depth and refuses to descend past a cap,
returning a clean, recoverable `GrammarParseError`
("input nests deeper than the supported limit (N)") instead of overflowing.

- Added a `depth` counter to `GrammarParser`, incremented on entry to
  `parse_rule` and decremented on exit via a thin wrapper around the
  (renamed) memoizing core `parse_rule_inner`, so the count is exact across
  all of the inner function's early-return paths (memo hit, left-recursion
  break, success, failure).
- Added `pub const DEFAULT_MAX_RULE_DEPTH: usize = 128`. The cap was chosen
  empirically: this implementation overflows the default ~2 MiB thread stack
  somewhere around depth ~200 in a debug build (release frames are smaller,
  so the overflow point only rises), and 128 trips the clean error with
  comfortable margin *below* that on the default stack — while sitting at 2×
  the SIR frontends' source-level `MAX_PAREN_DEPTH` (64), far above any real
  program's nesting. No real input is rejected; every existing test and every
  dependent language parser passes unchanged.
- Added `GrammarParser::with_max_depth(usize) -> Self` (builder-style) to
  override the cap, primarily for cheap, deterministic depth-guard testing.
- 4 new regression tests in `grammar_parser::tests`:
  - `test_deeply_nested_input_returns_error_not_overflow` — 5000 nested parens
    on a 32 MiB worker thread returns the depth-limit `Err`, never crashes.
  - `test_default_cap_trips_before_overflow_on_default_stack` — proves the
    default cap fires *before* native overflow on a default-stack thread.
  - `test_nesting_up_to_cap_still_parses` — input within the cap parses
    identically (no-regression half of the contract).
  - `test_low_cap_trips_depth_guard` — a lowered cap trips on shallower input
    with the precise depth-limit message.

No change to behaviour for any input that nests below the cap (i.e. every real
program and every existing test): public AST shape, error messages for genuine
syntax errors, memoization, and left-recursion handling are all unchanged.

## [0.3.1] - 2026-06-29

### Changed — adapt to `lexer::Token` gaining a `cv` field (CLOC27 P1)

`GrammarParser` and `Parser` internal `Token` construction (including test
helpers) now set `cv: None`. Mechanical adaptation to `lexer` 0.7.0; no public
API or behaviour change (all parser tests pass unchanged). Also reconciles the
crate version with this changelog's numbering.

## [0.3.0] - 2026-04-04

### Added
- `GrammarASTNode` position fields: `start_line`, `start_column`,
  `end_line`, `end_column` (all `Option<usize>`) — computed from the
  first and last leaf tokens in the node's children.
- `compute_node_position`, `find_first_token`, `find_last_token` —
  helper functions for AST node position computation.
- `ASTVisitor` trait with `enter`/`leave` callbacks for AST traversal.
- `walk_ast(node, visitor)` — depth-first walk with enter/leave phases;
  visitor callbacks can return replacement nodes.
- `find_nodes(node, rule_name)` — collect all nodes matching a rule name.
- `collect_tokens(node, type_filter)` — collect all tokens in depth-first
  order, optionally filtered by type name.
- `match_element` arms for new `GrammarElement` variants:
  - `PositiveLookahead` — succeeds without consuming input if inner matches.
  - `NegativeLookahead` — succeeds without consuming input if inner fails.
  - `OneOrMore` — matches one required then zero or more additional.
  - `SeparatedRepetition` — matches element { separator element } pattern.
- `element_references_newline` updated for new variants.
- New exports from `lib.rs`: `ASTNodeOrToken`, `ASTVisitor`, `walk_ast`,
  `find_nodes`, `collect_tokens`.

## [0.2.0] - 2026-03-23

### Added

- `GrammarParser::new_with_trace(tokens, grammar, trace: bool)` constructor
  - When `trace = true`, emits a `[TRACE]` line to stderr for every grammar
    rule attempt, showing the rule name, token index, token type and value,
    and whether the rule matched or failed
  - Format: `[TRACE] rule '<name>' at token <index> (<TYPE> "<value>") → match|fail`
  - Trace output goes to stderr so it does not pollute parser return values
  - `new()` is now a thin wrapper over `new_with_trace(..., false)` (no behaviour change)
- Added `trace: bool` field to `GrammarParser` struct
- 4 new unit tests for trace mode in `grammar_parser::tests`:
  - `test_trace_mode_parse_succeeds` — trace does not affect parse correctness
  - `test_trace_mode_no_panic_on_failure` — trace does not panic on bad input
  - `test_trace_mode_addition` — multi-token sequence works in trace mode
  - `test_trace_false_same_as_new` — `new_with_trace(false)` == `new()`

## [0.1.0] - 2026-03-19

### Added

- `ast` module with `ASTNode` enum: `Number`, `String`, `Name`, `BinaryOp`, `Assignment`, `ExpressionStmt`, `Program`.
- `parser` module with hand-written recursive descent parser for a Python subset:
  - Arithmetic expressions with operator precedence (`*`/`/` before `+`/`-`).
  - Parenthesized sub-expressions.
  - Variable assignments (`x = expr`).
  - Multi-statement programs with newline separation.
  - `Result`-based error handling with `ParseError` type.
- `grammar_parser` module with grammar-driven parser:
  - `GrammarParser` that reads rules from a `ParserGrammar` (from `grammar-tools`).
  - Backtracking support for alternation.
  - Handles Sequence, Alternation, Repetition, Optional, Group, RuleReference, TokenReference, and Literal grammar elements.
  - `GrammarASTNode` with `rule_name` and `children` (either nested nodes or tokens).
  - `is_leaf()` and `token()` helper methods on `GrammarASTNode`.
- Comprehensive test suite covering:
  - Expression parsing (addition, multiplication, precedence, parentheses).
  - Statement parsing (assignments, expression statements).
  - Multi-statement programs and blank line handling.
  - Error cases (unexpected tokens).
  - Grammar-driven parsing (single values, addition, chaining, alternation, optional, literals, groups).
  - Integration tests using the lexer to tokenize source code before parsing.

## [0.1.1] - 2026-03-23

### Fixed

- **`match_token_reference` custom type disambiguation**: tokens with a `type_name` set (e.g. `IDENT`, `VARIABLE`, `FUNCTION`) would previously match any token reference whose grammar name maps to `TokenType::Name` as a fallback — because `string_to_token_type` returns `Name` for unknown names. For example, an `IDENT` token would match a `VARIABLE` reference even though they are different grammar-level types. The fix: when `expected_type` maps to `Name` but is not literally `"NAME"`, and the current token already has a specific `type_name`, reject the match unless `type_name == expected_type`. This enables grammar rules like `rule = at_rule | qualified_rule` to correctly dispatch on the leading token type rather than collapsing all `Name`-typed tokens into the first alternative.
