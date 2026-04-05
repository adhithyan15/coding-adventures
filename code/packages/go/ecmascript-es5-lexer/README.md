# ECMAScript ES5 Lexer (Go)

Tokenizes ECMAScript 5 (2009) source code using the grammar-driven lexer engine. A thin wrapper that loads `ecmascript/es5.tokens` and delegates tokenization to the generic `GrammarLexer`.

## What Is ES5?

ECMAScript 5 (ECMA-262, 5th Edition, December 2009) landed a decade after ES3. The lexical changes are modest: `debugger` was promoted from future-reserved to a keyword, and the future-reserved word list was significantly reduced. The major innovations (strict mode, JSON, property descriptors) are semantic, not lexical.

## Usage

```go
import es5lexer "github.com/adhithyan15/coding-adventures/code/packages/go/ecmascript-es5-lexer"

// Tokenize in one call
tokens, err := es5lexer.TokenizeEs5("debugger;")

// Or create a lexer for incremental use
lexer, err := es5lexer.CreateEs5Lexer(source)
```

## How It Works

1. Reads `code/grammars/ecmascript/es5.tokens` at initialization
2. Parses the token grammar (keywords, operators, literal patterns)
3. Uses the generic `GrammarLexer` to match and classify tokens
