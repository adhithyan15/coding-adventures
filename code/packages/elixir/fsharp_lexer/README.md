# fsharp-lexer

Tokenizes F# source code using the shared grammar-driven lexer engine.

The package loads `code/grammars/fsharp/fsharp<version>.tokens`, caches the
parsed `TokenGrammar` in `:persistent_term`, and delegates to
`CodingAdventures.Lexer.GrammarLexer`.

## API

- `CodingAdventures.FSharpLexer.tokenize(source, version \\ nil)` returns
  `{:ok, tokens}` or `{:error, reason}`.
- `CodingAdventures.FSharpLexer.create_lexer(version \\ nil)` returns the
  parsed `TokenGrammar` for the requested F# version.

Supported versions: `1.0`, `2.0`, `3.0`, `3.1`, `4.0`, `4.1`, `4.5`, `4.6`,
`4.7`, `5`, `6`, `7`, `8`, `9`, `10`.

## Dependencies

- grammar-tools
- lexer

## Development

```bash
# Run tests
bash BUILD
```
