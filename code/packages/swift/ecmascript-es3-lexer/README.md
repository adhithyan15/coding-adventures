# EcmascriptES3Lexer (Swift)

An ECMAScript 3 (1999) lexer for Swift. Adds strict equality (===, !==), try/catch/finally/throw, instanceof, and regex literals over ES1.

## Usage

```swift
import EcmascriptES3Lexer

let tokens = try EcmascriptES3Lexer.tokenize("var x = 42;")
```

## Dependencies

- `GrammarTools` -- parses `es3.tokens`
- `Lexer` -- provides `GrammarLexer`

## Running tests

```bash
swift test --verbose
```
