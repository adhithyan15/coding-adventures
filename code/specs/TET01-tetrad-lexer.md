# TET01 — Tetrad Lexer Specification

## Overview

The Tetrad lexer (tokenizer) reads a stream of UTF-8 source characters and produces a
flat sequence of tokens. The parser (spec TET02) consumes this token stream. The lexer
operates in a single forward pass with one character of lookahead.

The lexer is intentionally simple. Tetrad has no string literals, no floating-point
literals, and no contextual keywords — every reserved word is unconditionally reserved.
This keeps the lexer stateless and trivially restartable.

---

## Token Types

Every token belongs to exactly one of these types:

### Integer Literals

| Token type | Pattern | Example | Value |
|---|---|---|---|
| `INT` | `[0-9]+` | `42`, `0`, `255` | Decimal integer |
| `HEX` | `0x[0-9A-Fa-f]+` | `0xFF`, `0x0A` | Hexadecimal integer |

Both `INT` and `HEX` tokens carry their numeric value as a Python `int`. The lexer does
not check whether the value fits in u8 — range checking is the compiler's responsibility
(spec TET03).

### Identifiers and Keywords

An identifier begins with a letter or underscore and continues with letters, digits, or
underscores: `[A-Za-z_][A-Za-z0-9_]*`.

When an identifier's text exactly matches a reserved word, the lexer emits a keyword
token rather than an `IDENT` token. The reserved words and their token types are:

| Text | Token type |
|---|---|
| `fn` | `KW_FN` |
| `let` | `KW_LET` |
| `if` | `KW_IF` |
| `else` | `KW_ELSE` |
| `while` | `KW_WHILE` |
| `return` | `KW_RETURN` |
| `in` | `KW_IN` |
| `out` | `KW_OUT` |
| `u8` | `KW_U8` |

Any identifier that is not a reserved word produces an `IDENT` token carrying the name
as a string.

### Operators and Punctuation

| Text | Token type | Notes |
|---|---|---|
| `+` | `PLUS` | |
| `-` | `MINUS` | |
| `*` | `STAR` | |
| `/` | `SLASH` | |
| `%` | `PERCENT` | |
| `&` | `AMP` | |
| `\|` | `PIPE` | |
| `^` | `CARET` | |
| `~` | `TILDE` | |
| `<<` | `SHL` | Greedy: `<<` before `<` |
| `>>` | `SHR` | Greedy: `>>` before `>` |
| `==` | `EQ_EQ` | |
| `!=` | `BANG_EQ` | |
| `<` | `LT` | |
| `<=` | `LT_EQ` | Greedy: `<=` before `<` |
| `>` | `GT` | |
| `>=` | `GT_EQ` | Greedy: `>=` before `>` |
| `&&` | `AMP_AMP` | |
| `\|\|` | `PIPE_PIPE` | |
| `->` | `ARROW` | Return type annotation; greedy: scanned before `-` |
| `!` | `BANG` | |
| `=` | `EQ` | Assignment; not equality |
| `:` | `COLON` | Type annotation separator |
| `(` | `LPAREN` | |
| `)` | `RPAREN` | |
| `{` | `LBRACE` | |
| `}` | `RBRACE` | |
| `,` | `COMMA` | |
| `;` | `SEMI` | |

### End-of-File

| Token type | Description |
|---|---|
| `EOF` | Emitted once when the source is exhausted |

The parser uses `EOF` to recognize end-of-input cleanly without needing a length check.

---

## Whitespace and Comments

### Whitespace

Spaces, tabs (`\t`), carriage returns (`\r`), and newlines (`\n`) are all whitespace.
The lexer skips whitespace between tokens. Whitespace is never significant (there is no
off-side rule or indentation sensitivity).

### Comments

Tetrad uses C-style line comments:

```
// This is a comment. Everything from // to end of line is ignored.
```

Block comments (`/* ... */`) are not supported in v1. A `//` inside a string literal
would not trigger a comment — but since Tetrad has no string literals, this edge case
does not exist.

