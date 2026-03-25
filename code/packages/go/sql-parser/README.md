# sql-parser (Go)

Parses an ANSI SQL subset into an Abstract Syntax Tree (AST), built on the grammar-driven parser infrastructure.

## What it does

Converts a SQL string into a hierarchical AST rooted at a `program` node. The
AST mirrors the grammar rules in `sql.grammar`.

## Where it fits

```
sql text
   ↓  sql-lexer      (tokenization, keyword normalization)
token stream
   ↓  sql-parser     (this package)
AST (program → statement → select_stmt / insert_stmt / …)
```

## Usage

```go
import sqlparser "github.com/adhithyan15/coding-adventures/code/packages/go/sql-parser"

// One-shot parsing
ast, err := sqlparser.ParseSQL("SELECT id, name FROM users WHERE active = TRUE")

// Reusable parser
p, err := sqlparser.CreateSQLParser("SELECT * FROM orders ORDER BY id DESC LIMIT 10")
ast, err := p.Parse()
```

## SQL subset supported

| Category | Statements |
|----------|-----------|
| DQL | `SELECT` with `FROM`, `WHERE`, `GROUP BY`, `HAVING`, `ORDER BY`, `LIMIT`/`OFFSET`, `JOIN` (INNER/LEFT/RIGHT/FULL/CROSS), `DISTINCT` |
| DML | `INSERT INTO … VALUES`, `UPDATE … SET`, `DELETE FROM` |
| DDL | `CREATE TABLE [IF NOT EXISTS]`, `DROP TABLE [IF EXISTS]` |
| Expressions | Arithmetic (`+`, `-`, `*`, `/`, `%`), comparisons (`=`, `!=`, `<>`, `<`, `>`, `<=`, `>=`), `AND`/`OR`/`NOT`, `BETWEEN…AND`, `IN(…)`, `LIKE`, `IS NULL`, `IS NOT NULL`, function calls, subexpressions `(…)` |

## AST example

For `SELECT name FROM users WHERE id = 1`:

```
program
  statement
    select_stmt
      KEYWORD("SELECT")
      select_list → select_item → expr → … → column_ref → NAME("name")
      KEYWORD("FROM")
      table_ref → table_name → NAME("users")
      where_clause
        KEYWORD("WHERE")
        expr → … → comparison
          column_ref → NAME("id")
          cmp_op → EQUALS("=")
          primary → NUMBER("1")
```

## Grammar files

- `code/grammars/sql.tokens` — token definitions (keyword list, operators, skip rules)
- `code/grammars/sql.grammar` — parser grammar (EBNF rules for all SQL constructs)

## Running tests

```
go test ./... -v -cover
```
