# sql-lexer (Go)

Tokenizer for an ANSI SQL subset, built on the grammar-driven lexer infrastructure.

## What it does

Converts a SQL string into a flat slice of `lexer.Token` values. Keywords are
case-insensitive: `select`, `SELECT`, and `Select` all produce `KEYWORD("SELECT")`.

## Where it fits

```
sql text
   ↓  sql-lexer      (this package)
token stream
   ↓  sql-parser     (next layer)
AST
```

## Usage

```go
import sqllexer "github.com/adhithyan15/coding-adventures/code/packages/go/sql-lexer"

// One-shot tokenization
tokens, err := sqllexer.TokenizeSQL("SELECT id, name FROM users WHERE active = TRUE")

// Reusable lexer
lex, err := sqllexer.CreateSQLLexer("SELECT * FROM orders")
tokens := lex.Tokenize()
```

## Token types produced

| TypeName        | Example input | Example value |
|-----------------|--------------|---------------|
| `KEYWORD`       | `select`     | `"SELECT"`    |
| `NAME`          | `user_id`    | `"user_id"`   |
| `NUMBER`        | `3.14`       | `"3.14"`      |
| `STRING`        | `'hello'`    | `"hello"`     |
| `NOT_EQUALS`    | `!=` or `<>` | `"!="` / `"<>"` |
| `LESS_EQUALS`   | `<=`         | `"<="`        |
| `GREATER_EQUALS`| `>=`         | `">="`        |
| `STAR`          | `*`          | `"*"`         |
| `LPAREN`        | `(`          | `"("`         |
| `SEMICOLON`     | `;`          | `";"`         |

Comments (`-- ...` and `/* ... */`) and whitespace are automatically skipped.

## Grammar files

- `code/grammars/sql.tokens` — token definitions (patterns, keywords, skip rules)
- `code/grammars/sql.grammar` — parser grammar (used by sql-parser)

## Running tests

```
go test ./... -v -cover
```
