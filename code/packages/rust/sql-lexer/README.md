# SQL Lexer

A grammar-driven lexer (tokenizer) for an ANSI SQL subset, covering SELECT, INSERT, UPDATE, DELETE, CREATE TABLE, and DROP TABLE statements.

## What it does

This crate tokenizes SQL source text into a stream of typed tokens. It does not hand-write tokenization rules — instead, it loads the `sql.tokens` grammar file and feeds it to the generic `GrammarLexer` from the `lexer` crate.

## How it fits in the stack

```text
sql.tokens           (grammar file — declares token patterns, keywords, comments)
       |
       v
grammar-tools        (parses .tokens file → TokenGrammar struct)
       |
       v
lexer::GrammarLexer  (tokenizes source using TokenGrammar)
       |
       v
sql-lexer            (this crate — thin glue layer)
       |
       v
sql-parser           (downstream consumer — parses tokens into AST)
```

## Case-insensitive keywords

SQL is case-insensitive for keywords. The `sql.tokens` grammar declares `# @case_insensitive true`, so:

- `select`, `SELECT`, and `Select` all produce `TokenType::Keyword` with value `"SELECT"`.
- Keyword values are always normalized to **uppercase**.
- Identifiers (NAME tokens) retain their original case.

## Token types

| Token          | Example             | Description                                       |
|----------------|---------------------|---------------------------------------------------|
| Keyword        | `SELECT`, `FROM`    | SQL keyword (50+), normalized to uppercase        |
| Name           | `users`, `age`      | Identifier (table name, column name, alias)       |
| Number         | `42`, `3.14`        | Integer or decimal literal                        |
| String         | `'hello'`           | Single-quoted string literal (quotes stripped)    |
| Equals         | `=`                 | Equality operator                                 |
| NOT_EQUALS     | `!=`, `<>`          | Inequality (both forms → type_name NOT_EQUALS)    |
| LESS_EQUALS    | `<=`                | Less-than-or-equal                                |
| GREATER_EQUALS | `>=`                | Greater-than-or-equal                             |
| LessThan       | `<`                 | Less-than                                         |
| GreaterThan    | `>`                 | Greater-than                                      |
| Plus           | `+`                 | Addition                                          |
| Minus          | `-`                 | Subtraction                                       |
| Star           | `*`                 | Multiplication / SELECT *                         |
| Slash          | `/`                 | Division                                          |
| Percent        | `%`                 | Modulo                                            |
| LParen         | `(`                 | Opening parenthesis                               |
| RParen         | `)`                 | Closing parenthesis                               |
| Comma          | `,`                 | Separator                                         |
| Semicolon      | `;`                 | Statement terminator                              |
| Dot            | `.`                 | Qualified name separator (`table.column`)         |

Comments (`-- line comment` and `/* block comment */`) are skipped and produce no tokens.

## Usage

```rust
use coding_adventures_sql_lexer::tokenize_sql;

let tokens = tokenize_sql("SELECT name, age FROM users WHERE age > 18").unwrap();
for token in &tokens {
    println!("{:?} {:?}", token.type_, token.value);
}
```

Or use the factory function for fine-grained control:

```rust
use coding_adventures_sql_lexer::create_sql_lexer;

let mut lexer = create_sql_lexer("SELECT * FROM t").unwrap();
let tokens = lexer.tokenize().expect("tokenization failed");
```

Both functions return `Result<_, String>` so errors (missing grammar file, unexpected characters) are propagated cleanly.

## Key differences from json-lexer

- **50+ keywords** — SQL has a large keyword vocabulary vs. JSON's zero.
- **Case-insensitive** — Keywords are normalized to uppercase.
- **Two comment styles** — Line (`--`) and block (`/* */`) comments.
- **Two inequality operators** — `!=` and `<>` both map to NOT_EQUALS.
- **Result-returning API** — Both `create_sql_lexer` and `tokenize_sql` return `Result` rather than panicking.

## Running tests

```bash
cargo test --package coding-adventures-sql-lexer -- --nocapture
```
