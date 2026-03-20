defmodule CodingAdventures.Lexer do
  @moduledoc """
  Grammar-driven lexer engine for Elixir.

  Provides `GrammarLexer.tokenize/2` which takes source code and a
  `TokenGrammar` and produces a list of `Token` structs, exactly like
  the Python `GrammarLexer` class.
  """

  defdelegate tokenize(source, grammar), to: CodingAdventures.Lexer.GrammarLexer
end