---

## Lexer Algorithm

```
Tetrad lexer pseudocode:

procedure tokenize(source: str) → list[Token]:
    pos = 0
    tokens = []

    while pos < len(source):
        skip_whitespace_and_comments()
        if pos >= len(source):
            break

        ch = source[pos]

        if ch is digit:
            tokens.append(scan_number())
        elif ch is letter or '_':
            tokens.append(scan_ident_or_keyword())
        else:
            tokens.append(scan_punctuation())

    tokens.append(Token(EOF, pos=pos))
    return tokens
```

### Scanning Numbers

```
procedure scan_number() → Token:
    start = pos
    if source[pos:pos+2] == '0x' or '0X':
        pos += 2
        scan while source[pos] is hex digit [0-9A-Fa-f]
        value = int(source[start:pos], 16)
        return Token(HEX, value, span(start, pos))
    else:
        scan while source[pos] is decimal digit [0-9]
        value = int(source[start:pos], 10)
        return Token(INT, value, span(start, pos))
```

### Scanning Identifiers and Keywords

```
procedure scan_ident_or_keyword() → Token:
    start = pos
    scan while source[pos] is letter, digit, or '_'
    text = source[start:pos]
    if text in RESERVED_WORDS:
        return Token(RESERVED_WORDS[text], text, span(start, pos))
    else:
        return Token(IDENT, text, span(start, pos))
```

### Scanning Punctuation (multi-character operators)

The lexer uses a greedy (maximal munch) strategy: always consume the longest valid token.
The two-character operators (`<<`, `>>`, `==`, `!=`, `<=`, `>=`, `&&`, `||`, `->`) must
be checked before their one-character prefixes (`<`, `>`, `=`, `!`, `&`, `|`, `-`):

```
procedure scan_punctuation() → Token:
    start = pos
    two = source[pos:pos+2]
    if two in TWO_CHAR_OPERATORS:
        pos += 2
        return Token(TWO_CHAR_OPERATORS[two], two, span(start, pos))
    one = source[pos]
    pos += 1
    if one in ONE_CHAR_OPERATORS:
        return Token(ONE_CHAR_OPERATORS[one], one, span(start, pos))
    raise LexError(f"unexpected character {one!r}", span(start, pos))
```

---

## Token Data Structure

Every token carries:

```python
@dataclass
class Token:
    type: TokenType           # The token type (enum)
    value: str | int | None   # Text for IDENT; int for INT/HEX; None for punctuation/keywords
    line: int                 # 1-based line number
    column: int               # 1-based column number of first character
    offset: int               # 0-based byte offset into source text
```

The `line` and `column` fields are used to produce helpful error messages in the parser
and compiler. The lexer increments `line` each time it passes a `\n` and resets
`column` to 1.

---

## Error Handling

The lexer raises `LexError` for the following conditions:

| Condition | Error message |
|---|---|
| Character not in any token pattern | `unexpected character '\x??' at line N col C` |
| Hex literal with no digits after `0x` | `empty hex literal at line N col C` |

The lexer does not produce partial tokens on error. It raises immediately. The compiler
(spec TET03) catches `LexError` and includes the position in its error output.

The lexer does **not** check:
- Whether integer literals exceed 255 (the compiler does this)
- Whether identifiers shadow reserved words (the compiler does this)
- Whether `in(` / `out(` are used correctly (the parser does this)

---

## Token Stream Examples

### Source

```tetrad
fn add(a, b) {
    return a + b;
}
```

### Token Stream

```
KW_FN      'fn'     line=1 col=1
IDENT      'add'    line=1 col=4
LPAREN     '('      line=1 col=7
IDENT      'a'      line=1 col=8
COMMA      ','      line=1 col=9
IDENT      'b'      line=1 col=11
RPAREN     ')'      line=1 col=12
LBRACE     '{'      line=1 col=14
KW_RETURN  'return' line=2 col=5
IDENT      'a'      line=2 col=12
PLUS       '+'      line=2 col=14
IDENT      'b'      line=2 col=16
SEMI       ';'      line=2 col=17
RBRACE     '}'      line=3 col=1
EOF               line=4 col=1
```

