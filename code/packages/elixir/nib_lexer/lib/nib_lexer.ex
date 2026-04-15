defmodule CodingAdventures.NibLexer do
  @moduledoc """
  Nib lexer backed by the shared grammar-driven lexer engine.
  """

  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer

  @default_grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "grammars"])
                        |> Path.expand()

  @spec create_nib_lexer(String.t() | nil) ::
          {:ok, TokenGrammar.t()} | {:error, String.t()}
  def create_nib_lexer(grammars_dir \\ nil) do
    dir = grammars_dir || @default_grammars_dir
    tokens_path = Path.join(dir, "nib.tokens")

    case File.read(tokens_path) do
      {:ok, text} ->
        TokenGrammar.parse(text)

      {:error, reason} ->
        {:error, "Cannot read nib.tokens: #{:file.format_error(reason)}"}
    end
  end

  @spec tokenize_nib(String.t()) ::
          {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize_nib(source) do
    grammar = get_grammar()
    GrammarLexer.tokenize(source, grammar)
  end

  defp get_grammar do
    case :persistent_term.get({__MODULE__, :grammar}, nil) do
      nil ->
        {:ok, grammar} = create_nib_lexer()
        :persistent_term.put({__MODULE__, :grammar}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
