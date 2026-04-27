# Parser

**Layer 7 of the computing stack** — builds abstract syntax trees from token streams.

## What this package does

Builds abstract syntax trees (ASTs) from token streams using recursive descent parsing:

| Feature | Description |
|---------|-------------|
| Recursive descent | Top-down parsing strategy, one function per grammar rule |
| Operator precedence | Correctly handles precedence and associativity |
| Expression parsing | Parses arithmetic expressions with +, -, *, / |
| Statement parsing | Parses assignments and expression statements |
| AST construction | Produces a tree representation suitable for compilation |
| Grammar-driven mode | Reads .grammar files to parse any language |

## Where it fits

```
Logic Gates -> Arithmetic -> CPU -> ARM -> Assembler -> Lexer -> [Parser] -> Compiler -> VM
```

This package is used by the **bytecode-compiler** package to generate bytecode from the AST.

## Two parsers in one package

### 1. Hand-written Parser (`Parser`)

Uses recursive descent with specific AST node types (`NumberLiteral`, `BinaryOp`, etc.). Great for learning and for cases where you want typed AST nodes.

### 2. Grammar-driven Parser (`GrammarParser`)

Reads grammar rules from a `.grammar` file (via `grammar-tools`) and produces generic `ASTNode` trees. Language-agnostic — swap the grammar file to parse a different language.

## Installation

```bash
npm install @coding-adventures/parser
```

## Usage

### Hand-written parser

```typescript
import { Parser } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";

const tokens: Token[] = [
  { type: "NAME", value: "x", line: 1, column: 1 },
  { type: "EQUALS", value: "=", line: 1, column: 3 },
  { type: "NUMBER", value: "42", line: 1, column: 5 },
  { type: "EOF", value: "", line: 1, column: 7 },
];
const parser = new Parser(tokens);
const ast = parser.parse();
// ast.kind === "Program"
// ast.statements[0].kind === "Assignment"
```

### Grammar-driven parser

```typescript
import { parseParserGrammar, parseTokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize } from "@coding-adventures/lexer";
import { GrammarParser } from "@coding-adventures/parser";
import { readFileSync } from "fs";

const tokenGrammar = parseTokenGrammar(readFileSync("python.tokens", "utf-8"));
const grammar = parseParserGrammar(readFileSync("python.grammar", "utf-8"));
const tokens = grammarTokenize("x = 1 + 2", tokenGrammar, {
  preserveSourceInfo: true,
});
const parser = new GrammarParser(tokens, grammar, {
  preserveSourceInfo: true,
});
const ast = parser.parse();
// ast.ruleName === "program"
```

## AST Node Types

### Hand-written parser

| Type | Description | Example |
|------|-------------|---------|
| `NumberLiteral` | Numeric literal | `42` |
| `StringLiteral` | String literal | `"hello"` |
| `Name` | Variable reference | `x` |
| `BinaryOp` | Binary operation | `1 + 2` |
| `Assignment` | Variable assignment | `x = 42` |
| `Program` | Root node | Contains all statements |

### Grammar-driven parser

| Type | Description |
|------|-------------|
| `ASTNode` | Generic node with `ruleName`, `children`, and optional source spans |

When `preserveSourceInfo` is enabled on `GrammarParser`, grammar-driven AST
nodes also retain:

- `startOffset` / `endOffset`
- `firstTokenIndex` / `lastTokenIndex`
- `leadingTrivia`

## Dependencies

- `@coding-adventures/lexer` — Token types and tokenization
- `@coding-adventures/grammar-tools` — Grammar file parsing (for grammar-driven mode)

## Spec

See [07-parser.md](../../../specs/03-parser.md) for the full specification.
