defmodule CodingAdventures.JavaLexer do
  @moduledoc """
  Java lexer backed by the shared grammar-driven lexer engine.

  The lexer reads `java<version>.tokens` from `code/grammars/java/`, parses the
  file into a `TokenGrammar`, caches the parsed grammar in `:persistent_term`,
  and delegates tokenization to `CodingAdventures.Lexer.GrammarLexer`.

  ## Version Support

  Java has evolved significantly since version 1.0 (1996). Pass an optional
  `version` argument to select a version-specific token grammar:

  | Version string  | Grammar file                              |
  |-----------------|-------------------------------------------|
  | `"1.0"`         | `grammars/java/java1.0.tokens`            |
  | `"1.1"`         | `grammars/java/java1.1.tokens`            |
  | `"1.4"`         | `grammars/java/java1.4.tokens`            |
  | `"5"`           | `grammars/java/java5.tokens`              |
  | `"7"`           | `grammars/java/java7.tokens`              |
  | `"8"`           | `grammars/java/java8.tokens`              |
  | `"10"`          | `grammars/java/java10.tokens`             |
  | `"14"`          | `grammars/java/java14.tokens`             |
  | `"17"`          | `grammars/java/java17.tokens`             |
  | `"21"`          | `grammars/java/java21.tokens`             |
  | `nil` / `""`    | `grammars/java/java21.tokens` (default)   |

  ## Usage

      {:ok, tokens} = CodingAdventures.JavaLexer.tokenize("public class Hello { }")
      {:ok, tokens} = CodingAdventures.JavaLexer.tokenize("public class Hello { }", "8")
      grammar = CodingAdventures.JavaLexer.create_lexer("1.0")

  The returned grammar can be reused directly with `GrammarLexer.tokenize/2` if
  you want to drive the shared lexer engine yourself.
  """

  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer

  alias CodingAdventures.JavaLexer.Grammar.{V1_0, V1_1, V1_4, V5, V7, V8, V10, V14, V17, V21}

  @default_version "21"
  @valid_versions ~w(1.0 1.1 1.4 5 7 8 10 14 17 21)

  @token_grammars %{
    "1.0" => &V1_0.token_grammar/0,
    "1.1" => &V1_1.token_grammar/0,
    "1.4" => &V1_4.token_grammar/0,
    "5" => &V5.token_grammar/0,
    "7" => &V7.token_grammar/0,
    "8" => &V8.token_grammar/0,
    "10" => &V10.token_grammar/0,
    "14" => &V14.token_grammar/0,
    "17" => &V17.token_grammar/0,
    "21" => &V21.token_grammar/0
  }

  @doc """
  Return the default Java version used when no version is specified.
  """
  @spec default_version() :: String.t()
  def default_version, do: @default_version

  @doc """
  Return the list of supported Java version strings.
  """
  @spec supported_versions() :: [String.t()]
  def supported_versions, do: @valid_versions

  @doc """
  Tokenize Java source code and return `{:ok, tokens}` on success.

  ## Parameters

  - `source` -- Java source code as a string.
  - `version` -- Optional Java version string. Must be one of:
    `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"`.
    Pass `nil` (default) to use the default grammar (Java 21).

  ## Returns

  `{:ok, tokens}` or `{:error, message}`. Each token is a
  `%CodingAdventures.Lexer.Token{}` struct and the list always ends with EOF.

  ## Examples

      iex> {:ok, tokens} = CodingAdventures.JavaLexer.tokenize("public class Hello { }")
      iex> Enum.map(tokens, & &1.type)
      ["KEYWORD", "KEYWORD", "NAME", "LBRACE", "RBRACE", "EOF"]

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised Java version identifier.
  """
  @spec tokenize(String.t(), String.t() | nil) ::
          {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source, version \\ nil) when is_binary(source) do
    grammar = get_grammar(resolve_version(version))
    GrammarLexer.tokenize(source, grammar)
  end

  @doc """
  Parse and return the `TokenGrammar` for the requested Java version.

  ## Parameters

  - `version` -- Optional Java version string (same values as `tokenize/2`).

  ## Returns

  A `TokenGrammar` struct cached per version.

  ## Examples

      iex> grammar = CodingAdventures.JavaLexer.create_lexer()
      iex> is_map(grammar)
      true
      iex> Enum.member?(grammar.keywords, "class")
      true

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised Java version identifier.
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
            "Unknown Java version #{inspect(version)}. " <>
              "Valid values: #{Enum.join(@valid_versions, ", ")}"
    end

    version
  end

  defp resolve_version(version) do
    raise ArgumentError,
          "Unknown Java version #{inspect(version)}. " <>
            "Valid values: #{Enum.join(@valid_versions, ", ")}"
  end

  defp get_grammar(version) do
    case :persistent_term.get({__MODULE__, :grammar, version}, nil) do
      nil ->
        grammar = Map.fetch!(@token_grammars, version).()
        :persistent_term.put({__MODULE__, :grammar, version}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
