# SQL Parser

Parses ANSI SQL text into ASTs using the grammar-driven parser — a thin
wrapper that loads `sql.grammar` and feeds it to the generic `GrammarParser`.

## What Is This?

This package is a **thin wrapper** around the grammar-driven `GrammarParser`. It
tokenizes SQL using the `sql-lexer` package (which normalizes all keywords to
uppercase), then parses the token stream using the EBNF rules defined in
`sql.grammar`. The result is a generic `ASTNode` tree.

The SQL grammar covers a broad ANSI SQL subset:

- **SELECT** — columns, `*`, `AS` aliases, `WHERE`, `GROUP BY`, `HAVING`,
  `ORDER BY` (ASC/DESC), `LIMIT`, `OFFSET`, and all JOIN types.
- **INSERT INTO** — with optional column list and `VALUES`.
- **UPDATE SET** — with optional `WHERE`.
- **DELETE FROM** — with optional `WHERE`.
- **CREATE TABLE** — with `IF NOT EXISTS`, column definitions, and constraints
  (`NOT NULL`, `PRIMARY KEY`, `UNIQUE`, `DEFAULT`).
- **DROP TABLE** — with optional `IF EXISTS`.
- **Expressions** — arithmetic, comparisons, `AND`/`OR`/`NOT`, `BETWEEN`,
  `IN`, `LIKE`, `IS NULL`, `IS NOT NULL`, function calls, and subexpressions.
- **Multiple statements** — semicolon-separated in a single `parse_sql()` call.

## How It Fits in the Stack

```
sql.tokens + sql.grammar  (declarative definitions)
    |              |
    v              v
sql_lexer      grammar_tools.parse_parser_grammar()
    |              |
    v              v
list[Token]    ParserGrammar
    |              |
    +------+-------+
           |
           v
    GrammarParser  (generic engine from parser package)
           |
           v
    sql_parser.parse_sql()  (this thin wrapper)
           |
           v
       ASTNode tree  (root rule_name = "program")
```

## Usage

```python
from sql_parser import parse_sql

# Simple SELECT
ast = parse_sql("SELECT id, name FROM users WHERE age > 18")
print(ast.rule_name)  # "program"

# INSERT
ast = parse_sql("INSERT INTO users (id, name) VALUES (1, 'Alice')")

# Multiple statements
ast = parse_sql("DELETE FROM temp; DROP TABLE temp")

# Use the factory for step-by-step control
from sql_parser import create_sql_parser

parser = create_sql_parser("SELECT COUNT(*) FROM orders GROUP BY status")
ast = parser.parse("program")
```

## Installation

```bash
pip install coding-adventures-sql-parser
```

## Dependencies

- `coding-adventures-sql-lexer` — tokenizes SQL text (case-insensitive keywords)
- `coding-adventures-grammar-tools` — parses the `.grammar` file
- `coding-adventures-lexer` — provides the token types
- `coding-adventures-parser` — provides the `GrammarParser` engine
