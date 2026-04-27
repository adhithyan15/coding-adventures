# SQL Parser (Elixir)

Thin wrapper around the grammar-driven parser engine for SQL parsing.

## Usage

```elixir
{:ok, ast} = CodingAdventures.SqlParser.parse_sql("SELECT id, name FROM users WHERE active = 1")
# => %ASTNode{rule_name: "program", children: [%ASTNode{rule_name: "statement", ...}]}
```

## How It Works

Combines `SqlLexer.tokenize_sql/1` with `GrammarParser.parse/2` using `sql.grammar`
from the shared grammars directory. The grammar is cached via `persistent_term`.

Because the SQL lexer normalizes all keywords to uppercase (via `@case_insensitive true`
in `sql.tokens`), the parser grammar can match uppercase keyword literals unconditionally.
This means `select`, `SELECT`, and `Select` all parse identically.

## Supported SQL Subset

| Statement         | Example                                          |
|-------------------|--------------------------------------------------|
| SELECT            | `SELECT * FROM t WHERE id > 0 ORDER BY id LIMIT 10` |
| INSERT            | `INSERT INTO t (id, name) VALUES (1, 'Alice')`   |
| UPDATE            | `UPDATE t SET col = val WHERE id = 1`            |
| DELETE            | `DELETE FROM t WHERE id = 1`                     |
| CREATE TABLE      | `CREATE TABLE t (id INTEGER PRIMARY KEY)`        |
| DROP TABLE        | `DROP TABLE IF EXISTS t`                         |

Multiple semicolon-separated statements are accepted as a single program.

## AST Structure

The root node always has `rule_name == "program"`. Each child represents a
SQL statement parsed according to the grammar rules in `sql.grammar`.

```
program
  └── statement
        └── select_stmt
              ├── KEYWORD "SELECT"
              ├── select_list
              │     └── STAR "*"
              ├── KEYWORD "FROM"
              ├── table_ref
              │     └── table_name
              │           └── NAME "users"
              └── where_clause
                    ├── KEYWORD "WHERE"
                    └── expr
                          └── …
```

## Dependencies

- `grammar_tools` — parses `.grammar` files
- `lexer` — grammar-driven tokenization engine
- `parser` — grammar-driven parsing engine
- `sql_lexer` — SQL tokenization
