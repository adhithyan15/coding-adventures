# haskell-parser

Parses Haskell source code into ASTs using the shared grammar-driven parser engine.

The package loads `code/grammars/haskell/haskell<version>.grammar`, caches the parsed
`ParserGrammar` in `:persistent_term`, tokenizes source with
`CodingAdventures.HaskellLexer`, and delegates AST construction to
`CodingAdventures.Parser.GrammarParser`.

## API

- `CodingAdventures.HaskellParser.parse(source, version \\ nil)` returns
  `{:ok, ast}` or `{:error, reason}`.
- `CodingAdventures.HaskellParser.create_parser(version \\ nil)` returns the parsed
  `ParserGrammar` for the requested Haskell version.

Supported versions: `1.0`, `1.1`, `1.4`, `5`, `7`, `8`, `10`, `14`, `17`, `21`.

## Dependencies

- parser
- haskell-lexer

## Development

```bash
# Run tests
bash BUILD
```
