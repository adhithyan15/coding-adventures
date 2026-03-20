defmodule CodingAdventures.Parser do
  @moduledoc """
  Grammar-driven parser engine for Elixir.

  Provides `GrammarParser.parse/2` which takes a list of tokens and a
  `ParserGrammar` and produces a generic AST tree.
  """

  defdelegate parse(tokens, grammar), to: CodingAdventures.Parser.GrammarParser
end
