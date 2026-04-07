# dartmouth-basic-lexer

A grammar-driven lexer for **Dartmouth BASIC 1964** — the original BASIC as
designed by John Kemeny and Thomas Kurtz for the GE-225 mainframe at Dartmouth
College.

## What Is Dartmouth BASIC?

In 1964, most computers were programmed by professional staff. Students who
wanted to use computing had to submit jobs on punch cards and wait hours for
results. John Kemeny and Thomas Kurtz had a different vision: give every student
interactive access to the computer, right now, from a teletype terminal.

They designed BASIC — **Beginner's All-purpose Symbolic Instruction Code** —
with a single goal: a complete beginner should be able to write a working program
in one afternoon.

Their design choices reflect this goal:

- **Line numbers for every statement**: `10 LET X = 5`, `20 PRINT X`, `30 END`.
  Line numbers are the only addressing scheme; `GOTO 30` jumps to line 30.
- **All variables pre-initialized to 0**: no "variable not declared" errors for
  beginners.
- **Case-insensitive**: the teletypes of 1964 only had uppercase keys. `print`
  and `PRINT` are the same.
- **Just 20 keywords**: small enough to memorize in an afternoon.
- **11 built-in math functions**: `SIN`, `COS`, `TAN`, `ATN`, `EXP`, `LOG`,
  `ABS`, `SQR`, `INT`, `RND`, `SGN`.
- **All numbers are floating-point**: even `42` is stored as `42.0`. No integer
  vs. float distinction for beginners.

BASIC was an enormous success. By the 1970s it had spread to minicomputers; by
the early 1980s it came pre-installed on virtually every personal computer
(Apple II, TRS-80, Commodore 64, IBM PC). Tens of millions of people learned to
program in BASIC before any other language.

This package implements the **original 1964 Dartmouth specification** — not the
later Microsoft BASIC, Applesoft BASIC, or GW-BASIC dialects. Those get their
own grammar files.

## How This Package Works

This package is a thin wrapper around the generic grammar-driven lexer. It:

1. Reads `code/grammars/dartmouth_basic.tokens` at initialization time
2. Passes the grammar to `GrammarLexer`, which compiles the regex patterns
3. Registers two BASIC-specific post-tokenize hooks (see below)

The grammar path is resolved at runtime using `runtime.Caller(0)` so tests and
the build tool can run from any working directory.

### Post-Tokenize Hook: relabelLineNumbers

The grammar cannot distinguish `10` (a line label) from `10` (a numeric value
in an expression) by regex alone — both are sequences of digits. The
`relabelLineNumbers` hook solves this by walking the finished token list and
reclassifying the first NUMBER on each source line as LINE_NUM:

```
"10 LET X = 5"  →  LINE_NUM("10") KEYWORD("LET") NAME("x") EQ("=") NUMBER("5")
"GOTO 30"       →  KEYWORD("GOTO") NUMBER("30")   ← target stays NUMBER
```

### Post-Tokenize Hook: suppressRemContent

REM introduces a comment that runs to the end of the line. The
`suppressRemContent` hook discards all tokens between `KEYWORD("REM")` and the
next NEWLINE. The NEWLINE itself is preserved — it is the statement terminator:

```
"10 REM HELLO WORLD"  →  LINE_NUM("10") KEYWORD("REM") NEWLINE
```

### Case Handling

The grammar sets `case_sensitive: false`, which causes the GrammarLexer to
lowercase the source before matching. Additionally, `@case_insensitive true`
causes keyword tokens to be normalized to uppercase. The result:

| Input | Token |
|-------|-------|
| `PRINT` | `KEYWORD("PRINT")` |
| `print` | `KEYWORD("PRINT")` |
| `Print` | `KEYWORD("PRINT")` |
| `SIN` | `BUILTIN_FN("sin")` |
| `X` | `NAME("x")` |
| `"HELLO"` | `STRING("HELLO")` |

Note that BUILTIN_FN, USER_FN, and NAME values are lowercase (the source is
lowercased before matching), while KEYWORD values are uppercase.

## Usage

```go
import dartmouthlexer "github.com/adhithyan15/coding-adventures/code/packages/go/dartmouth-basic-lexer"

// One-shot tokenization: BASIC source in, token slice out
tokens, err := dartmouthlexer.TokenizeDartmouthBasic(
    "10 LET X = 5\n20 PRINT X\n30 END\n",
)
if err != nil {
    log.Fatal(err)
}
for _, tok := range tokens {
    fmt.Printf("%s(%q) at %d:%d\n", tok.TypeName, tok.Value, tok.Line, tok.Column)
}

// Or create a reusable lexer for more control
lex, err := dartmouthlexer.CreateDartmouthBasicLexer(
    "10 PRINT \"HELLO\"\n20 END\n",
)
if err != nil {
    log.Fatal(err)
}
tokens = lex.Tokenize()
```

Example output for `"10 LET X = 5\n20 PRINT X\n30 END\n"`:

