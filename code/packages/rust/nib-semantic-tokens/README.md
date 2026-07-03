# `nib-semantic-tokens` — semantic-token extraction for Nib

Walks the `nib-lexer` token stream and emits a typed token stream —
keywords / booleans / numbers / names / types / operators —
suitable for **LSP semantic-tokens**, syntax highlighters, and
editor extensions.

The Nib counterpart to `twig-semantic-tokens` and
`basic-semantic-tokens`.  Part 2 of 3 of the syntax-highlighting
phase (task #39).

## Public API

- `semantic_tokens(source: &str) -> Vec<SemanticToken>` — tokenise + classify.
- `tokens_from(tokens: &[Token]) -> Vec<SemanticToken>` — skip the
  lex step when callers already have a `Vec<Token>` in hand.

Tokens come back in **document order**, which is what every LSP
client wants.

## Position model

All positions are **1-based** `(line, column)` in monospace cell
units.  `length` is the visible width of the token in cells.

## Token classifications

| Kind     | Examples                                            |
|----------|-----------------------------------------------------|
| Keyword  | `fn`, `let`, `static`, `const`, `return`, `for`, `while`, `in`, `if`, `else` |
| Boolean  | `true`, `false`                                     |
| Number   | `7`, `42`, `0xFF`                                   |
| Type     | `u4`, `u8`, `u16`, `u32`, `bool` (the known Nib types) |
| Variable | NAME tokens that are not a recognised type          |
| Operator | `+`, `-`, `*`, `/`, `=`, `<`, `>`, `==`, etc.       |
| Punctuation | `,`, `;`, `:`, `(`, `)`, `{`, `}`, `->`           |

## What this crate does NOT do

- **No comment tokens.**  Nib comments don't survive into the
  lexer's token stream today.  When the lexer grows a trivia
  channel, the comment payload will become a semantic token here
  too.
- **No newline / EOF tokens.**  Editors don't colour either.
- **No LSP encoding.**  Returns a typed `Vec<SemanticToken>`;
  conversion to LSP's delta-encoded wire format is one level up.

## Versions

- `0.1.0` — initial release.

See [CHANGELOG.md](./CHANGELOG.md) for details.
