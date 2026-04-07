defmodule CodingAdventures.PythonLexer do
  @moduledoc """
  Python lexer backed by the shared grammar-driven lexer engine.

  This package loads versioned Python grammar files from the repository's
  shared grammars directory, parses them into `TokenGrammar` structs, and
  delegates tokenization to `CodingAdventures.Lexer.GrammarLexer`.

  Supports Python versions 2.7, 3.0, 3.6, 3.8, 3.10, and 3.12. Each version
  has its own `.tokens` grammar file at `code/grammars/python/pythonX.Y.tokens`.
  Parsed grammars are cached per-version using `:persistent_term`.

  ## Usage

      {:ok, tokens} = PythonLexer.tokenize("x = 1", "3.12")
      {:ok, tokens} = PythonLexer.tokenize("x = 1")          # defaults to 3.12
  """

  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer

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
  The list of supported Python version strings.
  """
  @spec supported_versions() :: [String.t()]
  def supported_versions, do: @supported_versions

  @doc """
  Tokenize Python source code using the grammar for the given version.

  If `version` is `nil` or `""`, defaults to `"3.12"`.

  Returns `{:ok, tokens}` on success, `{:error, message}` on failure.
  """
  @spec tokenize(String.t(), String.t() | nil) :: {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source, version \\ nil) do
    grammar = get_grammar(resolve_version(version))
    GrammarLexer.tokenize(source, grammar)
  end

  @doc """
  Return the parsed TokenGrammar for the given Python version.

  If `version` is `nil` or `""`, defaults to `"3.12"`.
  """
  @spec create_lexer(String.t() | nil) :: TokenGrammar.t()
  def create_lexer(version \\ nil) do
    v = resolve_version(version)
    tokens_path = grammar_path(v)
    {:ok, grammar} = TokenGrammar.parse(File.read!(tokens_path))
    grammar
  end

  defp resolve_version(nil), do: @default_version
  defp resolve_version(""), do: @default_version
  defp resolve_version(v), do: v

  defp grammar_path(version) do
    Path.join([@grammars_dir, "python", "python#{version}.tokens"])
  end

  defp get_grammar(version) do
    case :persistent_term.get({__MODULE__, :grammar, version}, nil) do
      nil ->
        grammar = create_lexer(version)
        :persistent_term.put({__MODULE__, :grammar, version}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
