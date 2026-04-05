# ECMAScript ES1 Lexer (Go)

Tokenizes ECMAScript 1 (1997) source code using the grammar-driven lexer engine. A thin wrapper that loads `ecmascript/es1.tokens` and delegates tokenization to the generic `GrammarLexer`.

## What Is ES1?

ECMAScript 1 was the first standardized version of JavaScript (ECMA-262, 1st Edition, June 1997). It defines the core language: `var`, `function`, `if/else`, `while`, `for`, `switch`, and basic operators.

ES1 does NOT have: `===`/`!==`, `try`/`catch`, regex literals, `let`/`const`, arrow functions, or classes.

## Usage

```go
import es1lexer "github.com/adhithyan15/coding-adventures/code/packages/go/ecmascript-es1-lexer"

// Tokenize in one call
tokens, err := es1lexer.TokenizeEs1("var x = 1 + 2;")

// Or create a lexer for incremental use
lexer, err := es1lexer.CreateEs1Lexer("var x = 1;")
```

## How It Works

1. Reads `code/grammars/ecmascript/es1.tokens` at initialization
2. Parses the token grammar (keywords, operators, literal patterns)
3. Uses the generic `GrammarLexer` to match and classify tokens
