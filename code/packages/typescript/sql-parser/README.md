# sql-parser

Parses SQL text into abstract syntax trees using the grammar-driven parser — a thin wrapper that loads `sql.grammar`.

## Overview

This package is part of the [coding-adventures](../../..) educational computing stack. It demonstrates how the same generic grammar-driven parser infrastructure that handles JSON and Python can also handle SQL — simply by loading a different `.grammar` file.

The parser handles the ANSI SQL subset defined in `code/grammars/sql.grammar`, including:

- **SELECT** with WHERE, GROUP BY, HAVING, ORDER BY, LIMIT/OFFSET
- **JOIN** (INNER, LEFT, RIGHT, FULL, CROSS)
- **INSERT INTO VALUES**
- **UPDATE SET WHERE**
- **DELETE FROM WHERE**
- **CREATE TABLE** (with column definitions and constraints)
- **DROP TABLE IF EXISTS**
- **Expressions** with full operator precedence (arithmetic, comparison, AND/OR/NOT, BETWEEN, IN, LIKE, IS NULL)

## How It Fits in the Stack

```
sql.tokens (token grammar)     sql.grammar (parser grammar)
    │                                │
    ▼                                ▼
grammar-tools                   grammar-tools
    │                                │
    ▼                                ▼
lexer (grammarTokenize)         parser (GrammarParser)
    │                                │
    ▼                                ▼
sql-lexer (tokenize)  ------>   sql-parser ← YOU ARE HERE
```

## Usage

```typescript
import { parseSQL, createSQLParser } from "coding-adventures-sql-parser";

const ast = parseSQL("SELECT id, name FROM users WHERE age > 18");
console.log(ast.ruleName); // "program"

// Case-insensitive: all three parse identically
parseSQL("SELECT * FROM users");
parseSQL("select * from users");
parseSQL("Select * From Users");

// Insert
const insertAst = parseSQL("INSERT INTO users (name, age) VALUES ('Alice', 30)");

// Multiple statements
const multiAst = parseSQL("SELECT 1 FROM t; DELETE FROM temp WHERE expired = 1");
```

## AST Structure

The top-level rule is always `program`. Below it:

```
program
└── statement
    └── select_stmt
        ├── select_list
        │   └── select_item
        │       └── expr → column_ref → NAME("id")
        ├── table_ref
        │   └── table_name → NAME("users")
        └── where_clause
            └── expr → comparison → ...
```

## Running Tests

```bash
npm ci
npx vitest run --coverage
```

## Architecture

Like the sql-lexer, this parser contains no SQL-specific parsing logic. It:

1. Calls `tokenizeSQL()` from `coding-adventures-sql-lexer` to get the token stream
2. Reads `code/grammars/sql.grammar` from the repository root
3. Parses the grammar into a structured rule table (via `@coding-adventures/grammar-tools`)
4. Runs `GrammarParser` (via `@coding-adventures/parser`) for recursive descent with backtracking

Because the sql-lexer normalizes all keywords to uppercase, the parser grammar uses quoted strings like `"SELECT"` and `"WHERE"` — ensuring case-insensitive SQL works transparently.
