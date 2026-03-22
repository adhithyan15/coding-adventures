# TOML Lexer

A grammar-driven lexer (tokenizer) for [TOML](https://toml.io/) v1.0.0.

## What it does

This crate tokenizes TOML source text into a stream of typed tokens. It does not hand-write tokenization rules — instead, it loads the `toml.tokens` grammar file and feeds it to the generic `GrammarLexer` from the `lexer` crate.

## How it fits in the stack

```text
toml.tokens          (grammar file — declares token patterns)
       |
       v
grammar-tools        (parses .tokens file -> TokenGrammar struct)
       |
       v
lexer::GrammarLexer  (tokenizes source using TokenGrammar)
       |
       v
toml-lexer           (this crate — thin glue layer)
       |
       v
toml-parser          (downstream consumer — parses tokens into AST)
```

## Token types

| Token              | Example                    | Description                          |
|--------------------|----------------------------|--------------------------------------|
| BARE_KEY           | `my-key`                   | Unquoted key identifier              |
| BASIC_STRING       | `"hello"`                  | Double-quoted string                 |
| LITERAL_STRING     | `'hello'`                  | Single-quoted string (no escapes)    |
| ML_BASIC_STRING    | `"""..."""`                | Multi-line double-quoted string      |
| ML_LITERAL_STRING  | `'''...'''`                | Multi-line single-quoted string      |
| INTEGER            | `42`, `0xFF`, `0o77`, `0b01` | Decimal, hex, octal, binary integer |
| FLOAT              | `3.14`, `1e10`, `inf`, `nan` | Decimal, scientific, special float  |
| TRUE               | `true`                     | Boolean true literal                 |
| FALSE              | `false`                    | Boolean false literal                |
| OFFSET_DATETIME    | `1979-05-27T07:32:00Z`     | Date+time with timezone              |
| LOCAL_DATETIME     | `1979-05-27T07:32:00`      | Date+time without timezone           |
| LOCAL_DATE         | `1979-05-27`               | Date only                            |
| LOCAL_TIME         | `07:32:00`                 | Time only                            |
| EQUALS             | `=`                        | Key-value separator                  |
| DOT                | `.`                        | Dotted key separator                 |
| COMMA              | `,`                        | Element separator                    |
| LBRACKET           | `[`                        | Opening bracket                      |
| RBRACKET           | `]`                        | Closing bracket                      |
| LBRACE             | `{`                        | Opening brace                        |
| RBRACE             | `}`                        | Closing brace                        |
| NEWLINE            | `\n`                       | Line separator (significant in TOML) |

## Usage

```rust
use coding_adventures_toml_lexer::tokenize_toml;

let tokens = tokenize_toml("title = \"TOML Example\"\n[server]\nport = 8080");
for token in &tokens {
    println!("{:?} {:?}", token.type_, token.value);
}
```

Or use the factory function for fine-grained control:

```rust
use coding_adventures_toml_lexer::create_toml_lexer;

let mut lexer = create_toml_lexer("key = 42");
let tokens = lexer.tokenize().expect("tokenization failed");
```

## Key differences from json-lexer

- **Newline-sensitive** — TOML emits NEWLINE tokens; JSON skips all whitespace.
- **4 string types** — basic, literal, multi-line basic, multi-line literal.
- **Escape mode: none** — quotes are stripped but escapes are left as raw text for the semantic layer.
- **Date/time tokens** — TOML has 4 date/time types; JSON has none.
- **Bare keys** — TOML has unquoted key identifiers; JSON requires quoted strings.
- **Integer variants** — hex, octal, binary in addition to decimal.

## Running tests

```bash
cargo test -p coding-adventures-toml-lexer -- --nocapture
```
