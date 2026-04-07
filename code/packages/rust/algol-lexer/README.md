# ALGOL 60 Lexer

A grammar-driven lexer (tokenizer) for [ALGOL 60](https://en.wikipedia.org/wiki/ALGOL_60) (ALGOrithmic Language, 1960).

## What it does

This crate tokenizes ALGOL 60 source text into a stream of typed tokens. It does not hand-write tokenization rules — instead, it loads the `algol.tokens` grammar file and feeds it to the generic `GrammarLexer` from the `lexer` crate.

## How it fits in the stack

```text
algol.tokens         (grammar file — declares token patterns)
       |
       v
grammar-tools        (parses .tokens file → TokenGrammar struct)
       |
       v
lexer::GrammarLexer  (tokenizes source using TokenGrammar)
       |
       v
algol-lexer          (this crate — thin glue layer)
       |
       v
algol-parser         (downstream consumer — parses tokens into AST)
```

## Token types

| Token        | Example          | Description                                         |
|--------------|------------------|-----------------------------------------------------|
| IDENT        | `x`, `sum`, `A1` | Identifier (letter followed by letters/digits)      |
| INTEGER_LIT  | `0`, `42`, `1000`| Integer literal                                     |
| REAL_LIT     | `3.14`, `1.5E3`  | Real (floating-point) literal with decimal/exponent |
| STRING_LIT   | `'hello'`, `''`  | Single-quoted string literal (no escapes)           |
| ASSIGN       | `:=`             | Assignment operator (not `=`)                       |
| POWER        | `**`             | Exponentiation (Fortran convention)                 |
| CARET        | `^`              | Exponentiation (alternative)                        |
| LEQ          | `<=`             | Less-than-or-equal (ASCII for ≤)                    |
| GEQ          | `>=`             | Greater-than-or-equal (ASCII for ≥)                 |
| NEQ          | `!=`             | Not-equal (ASCII for ≠)                             |
| EQ           | `=`              | Equality test (not assignment)                      |
| LT           | `<`              | Less-than                                           |
| GT           | `>`              | Greater-than                                        |
| PLUS         | `+`              | Addition                                            |
| MINUS        | `-`              | Subtraction                                         |
| STAR         | `*`              | Multiplication                                      |
| SLASH        | `/`              | Division                                            |
| LPAREN       | `(`              | Left parenthesis                                    |
| RPAREN       | `)`              | Right parenthesis                                   |
| LBRACKET     | `[`              | Left bracket (array subscripts)                     |
| RBRACKET     | `]`              | Right bracket                                       |
| SEMICOLON    | `;`              | Statement separator                                 |
| COMMA        | `,`              | Separator in lists                                  |
| COLON        | `:`              | Array bound separator, label marker                 |

### Keywords (all produce `TokenType::Keyword`)

`begin`, `end`, `if`, `then`, `else`, `for`, `do`, `step`, `until`, `while`,
`goto`, `switch`, `procedure`, `integer`, `real`, `boolean`, `string`,
`array`, `own`, `label`, `value`, `true`, `false`,
`not`, `and`, `or`, `impl`, `eqv`, `div`, `mod`

## Historical context

ALGOL 60 introduced several lexical conventions that later languages adopted or adapted:

- **`:=` for assignment** — separates assignment from equality (`=`), avoiding the notorious C bug where `if (x = 0)` silently assigns instead of comparing.
- **`**` for exponentiation** — the original ALGOL report used the uparrow symbol `↑`, which hardware couldn't print. ASCII implementations standardized on `**`.
- **Word-based boolean operators** — `and`, `or`, `not` instead of `&&`, `||`, `!`. Mathematical and readable.
- **`div` and `mod`** — keyword operators for integer division and modulo, distinguishing them from floating-point `/`.
- **`comment ... ;` syntax** — comments end at the next semicolon, matching the statement-separator convention.

## Usage

```rust
use coding_adventures_algol_lexer::tokenize_algol;

let tokens = tokenize_algol("begin integer x; x := 42 end");
for token in &tokens {
    println!("{:?} {:?}", token.type_, token.value);
}
```

Or use the factory function for fine-grained control:

```rust
use coding_adventures_algol_lexer::create_algol_lexer;

let mut lexer = create_algol_lexer("x := 1 + 2 * 3");
let tokens = lexer.tokenize().expect("tokenization failed");
```

## Running tests

```bash
cargo test -p coding-adventures-algol-lexer -- --nocapture
```
