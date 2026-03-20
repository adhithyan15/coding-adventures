# @coding-adventures/json-parser

Parses JSON text (RFC 8259) into abstract syntax trees using the grammar-driven parser infrastructure.

## What It Does

This package is a **thin wrapper** around the generic `GrammarParser` from `@coding-adventures/parser`. It loads the `json.grammar` file, tokenizes the input with `@coding-adventures/json-lexer`, and delegates all parsing work to the generic engine.

JSON has just four grammar rules (value, object, pair, array), making it the simplest practical grammar in this project and an ideal test case for the recursive descent parser.

## How It Fits in the Stack

```
json.tokens (token grammar)    json.grammar (parser grammar)
    |                               |
    v                               v
grammar-tools                  grammar-tools
    |                               |
    v                               v
lexer (grammarTokenize)        parser (GrammarParser)
    |                               |
    v                               v
json-lexer (tokenize)  ------>  json-parser (this package)
```

## Usage

```typescript
import { parseJSON } from "@coding-adventures/json-parser";

const ast = parseJSON('{"name": "Alice", "age": 30}');
console.log(ast.ruleName); // "value"

// Walk the AST to find all key-value pairs:
function findPairs(node) {
  const results = [];
  if (node.ruleName === "pair") results.push(node);
  for (const child of node.children) {
    if (child.ruleName) results.push(...findPairs(child));
  }
  return results;
}

const pairs = findPairs(ast);
console.log(pairs.length); // 2
```

## Grammar Rules

| Rule   | Definition                                    | Description                      |
|--------|-----------------------------------------------|----------------------------------|
| value  | object \| array \| STRING \| NUMBER \| TRUE \| FALSE \| NULL | Any JSON value        |
| object | LBRACE [ pair { COMMA pair } ] RBRACE         | Key-value pairs in braces        |
| pair   | STRING COLON value                             | One key-value pair               |
| array  | LBRACKET [ value { COMMA value } ] RBRACKET   | Values in brackets               |

## Running Tests

```bash
npm install
npm test
```
