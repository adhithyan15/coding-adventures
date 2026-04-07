# dartmouth-basic-lexer

Tokenizer for the 1964 Dartmouth BASIC language — the programming language
that introduced computing to a generation of non-scientists and seeded the
personal computer revolution of the 1970s and 1980s.

## What This Package Does

`dartmouth_basic_lexer` is a **thin wrapper** around the generic `GrammarLexer`
from the `lexer` package. It loads the `dartmouth_basic.tokens` grammar file
and applies two post-tokenize hooks that handle BASIC's unique quirks:

1. **LINE_NUM relabeling** — The first integer on each source line is
   relabeled from `NUMBER` to `LINE_NUM`, distinguishing line labels from
   numeric literals in expressions.

2. **REM suppression** — Everything after a `REM` keyword until end-of-line
   is stripped from the token stream (implementing BASIC's comment syntax).

## Where It Fits in the Stack

```
BASIC source text
      │
      ▼
┌─────────────────────────────────┐
│   dartmouth_basic_lexer         │  ← this package
│   dartmouth_basic.tokens grammar│
└─────────────────────────────────┘
      │
      ▼  [{type, value, line, column}, ...]
┌─────────────────────────────────┐
│   dartmouth_basic_parser        │  (future package)
└─────────────────────────────────┘
      │
      ▼
┌─────────────────────────────────┐
│   dartmouth_basic_compiler      │  (future package)
└─────────────────────────────────┘
```

## History

In the spring of 1964, John Kemeny and Thomas Kurtz at Dartmouth College
created BASIC (Beginner's All-purpose Symbolic Instruction Code) with one
goal: make computing accessible to *every* student.

Their innovation ran on a GE-225 mainframe connected to teletype terminals.
Students typed programs and got results in seconds — revolutionary in an era
when computers required batch-job submissions that took hours.

BASIC spread rapidly: Microsoft's first product was a BASIC interpreter for
the Altair 8800 (1975). Apple II, Commodore 64, TRS-80, and IBM PC all
shipped with BASIC built in. It is estimated that more people learned
programming through BASIC than through any other language.

This lexer targets the *original* 1964 Dartmouth BASIC — 20 keywords, 11
built-in functions, integer line numbers, no string variables.

## Installation

```bash
uv pip install -e .  # from the package directory
```

Or via the build system from the repository root:

```bash
./build-tool code/packages/python/dartmouth-basic-lexer
```

## Usage

### Quick Start

```python
from dartmouth_basic_lexer import tokenize_dartmouth_basic

source = """10 LET X = 5
20 PRINT X
30 END
"""

tokens = tokenize_dartmouth_basic(source)
for token in tokens:
    token_type = token.type if isinstance(token.type, str) else token.type.name
    print(f"{token_type:12} {token.value!r}")
```

Output:
```
LINE_NUM     '10'
KEYWORD      'LET'
NAME         'X'
EQ           '='
NUMBER       '5'
NEWLINE      '\n'
LINE_NUM     '20'
KEYWORD      'PRINT'
NAME         'X'
NEWLINE      '\n'
LINE_NUM     '30'
KEYWORD      'END'
NEWLINE      '\n'
EOF          ''
```

### Factory Function

For direct access to the `GrammarLexer` (to add custom hooks):

```python
from dartmouth_basic_lexer import create_dartmouth_basic_lexer

lexer = create_dartmouth_basic_lexer("10 PRINT X\n")
lexer.add_post_tokenize(my_custom_hook)
tokens = lexer.tokenize()
```

## Token Reference

| Type | Example Values | Description |
|------|---------------|-------------|
| `LINE_NUM` | `"10"`, `"999"` | Integer at start of a line (line label) |
| `NUMBER` | `"42"`, `"3.14"`, `"1.5E3"` | Numeric literal in an expression |
| `STRING` | `'"HELLO"'` | Double-quoted string (includes quotes) |
| `KEYWORD` | `"LET"`, `"PRINT"`, `"GOTO"` | Reserved word (always uppercase) |
| `BUILTIN_FN` | `"SIN"`, `"LOG"`, `"RND"` | One of 11 built-in functions |
| `USER_FN` | `"FNA"`, `"FNZ"` | User-defined function (FN + one letter) |
| `NAME` | `"X"`, `"A1"`, `"B9"` | Variable name: letter + optional digit |
| `PLUS` | `"+"` | Addition |
| `MINUS` | `"-"` | Subtraction |
| `STAR` | `"*"` | Multiplication |
| `SLASH` | `"/"` | Division |
| `CARET` | `"^"` | Exponentiation |
| `EQ` | `"="` | Assignment and equality (context-dependent) |
| `LT` | `"<"` | Less-than |
| `GT` | `">"` | Greater-than |
| `LE` | `"<="` | Less-than-or-equal |
| `GE` | `">="` | Greater-than-or-equal |
| `NE` | `"<>"` | Not-equal |
| `LPAREN` | `"("` | Left parenthesis |
| `RPAREN` | `")"` | Right parenthesis |
| `COMMA` | `","` | Comma (PRINT zone separator) |
| `SEMICOLON` | `";"` | Semicolon (PRINT no-space separator) |
| `NEWLINE` | `"\n"` | Statement terminator |
| `EOF` | `""` | End of token stream |
| `UNKNOWN` | `"@"` | Unrecognized character (error recovery) |

## The 20 Keywords

```
LET    PRINT   INPUT   IF      THEN
GOTO   GOSUB   RETURN  FOR     TO
STEP   NEXT    END     STOP    REM
READ   DATA    RESTORE DIM     DEF
```

## The 11 Built-in Functions

| Function | Description |
|----------|-------------|
| `SIN(X)` | Sine of X (X in radians) |
| `COS(X)` | Cosine of X |
| `TAN(X)` | Tangent of X |
| `ATN(X)` | Arctangent of X (result in radians) |
| `EXP(X)` | e raised to the power X |
| `LOG(X)` | Natural logarithm of X |
| `ABS(X)` | Absolute value of X |
| `SQR(X)` | Square root of X |
| `INT(X)` | Floor of X (largest integer ≤ X) |
| `RND(X)` | Random number in [0,1); X is ignored |
| `SGN(X)` | Sign of X: -1 if X<0, 0 if X=0, 1 if X>0 |

## Case Insensitivity

Dartmouth BASIC is case-insensitive throughout. The 1964 teletypes had no
lowercase keys, so the language was designed for uppercase-only input. The
grammar uses `@case_insensitive true`, which uppercases the entire source
before matching:

```python
tokens = tokenize_dartmouth_basic("10 print x\n")
# Produces: LINE_NUM("10"), KEYWORD("PRINT"), NAME("X"), NEWLINE, EOF
# Identical to: tokenize_dartmouth_basic("10 PRINT X\n")
```

## REM Comments

```python
tokens = tokenize_dartmouth_basic("10 REM THIS IS A COMMENT\n20 LET X = 1\n")
# Line 10: LINE_NUM("10"), KEYWORD("REM"), NEWLINE
#          (comment text "THIS IS A COMMENT" is suppressed)
# Line 20: LINE_NUM("20"), KEYWORD("LET"), NAME("X"), EQ, NUMBER("1"), NEWLINE
```

## Scientific Notation

```python
tokens = tokenize_dartmouth_basic("10 LET X = 1.5E-3\n")
# NUMBER token has value "1.5E-3" (after uppercasing from "1.5e-3")
```

## Dependencies

- `coding-adventures-grammar-tools` — parses the `.tokens` grammar file
- `coding-adventures-lexer` — the `GrammarLexer` engine and `Token` type
- `coding-adventures-directed-graph` — transitive dep of grammar-tools
- `coding-adventures-state-machine` — transitive dep of lexer

## Testing

```bash
cd code/packages/python/dartmouth-basic-lexer
uv venv --clear
uv pip install -e ../grammar-tools -e ../directed-graph -e ../state-machine -e ../lexer -e ".[dev]"
.venv/bin/python -m pytest tests/ -v --cov=dartmouth_basic_lexer
```

Coverage target: ≥ 95%.
