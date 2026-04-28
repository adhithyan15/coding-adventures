# Haskell Lexer (TypeScript)

Tokenizes Haskell source code using the grammar-driven lexer approach.

## What Is This?

This package is a **thin wrapper** around the generic `grammarTokenize` from the `@coding-adventures/lexer` package. It loads the appropriate `haskell{version}.tokens` grammar file and delegates all tokenization to the generic engine.

## Usage

```typescript
import { tokenizeHaskell, createHaskellLexer } from "@coding-adventures/haskell-lexer";

// Default version (Haskell 21)
const tokens = tokenizeHaskell("class Hello { }");

// Specific version
const tokens8 = tokenizeHaskell("int x = 1;", "8");

// Class-based lexer with on-token callbacks
const lexer = createHaskellLexer("class Hello { }", "21");
const tokens = lexer.tokenize();
```

## Supported Haskell versions

| Version | Grammar file |
|---------|-------------|
| `"1.0"` | `grammars/haskell/haskell1.0.tokens` |
| `"1.1"` | `grammars/haskell/haskell1.1.tokens` |
| `"1.4"` | `grammars/haskell/haskell1.4.tokens` |
| `"5"` | `grammars/haskell/haskell5.tokens` |
| `"7"` | `grammars/haskell/haskell7.tokens` |
| `"8"` | `grammars/haskell/haskell8.tokens` |
| `"10"` | `grammars/haskell/haskell10.tokens` |
| `"14"` | `grammars/haskell/haskell14.tokens` |
| `"17"` | `grammars/haskell/haskell17.tokens` |
| `"21"` (default) | `grammars/haskell/haskell21.tokens` |

## Dependencies

- `@coding-adventures/lexer` -- provides `grammarTokenize` and `Token`
- `@coding-adventures/grammar-tools` -- parses `.tokens` files