### Source with Comment

```tetrad
let x = 0xFF;   // 255 in hex
```

### Token Stream

```
KW_LET  'let'  line=1 col=1
IDENT   'x'    line=1 col=5
EQ      '='    line=1 col=7
HEX     255    line=1 col=9
SEMI    ';'    line=1 col=13
EOF            line=2 col=1
```

The comment produces no tokens.

---

## Python Package

The lexer lives in `code/packages/python/tetrad-lexer/`.

### Public API

```python
from tetrad_lexer import tokenize, Token, TokenType, LexError

# Tokenize a source string.
# Returns a list ending with EOF.
# Raises LexError on illegal input.
def tokenize(source: str) -> list[Token]: ...

class TokenType(enum.Enum):
    INT = "INT"
    HEX = "HEX"
    IDENT = "IDENT"
    KW_FN = "KW_FN"
    KW_LET = "KW_LET"
    KW_IF = "KW_IF"
    KW_ELSE = "KW_ELSE"
    KW_WHILE = "KW_WHILE"
    KW_RETURN = "KW_RETURN"
    KW_IN = "KW_IN"
    KW_OUT = "KW_OUT"
    KW_U8 = "KW_U8"
    PLUS = "PLUS"
    MINUS = "MINUS"
    STAR = "STAR"
    SLASH = "SLASH"
    PERCENT = "PERCENT"
    AMP = "AMP"
    PIPE = "PIPE"
    CARET = "CARET"
    TILDE = "TILDE"
    SHL = "SHL"
    SHR = "SHR"
    EQ_EQ = "EQ_EQ"
    BANG_EQ = "BANG_EQ"
    LT = "LT"
    LT_EQ = "LT_EQ"
    GT = "GT"
    GT_EQ = "GT_EQ"
    AMP_AMP = "AMP_AMP"
    PIPE_PIPE = "PIPE_PIPE"
    BANG = "BANG"
    EQ = "EQ"
    ARROW = "ARROW"
    COLON = "COLON"
    LPAREN = "LPAREN"
    RPAREN = "RPAREN"
    LBRACE = "LBRACE"
    RBRACE = "RBRACE"
    COMMA = "COMMA"
    SEMI = "SEMI"
    EOF = "EOF"

@dataclass
class Token:
    type: TokenType
    value: str | int | None
    line: int
    column: int
    offset: int

class LexError(Exception):
    def __init__(self, message: str, line: int, column: int): ...
```

---

## Test Strategy

### Happy-path token tests

- Single integer literal: `42` → `[INT(42), EOF]`
- Hex literal: `0xFF` → `[HEX(255), EOF]`
- All 8 reserved words tokenize to their keyword type
- Identifier: `counter` → `[IDENT('counter'), EOF]`
- Every two-character operator tokenizes correctly
- Every one-character operator tokenizes correctly

### Whitespace and comment tests

- Leading/trailing whitespace is skipped
- Multiple blank lines produce no tokens
- Comment on its own line produces no tokens
- Comment at end of a line with tokens does not consume the newline's effect on
  line numbering

### Multi-token tests

- Full function declaration: verify token sequence matches expected
- `<=` is one token, not `<` followed by `=`
- `<<` is one token, not `<` followed by `<`
- `==` is one token, not `=` followed by `=`

### Position tests

- Verify `line` and `column` are correct across newlines
- Verify `offset` matches byte position in source

### Error tests

- `@` produces `LexError`
- `0x` with no following hex digits produces `LexError`
- `#` produces `LexError`
- Error message includes line and column

### Coverage target

95%+ line coverage.

---

## Version History

| Version | Date | Description |
|---|---|---|
| 0.1.0 | 2026-04-20 | Initial specification |
