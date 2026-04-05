# EcmascriptES5Lexer (Swift)

An ECMAScript 5 (2009) lexer for Swift. Adds the `debugger` keyword over ES3 and retains all ES3 features.

## Usage

```swift
import EcmascriptES5Lexer

let tokens = try EcmascriptES5Lexer.tokenize("debugger;")
```

## Dependencies

- `GrammarTools` -- parses `es5.tokens`
- `Lexer` -- provides `GrammarLexer`

## Running tests

```bash
swift test --verbose
```
