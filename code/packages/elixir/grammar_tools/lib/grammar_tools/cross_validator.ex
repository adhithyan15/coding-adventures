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

    # Build the set of implicit tokens that are always available.
    # EOF is always implicitly available (every token stream ends with it).
    # NEWLINE is always a valid synthetic token — the lexer emits it
    # whenever a bare newline is encountered and no skip pattern consumed it.
    implicit = MapSet.new(["EOF", "NEWLINE"])

    # In indentation mode, INDENT/DEDENT are also synthesized by the lexer.
    implicit =
      if token_grammar.mode == "indentation" do
        implicit |> MapSet.put("INDENT") |> MapSet.put("DEDENT")
      else
        implicit
      end

    # Combine defined tokens with implicit ones for reference checking.
    all_valid = MapSet.union(token_names, implicit)

    # Check for undefined token references
    undefined =
      referenced_tokens
      |> MapSet.difference(all_valid)
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
