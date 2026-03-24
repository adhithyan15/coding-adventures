# sql-lexer

Tokenizes SQL text using the grammar-driven lexer — a thin wrapper that loads `sql.tokens`.

## Overview

This package is part of the [coding-adventures](../../..) educational computing stack. It demonstrates how the same generic grammar-driven lexer engine that tokenizes JSON, Python, and Ruby can also tokenize SQL — simply by swapping the `.tokens` grammar file.

The lexer handles the full ANSI SQL subset defined in `code/grammars/sql.tokens`, including:

- **Case-insensitive keywords** — `SELECT`, `select`, and `Select` all produce `KEYWORD("SELECT")`
- **Single-quoted strings** — `'hello'` → `STRING("hello")` (quotes stripped)
- **Backtick-quoted identifiers** — `` `column` `` → `NAME("\`column\`")` (backticks kept)
- **Multi-character operators** — `!=`, `<>`, `<=`, `>=` (longest match first)
- **Both comment styles** — `-- line` and `/* block */` skipped silently

## How It Fits In the Stack

```
sql.tokens (grammar)
     │
     ▼
grammar-tools (parseTokenGrammar)
     │
     ▼
lexer (grammarTokenize / GrammarLexer)
     │
     ▼
sql-lexer ← YOU ARE HERE
     │
     ▼
sql-parser (builds ASTs from the token stream)
```

## Usage

```typescript
import { tokenizeSQL, createSQLLexer } from "coding-adventures-sql-lexer";

const tokens = tokenizeSQL("SELECT id, name FROM users WHERE age > 18");
// [KEYWORD("SELECT"), NAME("id"), COMMA(","), NAME("name"),
//  KEYWORD("FROM"), NAME("users"), KEYWORD("WHERE"), NAME("age"),
//  GREATER_THAN(">"), NUMBER("18"), EOF]

// Case-insensitive: all produce the same output
tokenizeSQL("select * from users");
tokenizeSQL("SELECT * FROM users");
tokenizeSQL("Select * From Users");
```

## Token Types

| Token          | Example        | Description                          |
|----------------|----------------|--------------------------------------|
| `NAME`         | `users`, `id`  | Identifier (table or column name)    |
| `NUMBER`       | `42`, `3.14`   | Integer or decimal number            |
| `STRING`       | `'hello'`      | Single-quoted string (quotes stripped) |
| `KEYWORD`      | `SELECT`       | SQL reserved keyword (uppercase)     |
| `EQUALS`       | `=`            | Equality / assignment                |
| `NOT_EQUALS`   | `!=` or `<>`   | Inequality (both forms normalized)   |
| `LESS_THAN`    | `<`            | Less-than comparison                 |
| `GREATER_THAN` | `>`            | Greater-than comparison              |
| `LESS_EQUALS`  | `<=`           | Less-than-or-equal                   |
| `GREATER_EQUALS` | `>=`         | Greater-than-or-equal                |
| `STAR`         | `*`            | Multiplication or `SELECT *`         |
| `LPAREN`       | `(`            | Left parenthesis                     |
| `RPAREN`       | `)`            | Right parenthesis                    |
| `COMMA`        | `,`            | Separator                            |
| `SEMICOLON`    | `;`            | Statement terminator                 |
| `DOT`          | `.`            | Qualifier (schema.table)             |
| `EOF`          | (synthetic)    | End of input                         |

## Running Tests

```bash
npm ci
npx vitest run --coverage
```

## Architecture

The lexer itself contains no SQL-specific logic. It:

1. Reads `code/grammars/sql.tokens` from the repository root
2. Parses it into a `TokenGrammar` object (via `@coding-adventures/grammar-tools`)
3. Passes the source text and grammar to `grammarTokenize` (via `@coding-adventures/lexer`)

The `@case_insensitive true` directive in `sql.tokens` tells the generic lexer engine to:
- Match keyword patterns case-insensitively
- Normalize keyword values to uppercase on emission

This is the grammar-driven architecture in action: the specification (the `.tokens` file) drives the behavior without any hardcoded SQL logic in TypeScript.
