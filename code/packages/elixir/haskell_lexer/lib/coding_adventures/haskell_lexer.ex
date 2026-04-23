defmodule CodingAdventures.HaskellLexer do
  @moduledoc """
  Haskell lexer backed by the shared grammar-driven lexer engine.

  The lexer reads `haskell<version>.tokens` from `code/grammars/haskell/`, parses the
  file into a `TokenGrammar`, caches the parsed grammar in `:persistent_term`,
  and delegates tokenization to `CodingAdventures.Lexer.GrammarLexer`.

  ## Version Support

  Haskell has evolved significantly since version 1.0 (1996). Pass an optional
  `version` argument to select a version-specific token grammar:

  | Version string  | Grammar file                              |
  |-----------------|-------------------------------------------|
  | `"1.0"`         | `grammars/haskell/haskell1.0.tokens`            |
  | `"1.1"`         | `grammars/haskell/haskell1.1.tokens`            |
  | `"1.4"`         | `grammars/haskell/haskell1.4.tokens`            |
  | `"5"`           | `grammars/haskell/haskell5.tokens`              |
  | `"7"`           | `grammars/haskell/haskell7.tokens`              |
  | `"8"`           | `grammars/haskell/haskell8.tokens`              |
  | `"10"`          | `grammars/haskell/haskell10.tokens`             |
  | `"14"`          | `grammars/haskell/haskell14.tokens`             |
  | `"17"`          | `grammars/haskell/haskell17.tokens`             |
  | `"21"`          | `grammars/haskell/haskell21.tokens`             |
  | `nil` / `""`    | `grammars/haskell/haskell21.tokens` (default)   |

  ## Usage

      {:ok, tokens} = CodingAdventures.HaskellLexer.tokenize("public class Hello { }")
      {:ok, tokens} = CodingAdventures.HaskellLexer.tokenize("public class Hello { }", "8")
      grammar = CodingAdventures.HaskellLexer.create_lexer("1.0")

  The returned grammar can be reused directly with `GrammarLexer.tokenize/2` if
  you want to drive the shared lexer engine yourself.
  """

  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer

  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  @default_version "2010"
  @valid_versions ~w(1.0 1.1 1.2 1.3 1.4 98 2010)

  @doc """
  Return the default Haskell version used when no version is specified.
  """
  @spec default_version() :: String.t()
  def default_version, do: @default_version

  @doc """
  Return the list of supported Haskell version strings.
  """
  @spec supported_versions() :: [String.t()]
  def supported_versions, do: @valid_versions

  @doc """
  Tokenize Haskell source code and return `{:ok, tokens}` on success.

  ## Parameters

  - `source` -- Haskell source code as a string.
  - `version` -- Optional Haskell version string. Must be one of:
    `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"`.
    Pass `nil` (default) to use the default grammar (Haskell 21).

  ## Returns

  `{:ok, tokens}` or `{:error, message}`. Each token is a
  `%CodingAdventures.Lexer.Token{}` struct and the list always ends with EOF.

  ## Examples

      iex> {:ok, tokens} = CodingAdventures.HaskellLexer.tokenize("public class Hello { }")
      iex> Enum.map(tokens, & &1.type)
      ["KEYWORD", "KEYWORD", "NAME", "LBRACE", "RBRACE", "EOF"]

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised Haskell version identifier.
  """
  @spec tokenize(String.t(), String.t() | nil) ::
          {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source, version \\ nil) when is_binary(source) do
    grammar = get_grammar(resolve_version(version))
    GrammarLexer.tokenize(source, grammar)
  end

  @doc """
  Parse and return the `TokenGrammar` for the requested Haskell version.

  ## Parameters

  - `version` -- Optional Haskell version string (same values as `tokenize/2`).

  ## Returns

  A `TokenGrammar` struct cached per version.

  ## Examples

      iex> grammar = CodingAdventures.HaskellLexer.create_lexer()
      iex> is_map(grammar)
      true
      iex> Enum.member?(grammar.keywords, "class")
      true

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised Haskell version identifier.
  """
  @spec create_lexer(String.t() | nil) :: TokenGrammar.t()
  def create_lexer(version \\ nil) do
    get_grammar(resolve_version(version))
  end

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  defp resolve_version(nil), do: @default_version
  defp resolve_version(""), do: @default_version

  defp resolve_version(version) when is_binary(version) do
    unless version in @valid_versions do
      raise ArgumentError,
            "Unknown Haskell version #{inspect(version)}. " <>
              "Valid values: #{Enum.join(@valid_versions, ", ")}"
    end

    version
  end

  defp resolve_version(version) do
    raise ArgumentError,
          "Unknown Haskell version #{inspect(version)}. " <>
            "Valid values: #{Enum.join(@valid_versions, ", ")}"
  end

  defp get_grammar(version) do
    case :persistent_term.get({__MODULE__, :grammar, version}, nil) do
      nil ->
        tokens_path = Path.join([@grammars_dir, "haskell", "haskell#{version}.tokens"])
        {:ok, grammar} = TokenGrammar.parse(File.read!(tokens_path))
        :persistent_term.put({__MODULE__, :grammar, version}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
