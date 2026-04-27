# @coding-adventures/lexer

Tokenizer/scanner that breaks source code into a stream of tokens — Layer 6 of the computing stack.

## What It Does

A **lexer** (also called a **tokenizer** or **scanner**) is the very first phase of understanding a programming language. Before a computer can execute code like `x = 1 + 2`, it needs to break that raw text into meaningful chunks called **tokens**.

Given the input `x = 1 + 2`, the lexer produces:

```
NAME("x")  EQUALS("=")  NUMBER("1")  PLUS("+")  NUMBER("2")  EOF
```

Each token has a **type** (what kind of thing it is), a **value** (the actual text), and a **position** (line and column numbers for error reporting).
Formatter-oriented callers can also opt into richer source preservation so
comments, whitespace, and exact offsets survive lexing.

## Two Lexer Implementations

This package provides two lexer implementations that produce identical output:

### 1. Hand-Written Lexer (`tokenize`)

A character-by-character scanner with hardcoded dispatching logic. This is the **reference implementation** — clear, well-documented, and easy to step through in a debugger.

```typescript
import { tokenize } from "@coding-adventures/lexer";

const tokens = tokenize("x = 1 + 2");
// [{ type: "NAME", value: "x", line: 1, column: 1 }, ...]

// With language-specific keywords:
const tokens = tokenize("if x == 1", {
  keywords: ["if", "else", "while", "def", "return"],
});
```

### 2. Grammar-Driven Lexer (`grammarTokenize`)

A regex-based lexer driven by a `.tokens` grammar file. This is the **flexible alternative** — language-agnostic, data-driven, and extensible.

```typescript
import { readFileSync } from "fs";
import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize } from "@coding-adventures/lexer";

const grammar = parseTokenGrammar(readFileSync("python.tokens", "utf-8"));
const tokens = grammarTokenize("x = 1 + 2", grammar);

// Rich source mode for formatter pipelines:
const richTokens = grammarTokenize("  // lead\nx = 1", grammar, {
  preserveSourceInfo: true,
});
```

## Token Format

Every token conforms to the `Token` interface:

```typescript
interface Token {
  type: string;    // e.g., "NAME", "NUMBER", "PLUS", "KEYWORD"
  value: string;   // e.g., "x", "42", "+", "if"
  line: number;    // 1-based line number
  column: number;  // 1-based column number
  // Optional in rich source mode:
  startOffset?: number;
  endOffset?: number;
  endLine?: number;
  endColumn?: number;
  tokenIndex?: number;
  leadingTrivia?: Trivia[];
}
```

When `preserveSourceInfo` is enabled on `GrammarLexer` or `grammarTokenize()`,
the lexer also preserves named skip matches as `Trivia` values attached to the
next emitted token.

## How It Fits in the Stack

The lexer sits between raw source code and the parser:

```
Source Code  -->  [Lexer]  -->  Token Stream  -->  [Parser]  -->  AST
```

The parser does not care which lexer produced the tokens — both implementations output the same `Token` objects, making them fully interchangeable.

## Development

```bash
npm install
npm test                   # Run tests
npm run test:coverage      # Run tests with coverage
npm run build              # Compile TypeScript
```

## Dependencies

- `@coding-adventures/grammar-tools` — Parses `.tokens` grammar files for the grammar-driven lexer.
