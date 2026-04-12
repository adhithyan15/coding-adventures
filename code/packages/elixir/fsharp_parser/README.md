# fsharp-parser

Parses F# source code into ASTs using the shared grammar-driven parser engine.

The package loads `code/grammars/fsharp/fsharp<version>.grammar`, caches the
parsed `ParserGrammar` in `:persistent_term`, tokenizes source with
`CodingAdventures.FSharpLexer`, and delegates AST construction to
`CodingAdventures.Parser.GrammarParser`.

## API

- `CodingAdventures.FSharpParser.parse(source, version \\ nil)` returns
  `{:ok, ast}` or `{:error, reason}`.
- `CodingAdventures.FSharpParser.create_parser(version \\ nil)` returns the
  parsed `ParserGrammar` for the requested F# version.

Supported versions: `1.0`, `2.0`, `3.0`, `3.1`, `4.0`, `4.1`, `4.5`, `4.6`,
`4.7`, `5`, `6`, `7`, `8`, `9`, `10`.

## Dependencies

- parser
- fsharp-lexer

## Development

```bash
# Run tests
bash BUILD
```
