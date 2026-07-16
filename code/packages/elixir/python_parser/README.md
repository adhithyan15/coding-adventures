# Python Parser (Elixir)

Grammar-driven Python parser for Elixir.

This package tokenizes source with `CodingAdventures.PythonLexer`, loads the shared parser grammar from `code/grammars/python/python.grammar`, and delegates AST construction to `CodingAdventures.Parser.GrammarParser`. The optional version selects the matching lexer grammar; the parser grammar covers the common Python expression and assignment subset used by the other thin-wrapper ports.

```elixir
{:ok, ast} = CodingAdventures.PythonParser.parse("x = 1 + 2\n")
{:ok, ast_27} = CodingAdventures.PythonParser.parse("x = 42\n", "2.7")
```

Python 3.12 is the default. Supported versions are 2.7, 3.0, 3.6, 3.8, 3.10, and 3.12.
