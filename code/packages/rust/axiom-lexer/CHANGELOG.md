# Changelog

## [0.1.0] - 2026-07-27

### Added

- Initial grammar-driven Rust Axiom tokenizer (MA13 §6, task MA-13b),
  following MA-13a's design-only kickoff spec
  ([`MA13-axiom-language.md`](../../../specs/MA13-axiom-language.md)).
- `code/grammars/axiom/axiom.tokens`, written to Axiom's own ordinary
  infix-expression CAS-family shape (MA13 §5) — structurally closer to
  `reduce.tokens`/`derive.tokens`/`maple.tokens` than to any array-family
  grammar in this repo. Covers the MA-13b-scoped surface:
  - Integer/float numeric literals (one `NUMBER` pattern, leading digit
    required, optional exponent — matching every sibling CAS-family
    lexer's convention).
  - **No dedicated rational-literal token**: `1/3` lexes as ordinary
    `NUMBER SLASH NUMBER` (three tokens), confirmed against MA13 §4's own
    surface table rather than assumed — the packed rational
    representation is built entirely at a later (lowering/evaluation)
    layer.
  - String literals (`"hello"`, one `STRING` token; `escapes: none` —
    no confirmed backslash-escape convention anywhere in MA13 §2/§3/§4,
    the same conservative choice `derive.tokens`/`maple.tokens`/
    `reduce.tokens`/`idl.tokens` already make absent a confirmed escape
    rule).
  - Identifiers (`NAME`), covering both ordinary variables/functions
    **and** every built-in domain/category name (`Integer`, `Boolean`,
    `Ring`, `PositiveInteger`, ...) — none of the fixed built-in
    domain/category table (MA13 §3) is a lexer-level keyword; only
    `if`/`then`/`else`/`has` are.
  - Parens/brackets/comma: `( ) [ ] ,`.
  - Arithmetic: `+ - * /`, both power spellings `^` (CARET) and `**`
    (POW, kept distinct — mirroring `reduce.tokens`'s own CARET/POW
    split; the parser collapses them onto one production).
  - Comparison: `=` (EQ), `~=` (NE — Axiom's own confirmed not-equal
    spelling, explicitly **not** Maple's `<>` or Wolfram's `!=`), and
    `< <= > >=`.
  - `:=` (ASSIGN, immediate assignment) — distinct from `==` (DEFINE,
    held-body function definition): Axiom needs BOTH tokens where
    Reduce/Derive/Maple each only needed one.
  - `:` (COLON, declaration — overloaded with the function-header
    type-annotation position at the lexer level, disambiguated later by
    the parser) and `::` (COERCE, coercion) — two of the three
    genuinely new tokens no prior symbolic-family grammar in this repo
    has needed (MA13 §3's central finding).
  - `has` (KEYWORD, category-membership query infix operator) — the
    third genuinely new token.
  - `;` (SEMI, the separator inside a parenthesised, semicolon-separated
    block).
  - `if`/`then`/`else` conditional keywords, lowercase and
    case-sensitive (matching `reduce.tokens`/`maple.tokens`'s identical
    convention, the mirror image of `derive.tokens`'s uppercase
    keywords).
  - `--` line comments (FriCAS/SPAD convention), an ordinary `skip:`
    pattern with no pre/post-tokenize hook needed — verified that
    `GrammarLexer`'s skip-pattern pass always runs before ordinary token
    matching, so `--` never collides with the bare `MINUS` token, the
    same declarative shape `sql.tokens`/`vhdl*.tokens`/`haskell*.tokens`
    already rely on.
  - No significant newline: blocks are `;`-separated (MA13 §4), never
    newline-separated — `axiom-repl`'s own numbered-prompt step counter
    (MA13 §5) is a REPL-layer concern, not a lexer one — mirroring
    `reduce.tokens`'s/`maple.tokens`'s identical rationale. No NEWLINE
    token, no bracket-interior-newline hook needed.
- **Finding: no pre/post-tokenize hooks needed at all.** Every
  multi-character operator is resolved by ordinary longest-match-first
  declaration order; `axiom.tokens` is entirely declarative, and
  `create_axiom_lexer` installs nothing beyond the compiled grammar —
  the same hook-free shape `idl-lexer` established, simpler than
  `q-lexer` (two hooks) or `scilab-lexer` (one hook).
- **No recursion-depth cap, by design.** `axiom-lexer` performs no
  recursive descent at all — tokenization is a single left-to-right scan
  with O(1) stack depth regardless of source nesting. This mirrors every
  sibling `*-lexer` crate in this repo (`idl-lexer`, `q-lexer`,
  `scilab-lexer`, `apl-lexer`, `j-lexer`), all of which document the
  identical finding and add no depth cap of their own — a
  `MAX_RULE_DEPTH`-style cap is a *parser*-level concern (this repo's own
  established convention for recursive-descent parsers, per
  `lessons.md`), belonging to a future `axiom-parser` (MA-13c), the first
  layer in Axiom's frontend that actually recurses. Verified directly
  with a regression tokenizing 50,000 levels of nested parens
  (`deeply_nested_parens_do_not_overflow_the_lexer_stack`), plus wide
  (non-nested) adversarial inputs covering the other half of the DoS
  surface: a 100,000-token flat stream and a 500,000-character comment.
- `code/packages/rust/Cargo.toml` workspace registration alongside the
  other CAS/array-language lexer/parser/runtime/repl/to-semantic-ir crate
  groups (only `axiom-lexer` itself is added; the sibling `axiom-parser`/
  `axiom-runtime`/`axiom-repl`/`axiom-to-semantic-ir` crates do not exist
  yet — they are MA-13c/d/e, separate follow-on tasks).
- 63 tests (6 lib smoke tests + 56 integration tests + 1 doc test),
  100% line coverage on this crate's own `src/lib.rs` and
  `src/_grammar.rs` (verified with `cargo tarpaulin -p
  coding-adventures-axiom-lexer`), covering: every integer/float/exponent
  numeric-literal shape and the leading-digit requirement; the
  rational-is-ordinary-division finding; string literals (including
  empty strings and grammar-punctuation-shaped content); plain
  identifiers and every built-in domain/category name resolving to
  `NAME`, never `KEYWORD`; case-sensitivity (uppercase `IF`/`HAS` are
  `NAME`s); parens/brackets/comma including the paren-optional
  single-argument call shape; every arithmetic operator including both
  power spellings and the `POW`-wins-over-`TIMES` longest-match
  regression; every comparison operator including the `~=`-not-Maple's-
  `<>`-not-Wolfram's-`!=` divergence-avoidance check and the
  `LE`/`GE`-win-over-`LESS`/`GREATER` longest-match regression; the
  `ASSIGN`/`COERCE`/`COLON`/`DEFINE`/`EQ` five-way longest-match
  regression family (including edge cases like `"::="` and `":=:"`); a
  held-body function definition, an undeclared function definition, a
  plain declaration, a tuple declaration, and a coercion expression; a
  true and a false `has` query (mirroring MA13's own worked
  `Polynomial(Integer) has Ring` / `List(Integer) has Ring` examples); an
  `if`-`then`-`else` conditional; a parenthesised `;`-separated block;
  `--` comments (mid-line, whole-line, non-newline-swallowing, and
  containing out-of-grammar characters) alongside a `MINUS`-is-not-
  swallowed-by-`--` regression; newlines-are-ordinary-whitespace; every
  explicitly deferred construct (`Record`/`Union`/`Any` and `macro` all
  lexing as plain `NAME`s, never reserved words; `$`/`@` as honest lex
  errors; and `+->`/`=>` decomposing into their constituent
  single-character tokens rather than erroring, since this cut declares
  no dedicated multi-character token for either); unrecognized-character
  errors; an unterminated string
  failing honestly rather than panicking; the two DoS-guard regressions
  described above; and a small realistic end-to-end "textbook Axiom
  session" snippet combining declaration, assignment, function
  definition, conditional, and a `has` query in one source string.
