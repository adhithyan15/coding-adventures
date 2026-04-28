# Lisp Parser (Elixir)

Grammar-driven Lisp parser for Elixir.

This package tokenizes source with `CodingAdventures.LispLexer`, loads `code/grammars/lisp.grammar`, and delegates parsing to `CodingAdventures.Parser.GrammarParser`.

```elixir
{:ok, ast} = CodingAdventures.LispParser.parse("(+ 1 2)")
```
