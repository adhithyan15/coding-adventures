# EcmascriptES1Lexer (Swift)

An ECMAScript 1 (1997) lexer that tokenizes ES1 source text into a stream of typed `Token` values. A thin wrapper around the grammar-driven `GrammarLexer` from the Lexer package, configured by `ecmascript/es1.tokens`.

## Usage

```swift
import EcmascriptES1Lexer

let tokens = try EcmascriptES1Lexer.tokenize("var x = 42;")
for token in tokens {
    print("\(token.type) \(token.value) (\(token.line):\(token.column))")
}
```

## Dependencies

- `GrammarTools` -- parses `es1.tokens`
- `Lexer` -- provides `GrammarLexer`

## Running tests

```bash
swift test --verbose
```