```
LINE_NUM("10") at 1:1
KEYWORD("LET") at 1:4
NAME("x") at 1:8
EQ("=") at 1:10
NUMBER("5") at 1:12
NEWLINE("\n") at 1:13
LINE_NUM("20") at 2:1
KEYWORD("PRINT") at 2:4
NAME("x") at 2:10
NEWLINE("\n") at 2:11
LINE_NUM("30") at 3:1
KEYWORD("END") at 3:4
NEWLINE("\n") at 3:7
EOF("") at 4:1
```

## Token Types

| Token | Example | Notes |
|-------|---------|-------|
| `LINE_NUM` | `10`, `100`, `32767` | Line label; relabeled from NUMBER by hook |
| `NUMBER` | `42`, `3.14`, `.5`, `1.5E3` | All numbers are floating-point internally |
| `STRING` | `"HELLO WORLD"` | No escape sequences in 1964 BASIC |
| `KEYWORD` | `LET`, `PRINT`, `GOTO` | Always uppercase in token value |
| `BUILTIN_FN` | `sin`, `cos`, `int` | Lowercase in token value |
| `USER_FN` | `fna`, `fnb`, `fnz` | User-defined functions FNA–FNZ |
| `NAME` | `x`, `a1`, `z9` | Variable names: one letter or letter+digit |
| `NEWLINE` | `"\n"` | Statement terminator — NOT skipped |
| `PLUS` | `+` | Addition |
| `MINUS` | `-` | Subtraction or negation |
| `STAR` | `*` | Multiplication |
| `SLASH` | `/` | Division |
| `CARET` | `^` | Exponentiation (right-associative) |
| `EQ` | `=` | Assignment (in LET) or equality (in IF) |
| `LT` | `<` | Less than |
| `GT` | `>` | Greater than |
| `LE` | `<=` | Less than or equal |
| `GE` | `>=` | Greater than or equal |
| `NE` | `<>` | Not equal |
| `LPAREN` | `(` | Open parenthesis |
| `RPAREN` | `)` | Close parenthesis |
| `COMMA` | `,` | Print zone separator in PRINT |
| `SEMICOLON` | `;` | Concatenate in PRINT (no space) |
| `EOF` | `""` | Always the last token |

## Keywords

All 20 keywords from the 1964 Dartmouth BASIC specification:

| Keyword | Category | Example |
|---------|----------|---------|
| `LET` | Assignment | `10 LET X = 5` |
| `PRINT` | Output | `20 PRINT X, Y` |
| `INPUT` | Input | `30 INPUT X` |
| `IF` / `THEN` | Conditional | `40 IF X > 0 THEN 100` |
| `GOTO` | Unconditional jump | `50 GOTO 10` |
| `GOSUB` / `RETURN` | Subroutine call | `60 GOSUB 200` |
| `FOR` / `TO` / `STEP` | Loop header | `70 FOR I = 1 TO 10 STEP 2` |
| `NEXT` | Loop footer | `80 NEXT I` |
| `END` | Program end | `90 END` |
| `STOP` | Halt execution | `95 STOP` |
| `REM` | Comment | `100 REM THIS IS IGNORED` |
| `READ` | Read from DATA | `110 READ X` |
| `DATA` | Data values | `120 DATA 1, 2, 3` |
| `RESTORE` | Reset DATA pointer | `130 RESTORE` |
| `DIM` | Declare array | `140 DIM A(10)` |
| `DEF` | Define function | `150 DEF FNA(X) = X * X` |

## Built-In Functions

All 11 built-in functions from the 1964 specification:

| Function | Description |
|----------|-------------|
| `SIN(X)` | Sine of X (X in radians) |
| `COS(X)` | Cosine of X |
| `TAN(X)` | Tangent of X |
| `ATN(X)` | Arctangent of X (result in radians) |
| `EXP(X)` | e raised to the power X |
| `LOG(X)` | Natural logarithm of X (base e) |
| `ABS(X)` | Absolute value of X |
| `SQR(X)` | Square root of X |
| `INT(X)` | Floor of X (largest integer ≤ X) |
| `SGN(X)` | Sign of X: −1 if X<0, 0 if X=0, 1 if X>0 |
| `RND(X)` | Random number in [0,1); X is ignored but required |

## Example: A Complete BASIC Program

```basic
10 REM COMPUTE FIBONACCI NUMBERS
20 LET A = 0
30 LET B = 1
40 PRINT A
50 LET C = A + B
60 LET A = B
70 LET B = C
80 IF A < 1000 THEN 40
90 END
```

Tokenized output (abbreviated):

```
LINE_NUM("10") KEYWORD("REM") NEWLINE
LINE_NUM("20") KEYWORD("LET") NAME("a") EQ NUMBER("0") NEWLINE
LINE_NUM("30") KEYWORD("LET") NAME("b") EQ NUMBER("1") NEWLINE
...
LINE_NUM("80") KEYWORD("IF") NAME("a") LT NUMBER("1000") KEYWORD("THEN") NUMBER("40") NEWLINE
LINE_NUM("90") KEYWORD("END") NEWLINE
EOF
```

## Stack

This package depends on:
- `go/lexer` — generic GrammarLexer engine
- `go/grammar-tools` — token grammar parser (`ParseTokenGrammar`)

The dartmouth-basic-parser package (future) will depend on this package.
