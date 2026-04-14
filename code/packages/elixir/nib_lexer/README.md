# SQL Lexer (Elixir)

Thin wrapper around the grammar-driven lexer engine for SQL tokenization.

## Usage

```elixir
{:ok, tokens} = CodingAdventures.SqlLexer.tokenize_sql("SELECT id, name FROM users WHERE age >= 18")
# => [%Token{type: "KEYWORD", value: "SELECT"}, %Token{type: "NAME", value: "id"}, ...]
```

## How It Works

Reads `sql.tokens` from the shared grammars directory and delegates to
`GrammarLexer.tokenize/2`. The grammar is cached via `persistent_term` for
fast repeated access.

The SQL grammar sets `@case_insensitive true`, so keywords are automatically
normalized to uppercase. `select`, `SELECT`, and `Select` all produce a KEYWORD
token with value `"SELECT"`.

## Token Types

| Type              | Description                                  |
|-------------------|----------------------------------------------|
| `KEYWORD`         | SQL reserved word, value always uppercase    |
| `NAME`            | Identifier or backtick-quoted identifier     |
| `NUMBER`          | Integer or decimal literal                   |
| `STRING`          | Single-quoted string literal (quotes stripped) |
| `EQUALS`          | `=`                                          |
| `NOT_EQUALS`      | `!=` or `<>`                                 |
| `LESS_THAN`       | `<`                                          |
| `GREATER_THAN`    | `>`                                          |
| `LESS_EQUALS`     | `<=`                                         |
| `GREATER_EQUALS`  | `>=`                                         |
| `PLUS`            | `+`                                          |
| `MINUS`           | `-`                                          |
| `STAR`            | `*`                                          |
| `SLASH`           | `/`                                          |
| `PERCENT`         | `%`                                          |
| `LPAREN`          | `(`                                          |
| `RPAREN`          | `)`                                          |
| `COMMA`           | `,`                                          |
| `SEMICOLON`       | `;`                                          |
| `DOT`             | `.`                                          |
| `EOF`             | End of input                                 |

Whitespace, `-- line comments`, and `/* block comments */` are skipped silently.

## Dependencies

- `grammar_tools` — parses `.tokens` files
- `lexer` — grammar-driven tokenization engine
