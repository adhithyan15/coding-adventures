# Changelog

All notable changes to the `parser` crate will be documented in this file.

## [0.4.5] - 2026-08-25

### Fixed — `find_nodes`/`collect_tokens` uncontrolled recursion (CWE-674)

Both `find_nodes` and `collect_tokens` are public entry points that accept
any caller-constructed `GrammarASTNode` — not only trees produced by
`GrammarParser::parse`'s own depth-capped recursion (e.g. at least one
`-to-semantic-ir` frontend's `compile()` works directly with a raw
`GrammarASTNode`). Their previous recursive implementations had no depth
limit of their own: a pathologically deep hand-built tree handed straight
to either function could overflow the *native* call stack, an
uncatchable crash no `Result` could report — flagged during
`/security-review` on the `java-to-semantic-ir` crate (which worked
around the gap locally with its own depth-guarded `collect_bounded`
rather than fixing the shared helper, since a signature/behavior change
here needed its own separately-considered PR).

Rewrote both as iterative traversals using an explicit heap-allocated
stack instead of the native call stack, with no depth limit needed at
all — the stack is bounded only by available memory, not the fixed
native thread stack size. `find_nodes` uses a plain stack of node
references (matching order doesn't depend on interleaving with token
siblings); `collect_tokens` uses a stack of child-slice iterators instead,
since token results must interleave correctly with descending into
nested nodes — a `children` list mixes `Token` and `Node` entries in
source order, so each node's remaining children need to resume exactly
where they left off after a nested node's own subtree is fully drained.
Both preserve the exact traversal order (and therefore output) of the
original recursive versions.

**Public API unchanged**: both functions keep their existing
`Vec<T>`-returning signatures — no `Result`, no new depth-cap parameter,
no breaking change for any of this shared engine's ~100 downstream
consumers. This was the deciding factor over an arbitrary depth-cap
approach (which would need a signature change to report "gave up").

**Caught by `/security-review`, fixed before this landed**: the first
version of this fix made the *traversal* iterative but left
`find_nodes`'s own `results.push(current.clone())` calling
`GrammarASTNode`'s derived `Clone` — which is itself exactly as
recursive as the traversal just fixed, walking the same nested
`Vec<ASTNodeOrToken>` structure. Matching a rule name near the *root* of
a deep tree (an ordinary usage pattern — searching for an ancestor or
wrapper rule, not only a deeply-buried leaf) silently reopened the same
native-stack-overflow the rewrite exists to close, just moved from the
traversal into the clone. Fixed with a new `clone_node_iterative` helper:
a post-order deep clone using an explicit stack of "still assembling
this node's cloned children" frames instead of the native call stack, so
depth is bounded only by available memory here too. `collect_tokens`
needed no equivalent fix — it only ever clones `Token`, a flat struct
with no nested `GrammarASTNode`, so its clones are already O(1)
regardless of tree depth.

New tests: traversal-order correctness (pre-order for `find_nodes`,
source-order-preserving-across-nested-nodes for `collect_tokens`), type
filtering, and a regression guard building a 50,000-level-deep hand-built
tree on a default-stack thread that specifically matches the *root* (not
just a shallow leaf, which the first version of this test did and which
would have silently passed despite the clone-recursion gap above) —
proving both functions, and the clone they now produce, are correct and
complete without overflowing.

**Found while writing that regression test, not fixed here**: merely
*dropping* a tree this deep also overflows the native stack, via
`GrammarASTNode`'s own compiler-generated recursive `Drop` glue —
entirely independent of `find_nodes`/`collect_tokens`. This is a real,
separate CWE-674-shaped gap (any public entry point that ends up holding
a caller-constructed deep tree is exposed, not just these two functions),
logged as its own follow-up rather than bundled into this fix or
silently hidden by shrinking the regression test's depth. The test works
around it with `std::mem::forget` on the trees it builds.

**Also found by `/security-review` (round 2), also not fixed here**:
`walk_ast`/`walk_node` — a third public traversal over the same untrusted
`GrammarASTNode` trust boundary, used by language packages for cover-
grammar rewriting and desugaring — is still fully recursive, three ways
over (direct self-recursion, the same recursive-`Clone` pattern this
change fixed in `find_nodes`, and a recursive `PartialEq` comparison to
detect whether a subtree changed). Reproduced empirically with a
200,000-level tree. Not fixed here: it's untouched by this diff and its
fix is architecturally larger — an iterative rewrite needs to preserve
visitor `enter`/`leave` call ordering and node-replacement semantics, not
just re-derive a `Vec<T>`. Logged as its own follow-up.

