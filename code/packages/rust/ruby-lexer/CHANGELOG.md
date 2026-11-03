# Changelog

All notable changes to the `coding-adventures-ruby-lexer` crate will be documented in this file.

## Unreleased — linear lexing and a source budget (#18814); whitespace after a heredoc (#18820)

- The heredoc body loop borrows the tag instead of cloning it once per body line, which cost tag-length × lines (a 250 KB tag over 548k empty lines took about 3.7 s). Timing regression `long_heredoc_tag_over_many_lines_lexes_in_linear_time` (#18814 re-review).

### Fixed

- **Every token-fusion pass is linear.**  `fuse_range_ops`,
  `fuse_radix_integers`, the three sweeps of `fuse_float_literals`,
  `fuse_right_shifts`, `fuse_compound_assigns`, `fuse_lambda_arrow`,
  `fuse_numeric_suffixes` and `fuse_safe_nav` folded each pair with
  `Vec::remove`, shifting the rest of the token vector every time:
  O(tokens × fusions).  80 000 lines of `a&.b` or `x = 2r` (~400 KB) took
  about 153 s.  They now share one engine, `fuse_adjacent`, which compacts
  the vector in place with a read and a write cursor (each token read once
  and moved at most once) and keeps the whitespace flags in step.  The
  output is unchanged token for token.  Debug-build timings through
  `tokenize_ruby_for_version`, lines of `a&.b`:

  | lines   | before                 | after |
  |---------|------------------------|-------|
  | 20 000  | 4.9 s                  | 0.8 s |
  | 40 000  | 23.3 s                 | 1.3 s |
  | 80 000  | ~153 s (the issue)     | 2.3 s |
  | 200 000 | ~10 min (extrapolated) | 5.5 s |

- **Heredoc tags are removed in one sweep** (#18814 security review).
  `finalize_heredoc` removed each heredoc's tag `Name` with `Vec::remove`
  as the heredoc closed, so `k` heredocs opened on one line
  (`foo(<<A, <<A, …)` then `A` ⏎ × k) cost O(k²): 80 000 took 30 s in a
  release build, a 1 MiB input about 2.5 min.  It now retires the tag, and
  `remove_retired_heredoc_tags` removes them all with one `Vec::retain` at
  the end of lexing (`finish`, `drain_tokens`).  150 000 heredocs on one
  line lex in ~3.8 s in a debug build; the old finalizer did not finish in
  180 s.  Output unchanged (differential-tested against the old finalizer).
- **Fusions after a heredoc see the right whitespace (#18820).**  The tag
  was removed without its entry in the parallel whitespace flags, so after
  any heredoc every token read the flag of the token before it, and the
  adjacency-gated fusions misfired: `foo(<<A)` ⏎ `A` ⏎ `x &.y` lexed `&`
  `.`, `1 ..2` lexed `.` `.`, `x = 1.5` lexed `1` `.` `5`.  The flag is now
  removed in the same sweep as the tag.  This changes output, to what Ruby
  lexes: `tests/heredoc_whitespace.rs` checks 15 whitespace-sensitive lines
  after four heredoc shapes against Ruby 3.3's `Ripper.lex`.
- **The state machine's step trace no longer grows with the input.**  The
  engine keeps a copy of every step it takes; `push` now drops it every
  4 096 steps (`reset`, then `set_current_state` back).  Lexing 1 MiB
  peaked at ~593 MB; it now peaks at ~146 MB.

### Added

- **`MAX_SOURCE_BYTES` (1 MiB)**, the parser release API's
  `MAX_TRUSTED_SOURCE_BYTES`.  `RubyLexer::push` holds the running total
  against it across calls (a refused call changes nothing) and
  `tokenize_ruby_for_version` checks it before building anything.
  `tokenize_ruby`, `tokenize_ruby_with_oracle` and `tokenize_ruby_diag`
  cannot return an error, so they fail closed with
  `budget_refusal_tokens`: one synthetic token whose `type_name` is
  `BUDGET_REFUSAL_TYPE_NAME` (`RUBY_LEX_BUDGET_EXCEEDED`) and whose value
  is the message, then `Eof`.  No grammar matches it, so a parser rejects
  it; an empty list or a lone `Eof` would have parsed as an empty program.
  `tokenize_ruby_diag` also adds one `lex-error:` diagnostic naming the
  budget.
- **`RubyLexError`**: `Budget { which, limit, actual }` or `Lex(String)`.

### Changed

- `RubyLexer::push` and `tokenize_ruby_for_version` return
  `Result<_, RubyLexError>` instead of `Result<_, String>`.  `Lex` carries
  the message they used to return, `Display` prints it unchanged, and
  `impl From<RubyLexError> for String` keeps `?` working in callers that
  return `String` errors (`try_create_ruby_parser`).

### Tests

- `differential_tests.rs` keeps the old pass bodies (`reference_passes.rs`,
  test-only) and compares tokens and whitespace flags after every pipeline
  step, under all 15 eras, on every string literal in `lib.rs`, every
  `.rb` file in `code/grammars/ruby/corpus` and `ruby-parser`'s fixtures,
  and 1 000 seeded fragment soups.  It also checks that dropping the
  engine trace after every step changes nothing.
- `linear_time_tests.rs`: one timing regression per fused construct, at
  200 000 occurrences each, bound 20 s (1–3.5 s in a debug build; the old
  passes did not finish `a&.b` in 120 s).
- `tests/linear_time.rs`: the issue's two reproductions end to end, a
  source of exactly `MAX_SOURCE_BYTES` (lexed, ~6.5 s in debug), one byte
  past it (refused), UTF-8 byte counting, cumulative `push`, and the
  infallible entry points' refusal tokens.
- `batched_heredoc_tag_removal_matches_reference` (old finalizer kept in
  `reference_passes.rs`), `many_heredocs_on_one_line_lex_in_linear_time`,
  and `tests/heredoc_whitespace.rs` (#18820, against Ripper).

## Unreleased — a newline or `;` ends a statement before `-`, `+`, `(` and after a lone name or a `rescue` list (#18798)

- A keyword in method-name position (`5.then`, `obj&.do`, `A::begin`) no longer opens a statement, so `x = 5.then foo` ⏎ `bar` keeps `foo` as the call's argument instead of splitting after `then` (#18798 third review).

### Fixed (re-review of #18798)

- **A statement starts after an opener on the same line too.**  Step 4
  only ended a name or keyword that was first on its line, so `xs.each {
  |x| foo; bar }`, `wrap { yield; cleanup }`, `{ |x| next; bar }`, `begin
  foo; bar end`, `loop do foo; bar end`, `-> { foo; bar }`, `[1].each {
  puts; puts 2 }` and `if c then return; x end` silently read the second
  statement as the first one's argument.  One predicate,
  `opens_statement` (`{`, a block's closing `|`, `do`, `then`, `else`,
  `begin`, `ensure`, `rescue`, a step-2 `;`), now decides for lone names
  and the line-ending keywords alike.  Test
  `a_statement_starts_after_an_opener_on_the_same_line`.
- **`super` and `yield` end an expression**, so a `-`, `+`, `(` or `[`
  that starts the next line is a new statement (`x = super` ⏎ `-1` read
  as `super - 1`).  Test `super_and_yield_end_an_expression`.

### Fixed (security review of #18798)

- **A keyword that ends its line ends its argument list.**  `super`,
  `yield`, `return`, `break` and `next` take a paren-less argument list,
  and the lone-name rule only covered names: `super` ⏎ `puts "hi"` (the
  common `initialize` shape) read as `super(puts)` then `"hi"`, and since
  `;` became a line break, `super; setup` / `yield; foo` / `return; 42`
  read as `super(setup)` / `yield(foo)` / `return 42`.  Step 4 (now
  `ends_its_statement`) puts a synthetic `;` after such a keyword too:
  `super` wherever it ends a line (it can be a value, `x = super`) unless
  it is a method name (`obj.super`, `def super`, `:super`); the other four
  only where a statement starts.  Test
  `a_keyword_ending_its_line_ends_its_argument_list`.
- **The pass is linear.**  Each synthetic `;` was a `Vec::insert`, so the
  pass was O(tokens × insertions); `;`-in-block-parameters and
  closing-bar lookups were O(n × blocks).  The insertions are now spliced
  in by one rebuild (`insert_statement_ends`), the block-parameter ranges
  are walked with one pointer, the closing bars are a `HashSet`, and the
  bar search stops at the first unclosed `|`.  Test
  `many_statement_ends_lex_in_bounded_time` (200k lines, ~2 s for the whole
  lex in a debug build).

### Added

- **`mark_statement_ends`**, the lexer's last pass (after every era
  fusion, so a lambda arrow `->` is never a minus).  It replaces
  `tag_line_start_brackets` and runs four steps:
  1. a `;` between statements becomes a `Newline` token (value `;`), so
     `x = 1; puts x` and `def risky; raise "x"; rescue => e; 1; end` lex
     as the same programs on several lines (a `;` used to stop the
     parse).  The `;` that starts block-local names (`do |x; y|`) stays a
     `Semicolon`;
  2. a `rescue` line with no `=>` gets a synthetic `;` after its class
     list, or after a bare `rescue` (a trailing comma continues the list
     onto the next line);
  3. a `(`, `-` or `+` that starts a line after a complete expression is
     re-tagged `( line start` / `- line start` / `+ line start` (type
     kept), alongside the existing `[ line start`;
  4. a name alone on its line, where a statement starts, gets a synthetic
     `;` when the next line could be read as its argument.
- Constants `LINE_START_LPAREN`, `LINE_START_MINUS`, `LINE_START_PLUS`,
  `STATEMENT_END`; synthetic tokens carry `TOKEN_SYNTHETIC`.
- `source_spelling(token)`: what the source wrote for a token (`None` for a
  synthetic `;`, the bare character for a line-start tag), for tools that
  compare this lexer with another.

### Changed

- `ends_expression` counts a `Name` token only when it spells an
  identifier or a sigil variable: the operators the lexer leaves on `Name`
  tokens (`&&`, `||`, `=>`, `?`, `|`, `->`) do not end an expression, so
  `x = a &&` ⏎ `-b` is one expression.
- The `newline_is_significant` test expects the synthetic `;` in `a` ⏎ `b`.

Tests: `paren_and_signs_starting_a_statement_are_retagged`,
`paren_and_signs_that_continue_an_expression_are_left_alone`,
`a_name_alone_on_its_line_ends_its_statement`,
`a_rescue_line_without_a_binding_ends_with_a_semicolon`,
`a_semicolon_between_statements_is_a_line_break`,
`source_spelling_undoes_the_tags`.

## Unreleased — a `[` that starts a line starts a statement (#18778)

### Changed (re-review of #18778)

- The `LINE_START_LBRACKET` doc comment no longer claims a `String` token
  can never match the tag: only the Rust grammar parser skips `String`
  tokens for literals.  The other languages' parsers do not, so there
  `puts "[ line start"` does not parse, like `puts "<"` (#18798).

### Changed (review of #18778)

- The tag is now `[ line start` (the public constant `LINE_START_LBRACKET`),
  not `lbracket_line_start`.  A grammar literal matches a token's value
  whatever its type, and the first tag was an identifier, so the grammar's
  lookahead refused `puts lbracket_line_start`.  No name, keyword, number or
  operator can contain a space.

### Added

- **`tag_line_start_brackets`.**  A newline after a complete expression
  ends a Ruby statement, so a line that starts with `[` starts a new one:
  `x = 5` ⏎ `[1].each …` is two statements.  The legacy grammar ignores
  newlines and read `5[1].each …` (the C backend failed with `undefined
  method []`; `puts` ⏎ `[7].each …` printed the wrong thing everywhere).  A
  `[` with a `Newline` token between it and an expression-ending token is
  re-tagged `[ line start` (type still `LBracket`); the grammar's
  `index_suffix` and paren-less-call argument refuse that value.  All eras.
  Tests `bracket_starting_a_line_after_an_expression_is_retagged` and
  `bracket_that_continues_an_expression_is_left_alone`.
- `ends_expression`, the expression-ending test, now shared by the
  modifier-keyword pass and the new pass (no behaviour change for
  modifiers).

## Unreleased — LANG84 R1b: embedded documents, `__END__` and the refusals

### Changed

- **Security review (F1).** The CHAR classes no longer accept `?"`, `?'`,
  `` ?` `` or `?#`. A new token, `QUESTION_OPENER` (`/\?["'`#]/`), is refused
  in every mode, so a mode that disagrees with Ruby's lexer state (after
  `.end`, after a local) cannot turn a quote or `#` into a string or comment
  boundary Ruby does not have. 0.49 and 0.50 are unchanged: their `?` is
  always a character literal.
- **Security review (F2).** The other direction of mode disagreement: where
  we read a character literal but Ruby reads a ternary `?` (after a local,
  after a value in a `def` parameter list), `??`, `?%`, `?/` and `?<` open a
  character literal, `%`-literal, regexp or heredoc on Ruby's side only.
  Those four characters are out of every CHAR class too, and
  `QUESTION_OPENER` is now `/\?["'`#?%\/<]/`. The CHAR class is
  `[!$&(-.:;=>@\[\]^{|}~]`.
- `src/_grammar.rs` is regenerated from the R1b pairs. The three 3.3.x token
  grammars (still one deduplicated class) gain the following.
  - **Skip patterns.** `EMBDOC` (`=begin` … `=end`) and `DATA_END` (a
    `__END__` line plus the rest of the input), both guarded by
    `@line_start` (LANG84 E1).
  - **Refused tokens.** `HEREDOC_OPEN` and `EMBDOC_UNTERMINATED`.
  - **Def-head modes.** `deffname`, `defn` and `defparams`, with their group
    tokens.
  - **Refusal rules.** E8 `-> refuse` rules for `/` `/=` `%` `%=` when no
    whitespace follows, in `beg`, and in the def head.
  - **NEWLINE.** A rule now resets the mode to `beg`.
- 0.49 and 0.50 change only in comments: they have no embedded documents,
  and their `__END__` predates R1b.
- Every `TokenDefinition` literal carries `guard:` (239 of 239), and every
  `ModeTransition` literal carries `conditions:` (85 of 85).

## Unreleased — LANG84 release token tables (R0)

### Added — LANG84 release token tables

- `src/_grammar.rs`, generated by `grammar-tools generate-rust-compiled-grammars
  ruby` from the `code/grammars/ruby/ruby<release>.tokens` pairs listed in
  `releases.csv` (LANG84 phase 3.2, slice R0). Five releases (0.49, 0.50,
  3.3.5, 3.3.6, 3.3.7) compile to three distinct token grammars.
- `pub mod releases` re-exports `SUPPORTED_RELEASES`, `token_grammar(release)`
  (exact release IDs only; `None` otherwise) and `grammar_class(release)`.
- New direct dependency on `grammar-tools` (the generated module names its
  types).
- The hand-written state-machine lexer and every existing entry point are
  unchanged.
- Tests: `release_table_tests` (2).

## [0.25.0] - 2026-06-03

### Added (FC — `__END__` program terminator)

`__END__` alone on a line (column 0) now ends tokenization: everything
after it is Ruby's `DATA` section, not code, so it is no longer
mis-lexed. The `push` scan loop checks, at each line start, whether the
upcoming line is exactly `__END__` (via the new `is_end_marker` helper)
and stops feeding the engine; `finish()` still flushes the EOF token.

Scope: this halts tokenization only. `DATA` itself stays an ordinary
constant read (no synthesized file handle) — a deliberate follow-up.
Indented or mid-line `__END__` is unaffected (lexes as a normal Name),
matching Ruby's column-0 requirement.

New tests: `end_marker_halts_token_stream`,
`end_marker_at_eof_without_trailing_newline`,
`end_marker_requires_column_zero`, `end_marker_not_triggered_mid_line`.

## [0.24.0] - 2026-05-26

### Added (Phase 8a-2 (FC) — `>>` and `>>=` token fusion)

New pre-fusion pass `fuse_right_shifts()` runs immediately before `fuse_compound_assigns()`:

| Incoming pair (adjacent, no whitespace gap) | Folded into     |
|---------------------------------------------|-----------------|
| `Name(">")` + `Name(">")`                   | `Name(">>")`    |
| `Name(">")` + `Name(">=")`                  | `Name(">>=")`   |

This sidesteps the 1.8-era state-machine quirk where the greedy `>=` classifier already ate the `=` from `>>=` before the compound-assign pass got a chance.  Same adjacency rules as the existing compound-assign fusion: same line, no whitespace gap.

### Tests

- `coding-adventures-ruby-lexer`: 169 → **173** (+4):
  - `right_shift_compound_assign_fuses_into_single_token`
  - `right_shift_binary_operator_fuses_into_single_token`
  - `right_shift_fusion_respects_whitespace_gap`
  - `right_shift_fusion_leaves_unrelated_ge_alone` (regression)

## [0.23.0] - 2026-05-26

### Added (Phase 8a (FC) companion — fuse more compound-assign operators)

`fuse_compound_assigns` recognises six additional left-flank operator tokens:

- `%`, `**`, `<<`, `&`, `|`, `^` (all emitted by the 1.8-baseline state machine as `Name`-typed tokens)

When immediately followed by a `=` with no whitespace gap, the pair folds into a single `Name("%=")` / `Name("**=")` / `Name("<<=")` / `Name("&=")` / `Name("|=")` / `Name("^=")` token, matching the existing fusion strategy for `+= -= *= /= ||= &&=`.

This lets the parser's `assignment` rule match the new compound forms by value, the same way it already matches `+=` and friends.

### Deferred

- `>>=` is NOT yet handled because the state machine splits `>>` into two `>` tokens.  Folding it requires a separate `>>` pre-fusion pass and is tracked as a follow-up.

## [0.22.0] - 2026-05-24

### Added (Phase 6q companion — re-tag trailing-modifier keywords)

New post-pass `tag_modifier_keywords` rewrites `if`/`unless`/`while`/`until` Keyword tokens to `if_modifier`/`unless_modifier`/`while_modifier`/`until_modifier` when they appear after an expression-ending token on the same line.  This is the lexer-side disambiguation that lets the grammar distinguish trailing-modifier syntax (`x if y`) from leading-keyword statement forms (`if y\n  x\nend`) without making newlines globally significant.

Re-tag trigger:

- Token value is `if`, `unless`, `while`, or `until` (Keyword type).
- A preceding non-Newline token exists on the same `line`.
- That preceding token's type is one of: `Number`, `String`, `Name`, `RParen`, `RBracket`, `RBrace`, or `Keyword` with value in `{nil, true, false, self, end}`.

Effect: only the token's `value` is mutated (to `<kw>_modifier`); the `type_` stays `Keyword`.  Leading-position keywords (at file start, after a newline, after `;`/`,`/`(`/etc.) are untouched and continue to drive the `if_statement` / `while_statement` / etc. grammar rules.

Era: pre-1.0 Ruby (modifier conditionals predate 1.0).  No era gating.

### Tests

- `coding-adventures-ruby-lexer`: 164 → **169** (+5):
  - `modifier_if_after_method_call_no_paren_is_retagged` — `puts "hi" if cond` produces `if_modifier`.
  - `modifier_unless_while_until_all_retagged` — all four forms re-tag uniformly.
  - `leading_if_at_statement_start_is_not_retagged` — `if y ... end` survives bare.
  - `newline_between_expr_and_if_prevents_retag` — `x = 1\nif y...` keeps bare `if`.
  - `modifier_retag_uniform_across_all_eras` — same shape from 1.8 through 3.0.

## [0.21.0] - 2026-05-24

### Added (Phase 4o — heredoc opener variants `<<-TAG` and `<<~TAG`)

Extends the existing Phase 3c `<<TAG` plain heredoc support with the two modifier forms:

| Form | Since | Terminator | Body |
|---|---|---|---|
| `<<TAG` | 1.0 | exact at col 0 | verbatim |
| `<<-TAG` | 1.9 | indent-tolerant | verbatim |
| `<<~TAG` | 2.3 | indent-tolerant | common leading-ws stripped |

The token shape is unchanged: a single `TokenType::String` token whose value is the reconstructed source (`<<TAG\nBODY\nTAG`, `<<-TAG\n…\nTAG`, or `<<~TAG\n…\nTAG`).  Parser distinguishes by the leading `<<` / `<<-` / `<<~` prefix.

#### State-machine additions

- New state: `after_lt_lt` (saw `<<`, peeks for `-` or `~`).
- `after_lt → <` no longer emits immediately; instead enters `after_lt_lt`.
- `after_lt_lt → -` emits `Op("<<-")`.
- `after_lt_lt → ~` emits `Op("<<~")`.
- Catch-all: emit plain `Op("<<")` and re-dispatch the follower.

#### Action interpreter additions

- `is_heredoc_open` now accepts `<<`, `<<-`, `<<~` as openers (was: `<<` only).
- `PendingHeredoc` gains a `variant: HeredocVariant` field tracking which opener form was seen.
- `capture_heredoc_bodies` matches terminator lines against the front heredoc's variant:
  - `Plain`: line == tag (exact).
  - `DashIndent` / `TildeIndent`: `line.trim_start()` == tag.
- `finalize_heredoc` invokes `strip_common_leading_whitespace` on the body when the variant is `TildeIndent` and emits the appropriate `<<` / `<<-` / `<<~` prefix in the reconstructed token value.

#### New helper

- `strip_common_leading_whitespace(body) -> String` — computes the minimum leading-ws prefix across all non-empty body lines and strips it from every non-empty line; empty lines pass through unchanged.

### Tests (+5 new, total 164)
- `heredoc_dash_indent_terminator_allows_leading_whitespace`
- `heredoc_tilde_indent_strips_common_leading_whitespace`
- `heredoc_tilde_indent_uses_minimum_prefix_across_lines` — `2 vs 4 spaces → 2-space strip`.
- `heredoc_plain_form_still_requires_exact_terminator` — leading-ws on `<<EOF` terminator → unterminated-heredoc diagnostic.
- `heredoc_dash_and_tilde_variants_lex_uniformly_across_eras` — era invariance check.

## [0.20.0] - 2026-05-24

### Added (Phase 6p companion — compound-assignment operator fusion)

New post-pass `fuse_compound_assigns` folds adjacent `Op` + `Equals` token pairs into a single `Name`-typed token whose value is the fused operator:

| Source | Lexed before fusion | After fusion |
|---|---|---|
| `x += 1` | `Name(x)`, `Plus`, `Equals`, `Number(1)` | `Name(x)`, `Name("+=")`, `Number(1)` |
| `x -= 1` | `Name(x)`, `Minus`, `Equals`, `Number(1)` | `Name(x)`, `Name("-=")`, `Number(1)` |
| `x *= 1` | `Name(x)`, `Star`, `Equals`, `Number(1)` | `Name(x)`, `Name("*=")`, `Number(1)` |
| `x /= 1` | `Name(x)`, `Slash`, `Equals`, `Number(1)` | `Name(x)`, `Name("/=")`, `Number(1)` |
| `x ||= 1` | `Name(x)`, `Name("||")`, `Equals`, `Number(1)` | `Name(x)`, `Name("||=")`, `Number(1)` |
| `x &&= 1` | `Name(x)`, `Name("&&")`, `Equals`, `Number(1)` | `Name(x)`, `Name("&&=")`, `Number(1)` |

The parser's `assignment` rule matches these by literal value (`"+="`, etc.) — same convention as `"=>"`, `"<="`, `"&&"`.

**Adjacency gate**: the fusion requires no whitespace between the op and `=`.  `x + = 1` (with a space) stays two tokens — that's a syntax error in real Ruby but it's not a compound assignment.

**Era**: pre-1.0 Ruby — every era ≥ 1.8 emits the same fused shape, so no gating.

### `/=` regex disambiguation guard

`x /= 1` previously lexed as `Name(x)` followed by an unterminated regex (`/...`) because the `/` after a non-local name triggers the regex-vs-divide oracle.  New `suppress_regex_open` flag set by `push` for exactly one `step_char` call when the upcoming `/` is immediately followed by `=` — forces the state machine to emit `/` as a plain Op so `fuse_compound_assigns` can fold it.

### Tests (+3 new, total 159)
- `compound_assign_arithmetic_ops_fuse_into_single_token` — `+=`, `-=`, `*=`, `/=`.
- `compound_assign_logical_ops_fuse_into_single_token` — `||=`, `&&=`.
- `compound_assign_does_not_fuse_with_whitespace_gap` — `x + = 1` stays two tokens.

## [0.19.0] - 2026-05-24

### Added (Phase 4n — `%r{regex}`, `%s{symbol}`, `%x{cmd}` percent literals)

Extends the existing `%w[…]` / `%q{…}` / `%i[…]` / `%I[…]` family with the three remaining built-in percent-literal flavors:

- `%r{…}` — regex literal (interpolation-free in v0).
- `%s{…}` — symbol literal (non-interpolating).
- `%x{…}` — command-execution literal (sibling of `` `…` ``).

All three pre-date the era split — every era ≥ 1.8 emits the same token shape.

#### Token shape

Each emits as a `TokenType::String` whose value is the verbatim source (`%r{pat}`, `%s{name}`, `%x{cmd}`).  Parser code distinguishes them from plain strings — and from each other — by inspecting the leading `%` + type letter.  Same sentinel-by-prefix trick used by `%w[…]` / `%q{…}` / heredocs / backticks.

#### State-machine additions

- Alphabet: `s`, `x` added (`r` was already present for the `\r` escape).
- New tokens: `PercentR`, `PercentS`, `PercentX` (all mapped to `TokenType::String` in the emit handler).
- New states: `percent_{r,s,x}_open` (peek for `{`) and `percent_{r,s,x}_body` (slurp until `}`).
- `after_percent` gains three new dispatch arms (`r` → `percent_r_open`, etc.).
- Body states emit on the matching `}` and parse-error on EOF (`unterminated_percent_{r,s,x}`).
- Open states parse-error if the follower isn't `{` (`percent_{r,s,x}_no_delim`) and fall back to `% is modulo, letter is identifier`.

#### Out of scope (deferred to follow-up)

- Alternate delimiters (`%r[…]`, `%r(…)`, `%r/…/`, etc.).  V0 supports only `{` to keep the state count down — matches the existing `%q{…}` convention.
- `#{}` interpolation inside `%r{…}` / `%x{…}` (real Ruby allows it).  Parser-side phases can layer interpolation later.
- Regex flags after `%r{…}imx` — out of scope; flag-letter slurping after `%r}` is a follow-up.

### Tests (+7 new, total 156)
- `percent_r_regex_literal_lexes_as_string_with_prefix`
- `percent_s_symbol_literal_lexes_as_string_with_prefix`
- `percent_x_command_literal_lexes_as_string_with_prefix`
- `percent_r_empty_body_lexes` — `%r{}`
- `percent_r_s_x_lex_uniformly_across_all_eras` — era invariance.
- `percent_x_does_not_swallow_following_tokens` — `%x{pwd} + 1` lexes cleanly.
- `percent_r_unterminated_reports_diagnostic` — `parse_error(unterminated_percent_r)` path.

## [0.18.0] - 2026-05-24

### Added (Phase 4m — backtick command literals `` `cmd args` ``)

Ruby has had `` `cmd args` `` command-execution literals since 1.0 — they spawn a shell, execute the body, and yield the standard-output as a string.  The lexer was the missing piece: until this chunk the leading backtick was an unrecognised character.

Implementation lives entirely in the state machine — no post-pass needed.

#### Token shape

- **`` `cmd args` ``** → `TokenType::String` with value `` `cmd args` `` (literally, with the backticks re-wrapped).
- Parser-side distinguishes backtick literals from plain strings by inspecting the lexeme's leading character — same sentinel-by-prefix trick used by percent literals (`%w[…]`) and heredocs (`<<TAG\n…TAG`).

#### State-machine additions

- Alphabet: `` ` `` (backtick).
- New token kind: `Backtick` (mapped to `TokenType::String` in the emit handler).
- New states: `backtick_body`, `backtick_escape`.
- `data` → `backtick_body` on `` ` ``.
- Inside `backtick_body`:
  - `` ` `` → `data` (`emit(Backtick)`).
  - `\\` → `backtick_escape`.
  - `\n` is allowed (multi-line bodies, matching `string_d_body`).
  - EOF → `parse_error(unterminated_backtick)`.
  - Anything else → append.
- `backtick_escape` handles the same five escapes as `string_d_escape` (`n`, `t`, `r`, `\\`, plus `` ` `` instead of `"` since `` ` `` is the close char) and falls through to `append_text(current)` otherwise.

#### Out of scope (deferred to follow-up)

- `#{}` interpolation inside `` `…` ``.  Real Ruby allows it; v0 treats `#` as a literal character inside the body.  The parser-side phase 7a (backtick parsing) can layer interpolation later.
- Backtick as a *method name* (`def \`(cmd); …; end`).  Outside scope for v0 — that needs special lex-state feedback.

### Tests (+7 new, total 149)
- `backtick_simple_command_lexes_as_string_with_backticks`
- `backtick_empty_body_lexes_to_two_backticks`
- `backtick_escape_sequences_resolved_in_body`
- `backtick_multiline_command_keeps_newlines`
- `backtick_lexing_is_era_invariant` — every era ≥ 1.8 produces the same token shape (backticks are pre-1.0, hence era-invariant).
- `backtick_does_not_swallow_following_tokens`
- `backtick_unterminated_reports_diagnostic` — exercises the `parse_error(unterminated_backtick)` action.

## [0.17.0] - 2026-05-24

### Added (Phase 4l — radix-prefixed integers `0x1F`, `0b1010`, `0o17`, `0d42`)

Ruby's four explicit-radix integer prefixes have been in the language since 1.0 but were never lexed by the v0 state machine.  Implemented as a post-pass fusion (same pattern as Phase 4k float fusion, Phase 4f numeric suffixes, etc.) — the state machine emits `Int("0")` + `Name("xDEAD")` and the post-pass fuses them.

#### Supported shapes

| Source        | Base | Result          |
|---------------|------|-----------------|
| `0x1F`        | 16   | `Number "0x1F"` |
| `0xDEAD_BEEF` | 16   | `Number "0xDEAD_BEEF"` |
| `0Xff`        | 16   | `Number "0Xff"` |
| `0b1010`      |  2   | `Number "0b1010"` |
| `0B1010_1100` |  2   | `Number "0B1010_1100"` |
| `0o755`       |  8   | `Number "0o755"` |
| `0O17`        |  8   | `Number "0O17"` |
| `0d42`        | 10   | `Number "0d42"` |
| `0D100_000`   | 10   | `Number "0D100_000"` |

#### What this is **not** doing

- **Old-style C-flavoured octal (`017`)**: already a single `Int("017")` from int_body — interpretation as octal vs decimal-with-padding is a parser/SIR-lowerer concern.
- **Invalid digits**: `0xZZ` does NOT fuse — `Z` isn't a hex digit, so the post-pass declines and the parser later rejects the bad shape.  No diagnostic emitted at the lexer layer (out of scope for v0).
- **Whitespace breaks fusion**: `0 x1F` is two tokens (`Int "0"`, `Name "x1F"`) — checks `whitespace_before_token`.
- **Method calls**: `0.method` stays `Int Dot Name` — radix fusion only matches `Int(0) Name(<radix>...)` adjacency.

### Helpers added
- `fuse_radix_integers` — single-step fusion pass.
- `is_radix_integer_body(&str)` (module-scope) — validates that a `Name` lexeme matches one of the four radix-body shapes (prefix letter + at least one valid digit + optional `_` separators).

### Tests (+9 new, total 142)
- `lexes_hex_integer` (`0x1F`, `0xDEAD_BEEF`, `0Xff`)
- `lexes_binary_integer` (`0b1010`, `0B1010_1100`)
- `lexes_octal_integer` (`0o755`, `0O17`)
- `lexes_decimal_explicit_radix` (`0d42`, `0D100_000`)
- `invalid_hex_does_not_fuse` (`0xZZ` stays as two tokens)
- `radix_integer_requires_no_whitespace` (`0 x1F` stays as two tokens)
- `radix_does_not_swallow_method_call` (`0.method` stays `Int Dot Name`)
- `radix_integers_lex_uniformly_across_all_eras` (pin across all 15 `ERA_VERSIONS`)
- `is_radix_integer_body_smoke` (direct helper test)

## [0.16.0] - 2026-05-24

### Added (Phase 4k — float literals `1.5`, `1e10`, `1.5e-3`)

Float literals have been in Ruby since 1.0 but were never lexed by the v0 state machine.  Implemented via a **post-pass fusion** (same pattern as Phase 4b's `->` arrow, Phase 4c's `&.`, Phase 4f's numeric suffixes, etc.) — keeps the state machine TOML simple and avoids the lookahead-then-unpeek dance that would be needed in the engine itself.

#### Supported lexeme shapes

| Source       | Pre-fusion tokens                   | Post-fusion Number    |
|--------------|-------------------------------------|-----------------------|
| `1.5`        | Int "1", Dot, Int "5"               | "1.5"                 |
| `1e10`       | Int "1", Name "e10"                 | "1e10"                |
| `1E5`        | Int "1", Name "E5"                  | "1E5"                 |
| `1.5e10`     | Int, Dot, Int, Name "e10"           | "1.5e10"              |
| `1.5e-3`     | Int, Dot, Int, Name "e", Minus, Int | "1.5e-3"              |
| `1e+10`      | Int, Name "e", Plus, Int            | "1e+10"               |
| `1_000.5`    | Int "1_000", Dot, Int "5"           | "1_000.5"             |

#### What this is **not** doing

- **Method calls**: `1.method` stays as `Int "1"`, `Dot`, `Name "method"` (the fusion only fires when the dot is flanked by integer-shaped tokens).
- **Range operator**: `1..5` stays a range — `fuse_range_ops` runs before `fuse_float_literals` and consumes the two dots into `Name ".."` before float fusion sees the stream.
- **Whitespace-separated**: `1 . 5` is three separate tokens — every fusion step checks `whitespace_before_token` and same-line.
- **Sign-only exponent**: signed exponents (`1e+10`) need three additional tokens (Name "e", Plus/Minus, Int) — handled by the third fusion step.

#### Why a post-pass, not a TOML state?

The state machine would need lookahead to decide between `1.5` (one float token) and `1.method` (Int + Dot + Name).  Our TOML doesn't support multi-char lookahead cleanly — we'd have to consume the `.` and then somehow re-emit a Dot if the next char isn't a digit.  The post-pass approach is uniform with how `..` / `...` / `->` / `&.` / `2r` / `_1` already work and stays cleanly testable.

### Helpers added
- `fuse_float_literals` — three-step fusion: (1) `Int Dot Int`, (2) `Number Name(e<digits>)`, (3) `Number Name(e) (+|-) Int`.
- `is_integer_lexeme` — true iff the lexeme is just digits and `_`.
- `is_unsigned_exponent_lexeme` (module-scope) — true iff the lexeme is `[eE]<digit_or_underscore>+`.

### Tests (+9 new, total 133)
- `lexes_simple_float` — `1.5` → Number "1.5".
- `lexes_float_in_assignment` — `x = 1.5` produces the full expected token stream.
- `lexes_float_with_unsigned_exponent` — `1e10`, `2E5`, `1.5e10`.
- `lexes_float_with_signed_exponent` — `1.5e-3`, `1e+10`.
- `float_does_not_swallow_range_operator` — `1..5` stays a range.
- `float_does_not_swallow_method_call` — `1.method` stays `Int Dot Name`.
- `float_requires_no_whitespace` — `1 . 5` is three tokens.
- `float_lexes_uniformly_across_all_eras` — pins float-Number stream across every era in `ERA_VERSIONS`.
- `is_unsigned_exponent_lexeme_smoke` — direct helper test.

## [0.15.0] - 2026-05-23

### Added (Phase 4i / 4j — instance vars `@x`, class vars `@@x`, globals `$x`)

Three sigil-prefixed variable shapes that have existed since Ruby 1.0 but were never lexed by the v0 state machine:

- **Instance variables**: `@count`, `@_private`, `@foo_bar2`
- **Class variables**: `@@all`, `@@cache_key`
- **Global variables (regular form)**: `$LOAD_PATH`, `$stderr`, `$x`

All three emit as `TokenType::Name` with the **full lexeme including the sigil** preserved in `value` (e.g. `@count`, `@@all`, `$LOAD_PATH`).  The parser and SIR lowerer dispatch by inspecting the leading character — the same trick the lexer already uses for `::` (encoded as `TokenType::Colon` with value `::`).

#### TOML changes (`ruby-1.8.lexer.states.toml`)

New states:
- `after_at` — peek state after `@`; decides ivar vs cvar by checking for a second `@`.  Invalid follower (e.g. `@1`) records `invalid_ivar` and falls back to emitting `@` as a bare Op.
- `ivar_body` — slurps `[a-zA-Z0-9_]*` after `@<starter>`.
- `cvar_body` — slurps `[a-zA-Z0-9_]*` after `@@`.
- `after_dollar` — peek state after `$`; requires an ident-starter first char.  Invalid follower records `invalid_gvar` and emits `$` as a bare Op.
- `dollar_body` — slurps `[a-zA-Z0-9_]*` after `$<starter>`.

New alphabet entries: `@`, `$`.

New `data → ...` dispatcher transitions: `@` → `after_at`, `$` → `after_dollar`.

#### Scope notes

v0 deliberately does NOT handle Ruby's punctuation globals:
- `$~` (last match), `$&` (matched string), `$_` (last read line)
- `$0`..`$9` (regex capture groups)
- `$$`, `$?`, `$!`, etc.

These will land in a follow-up phase that splits `after_dollar` into per-char arms.  The v0 fallback (emit `$` as Op + diagnostic) keeps the token stream clean for the parser.

#### No era gating

All three sigils have been in Ruby since the beginning, so the lexing is era-invariant.  The `sigil_vars_unchanged_across_all_eras` test pins this across all 15 `ERA_VERSIONS`.

### Tests (+8 new, total 124)
- `lexes_instance_variable` — `@count` → one `Name` token with value `@count`.
- `lexes_class_variable` — `@@all` → one `Name` token with value `@@all`.
- `lexes_global_variable` — `$LOAD_PATH` → one `Name` token with value `$LOAD_PATH`.
- `ivar_with_digits_and_underscore` — `@foo_bar2` is one ivar (digits/underscore allowed after first ident-starter).
- `sigil_vars_in_assignment_context` — `@x = 1` lexes as `Name(@x) Equals Number(1)`.
- `invalid_ivar_falls_back_to_op_with_diagnostic` — `@1` records `invalid_ivar` and emits `@` as Op.
- `invalid_gvar_falls_back_to_op_with_diagnostic` — `$ x` records `invalid_gvar` and emits `$` as Op.
- `sigil_vars_unchanged_across_all_eras` — `@a + @@b + $c` produces the identical Name-value stream across every era.

## [0.14.0] - 2026-05-22

### Added (Phase 4h — 1.9.1 hash shorthand `{a: 1}` — confirmation pass)
Hash shorthand `{a: 1}` was introduced in Ruby 1.9.1.  Pre-1.9.1, the only valid hash literal was the rocket form `{:a => 1}`.

At the **lexer** level, no token-level change is needed: `{a: 1}` lexes uniformly across all 15 eras as `LBrace Name Colon Number RBrace` because the colon is just a standalone `Colon` token in every era.  Real Ruby differentiates the two forms at the **parser** level, which already shipped as Phase 6d's `hash_entry = NAME COLON expression | …`.  So this phase is intentionally a no-op at token granularity, accompanied by:
- A design-note doc block in `src/lib.rs` explaining why no token-level change is needed and where the era-gating actually lives.
- 4 new tests pinning the invariant that the token stream for `{a: 1}` / `{a: 1, b: 2}` / `{:a => 1}` is era-independent — so backends can trust it.

The era-gating (rejecting pre-1.9.1 hash shorthand) belongs at the parser layer (already shipped) and at a later AST-level pass.  This PR completes the lexer chunk queue for the v0 era-delta surface.

### Tests (+4 new, total 116)
- `hash_shorthand_lexes_uniformly_across_all_eras` — `{a: 1}` produces the identical token kind stream under every era in `ERA_VERSIONS`; baseline shape is asserted explicitly.
- `hash_shorthand_with_two_entries_lexes_uniformly` — `{a: 1, b: 2}` is era-independent (the comma is too).
- `hash_rocket_form_lexes_uniformly_across_all_eras` — `{:a => 1}` produces a `=>` token under every era.
- `hash_shorthand_and_rocket_differ_only_in_value_tokens` — the two hash forms produce different token streams (shorthand has `Colon`, rocket has `=>`), but each form is era-invariant; this is the load-bearing guarantee parsers depend on when era-gating hash-shorthand acceptance.

## [0.13.0] - 2026-05-20

### Added (Phase 4g — 2.0 `%i[]` / `%I[]` symbol-array percent literals)
- New `PercentI` and `PercentBigI` token kinds declared in `ruby-1.8.lexer.states.toml`.
- New states `percent_i_open`, `percent_i_body`, `percent_big_i_open`, `percent_big_i_body` mirror the existing `%w[]` / `%q{}` state shapes.
- `after_percent` now has arms for `i` and `I` follower letters — they enter the corresponding *open* state, which then requires `[` as the canonical delimiter (other followers bail to "% is modulo").
- The action interpreter emits `PercentI` / `PercentBigI` as `TokenType::String` carrying the verbatim source-shape (`%i[a b c]` / `%I[a b c]`), matching the Phase 3a precedent for `%w[]` / `%q{}`.
- v0 caveat: the lexer accepts `%i[]` / `%I[]` for *all* eras (since they're lexically the same shape as `%w[]`); pre-2.0 Ruby would have rejected them at the parser level.  A future era-aware downgrade can split them back into `%` + identifier + bracket tokens.

### Tests (+4 new, total 112)
- 2.0 lexes `%i[a b c]` as a single String token with the verbatim value.
- 2.0 lexes `%I[a b c]`.
- Unterminated `%i[a b c` records an `unterminated_percent_i` diagnostic.
- Plain `5 % 2` (modulo) still works — the `% → after_percent` route correctly bails when the follower isn't a recognised type letter.

## [0.12.0] - 2026-05-20

### Added (Phase 4f — 2.1 numeric suffixes `r` / `i`)
- New `fuse_numeric_suffixes` post-pass under era ≥ 2.1 — folds a `Number` token followed (no whitespace) by `Name("r")` or `Name("i")` into a single fused `Number` token (e.g. `2` + `r` → `2r`).  Ruby 2.1's rational and complex literal forms.
- Pre-2.1 eras leave them split — the era gate is precise.

### Tests (+5 new, total 108)
- 2.1 fuses `2r + 3r` to two Number tokens with values "2r", "3r".
- 2.1 fuses `4i`.
- 2.0 does NOT fuse — the `r` stays as a Name.
- `2 r` (whitespace) does NOT fuse even under 2.1.
- `2x` (`x` isn't a recognised suffix) stays split.

## [0.11.0] - 2026-05-20

### Added (Phase 4e — range fusion + 2.6 endless ranges)
- New unconditional `fuse_range_ops` post-pass — folds adjacent `Dot` tokens into `Op("..")` (inclusive range) or `Op("...")` (exclusive range).  Ruby has had range literals since 1.0, so this fires for every era.
- New `ENDLESS_RANGE_FLAG: u32 = 1 << 1` — set on `..` / `...` range tokens under era ≥ 2.6 when they're followed by a *closer* (`)`, `]`, `}`, `,`, `;`, `\n`, or EOF).  Pre-2.6 these positions were parse errors; 2.6 made them legal endless ranges (`(1..)`, `arr[2..]`, etc.).
- New `mark_endless_ranges` post-pass runs only under era ≥ 2.6.

### Tests (+6 new, total 103)
- `..` fuses unconditionally across eras 1.0/1.8/2.0/3.3.
- `...` fuses unconditionally.
- 2.6 flags `(1..)` as endless range.
- 2.3 does NOT flag — era gate is precise.
- 2.6 leaves normal ranges (`1..5`) unflagged.
- 2.6 flags endless ranges before newline and before comma.

## [0.10.0] - 2026-05-20

### Added (Phase 4d — 2.7 numbered block params `_1`..`_9`)
- New public constant `NUMBERED_BLOCK_PARAM_FLAG: u32 = 1 << 0` — flag bit set on `Token.flags` for Name tokens whose lexeme matches the `_<digit>` (1–9) pattern under era ≥ 2.7.
- New `mark_numbered_block_params` post-pass — runs under era ≥ 2.7 and tags every `Name` token whose value is `_1`..`_9` with the flag bit.  Pre-2.7 eras leave the flag clear so callers treat them as ordinary locals.
- The lexer can't tell whether a given `_1` is actually *inside a block* (that's parser-level context), but it can flag every `_N` lexeme as a *candidate* numbered-param so downstream consumers can apply the era-aware semantics without re-scanning the token stream.
- `is_numbered_block_param` helper exactly matches `_1`..`_9` and explicitly excludes `_0`, `_10`, `_`, `_foo`, etc.

### Tests (+5 new, total 97)
- Era 2.7 flags `_1` and `_2`.
- Era 2.6 does NOT flag them — the era gate is precise.
- Era 2.7 leaves `_foo`, `_`, `_0`, `_10` unflagged (they're not numbered params).
- Eras 2.7, 3.0, 3.3 all flag `_1` consistently.
- `is_numbered_block_param` classifier table-test covering the +/- boundaries.

## [0.9.0] - 2026-05-20

### Added (Phase 4c — 2.3 safe-nav `&.` token fusion)
- New `fuse_safe_nav` post-pass — under era ≥ 2.3, adjacent `Op("&")` + `Dot(".")` tokens fuse into a single `Op("&.")` (the safe-navigation operator).  Pre-2.3 eras keep them split so the lexing is faithful to what each Ruby version originally accepted.
- **Whitespace-aware adjacency**: a new parallel `whitespace_before_token: Vec<bool>` field on `RubyLexer` records whether any whitespace (`' '` / `'\t'`) was consumed between the previous emit and this one.  The fusion post-pass consults this array directly — peek-state operators (`>`, `&`, `=`, `!`, `<`, `|`) report the column of the *follower* character, so the per-token column field alone can't always distinguish `&.` from `& .`.  Tracking whitespace explicitly is robust.
- Both Phase 4b's `fuse_lambda_arrow` and the new `fuse_safe_nav` now share the same adjacency criterion: *same line, no whitespace between*.  The fragile "column distance ≤ 2" heuristic from Phase 4b is replaced.

### Tests (+5 new, total 92)
- 2.3 fuses `a&.b`; 1.8 does NOT; `a & .b` (with whitespace) does NOT fuse even under 2.3.
- Eras 2.5, 2.7, 3.0, 3.3 all inherit the fusion from 2.3.
- Era 2.1 (one notch before 2.3) does NOT fuse — the era gate is precise.

## [0.8.0] - 2026-05-20

### Added (Phase 4b — 1.9.1 lambda `->` token fusion)
- `RubyLexer` now records its target era (`era: String`) and applies era-gated token-stream rewrites in `finish()` after the engine reaches its final state.
- New post-pass `apply_era_token_fusions` — extensible hook for every era's "compose multiple 1.8 tokens into a single newer-era operator" rewrite.  Phase 4b ships the first one; later phases (`&.` in 2.3, etc.) plug in here without touching the TOML state machine.
- **Lambda arrow `->` (1.9.1+)** — adjacent `Op("-")` + `Op(">")` tokens, with no source whitespace between them, fuse into a single `Op("->")` token under era ≥ 1.9.1.  The 1.8 baseline keeps them as two tokens (Ruby 1.8 doesn't know about lambda literals).
- New `era_at_least` helper — total ordering of era strings against `machine::ERA_VERSIONS` (chronological).  Unknown era strings fold to the `1.8` baseline so misconfigured callers get the conservative behaviour.

### Adjacency heuristic
- The 1.8 state machine emits single-char operators like `>` on a *follower* character (it peeks one char ahead to disambiguate `>` vs `>=`), so the `>` token's recorded column is the *follower's* column, 1–2 ahead of the `>` source position.  The fusion check allows a "virtual gap" of up to 2 columns — strictly less than the ≥ 3 a real whitespace-separated `-` `>` would produce.

### Tests (+5 new, total 87)
- 1.9.1 fuses `->(a)` to a single token.
- 1.8 does NOT fuse `->(a)` — the era gate works.
- `1 - > 2` (with a space) does NOT fuse even under 1.9.1 — the adjacency check is strict enough.
- Eras 2.0, 2.3, 2.7, 3.0, 3.3 all inherit the lambda fusion from 1.9.1.
- `era_at_least` is total + chronological, with sensible fallback for unknown / empty era strings.

## [0.7.0] - 2026-05-20

### Added (Phase 4a — 15-era version dispatch)
- `ERA_VERSIONS: &[&str]` constant listing every era version modelled by [code/specs/ruby-version-evolution.md](../../../specs/ruby-version-evolution.md): `"1.0"`, `"1.6"`, `"1.8"`, `"1.9.1"`, `"1.9.3"`, `"2.0"`, `"2.1"`, `"2.3"`, `"2.5"`, `"2.6"`, `"2.7"`, `"3.0"`, `"3.1"`, `"3.2"`, `"3.3"` (chronological order).  Re-exported from the crate root.
- `tokenize_ruby_for_version(source, version)` convenience entry point.  Validates `version` against `ERA_VERSIONS` and returns a `Result<Vec<Token>, String>`; the existing `RubyLexer::new(version)` constructor accepts the same set of strings.
- `definition_for_version` now accepts any era string (was: only `"1.8"`) and tags the returned `StateMachineDefinition`'s `name` field with the requested era (e.g. `ruby-2.3-lexer`) so downstream tooling can identify which grammar produced the tokens.
- Error message on unknown versions now points callers at the spec (`see code/specs/ruby-version-evolution.md`) so they know where the canonical era list lives.

### Notes
- v0 inheritance model: every era currently shares the **1.8 baseline TOML** — only the machine name differs.  This is deliberate: physically duplicating ~1100 lines of TOML 14 times would be massive churn for zero behaviour change in this PR.  Phase 4b+ will fork the era TOMLs as real syntactic deltas land (lambda `->` in 1.9.1, `%i[]` in 2.0, `&.` and `<<~` in 2.3, endless ranges in 2.6, numbered block params in 2.7, …).
- The version-string surface is the load-bearing piece of this phase: callers that need version-gated tooling can already plumb the era string through; the underlying grammar will diverge incrementally as later phases land.

### Tests (+5 new, total 82)
- All 15 era versions parse cleanly and produce a uniquely-named `StateMachineDefinition`.
- `ERA_VERSIONS` has no duplicates and is chronological (1.8 present, 3.3 last).
- `tokenize_ruby_for_version` produces the expected token stream for the baseline source `"x = 1 + 2\n"` under every era.
- Unknown version strings produce a helpful error pointing at the spec.

## [0.6.0] - 2026-05-20

### Added (Phase 3c — heredocs `<<TAG`)
- `<<` is now recognized as a single Op token in [`ruby-1.8.lexer.states.toml`](./ruby-1.8.lexer.states.toml) (new transition `after_lt + "<" → emit(Op)` with text `<<`).  In Phase 1/2/3a/3b the engine emitted two separate `<` Ops, never `<<`.
- `pending_heredocs: VecDeque<PendingHeredoc>` field on `RubyLexer`.  Each entry tracks the tag string, the indices of the `<<` Op and tag Name tokens in `self.tokens`, and the body buffer.  Multiple heredocs on one line are queued FIFO and finalized in source order.
- `heredoc_op_candidate: Option<usize>` field on `RubyLexer`.  Set when an `<<` Op is emitted at expression-start (`ExprBeg` / `ExprMid` per the Phase 2 lex-state machine); consumed on the very next emit — if it's a name-shaped Name or Keyword token, the heredoc is queued.
- `RubyLexer::push` rewritten to be heredoc-aware.  After every `\n` that closes a line with pending heredoc openers, control jumps to `capture_heredoc_bodies`, which slurps whole lines from the source cursor (bypassing the engine) until each pending terminator has been seen, then resumes normal lexing.
- `finalize_heredoc` splices each captured heredoc into the token stream — the `<<` Op becomes a `String` token carrying the verbatim `<<TAG\n<body>TAG` source-shape; the tag Name (or Keyword) token is removed.  Replacements are applied in reverse index order so earlier indices remain valid as later tokens are removed.

### Tests (+12 new, total 77)
- Simple, empty-body, single-line-body, and chained-method (`<<EOF.upcase`) cases.
- Multi-heredoc-per-line FIFO (`<<A; y = <<B\nA body\nA\nB body\nB`) — bodies arrive in opener order.
- Lowercase tag (`<<eof`) and keyword-shaped tag (`<<END`) are both accepted.
- `<<` after a value (`3 << 1`) is a left-shift operator, not a heredoc opener.
- `<<EOF` after `(` is recognized (paren is expression-start).
- Unterminated heredoc records an `unterminated-heredoc` diagnostic and still emits the partial body as a `String` token.
- Body content with `#{...}` interpolation syntax is preserved verbatim (no `#{}` expansion at the lexer level, matching the Phase 3b precedent).
- Heredoc tag shape predicate `is_heredoc_tag` accepts identifiers (letters/digits/underscore, non-digit first) and rejects empty / digit-leading / whitespace-containing inputs.

### Deferred (subsequent Phase 3 follow-ups)
- Phase 3d: indent-modifier heredoc forms — `<<-TAG` (terminator may be indented) and `<<~TAG` (terminator may be indented *and* common leading whitespace is stripped from every body line).
- Quoted-tag forms: `<<"TAG"` (interpolating, same as `<<TAG`) and `<<'TAG'` (non-interpolating, suppress `#{...}` even semantically).
- Recursive sub-lexing of `#{...}` expressions in heredoc bodies (currently captured verbatim, same as Phase 3b strings).

## [0.5.0] - 2026-05-20

### Added (Phase 3b — string interpolation `"#{...}"`)
- `string_d_hash` and `string_d_interp` states in [`ruby-1.8.lexer.states.toml`](./ruby-1.8.lexer.states.toml).  `"..."` body now branches on `#`: if followed by `{` it enters interpolation (`string_d_interp`), otherwise the `#` is treated as a literal character and the follower is re-dispatched to `string_d_body`.
- New `interp_brace_depth: usize` field on `RubyLexer`.  Tracks nesting of `{` / `}` inside an interpolation expression so that `"#{ {a: 1} }"` correctly closes only on the outermost matching `}`.
- The action interpreter intercepts `}` in `string_d_interp` — at depth 0 it appends the `}` to the string buffer and forces the engine back to `string_d_body` via `set_current_state`.  Same pattern as the Phase 2 `/` interceptor.
- v0 captures interpolation **verbatim** — the `#{expr}` substring is preserved inside the `TokenType::String` value.  Recursive sub-lexing of the embedded expression is a future refinement; the parser can dispatch on the `#{` substring when it needs to evaluate.

### Tests (+11 new, total 65)
- Simple, expression, at-start, at-end, multiple-interpolation cases.
- `#` without following `{` is literal (`"a # b"`, `"trailing #"`).
- Nested braces inside interpolation (`"#{ {a: 1} }"`) close correctly.
- Method call inside interpolation (`"#{arr.length}"`).
- Single-quoted strings do **not** interpolate (`'#{name}'` stays as `#{name}`).
- Embedded double-quotes inside `#{...}` are accepted (brace tracker is string-agnostic).

### Deferred (subsequent Phase 3 follow-ups)
- Phase 3c: heredocs (`<<X`, `<<-X`, `<<~X`) with deferred body capture and FIFO queue for multi-per-line heredocs.
- Future refinement: recursive sub-lexing of the interpolation expression (so the parser receives individual tokens for the embedded code instead of one verbatim string).

## [0.4.0] - 2026-05-20

### Added (Phase 3a — regex flags + `%w[]` / `%q{}` percent literals)
- Regex flag suffixes (`/foo/i`, `/foo/im`, `/foo/IM`, …) — both cases accepted, greedy slurp on `imxoeunsIMXOEUNS`.
- `%w[...]` string-array percent literal (canonical `[` delimiter only in v0).
- `%q{...}` non-interpolating string percent literal (canonical `{` delimiter only in v0).
- New `regex_flags`, `after_percent`, `percent_w_open`, `percent_w_body`, `percent_q_open`, `percent_q_body` states in [`ruby-1.8.lexer.states.toml`](./ruby-1.8.lexer.states.toml).
- New `PercentW` and `PercentQ` token kinds in the declared token vocabulary (action interpreter maps both to `TokenType::String` with the verbatim source-shape preserved as the value).
- 11 new unit tests; total now 54 (was 43 in Phase 2).

### Changed
- The `data → %` transition now routes through `after_percent` to peek for a type letter.  When the follower is not `w` or `q`, `%` falls back to the modulo operator (matches Phase 1 behaviour).
- The `regex_body → /` closing transition now goes to `regex_flags` (instead of `data`), appending the `/` to the text buffer.  When `regex_flags` exits, the emitted token's value has the source-shape `/body/` (no flags) or `/body/flags` (with flags).

### Deferred (subsequent Phase 3 follow-ups)
- Phase 3b: string interpolation `"a#{expr}b"` — recursive sub-lexer / sub-machine.
- Phase 3c: heredocs (`<<X`, `<<-X`, `<<~X`) with deferred body capture and FIFO queue for multi-per-line heredocs.
- Additional percent literals: `%W`, `%Q`, `%i`, `%I`, `%r`, `%s`, `%x` and the full set of delimiter pairs (`(...)`, `<...>`, `|...|`, any non-alphanumeric).

## [0.3.0] - 2026-05-20

### Added (Phase 2 — parser-feedback)
- `LexState` enum (`ExprBeg`, `ExprMid`, `ExprEnd`, `ExprArg`, `ExprFname`, `ExprDot`) — tracked by the lexer and updated after every emitted token.  See `code/specs/ruby-lexer-state-machine.md` §1.
- `ParserOracle` trait with default impls (`NoLocals`, `StaticLocals`).  Consulted by the lexer when `/` follows a name in `ExprArg` position — local-variable names get binary division, method-like names get a regex literal.  See `code/specs/ruby-lexer-state-machine.md` §3.
- `RubyLexer::with_oracle(version, oracle)` constructor and `tokenize_ruby_with_oracle(source, oracle)` convenience entry point.
- `regex_body` / `regex_escape` sub-states in `ruby-1.8.lexer.states.toml`.  Reached via `set_current_state` from the action interpreter once it has decided to open a regex literal; the engine then accumulates the body until the closing `/`.
- Regex literals emit as `TokenType::String` tokens with the value framed as `/.../` so the parser can dispatch by lexeme until a dedicated `TokenType::Regex` lands in a later phase.

### Changed
- `tokenize_ruby` default semantics for `/` after a name now follow the spec: with the `NoLocals` oracle every name is a method, so `f /x/` lexes as a method call with a regex argument.  Callers that want binary division on locals must pass a `ParserOracle` via `tokenize_ruby_with_oracle` (or `RubyLexer::with_oracle`).
- The Phase 1 `binary_operators_dispatch_to_dedicated_kinds` test was rewritten to pass an explicit `StaticLocals` oracle declaring its operands as locals.

### Notes
- Phase 2 leaves the `+` / `-` / `*` whitespace-sensitive disambiguation untouched — the spec defers it to a Phase 2b refinement.  Only `/` is interpreted via lex-state + oracle in this cut.
- Regex flags (`/foo/i`, `/foo/m`, …) remain unhandled — they arrive alongside heredoc / interpolation in Phase 3.

## [0.2.0] - 2026-05-19

### Changed (BREAKING)
- Replaced the regex-based `lexer::GrammarLexer` backend with a TOML-encoded state machine driven by `state_machine::EffectfulStateMachine`.  This is **Phase 1** of the multi-phase plan in [code/specs/ruby-parser.md](../../../specs/ruby-parser.md).
- Source of truth is [`ruby-1.8.lexer.states.toml`](./ruby-1.8.lexer.states.toml) at the crate root; the action interpreter in `src/lib.rs` turns its effect strings into `lexer::token::Token` values.
- `create_ruby_lexer(source)` is gone; replaced by `RubyLexer::new(version)` which constructs a versioned lexer (`"1.8"` is currently the only accepted value).
- `tokenize_ruby(source)` keeps its signature.  A new `tokenize_ruby_diag(source)` variant returns the diagnostic list alongside the tokens.

### Added
- `RubyLexer` struct with explicit `push` / `finish` / `drain_tokens` / `diagnostics` methods.
- `Diagnostic` struct for non-fatal lex errors — the lexer never panics on malformed input.
- Newline as a first-class token (`TokenType::Newline`); Ruby treats `\n` as a statement terminator.
- Method-name suffixes `?` and `!` are now part of the identifier token (`empty?`, `save!`).

### Phase 1 scope
- Identifiers (with `?` / `!` suffix), integers (with `_` separators), strings (`"..."` and `'...'`, no interpolation), line comments, common operators (`+ - * / % == != < > <= >= = ! && || => ** ::` …), and basic punctuation.
- **Heredocs, regex literals, string interpolation, parser-driven `f /x/` disambiguation, and the 1.9.1+ syntax additions are deferred to subsequent phases** (see [ruby-parser.md](../../../specs/ruby-parser.md) §"Phasing").

## [0.1.0] - 2026-03-21

### Added
- `create_ruby_lexer(source)` — factory function that loads `ruby.tokens` and returns a configured `GrammarLexer`.
- `tokenize_ruby(source)` — convenience function that tokenizes Ruby source and returns `Vec<Token>`.
- Loads grammar from `ruby.tokens` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering assignments, keywords, arithmetic operators, comparison operators, strings, numbers, comments, delimiters, whitespace, method definitions, symbols, and the factory function.
