# CSS Parser (Elixir)

Grammar-driven CSS parser for Elixir.

This package tokenizes source with `CodingAdventures.CssLexer`, loads `code/grammars/css.grammar`, and delegates parsing to `CodingAdventures.Parser.GrammarParser`.

```elixir
{:ok, ast} = CodingAdventures.CssParser.parse("h1 { color: red; }")
```
