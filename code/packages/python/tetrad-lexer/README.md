# tetrad-lexer

The first stage of the [Tetrad](../../specs/TET00-tetrad-language.md) pipeline.  It converts raw Tetrad source text into a flat, ordered list of `Token` objects ready for the parser.

## What is Tetrad?

Tetrad is a small interpreted language whose bytecode runs on a register VM small enough to execute on an **Intel 4004** — 128 bytes of usable RAM, 4 KB ROM.  The name plays on *tetrad* (a 4-element group), echoing the 4-bit word width of the 4004.

## Where this fits in the pipeline

```
source text
    → [tetrad-lexer]       token stream   ← you are here
    → [tetrad-parser]      AST
    → [tetrad-type-checker] typed AST
    → [tetrad-compiler]    bytecode (CodeObject)
    → [tetrad-vm]          execution + metrics
    → [tetrad-jit]         native code for hot functions
```

## Public API

```python
from tetrad_lexer import tokenize, Token, TokenType, LexError

tokens = tokenize("fn add(a: u8, b: u8) -> u8 { return a + b; }")
for tok in tokens:
    print(tok)
```

### `tokenize(source: str) -> list[Token]`

Lex a Tetrad source string.  Returns a list ending with `Token(EOF)`.  Raises `LexError` on the first illegal character or malformed literal.

The lexer:
- Skips all ASCII whitespace (space, tab, CR, LF).
- Skips C-style line comments (`//` to end of line).
- Uses **maximal munch** for two-character operators (`->` before `-`, `<<` before `<`, etc.).
- Tracks 1-based line and column numbers, plus 0-based byte offset, for every token.

### `Token`

A frozen dataclass:

| Field    | Type              | Description                                              |
|----------|-------------------|----------------------------------------------------------|
| `type`   | `TokenType`       | Token category                                           |
| `value`  | `int \| str \| None` | `int` for INT/HEX; `str` for IDENT; `None` otherwise |
| `line`   | `int`             | 1-based line number of the first character               |
| `column` | `int`             | 1-based column number of the first character             |
| `offset` | `int`             | 0-based byte offset into the source string               |

### `TokenType`

Enum of all 42 token categories: literals (`INT`, `HEX`), `IDENT`, 9 keywords (`fn`, `let`, `if`, `else`, `while`, `return`, `in`, `out`, `u8`), arithmetic/bitwise/comparison/logical operators, two-character tokens (`<<`, `>>`, `==`, `!=`, `<=`, `>=`, `&&`, `||`, `->`) plus `COLON`, delimiters, and `EOF`.

### `LexError`

Raised on illegal characters or malformed literals.  Carries `.line` and `.column` attributes.

## Token categories

### Literals

| Token | Example | `value` |
|-------|---------|---------|
| `INT` | `42` | `42` (Python `int`) |
| `HEX` | `0xFF` | `255` (Python `int`) |

### Identifiers and keywords

| Token | Lexeme |
|-------|--------|
| `IDENT` | any `[a-zA-Z_][a-zA-Z0-9_]*` not in reserved set |
| `KW_FN` | `fn` |
| `KW_LET` | `let` |
| `KW_IF` | `if` |
| `KW_ELSE` | `else` |
| `KW_WHILE` | `while` |
| `KW_RETURN` | `return` |
| `KW_IN` | `in` |
| `KW_OUT` | `out` |
| `KW_U8` | `u8` |

### Operators (two-char checked first, then one-char)

| Token | Lexeme | Note |
|-------|--------|------|
| `SHL` | `<<` | shift left |
| `SHR` | `>>` | shift right |
| `EQ_EQ` | `==` | equality |
| `BANG_EQ` | `!=` | inequality |
| `LT_EQ` | `<=` | less-or-equal |
| `GT_EQ` | `>=` | greater-or-equal |
| `AMP_AMP` | `&&` | logical AND |
| `PIPE_PIPE` | `\|\|` | logical OR |
| `ARROW` | `->` | return-type annotation |
| `PLUS` | `+` | |
| `MINUS` | `-` | |
| `STAR` | `*` | |
| `SLASH` | `/` | |
| `PERCENT` | `%` | modulo |
| `AMP` | `&` | bitwise AND |
| `PIPE` | `\|` | bitwise OR |
| `CARET` | `^` | bitwise XOR |
| `TILDE` | `~` | bitwise NOT |
| `BANG` | `!` | logical NOT |
| `EQ` | `=` | assignment |
| `COLON` | `:` | type annotation separator |
| `LT` | `<` | less-than |
| `GT` | `>` | greater-than |

### Delimiters

`LPAREN` `(`, `RPAREN` `)`, `LBRACE` `{`, `RBRACE` `}`, `COMMA` `,`, `SEMI` `;`

## Installation

```bash
pip install coding-adventures-tetrad-lexer
```

Or for development:

```bash
uv venv
uv pip install -e .[dev]
```

## Running tests

```bash
python -m pytest tests/ -v
```

Coverage target: ≥95%.

## Spec

See [`code/specs/TET01-tetrad-lexer.md`](../../specs/TET01-tetrad-lexer.md) for the full specification.
