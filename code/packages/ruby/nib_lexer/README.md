# SQL Lexer

A Ruby gem that tokenizes ANSI SQL text using the grammar-driven lexer engine.

## Overview

This gem is a thin wrapper around `coding_adventures_lexer`'s `GrammarLexer`. Instead of hardcoding SQL-specific tokenization rules, it loads the `sql.tokens` grammar file and feeds it to the general-purpose lexer engine.

SQL (Structured Query Language, ANSI SQL subset) has significantly more structure than JSON. It has ~50 keywords, comment forms (line and block), operators with precedence considerations, single-quoted string literals, and backtick-quoted identifiers.

A key feature is **case-insensitive keyword matching**: `sql.tokens` includes the directive `@case_insensitive true`, which causes the lexer to normalize all keyword values to uppercase. This means `select`, `SELECT`, and `Select` all produce `Token(type: KEYWORD, value: "SELECT")`.

This demonstrates the core idea behind grammar-driven language tooling: the same engine can process any language, as long as you provide the right grammar file.

## How It Fits in the Stack

```
sql.tokens (grammar file)
       |
       v
grammar_tools (parses .tokens into TokenGrammar)
       |
       v
lexer (GrammarLexer uses TokenGrammar to tokenize)
       |
       v
sql_lexer (this gem -- thin wrapper providing SQL API)
```

## Usage

```ruby
require "coding_adventures_sql_lexer"

# Tokenize a SQL query
tokens = CodingAdventures::SqlLexer.tokenize_sql("SELECT id FROM users WHERE active = 1")
tokens.each { |t| puts "#{t.type}: #{t.value}" }
# KEYWORD: SELECT
# NAME: id
# KEYWORD: FROM
# NAME: users
# KEYWORD: WHERE
# NAME: active
# EQUALS: =
# NUMBER: 1
# EOF:

# Or get a lexer object directly
lexer = CodingAdventures::SqlLexer.create_sql_lexer("SELECT 1")
tokens = lexer.tokenize
```

## Token Types

### Keywords (type = "KEYWORD", value = uppercase)

`SELECT`, `FROM`, `WHERE`, `GROUP`, `BY`, `HAVING`, `ORDER`, `LIMIT`, `OFFSET`, `INSERT`, `INTO`, `VALUES`, `UPDATE`, `SET`, `DELETE`, `CREATE`, `DROP`, `TABLE`, `IF`, `EXISTS`, `NOT`, `AND`, `OR`, `NULL`, `IS`, `IN`, `BETWEEN`, `LIKE`, `AS`, `DISTINCT`, `ALL`, `UNION`, `INTERSECT`, `EXCEPT`, `JOIN`, `INNER`, `LEFT`, `RIGHT`, `OUTER`, `CROSS`, `FULL`, `ON`, `ASC`, `DESC`, `TRUE`, `FALSE`, `CASE`, `WHEN`, `THEN`, `ELSE`, `END`, `PRIMARY`, `KEY`, `UNIQUE`, `DEFAULT`

### Identifiers

- `NAME` -- unquoted identifiers (e.g., `users`, `order_id`)
- `NAME` -- backtick-quoted identifiers (e.g., `` `my table` ``), value **includes** backticks

### Literals

- `NUMBER` -- integers and decimals (e.g., `42`, `3.14`)
- `STRING` -- single-quoted strings (e.g., `'hello'`), quotes **stripped**

### Operators

- `EQUALS` (`=`), `NOT_EQUALS` (`!=` or `<>`), `LESS_THAN` (`<`), `GREATER_THAN` (`>`)
- `LESS_EQUALS` (`<=`), `GREATER_EQUALS` (`>=`)
- `PLUS` (`+`), `MINUS` (`-`), `STAR` (`*`), `SLASH` (`/`), `PERCENT` (`%`)

### Punctuation

- `LPAREN` (`(`), `RPAREN` (`)`), `COMMA` (`,`), `SEMICOLON` (`;`), `DOT` (`.`)

### Skipped

- `WHITESPACE` -- spaces, tabs, carriage returns, newlines
- `LINE_COMMENT` -- `-- ...` to end of line
- `BLOCK_COMMENT` -- `/* ... */` (may span multiple lines)

## Key Differences from JSON Lexer

- **Keywords**: SQL has ~50 keywords reclassified from NAME tokens. JSON has none.
- **Case insensitivity**: `@case_insensitive true` normalizes keyword values to uppercase.
- **Comments**: SQL supports line (`--`) and block (`/* */`) comments. JSON has none.
- **String quoting**: SQL uses single quotes. JSON uses double quotes.
- **Operators**: SQL has rich comparison and arithmetic operators.

## Dependencies

- `coding_adventures_grammar_tools` -- reads the `.tokens` grammar file
- `coding_adventures_lexer` -- the grammar-driven lexer engine

## Development

```bash
bundle install
bundle exec rake test
```
