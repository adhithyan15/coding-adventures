# JSON Lexer

A grammar-driven lexer (tokenizer) for [JSON](https://www.json.org/) (RFC 8259).

## What it does

This crate tokenizes JSON source text into a stream of typed tokens. It does not hand-write tokenization rules — instead, it loads the `json.tokens` grammar file and feeds it to the generic `GrammarLexer` from the `lexer` crate.

## How it fits in the stack

```text
json.tokens          (grammar file — declares token patterns)
       |
       v
grammar-tools        (parses .tokens file → TokenGrammar struct)
       |
       v
lexer::GrammarLexer  (tokenizes source using TokenGrammar)
       |
       v
json-lexer           (this crate — thin glue layer)
       |
       v
json-parser          (downstream consumer — parses tokens into AST)
```

## Token types

| Token     | Example          | Description                    |
|-----------|------------------|--------------------------------|
| STRING    | `"hello"`        | Double-quoted string           |
| NUMBER    | `42`, `-3.14e2`  | Integer, decimal, or exponent  |
| TRUE      | `true`           | Boolean true literal           |
| FALSE     | `false`          | Boolean false literal          |
| NULL      | `null`           | Null literal                   |
| LBRACE    | `{`              | Opening brace                  |
| RBRACE    | `}`              | Closing brace                  |
| LBRACKET  | `[`              | Opening bracket                |
| RBRACKET  | `]`              | Closing bracket                |
| COLON     | `:`              | Key-value separator            |
| COMMA     | `,`              | Element separator              |

## Usage

```rust
use coding_adventures_json_lexer::tokenize_json;

let tokens = tokenize_json("{\"name\": \"Alice\", \"age\": 30}");
for token in &tokens {
    println!("{:?} {:?}", token.type_, token.value);
}
```

Or use the factory function for fine-grained control:

```rust
use coding_adventures_json_lexer::create_json_lexer;

let mut lexer = create_json_lexer("{\"key\": 42}");
let tokens = lexer.tokenize().expect("tokenization failed");
```

## Key differences from starlark-lexer

- **No keywords** — JSON has no keywords. `true`, `false`, and `null` are their own token types.
- **No indentation** — JSON uses braces and brackets for structure, not whitespace.
- **No comments** — JSON does not support comments.
- **Negative numbers** — The minus sign is part of the NUMBER token, not a separate operator.

## Running tests

```bash
cargo test -p coding-adventures-json-lexer -- --nocapture
```