## [0.4.4] - 2026-08-25

### Added — contextual `>>`/`>>>` token-splitting for nested generic-argument-list closers

C-family grammars (Java, C#) lex a run of consecutive `>` characters into a
single `RIGHT_SHIFT`/`UNSIGNED_RIGHT_SHIFT`-typed token — the same shape a
real `x >> 2`/`x >>> 1` shift expression uses — because a context-free
lexer cannot tell `Map<String, List<Integer>>`'s two adjacent closing `>`s
apart from an actual right-shift operator without knowing it's inside a
type-argument list. Only the *parser* has that context: it only ever asks
for a lone `GREATER_THAN` when closing exactly one generic-argument-list
level. `match_token_reference` now recognizes this specific situation —
`expected_type == "GREATER_THAN"` against an actual `RIGHT_SHIFT`/
`UNSIGNED_RIGHT_SHIFT` token whose value is a bare run of `>` characters —
and splits off one `>` as the match, writing the shorter remainder token
back into the token stream at the same position so the very next match
attempt sees it fresh. This is what lets `Map<String, List<Integer>>`
close both nesting levels from one merged `>>` token, one `GREATER_THAN`
at a time, and `Box<Box<Box<T>>>` close three levels from one `>>>`.
Deliberately narrow: only fires for that exact (expected, actual) pairing,
so grammars that don't share Java/C#'s token-naming convention (the vast
majority this shared engine also serves) never take the new branch at
all — confirmed inert for every other grammar in this repo.

**Backtracking correctness, found and closed by three rounds of
`/security-review`**: mutating `self.tokens[self.pos]` in place inside a
general backtracking PEG engine is unsound unless the mutation is
undone whenever the attempt that caused it is abandoned. Three real
issues surfaced, each fixed before this landed:

1. **Stale packrat-memo reuse** (round 1): the `memo: HashMap<(usize,
   usize), MemoEntry>` cache assumes the token stream never changes
   after parsing starts, so a cached rule result spanning the mutated
   position could be served stale after a split. First fixed with a
   full `self.memo.clear()` on every split.
2. **`clear()` is an algorithmic-complexity DoS** (round 2): the memo
   table grows with how much of the file has already been parsed, so a
   full clear on every split turns ordinary, non-adversarial Java/C#
   source with many nested-generic occurrences into roughly
   O(fileLength²) parsing — not just pathological input. Tightened to
   `retain`, dropping only entries whose recorded span reached the
   mutated position.
3. **The mutation itself was never undone on backtrack** (round 2,
   deeper finding): every other kind of "try, maybe fail, roll back" in
   this engine (`Sequence`, `Alternation`, `Repetition`, lookahead
   predicates, rule-level failure) restores `self.pos` alone, because
   until splitting existed, `self.pos` was the only mutable state an
   abandoned attempt could have touched. A failed `Alternation` arm (or
   a lookahead predicate documented as "consume no input") that
   triggered a split left the *mutated* token behind for a sibling
   attempt expecting the original merged shape — silently corrupting an
   otherwise-unrelated parse, demonstrated with a minimal synthetic
   grammar. Fixed with a new `split_undo_log: Vec<(usize, Token)>` and a
   `Checkpoint { pos, undo_len }` pair (`checkpoint()`/`restore_to()`)
   that every backtracking site in `match_element` and
   `parse_rule_inner` now uses instead of a bare `self.pos` save/restore,
   so an abandoned attempt undoes the token mutation too, via the same
   `set_token_and_invalidate_memo` helper the forward split uses (memo
   invalidation is symmetric: an entry cached *while* a token was split
   is just as stale, in the opposite direction, once the split is
   undone).
4. **Off-by-one in the invalidation boundary** (round 3, found via the
   above machinery on a real grammar, not the synthetic repro): a split
   deliberately does *not* advance `self.pos`, so a rule whose own last
   step *was* a split ends with `end_pos == pos` — the same signature an
   *ordinary* (non-split) rule has for "never touched `pos`." The
   original `entry.end_pos <= pos` keep-condition treated both cases
   alike, silently keeping a stale entry for the split-ending rule.
   Reproduced concretely: `class C { Map<String, List<Integer>> f; }`
   forces `class_body_declaration = method_declaration | field_declaration`
   to try `method_declaration` first (splitting `>>` while parsing the
   return type), fail at the missing `(`, and backtrack to
   `field_declaration` — the stale entry for the *inner* `List<Integer>`
   closer left one `>` unconsumed, breaking the retry. Fixed by dropping
   entries on `end_pos == pos` too (`entry.end_pos < pos` to keep).
5. **Out-of-bounds panic in the split write itself** (round 3): `current()`
   falls back to *reading* `tokens[len - 1]` once `self.pos` runs past
   the end of the stream, but the split branch *wrote* to
   `self.tokens[self.pos]` using the raw, un-clamped `self.pos` —
   indexing with an out-of-range value panics. Never observed through
   this repo's own Java/C# pipelines (both always append a trailing EOF
   token first), but `GrammarParser` is a public, reusable engine with
   no enforced "must end in EOF" precondition. Fixed by only attempting
   a split when `self.pos < self.tokens.len()` — splitting a token
   you're not genuinely positioned at doesn't make sense anyway.
