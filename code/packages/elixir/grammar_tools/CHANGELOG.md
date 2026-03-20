# Changelog

## [0.1.0] - 2026-03-20

### Added
- Initial release — port of the Python grammar-tools package to Elixir.
- `TokenGrammar` module: parses `.tokens` files into structured data.
- `ParserGrammar` module: parses `.grammar` files (EBNF notation).
- `CrossValidator` module: validates token/grammar cross-references.
- Full extended format support: skip, aliases, reserved, mode directives.
