defmodule CodingAdventures.NibParser do
  @moduledoc """
  Nib parser built on the shared grammar-driven parser engine.
  """

  alias CodingAdventures.GrammarTools.ParserGrammar
  alias CodingAdventures.NibLexer
  alias CodingAdventures.Parser.{GrammarParser, ASTNode}

  @default_grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "grammars"])
                        |> Path.expand()

  @spec parse_nib(String.t()) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def parse_nib(source) do
    grammar = get_grammar()

    case NibLexer.tokenize_nib(source) do
      {:ok, tokens} -> GrammarParser.parse(tokens, grammar)
      {:error, msg} -> {:error, msg}
    end
  end

  @spec create_nib_parser(String.t() | nil) ::
          {:ok, ParserGrammar.t()} | {:error, String.t()}
  def create_nib_parser(grammars_dir \\ nil) do
    dir = grammars_dir || @default_grammars_dir
    grammar_path = Path.join(dir, "nib.grammar")

    case File.read(grammar_path) do
      {:ok, text} ->
        ParserGrammar.parse(text)

      {:error, reason} ->
        {:error, "Cannot read nib.grammar: #{:file.format_error(reason)}"}
    end
  end

  defp get_grammar do
    case :persistent_term.get({__MODULE__, :grammar}, nil) do
      nil ->
        {:ok, grammar} = create_nib_parser()
        :persistent_term.put({__MODULE__, :grammar}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
