# Dartmouth BASIC Lexer

A grammar-driven lexer (tokenizer) for the original [1964 Dartmouth BASIC](https://en.wikipedia.org/wiki/Dartmouth_BASIC) programming language.

## What it does

This crate tokenizes Dartmouth BASIC source text into a stream of typed tokens.
It does not hand-write tokenization rules — instead, it loads the
`dartmouth_basic.tokens` grammar file and feeds it to the generic `GrammarLexer`
from the `lexer` crate. Two post-tokenize hooks handle the language-specific
edge cases that a pure grammar cannot express.

## How it fits in the stack

```text
dartmouth_basic.tokens   (grammar file — declares token patterns)
        |
        v
grammar-tools            (parses .tokens file → TokenGrammar struct)
        |
        v
lexer::GrammarLexer      (tokenizes source using TokenGrammar)
        |
        v
dartmouth-basic-lexer    (this crate — adds post-tokenize hooks)
        |
        v
dartmouth-basic-parser   (downstream — parses tokens into AST)
```

## Post-tokenize hooks

### Hook 1: relabel_line_numbers

The grammar defines `LINE_NUM = /[0-9]+/` before `NUMBER`. Because
first-match-wins, every bare integer in the source initially becomes LINE_NUM.
This hook walks the token list and relabels integers that are NOT the first
token on their line to NUMBER:

```text
Before: LINE_NUM("10") KEYWORD("GOTO") LINE_NUM("100") NEWLINE
After:  LINE_NUM("10") KEYWORD("GOTO") NUMBER("100")   NEWLINE
```

### Hook 2: suppress_rem_content

`REM` introduces a comment running to the end of the line. The hook discards
all tokens between REM and the next NEWLINE:

```text
Source:  10 REM THIS IS A COMMENT
Before:  LINE_NUM("10") KEYWORD("REM") NAME("THIS") NAME("IS") … NEWLINE
After:   LINE_NUM("10") KEYWORD("REM") NEWLINE
```

## Token types

| Token       | Example             | Description                                          |
|-------------|---------------------|------------------------------------------------------|
| `LINE_NUM`  | `10`, `999`         | Line label at the start of each BASIC line           |
| `NUMBER`    | `3.14`, `42`, `1E3` | Numeric literal in expressions                       |
| `STRING`    | `"HELLO WORLD"`     | Double-quoted string (no escape sequences)           |
| `KEYWORD`   | `PRINT`, `LET`      | Reserved word (always uppercase after normalization) |
| `BUILTIN_FN`| `SIN`, `LOG`, `RND` | One of the 11 built-in mathematical functions        |
| `USER_FN`   | `FNA`, `FNZ`        | User-defined function (FN + one letter)              |
| `NAME`      | `X`, `A1`, `B9`     | Variable name (letter, or letter+digit)              |
| `PLUS`      | `+`                 | Addition                                             |
| `MINUS`     | `-`                 | Subtraction                                          |
| `STAR`      | `*`                 | Multiplication                                       |
| `SLASH`     | `/`                 | Division                                             |
| `CARET`     | `^`                 | Exponentiation                                       |
| `EQUALS`    | `=`                 | Assignment (in LET) and equality (in IF)             |
| `LT`        | `<`                 | Less-than                                            |
| `GT`        | `>`                 | Greater-than                                         |
| `LE`        | `<=`                | Less-than-or-equal                                   |
| `GE`        | `>=`                | Greater-than-or-equal                                |
| `NE`        | `<>`                | Not-equal                                            |
| `LPAREN`    | `(`                 | Left parenthesis                                     |
| `RPAREN`    | `)`                 | Right parenthesis                                    |
| `COMMA`     | `,`                 | Print zone separator in PRINT                        |
| `SEMICOLON` | `;`                 | No-space separator in PRINT                          |
| `NEWLINE`   | `\n`                | Statement terminator (significant — kept in stream)  |
| `EOF`       | `""`                | Always the last token                                |
| `UNKNOWN`   | `@`                 | Unrecognized character (error recovery)              |

### Keywords (all produce `TokenType::Keyword`)

`LET`, `PRINT`, `INPUT`, `IF`, `THEN`, `GOTO`, `GOSUB`, `RETURN`, `FOR`,
`TO`, `STEP`, `NEXT`, `END`, `STOP`, `REM`, `READ`, `DATA`, `RESTORE`,
`DIM`, `DEF`

### Built-in functions (all produce `BUILTIN_FN`)

`SIN`, `COS`, `TAN`, `ATN`, `EXP`, `LOG`, `ABS`, `SQR`, `INT`, `RND`, `SGN`

## Historical context

Dartmouth BASIC was designed in 1964 by John G. Kemeny and Thomas E. Kurtz at
Dartmouth College. It ran on a GE-225 mainframe connected to teletypes that only
had uppercase characters — this is why BASIC is case-insensitive. The language
was designed to be beginner-friendly: no declarations, pre-initialized variables,
and simple line-numbered structure.

Key lexical features:

- **Line numbers** — every statement begins with a number that serves as both an
  address (`GOTO 100` jumps to line 100) and an ordering key (lines execute in
  numeric order, regardless of input order).

- **Case insensitivity** — `print`, `Print`, and `PRINT` all produce the same
  `KEYWORD("PRINT")` token. The grammar's `@case_insensitive true` directive
  uppercases the entire source before matching.

- **= for both assignment and equality** — `LET X = 5` (assign) and
  `IF X = 5 THEN` (compare) both produce an `EQ` token. The parser resolves
  the ambiguity from context.

- **No escape sequences in strings** — a double quote cannot appear inside a
  string literal. The grammar rule `/"[^"]*"/` captures this exactly.

## Usage

```rust
use coding_adventures_dartmouth_basic_lexer::tokenize_dartmouth_basic;

let tokens = tokenize_dartmouth_basic("10 LET X = 42\n20 PRINT X\n30 END\n");
for token in &tokens {
    println!("{} {:?}", token.effective_type_name(), token.value);
}
```

Or use the factory function for fine-grained control:

```rust
use coding_adventures_dartmouth_basic_lexer::create_dartmouth_basic_lexer;

let mut lexer = create_dartmouth_basic_lexer("10 PRINT \"HELLO WORLD\"");
let tokens = lexer.tokenize().expect("tokenization failed");
```

## Running tests

```bash
cargo test -p coding-adventures-dartmouth-basic-lexer -- --nocapture
```
