# Changelog — mccarthy-lisp-lexer

## v0.2.0 — 2026-06-03 — grammar-driven rewrite (L1)

**Breaking:** replaced the hand-written tokenizer with a thin wrapper
over the shared, grammar-driven `GrammarLexer`.  The McCarthy 1.0
dialect now lives in `code/grammars/mccarthy_lisp.tokens`, compiled to
Rust at build time via a `build.rs` (the `twig-lexer` pattern).  This
brings the crate in line with the repo rule that language frontends
must wrap `GrammarLexer`/`GrammarParser` rather than hand-write
lexers/parsers.

* **Removed** the hand-written `Token` enum, `TokenWithLoc`, `Loc`, and
  the bespoke `LexError` (with its `LoneMinus` / `LowercaseInSymbol` /
  `IntegerOverflow` variants).
* **Added**:
  * `tokenize_mccarthy(src) -> Result<Vec<lexer::token::Token>, LexerError>`
  * `create_mccarthy_lexer(src) -> GrammarLexer`
  * `mccarthy_token_grammar_spec() -> &'static TokenGrammar`
  * re-exports of `lexer::token::{Token as LispToken, TokenType, LexerError}`.
* The dialect restrictions (all-uppercase symbols, integers only, no
  strings, no operator symbols) are now enforced by the token regexes
  in `mccarthy_lisp.tokens`.  Lowercase, bare `-`, and string literals
  are still rejected — now as generic `LexerError`s rather than bespoke
  variants.
* New deps: `grammar-tools`, `lexer` (+ `grammar-tools` build-dep).

## v0.1.0 — 2026-06-03 — initial release (L1)

McCarthy 1960 Lisp tokenizer.  Recognises all-uppercase atoms,
signed-decimal integers, parens, `'` quote sugar, `.` dotted-pair
separator, and `;` line comments (a Lisp 1.5 convenience).

* `Token` enum with 6 variants + a `Loc { line, column }` triple.
* `tokenize(src) -> Result<Vec<TokenWithLoc>, LexError>`.
* `LexError` carries the offending byte, line, column, and a
  human-readable reason.

Tests pin every token shape and the standard McCarthy example
sources from the 1960 paper (`(CAR '(A B C))`, `(LAMBDA (X) X)`,
etc.).