6. **`restore_to` scanned the memo table once per reverted split, not
   once per backtrack** (round 3): a single abandoned deeply-nested
   attempt (bounded by a frontend's own `max_depth`) could multiply the
   already-accepted per-split `retain` cost by however many layers it
   unwound. Fixed by reverting all the batch's tokens first (each a
   plain O(1) index write), tracking the smallest position touched, then
   issuing exactly one `retain` call against that minimum — `retain`'s
   effect is monotonic in its threshold, so invalidating once against
   the smallest touched position produces the same result as the
   per-entry version, in one scan instead of up to `max_depth` of them.

New tests: three synthetic-grammar unit tests in `parser` itself —
two reproduce the round-2 finding directly (a failed `Alternation` arm
and a `PositiveLookahead`, each undoing their own split), one reproduces
the round-3 out-of-bounds panic (a token stream with no trailing EOF).
In both `coding-adventures-java-parser` and `coding-adventures-csharp-parser`:
two- and three-level nested generics parse and produce the expected
number of `type_arguments`/`type_argument_list` nodes; a real `>>`/`>>>`
shift expression still survives as a single, unsplit token; a nested
generic and a real shift expression coexisting in one file don't corrupt
each other; 300 scattered nested-generic field declarations in one file
(600 closer-splits) still parse correctly; and (java-parser only, the
round-3 memo-boundary repro) a class-body field with a nested generic
survives the `method_declaration`-vs-`field_declaration` backtrack.

## [0.4.3] - 2026-08-03

### Fixed — `GrammarElement::Literal` matched a `String` token by its content, not just an operator lexeme

`GrammarParser::parse_element`'s `Literal` arm compared ONLY `token.value`
against the literal text, ignoring `token.type_` entirely:
`if self.current().value == *value`. A `Literal` element exists to match
an operator/keyword LEXEME by its spelling — the "the parser dispatches by
value" trick many downstream grammars (Ruby's comparison/logical
operators, among others) use for tokens the lexer leaves on a catch-all
type rather than giving a dedicated `TokenType`. But `TokenType::String`
carries arbitrary user-supplied string-LITERAL CONTENT, not a lexeme — a
Ruby program containing a string literal whose content happened to equal
an operator spelling (`foo(1, "*")`, `x = "&&"`, `"hello".ljust(8, "*")`)
had that STRING TOKEN silently swallowed by an unrelated `Literal`
element (in Ruby's case, `call_arg`'s `[ "*" | "**" | "&" ] expression`
splat-marker alternative), leaving the rest of that grammar rule with
nothing to match — a confusing parse failure (or a hard panic in a
panic-on-parse-error caller) for a perfectly ordinary program.

Fixed by excluding `TokenType::String` from `Literal` matching. This is a
pure narrowing of what `Literal` can match — no grammar's `Literal`
element is ever intended to match arbitrary string-literal content, so
the fix can only correct previously-wrong matches, never reject a
previously-correct one. Verified against the full downstream consumer set
(~130 crates depend on this `parser` crate transitively) via
`cargo test --workspace` (excluding pre-existing, unrelated
platform-gated build failures on this host) with zero new test failures.

`parser` 0.4.2 -> 0.4.3.

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
