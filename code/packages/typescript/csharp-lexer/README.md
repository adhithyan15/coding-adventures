# C# Lexer (TypeScript)

Tokenizes C# source code using the grammar-driven lexer approach.

## What Is This?

This package is a **thin wrapper** around the generic `grammarTokenize` from the `@coding-adventures/lexer` package. It loads the appropriate `csharp{version}.tokens` grammar file and delegates all tokenization to the generic engine.

## Usage

```typescript
import { tokenizeCSharp, createCSharpLexer } from "@coding-adventures/csharp-lexer";

// Default version (C# 12.0)
const tokens = tokenizeCSharp("class Hello { }");

// Specific version
const tokens8 = tokenizeCSharp("int x = 1;", "8.0");

// Lambda and null-coalescing (C# 3.0+)
const tokens3 = tokenizeCSharp("var result = value ?? fallback;", "3.0");

// Class-based lexer with on-token callbacks
const lexer = createCSharpLexer("class Hello { }", "12.0");
const tokens = lexer.tokenize();
```

## Supported C# versions

| Version | Grammar file | Notable additions |
|---------|-------------|-------------------|
| `"1.0"` | `grammars/csharp/csharp1.0.tokens` | Original C# |
| `"2.0"` | `grammars/csharp/csharp2.0.tokens` | Generics, nullable types |
| `"3.0"` | `grammars/csharp/csharp3.0.tokens` | LINQ, `var`, lambda `=>` |
| `"4.0"` | `grammars/csharp/csharp4.0.tokens` | `dynamic`, named/optional params |
| `"5.0"` | `grammars/csharp/csharp5.0.tokens` | `async`/`await` |
| `"6.0"` | `grammars/csharp/csharp6.0.tokens` | String interpolation, `?.` |
| `"7.0"` | `grammars/csharp/csharp7.0.tokens` | Tuples, pattern matching |
| `"8.0"` | `grammars/csharp/csharp8.0.tokens` | Nullable reference types |
| `"9.0"` | `grammars/csharp/csharp9.0.tokens` | Records, top-level programs |
| `"10.0"` | `grammars/csharp/csharp10.0.tokens` | Global usings, file-scoped namespaces |
| `"11.0"` | `grammars/csharp/csharp11.0.tokens` | Required members, list patterns |
| `"12.0"` (default) | `grammars/csharp/csharp12.0.tokens` | Primary constructors, collection expressions |

## Dependencies

- `@coding-adventures/lexer` — provides `grammarTokenize` and `Token`
- `@coding-adventures/grammar-tools` — parses `.tokens` files
