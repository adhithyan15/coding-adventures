# coding-adventures-dartmouth-basic-lexer (Lua)

A Dartmouth BASIC 1964 lexer that tokenizes original BASIC source text into a flat stream of typed tokens. It is a thin wrapper around the grammar-driven `GrammarLexer` from `coding-adventures-lexer`, configured by the shared `dartmouth_basic.tokens` grammar file.

## What it does

Given the multi-line program:

```basic
10 LET X = 5
20 PRINT X
30 END
```

The lexer produces:

| # | Type     | Value   |
|---|----------|---------|
| 1 | LINE_NUM | `10`    |
| 2 | KEYWORD  | `LET`   |
| 3 | NAME     | `X`     |
| 4 | EQ       | `=`     |
| 5 | NUMBER   | `5`     |
| 6 | NEWLINE  | `\n`    |
| 7 | LINE_NUM | `20`    |
| 8 | KEYWORD  | `PRINT` |
| 9 | NAME     | `X`     |
|10 | NEWLINE  | `\n`    |
|11 | LINE_NUM | `30`    |
|12 | KEYWORD  | `END`   |
|13 | NEWLINE  | `\n`    |
|14 | EOF      |         |

Whitespace (spaces and tabs between tokens) is consumed silently. NEWLINE is kept because it is the statement terminator in BASIC.

## Token types

### Special tokens

| Token type | Description |
|------------|-------------|
| LINE_NUM   | A digit sequence at the start of a program line; distinguishes line labels from numeric expressions |
| NUMBER     | A numeric literal inside an expression: `42`, `3.14`, `.5`, `1.5E3`, `1.5E-3` |
| STRING     | A double-quoted string: `"HELLO WORLD"` |
| NEWLINE    | Statement terminator — kept in the stream, not discarded |
| EOF        | End of input — always the final token |
| UNKNOWN    | Unrecognised character; lexer recovers and continues |

### Keywords (always uppercase after case normalisation)

| Keyword | Purpose |
|---------|---------|
| LET     | Variable assignment: `LET X = 5` |
| PRINT   | Output values to the teletype |
| INPUT   | Read values from the user |
| IF      | Conditional: `IF X > 0 THEN 100` |
| THEN    | Part of IF statement |
| GOTO    | Unconditional jump: `GOTO 100` |
| GOSUB   | Call a subroutine: `GOSUB 500` |
| RETURN  | Return from a subroutine |
| FOR     | Loop start: `FOR I = 1 TO 10` |
| TO      | Part of FOR loop |
| STEP    | Optional loop increment: `STEP 2` |
| NEXT    | Loop end: `NEXT I` |
| END     | End of program |
| STOP    | Pause execution |
| REM     | Remark (comment) — text after REM is discarded |
| READ    | Read from DATA |
| DATA    | Inline data: `DATA 1,2,3` |
| RESTORE | Reset DATA pointer |
| DIM     | Declare array dimensions |
| DEF     | Define a user function: `DEF FNA(X) = X * X` |

### Built-in functions

| Token type | Value | Description |
|------------|-------|-------------|
| BUILTIN_FN | SIN   | Sine (radians) |
| BUILTIN_FN | COS   | Cosine (radians) |
| BUILTIN_FN | TAN   | Tangent (radians) |
| BUILTIN_FN | ATN   | Arctangent (radians) |
| BUILTIN_FN | EXP   | e^x |
| BUILTIN_FN | LOG   | Natural logarithm |
| BUILTIN_FN | ABS   | Absolute value |
| BUILTIN_FN | SQR   | Square root |
| BUILTIN_FN | INT   | Floor to integer |
| BUILTIN_FN | RND   | Random number in [0,1) |
| BUILTIN_FN | SGN   | Sign: -1, 0, or 1 |

User-defined functions appear as `USER_FN` tokens: `FNA`, `FNB`, ..., `FNZ`.

### Variables

Variable names in 1964 Dartmouth BASIC are exactly one or two characters:
- Single letter: `A`, `B`, ..., `Z` (26 scalar variables)
- Letter + digit: `A0`–`A9`, ..., `Z0`–`Z9` (260 more)

These appear as `NAME` tokens: `NAME("X")`, `NAME("A1")`, `NAME("Z9")`.

### Operators and delimiters

| Token type | Source text | Notes |
|------------|-------------|-------|
| LE         | `<=`        | Less-than-or-equal |
| GE         | `>=`        | Greater-than-or-equal |
| NE         | `<>`        | Not-equal |
| PLUS       | `+`         | |
| MINUS      | `-`         | |
| STAR       | `*`         | |
| SLASH      | `/`         | |
| CARET      | `^`         | Exponentiation |
| EQ         | `=`         | Assignment and equality (parser determines which) |
| LT         | `<`         | |
| GT         | `>`         | |
| LPAREN     | `(`         | |
| RPAREN     | `)`         | |
| COMMA      | `,`         | PRINT zone separator |
| SEMICOLON  | `;`         | PRINT compact separator |

## Usage

```lua
local basic = require("coding_adventures.dartmouth_basic_lexer")

local src = "10 LET X = SIN(3.14) + 1\n20 PRINT X\n30 END\n"
local tokens = basic.tokenize(src)
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
```

## Line numbers

Every Dartmouth BASIC statement begins with a line number:

```basic
10 LET X = 5
20 PRINT X
30 GOTO 10
```

The lexer distinguishes line number labels from numeric expressions by position: the **first number on each source line** is promoted from `NUMBER` to `LINE_NUM`. Numbers inside expressions (like `GOTO 10`'s target) remain `NUMBER`.

## REM (remarks)

`REM` introduces a comment that extends to the end of the line:

```basic
10 REM THIS IS A COMMENT
20 LET X = 1
```

The lexer keeps the `KEYWORD("REM")` token but discards everything after it until the `NEWLINE`. The `NEWLINE` itself is preserved.

## Case insensitivity

The grammar uses `@case_insensitive true`. The entire source is normalised to uppercase before matching:

```lua
-- All three produce the same token stream:
basic.tokenize("10 PRINT X\n")
basic.tokenize("10 print x\n")
basic.tokenize("10 Print X\n")
-- → LINE_NUM("10"), KEYWORD("PRINT"), NAME("X"), NEWLINE, EOF
```

This reflects the historical reality: the GE-225 teletypes used at Dartmouth in 1964 only had uppercase characters.

## How it fits in the stack

```
dartmouth_basic.tokens  (code/grammars/)
    ↓  parsed by grammar_tools
TokenGrammar
    ↓  drives
GrammarLexer  (coding-adventures-lexer)
    ↓  wrapped by
dartmouth_basic_lexer  ← you are here
    ↓  post-processed (relabel_line_numbers, suppress_rem_content)
    ↓  feeds
dartmouth_basic_parser
```

## Dependencies

- `coding-adventures-grammar-tools` — parses `dartmouth_basic.tokens`
- `coding-adventures-lexer` — provides `GrammarLexer`
- `coding-adventures-state-machine` — used internally by the lexer
- `coding-adventures-directed-graph` — used internally by grammar tools

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```

## Version

0.1.0
