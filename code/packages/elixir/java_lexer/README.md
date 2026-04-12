# java-lexer

Tokenizes Java source code using the shared grammar-driven lexer engine.

The package loads `code/grammars/java/java<version>.tokens`, caches the parsed
`TokenGrammar` in `:persistent_term`, and delegates to
`CodingAdventures.Lexer.GrammarLexer`.

## API

- `CodingAdventures.JavaLexer.tokenize(source, version \\ nil)` returns
  `{:ok, tokens}` or `{:error, reason}`.
- `CodingAdventures.JavaLexer.create_lexer(version \\ nil)` returns the parsed
  `TokenGrammar` for the requested Java version.

Supported versions: `1.0`, `1.1`, `1.4`, `5`, `7`, `8`, `10`, `14`, `17`, `21`.

## Dependencies

- grammar-tools
- lexer

## Development

```bash
# Run tests
bash BUILD
```
