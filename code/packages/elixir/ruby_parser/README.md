# Ruby Parser (Elixir)

Grammar-driven Ruby parser for Elixir.

This package tokenizes source with `CodingAdventures.RubyLexer`, loads `code/grammars/ruby.grammar`, and delegates parsing to `CodingAdventures.Parser.GrammarParser`.

```elixir
{:ok, ast} = CodingAdventures.RubyParser.parse("x = 1 + 2")
```
