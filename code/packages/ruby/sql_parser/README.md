# SQL Parser

A Ruby gem that parses ANSI SQL text into Abstract Syntax Trees using the grammar-driven parser engine.

## Overview

This gem is a thin wrapper around `coding_adventures_parser`'s `GrammarDrivenParser`. It loads `sql.grammar` and `sql.tokens`, then uses the generic lexer and parser engines to transform SQL text into an AST.

SQL (ANSI SQL subset) is defined by a grammar covering SELECT, INSERT, UPDATE, DELETE, CREATE TABLE, and DROP TABLE statements, along with rich expression syntax including comparisons, boolean operators, arithmetic, function calls, and joins.

This demonstrates the full grammar-driven pipeline: the same engines that could parse one language can parse any language, just by swapping the grammar files.

## How It Fits in the Stack

```
sql.tokens + sql.grammar (grammar files)
       |                |
       v                v
grammar_tools     grammar_tools
(TokenGrammar)   (ParserGrammar)
       |                |
       v                v
lexer              parser
(GrammarLexer)   (GrammarDrivenParser)
       |                |
       v                v
sql_lexer        sql_parser (this gem)
(tokens)         (AST)
```

## Usage

```ruby
require "coding_adventures_sql_parser"

# Parse a SELECT statement
ast = CodingAdventures::SqlParser.parse_sql("SELECT id, name FROM users WHERE active = TRUE")
# => ASTNode(rule_name: "program", children: [
#      ASTNode(rule_name: "statement", children: [
#        ASTNode(rule_name: "select_stmt", children: [...])
#      ])
#    ])

# Or get a parser object for introspection
parser = CodingAdventures::SqlParser.create_sql_parser("SELECT 1 FROM t")
ast = parser.parse
```

## Case Insensitivity

Because `sql.tokens` has `@case_insensitive true`, the `sql_lexer` normalizes keyword values to uppercase. The `sql_parser` grammar matches uppercase keyword literals, so lowercase or mixed-case SQL is parsed identically:

```ruby
# All three parse to the same AST:
CodingAdventures::SqlParser.parse_sql("SELECT id FROM users")
CodingAdventures::SqlParser.parse_sql("select id from users")
CodingAdventures::SqlParser.parse_sql("Select Id From Users")
```

## Supported SQL Subset

### Statements

- `SELECT [DISTINCT|ALL] ... FROM ... [JOIN ...] [WHERE ...] [GROUP BY ...] [HAVING ...] [ORDER BY ...] [LIMIT ... [OFFSET ...]]`
- `INSERT INTO ... [(cols)] VALUES (...)`
- `UPDATE ... SET ... [WHERE ...]`
- `DELETE FROM ... [WHERE ...]`
- `CREATE TABLE [IF NOT EXISTS] ... (...)`
- `DROP TABLE [IF EXISTS] ...`

### Expressions

- Boolean: `OR`, `AND`, `NOT`
- Comparison: `=`, `!=`, `<>`, `<`, `>`, `<=`, `>=`
- Range: `BETWEEN ... AND ...`
- Set: `IN (...)`
- Pattern: `LIKE`
- Null: `IS NULL`, `IS NOT NULL`
- Arithmetic: `+`, `-`, `*`, `/`, `%`
- Function calls: `name(args)`
- Column references: `table.column`
- Primary values: `NUMBER`, `STRING`, `NULL`, `TRUE`, `FALSE`

### Join Types

`INNER JOIN`, `LEFT [OUTER] JOIN`, `RIGHT [OUTER] JOIN`, `FULL [OUTER] JOIN`, `CROSS JOIN`

Note: bare `JOIN` (without a qualifier) is not in the grammar. Use `INNER JOIN` instead.

## Dependencies

- `coding_adventures_grammar_tools` -- reads `.tokens` and `.grammar` files
- `coding_adventures_parser` -- the grammar-driven parser engine
- `coding_adventures_sql_lexer` -- tokenizes SQL text with case-insensitive keyword normalization

## Development

```bash
bundle install
bundle exec rake test
```
