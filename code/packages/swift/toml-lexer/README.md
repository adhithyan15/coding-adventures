# TOMLLexer (Swift)

A TOML lexer that tokenizes TOML source text into a stream of typed `Token` values. A thin wrapper around the grammar-driven `GrammarLexer` from the Lexer package, configured by `toml.tokens`.

## Usage

```swift
import TOMLLexer

let tokens = try TOMLLexer.tokenize("""
[server]
host = "localhost"
""")
for token in tokens {
    print("\(token.type) \(token.value) (\(token.line):\(token.column))")
}
```

## Dependencies

- `GrammarTools` -- parses `toml.tokens`
- `Lexer` -- provides `GrammarLexer`

## Running tests

```bash
swift test --verbose
```
