defmodule CodingAdventures.GrammarTools.CrossValidator do
  @moduledoc """
  Validates cross-references between `.tokens` and `.grammar` files.

  Checks that every UPPERCASE token referenced in the grammar exists in the
  token definitions, and reports tokens that are defined but never used.

  Why cross-validate?
  -------------------

  Each file can be valid on its own but broken when used together:

  - A grammar might reference `SEMICOLON`, but the `.tokens` file only
    defines `SEMI`. Each file is fine individually, but the pair is broken.
  - A `.tokens` file might define `TILDE = "~"` that no grammar rule ever
    uses. This is not an error — it might be intentional — but it is worth
    warning about because unused tokens add complexity without value.

  This is analogous to how a C compiler checks that every function you call
  is actually declared (and vice versa, warns about unused functions).

  What We Check
  -------------

  1. **Missing token references**: Every UPPERCASE name in the grammar must
     correspond to a token definition.
  2. **Unused tokens**: Every token defined in `.tokens` should ideally be
     referenced somewhere in the grammar. Reported as warnings.
  3. **Synthetic tokens** (`NEWLINE`, `INDENT`, `DEDENT`, `EOF`) are always
     valid regardless of what is defined in the tokens file.
  """

  alias CodingAdventures.GrammarTools.{TokenGrammar, ParserGrammar}

  @doc """
  Cross-validate a token grammar against a parser grammar.

  Uses `TokenGrammar.token_names/1` to build the set of known token names
  (including both definition names and their aliases, plus group tokens).

  Returns a list of issue strings. An empty list means no problems found.
  Issues starting with "Warning:" are informational; all others are errors.
  """
  @spec validate(TokenGrammar.t(), ParserGrammar.t()) :: [String.t()]
  def validate(%TokenGrammar{} = token_grammar, %ParserGrammar{} = parser_grammar) do
    # Use the TokenGrammar helper that already handles aliases and groups.
    # This includes both definition names AND their aliases, so the grammar
    # can reference either form.
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

    # Check for unused token definitions.
    # A definition is "used" if:
    #   1. Its name is directly referenced in the grammar, OR
    #   2. Its alias is referenced (because the lexer emits the alias)
    unused =
      token_grammar.definitions
      |> Enum.reject(fn defn ->
        MapSet.member?(referenced_tokens, defn.name) or
          (defn.alias != nil and MapSet.member?(referenced_tokens, defn.alias))
      end)
      |> Enum.map(&"Token '#{&1.name}' is defined but never referenced in the grammar")

    undefined ++ unused
  end
end
