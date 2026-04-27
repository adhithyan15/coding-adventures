# Haskell Parser (Go)

Grammar-driven Haskell parser for Go.

This package tokenizes Haskell source with `go/haskell-lexer`, loads the matching `code/grammars/haskell/haskell*.grammar` file, and delegates parsing to the shared Go `parser` package.

- `CreateHaskellParser(source, version)` returns a configured grammar parser.
- `ParseHaskell(source, version)` returns the parse tree root.
- `DefaultVersion` is `2010`.

Supported versions are `1.0`, `1.1`, `1.2`, `1.3`, `1.4`, `98`, and `2010`.
