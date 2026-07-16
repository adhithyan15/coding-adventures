defmodule CodingAdventures.PythonParser do
  @moduledoc """
  Python parser backed by the shared grammar-driven parser engine.

  The parser selects a versioned lexer grammar, tokenizes source with
  `CodingAdventures.PythonLexer`, and delegates AST construction to
  `CodingAdventures.Parser.GrammarParser` using the shared Python subset
  grammar.

  Python 3.12 is used by default. Supported versions are 2.7, 3.0, 3.6, 3.8,
  3.10, and 3.12.
  """

  alias CodingAdventures.GrammarTools.ParserGrammar
  alias CodingAdventures.Parser.{ASTNode, GrammarParser}
  alias CodingAdventures.PythonLexer

  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  @default_version "3.12"
  @supported_versions ["2.7", "3.0", "3.6", "3.8", "3.10", "3.12"]

  @doc """
  The default Python version used when no version is specified.
  """
  @spec default_version() :: String.t()
  def default_version, do: @default_version

  @doc """
  The Python versions with matching lexer grammars.
  """
  @spec supported_versions() :: [String.t()]
  def supported_versions, do: @supported_versions

  @doc """
  Parse Python source code with the grammar for `version`.

  A `nil` or empty version selects Python 3.12.
  """
  @spec parse(String.t(), String.t() | nil) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def parse(source, version \\ nil) when is_binary(source) do
    resolved = resolve_version!(version)

    with {:ok, tokens} <- PythonLexer.tokenize(source, resolved) do
      GrammarParser.parse(tokens, create_parser())
    end
  end

  @doc """
  Return the cached shared Python `ParserGrammar`.
  """
  @spec create_parser() :: ParserGrammar.t()
  def create_parser do
    case :persistent_term.get({__MODULE__, :grammar}, nil) do
      nil ->
        grammar_path = Path.join([@grammars_dir, "python", "python.grammar"])
        {:ok, grammar} = ParserGrammar.parse(File.read!(grammar_path))
        :persistent_term.put({__MODULE__, :grammar}, grammar)
        grammar

      grammar ->
        grammar
    end
  end

  defp resolve_version!(nil), do: @default_version
  defp resolve_version!(""), do: @default_version

  defp resolve_version!(version) when version in @supported_versions, do: version

  defp resolve_version!(version) do
    raise ArgumentError,
          "Unknown Python version #{inspect(version)}. " <>
            "Valid values: #{Enum.join(@supported_versions, ", ")}"
  end
end
