# java-parser

Parses Java source code into ASTs using the shared grammar-driven parser engine.

The package loads `code/grammars/java/java<version>.grammar`, caches the parsed
`ParserGrammar` in `:persistent_term`, tokenizes source with
`CodingAdventures.JavaLexer`, and delegates AST construction to
`CodingAdventures.Parser.GrammarParser`.

## API

- `CodingAdventures.JavaParser.parse(source, version \\ nil)` returns
  `{:ok, ast}` or `{:error, reason}`.
- `CodingAdventures.JavaParser.create_parser(version \\ nil)` returns the parsed
  `ParserGrammar` for the requested Java version.

Supported versions: `1.0`, `1.1`, `1.4`, `5`, `7`, `8`, `10`, `14`, `17`, `21`.

## Dependencies

- parser
- java-lexer

## Development

```bash
# Run tests
bash BUILD
```
