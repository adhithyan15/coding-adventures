# coding-adventures-sql-lexer (Lua)

A SQL lexer that tokenizes SQL source text into a flat stream of typed tokens. It is a thin wrapper around the grammar-driven `GrammarLexer` from `coding-adventures-lexer`, configured by the shared `sql.tokens` grammar file.

## What it does

Given the input `SELECT * FROM users WHERE id = 1`, the lexer produces:

| # | Type         | Value   |
|---|--------------|---------|
| 1 | SELECT       | `SELECT` |
| 2 | STAR         | `*`     |
| 3 | FROM         | `FROM`  |
| 4 | NAME         | `users` |
| 5 | WHERE        | `WHERE` |
| 6 | NAME         | `id`    |
| 7 | EQUALS       | `=`     |
| 8 | NUMBER       | `1`     |
| 9 | EOF          |         |

Whitespace, line comments (`-- ...`), and block comments (`/* ... */`) are silently consumed. The last token is always `EOF`.

## Token types

### Keywords (case-insensitive)

`SELECT`, `FROM`, `WHERE`, `GROUP`, `BY`, `HAVING`, `ORDER`, `LIMIT`, `OFFSET`, `INSERT`, `INTO`, `VALUES`, `UPDATE`, `SET`, `DELETE`, `CREATE`, `DROP`, `TABLE`, `IF`, `EXISTS`, `NOT`, `AND`, `OR`, `NULL`, `IS`, `IN`, `BETWEEN`, `LIKE`, `AS`, `DISTINCT`, `ALL`, `UNION`, `INTERSECT`, `EXCEPT`, `JOIN`, `INNER`, `LEFT`, `RIGHT`, `OUTER`, `CROSS`, `FULL`, `ON`, `ASC`, `DESC`, `TRUE`, `FALSE`, `CASE`, `WHEN`, `THEN`, `ELSE`, `END`, `PRIMARY`, `KEY`, `UNIQUE`, `DEFAULT`

### Values

| Token type | Example match |
|------------|---------------|
| NAME       | `users`, `id`, `department` |
| NUMBER     | `42`, `3.14` |
| STRING     | `'hello'`, `'it\'s'` |

### Operators

| Token type      | Symbol |
|-----------------|--------|
| EQUALS          | `=`    |
| NOT_EQUALS      | `!=`, `<>` |
| LESS_THAN       | `<`    |
| GREATER_THAN    | `>`    |
| LESS_EQUALS     | `<=`   |
| GREATER_EQUALS  | `>=`   |
| PLUS            | `+`    |
| MINUS           | `-`    |
| STAR            | `*`    |
| SLASH           | `/`    |
| PERCENT         | `%`    |

### Delimiters

| Token type | Symbol |
|------------|--------|
| LPAREN     | `(`    |
| RPAREN     | `)`    |
| COMMA      | `,`    |
| SEMICOLON  | `;`    |
| DOT        | `.`    |

## Usage

```lua
local sql_lexer = require("coding_adventures.sql_lexer")

local tokens = sql_lexer.tokenize("SELECT * FROM users")
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
```

## How it fits in the stack

```
sql.tokens  (code/grammars/)
    ↓  parsed by grammar_tools
TokenGrammar
    ↓  drives
GrammarLexer  (coding-adventures-lexer)
    ↓  wrapped by
sql_lexer  ← you are here
    ↓  feeds
sql_parser  (future)
```

## SQL-specific notes

**Case-insensitive keywords** — The `sql.tokens` grammar is marked `@case_insensitive true`. Keywords like `SELECT`, `select`, and `Select` all produce a `SELECT` token. The original source casing is preserved in the token value.

**Operator precedence** — Longer operators match before shorter ones: `<=` before `<`, `>=` before `>`, `!=` before no match. This is enforced by definition ordering in `sql.tokens`.

**NEQ_ANSI alias** — Both `!=` and `<>` produce a `NOT_EQUALS` token. The grammar aliases `<>` to `NOT_EQUALS` so a parser handles only one token type for inequality.

**String alias** — Single-quoted strings (`'...'`) are matched as `STRING_SQ` internally and aliased to `STRING`. Backtick-quoted identifiers are matched as `QUOTED_ID` and aliased to `NAME`.

## Dependencies

- `coding-adventures-grammar-tools` — parses `sql.tokens`
- `coding-adventures-lexer` — provides `GrammarLexer`
- `coding-adventures-state-machine` — used internally by the lexer
- `coding-adventures-directed-graph` — used internally by grammar tools

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```
