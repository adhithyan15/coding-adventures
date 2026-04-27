# XMLLexer (Swift)

An XML lexer that tokenizes XML text into a stream of typed `Token` values. A thin wrapper around the grammar-driven `GrammarLexer` from the Lexer package, configured by `xml.tokens`.

## Usage

```swift
import XMLLexer

let tokens = try XMLLexer.tokenize("<root>hello</root>")
for token in tokens {
    print("\(token.type) \(token.value) (\(token.line):\(token.column))")
}
```

## Dependencies

- `GrammarTools` -- parses `xml.tokens`
- `Lexer` -- provides `GrammarLexer`

## Running tests

```bash
swift test --verbose
```
