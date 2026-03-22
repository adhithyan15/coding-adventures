# @coding-adventures/toml-parser

Parses TOML text (v1.0.0) into abstract syntax trees using the grammar-driven parser infrastructure.

## What It Does

This package is a **thin wrapper** around the generic `GrammarParser` from `@coding-adventures/parser`. It loads the `toml.grammar` file and delegates all parsing work to the generic engine.

TOML has ~12 grammar rules -- significantly more than JSON's 4 rules. The additional complexity comes from newline-delimited expressions, table headers, array-of-tables headers, dotted keys, bare keys, inline tables, and multi-line arrays.

## How It Fits in the Stack

```
toml.tokens         toml.grammar
    |                    |
    v                    v
grammar-tools       grammar-tools
    |                    |
    v                    v
lexer               parser (GrammarParser)
    |                    |
    v                    v
toml-lexer ------> toml-parser (this package)
                         |
                         v
                    AST (ASTNode tree)
```

## Usage

```typescript
import { parseTOML } from "@coding-adventures/toml-parser";

const ast = parseTOML(`
[server]
host = "localhost"
port = 8080
enabled = true
`);

console.log(ast.ruleName); // "document"
```

## Grammar Rules

| Rule                 | Description                                     |
|----------------------|-------------------------------------------------|
| document             | Top-level: sequence of expressions + newlines   |
| expression           | array_table_header, table_header, or keyval     |
| keyval               | key = value                                     |
| key                  | simple_key { DOT simple_key } (dotted keys)     |
| simple_key           | BARE_KEY, quoted string, or value-as-key token  |
| table_header         | [key]                                           |
| array_table_header   | [[key]]                                         |
| value                | Any TOML value (string, number, bool, date, etc)|
| array                | [values]                                        |
| array_values         | Comma-separated values with optional newlines   |
| inline_table         | { keyval, keyval, ... }                         |

## Running Tests

```bash
npm install
npm test
```
