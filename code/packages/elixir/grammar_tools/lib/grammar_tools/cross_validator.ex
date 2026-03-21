defmodule CodingAdventures.GrammarTools.CrossValidator do
  @moduledoc """
  Validates cross-references between `.tokens` and `.grammar` files.

  Checks that every UPPERCASE token referenced in the grammar exists in the
  token definitions, and reports tokens that are defined but never used.
  """

  alias CodingAdventures.GrammarTools.{TokenGrammar, ParserGrammar}

  @doc """
  Cross-validate a token grammar against a parser grammar.

  Returns a list of issue strings. An empty list means no problems found.
  """
  @spec validate(TokenGrammar.t(), ParserGrammar.t()) :: [String.t()]
  def validate(%TokenGrammar{} = token_grammar, %ParserGrammar{} = parser_grammar) do
    token_names = TokenGrammar.token_names(token_grammar)
    referenced_tokens = ParserGrammar.token_references(parser_grammar)

    # Check for undefined token references
    undefined =
      referenced_tokens
      |> MapSet.difference(token_names)
      |> Enum.sort()
      |> Enum.map(&"Undefined token reference: '#{&1}'")

    # Check for unused token definitions
    unused =
      token_names
      |> MapSet.difference(referenced_tokens)
      |> Enum.sort()
      |> Enum.map(&"Token '#{&1}' is defined but never referenced in the grammar")

    undefined ++ unused
  end
end
