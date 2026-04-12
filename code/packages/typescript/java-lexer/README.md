# Java Lexer (TypeScript)

Tokenizes Java source code using the grammar-driven lexer approach.

## What Is This?

This package is a **thin wrapper** around the generic `grammarTokenize` from the `@coding-adventures/lexer` package. It loads the appropriate `java{version}.tokens` grammar file and delegates all tokenization to the generic engine.

## Usage

```typescript
import { tokenizeJava, createJavaLexer } from "@coding-adventures/java-lexer";

// Default version (Java 21)
const tokens = tokenizeJava("class Hello { }");

// Specific version
const tokens8 = tokenizeJava("int x = 1;", "8");

// Class-based lexer with on-token callbacks
const lexer = createJavaLexer("class Hello { }", "21");
const tokens = lexer.tokenize();
```

## Supported Java versions

| Version | Grammar file |
|---------|-------------|
| `"1.0"` | `grammars/java/java1.0.tokens` |
| `"1.1"` | `grammars/java/java1.1.tokens` |
| `"1.4"` | `grammars/java/java1.4.tokens` |
| `"5"` | `grammars/java/java5.tokens` |
| `"7"` | `grammars/java/java7.tokens` |
| `"8"` | `grammars/java/java8.tokens` |
| `"10"` | `grammars/java/java10.tokens` |
| `"14"` | `grammars/java/java14.tokens` |
| `"17"` | `grammars/java/java17.tokens` |
| `"21"` (default) | `grammars/java/java21.tokens` |

## Dependencies

- `@coding-adventures/lexer` -- provides `grammarTokenize` and `Token`
- `@coding-adventures/grammar-tools` -- parses `.tokens` files
