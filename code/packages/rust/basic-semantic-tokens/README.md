# `basic-semantic-tokens` — semantic-token extraction for Dartmouth BASIC

Walks the `dartmouth-basic-lexer` token stream and emits a typed
token stream — line numbers / keywords / numbers / strings /
variables / built-in functions / user functions / operators —
suitable for **LSP semantic-tokens**, syntax highlighters, and
editor extensions.

The BASIC counterpart to `twig-semantic-tokens`.  First piece of
the BASIC LSP / editor stack (task #38).

## Public API

- `semantic_tokens(source: &str) -> Vec<SemanticToken>` — the
  common case.  Tokenises and classifies.
- `tokens_from(tokens: &[Token]) -> Vec<SemanticToken>` — skip
  the lex step when you already have a `Vec<Token>` in hand.

Tokens come back in **document order**, which is what every LSP
client wants.

## Position model

All positions are **1-based** `(line, column)` in monospace cell
units, matching the underlying `lexer` crate.  `length` is the
visible width of the token in cells (char count for ASCII source
— BASIC identifiers are ASCII).

## Token classifications

| Kind      | Examples                                  |
|-----------|-------------------------------------------|
| LineNumber | `10`, `100`, `9999` at start of a line   |
| Keyword   | `LET`, `PRINT`, `INPUT`, `IF`, `THEN`, `GOTO`, `FOR`, `NEXT`, `END`, `REM`, … |
| Number    | `42`, `3.14`, `-7`                        |
| String    | `"HELLO"`, `"WORLD"`                       |
| Variable  | `A`, `B7`, `X0`                            |
| BuiltinFn | `SIN`, `COS`, `SQR`, `ABS`, `INT`, `RND`  |
| UserFn    | `FNA`, `FNB`, … (DEF-FN names)            |
| Operator  | `+`, `-`, `*`, `/`, `=`, `<`, `>`, `<=`, `>=`, `<>` |
| Punctuation | `,`, `;`, `:`, `(`, `)`                  |

## What this crate does NOT do

- **No comment tokens.**  `REM`-lines are recognised as keyword + a
  rest-of-line comment payload; the comment payload is dropped
  (the BASIC lexer collapses it).  When the lexer grows a trivia
  channel, the comment will become a semantic token here too.
- **No newline / EOF tokens.**  Editors don't colour either.
- **No LSP encoding.**  Returns a typed `Vec<SemanticToken>`;
  conversion to LSP's delta-encoded wire format is one level up
  (so this crate stays usable from non-LSP consumers like syntax
  highlighters).

## Versions

- `0.1.0` — initial release.

See [CHANGELOG.md](./CHANGELOG.md) for details.
