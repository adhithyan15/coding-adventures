# Haskell Lexer (Go)

Grammar-driven Haskell lexer for Go.

This package loads one of the repository's `code/grammars/haskell/haskell*.tokens` files and delegates tokenization to the shared Go `lexer` package. It matches the public surface used by the Python and TypeScript Haskell lexer packages:

- `CreateHaskellLexer(source, version)` returns a configured grammar lexer.
- `TokenizeHaskell(source, version)` returns the complete token stream.
- `DefaultVersion` is `2010`.

Pass an empty version string to use Haskell 2010. Supported versions are `1.0`, `1.1`, `1.2`, `1.3`, `1.4`, `98`, and `2010`.
