# ECMAScript ES3 Lexer (Go)

Tokenizes ECMAScript 3 (1999) source code using the grammar-driven lexer engine. A thin wrapper that loads `ecmascript/es3.tokens` and delegates tokenization to the generic `GrammarLexer`.

## What Is ES3?

ECMAScript 3 (ECMA-262, 3rd Edition, December 1999) made JavaScript a real, complete language. It added strict equality (`===`/`!==`), error handling (`try`/`catch`/`finally`/`throw`), `instanceof`, and regular expression literals.

## Usage

```go
import es3lexer "github.com/adhithyan15/coding-adventures/code/packages/go/ecmascript-es3-lexer"

// Tokenize in one call
tokens, err := es3lexer.TokenizeEs3("try { x === 1; } catch (e) {}")

// Or create a lexer for incremental use
lexer, err := es3lexer.CreateEs3Lexer(source)
```

## How It Works

1. Reads `code/grammars/ecmascript/es3.tokens` at initialization
2. Parses the token grammar (keywords, operators, literal patterns)
3. Uses the generic `GrammarLexer` to match and classify tokens
