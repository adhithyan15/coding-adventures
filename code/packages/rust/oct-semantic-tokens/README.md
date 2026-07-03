# `oct-semantic-tokens` — semantic-token extraction for Oct

Walks the `oct-lexer` token stream and emits a typed token stream
— keywords / booleans / numbers / names / types / operators /
comments — suitable for **LSP semantic-tokens**, syntax
highlighters, and editor extensions.

The Oct counterpart to `twig-semantic-tokens`,
`basic-semantic-tokens`, and `nib-semantic-tokens`.  **Final piece**
of the syntax-highlighting phase (task #40 part 3 of 3).

## Public API

- `semantic_tokens(source: &str) -> Vec<SemanticToken>` — tokenise + classify.
- `tokens_from(tokens: &[Token]) -> Vec<SemanticToken>` — skip the
  lex step when callers already have a `Vec<Token>` in hand.

Tokens come back in **document order**.

## Position model

All positions are **1-based** `(line, column)` in monospace cell
units.

## Token classifications

| Kind     | Examples                                                  |
|----------|-----------------------------------------------------------|
| Keyword  | `fn`, `let`, `static`, `if`, `else`, `while`, `loop`, `break`, `return` |
| Intrinsic | `in`, `out`, `adc`, `sbb`, `rlc`, `rrc`, `ral`, `rar`, `carry`, `parity` — Oct's 8008 hardware intrinsics, distinguished from ordinary keywords so themes can colour hardware ops specially |
| Boolean  | `true`, `false`                                           |
| Number   | `42`, `0xFF`, `0b1010`                                    |
| Type     | `u8`, `bool` (the known Oct types)                        |
| Variable | NAME tokens that are not a recognised type                |
| Operator | `+`, `-`, `&`, `\|`, `^`, `~`, `!`, `=`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `&&`, `\|\|` |
| Punctuation | `,`, `;`, `:`, `(`, `)`, `{`, `}`, `->`                |
| Comment  | `// …` to end of line (if the lexer preserves it)         |

## What this crate does NOT do

- **No LSP encoding.**  Returns a typed `Vec<SemanticToken>`;
  conversion to LSP's delta-encoded wire format is one level up.

## Versions

- `0.1.0` — initial release.

See [CHANGELOG.md](./CHANGELOG.md) for details.
