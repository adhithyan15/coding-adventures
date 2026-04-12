defmodule CodingAdventures.FSharpLexer do
  @moduledoc """
  F# lexer backed by the shared grammar-driven lexer engine.

  The lexer reads `fsharp<version>.tokens` from `code/grammars/fsharp/`, parses
  the file into a `TokenGrammar`, caches the parsed grammar in `:persistent_term`,
  and delegates tokenization to `CodingAdventures.Lexer.GrammarLexer`.

  ## Version Support

  The lexer supports the F# release line from 1.0 through 10:

  | Version string | Grammar file                      |
  |----------------|-----------------------------------|
  | `"1.0"`        | `grammars/fsharp/fsharp1.0.tokens` |
  | `"2.0"`        | `grammars/fsharp/fsharp2.0.tokens` |
  | `"3.0"`        | `grammars/fsharp/fsharp3.0.tokens` |
  | `"3.1"`        | `grammars/fsharp/fsharp3.1.tokens` |
  | `"4.0"`        | `grammars/fsharp/fsharp4.0.tokens` |
  | `"4.1"`        | `grammars/fsharp/fsharp4.1.tokens` |
  | `"4.5"`        | `grammars/fsharp/fsharp4.5.tokens` |
  | `"4.6"`        | `grammars/fsharp/fsharp4.6.tokens` |
  | `"4.7"`        | `grammars/fsharp/fsharp4.7.tokens` |
  | `"5"`          | `grammars/fsharp/fsharp5.tokens`   |
  | `"6"`          | `grammars/fsharp/fsharp6.tokens`   |
  | `"7"`          | `grammars/fsharp/fsharp7.tokens`   |
  | `"8"`          | `grammars/fsharp/fsharp8.tokens`   |
  | `"9"`          | `grammars/fsharp/fsharp9.tokens`   |
  | `"10"`         | `grammars/fsharp/fsharp10.tokens`  |
  | `nil` / `""`   | `grammars/fsharp/fsharp10.tokens` (default) |

  ## Usage

      {:ok, tokens} = CodingAdventures.FSharpLexer.tokenize("let value = 1")
      {:ok, tokens} = CodingAdventures.FSharpLexer.tokenize("let value = 1", "4.0")
      grammar = CodingAdventures.FSharpLexer.create_lexer("1.0")

  The returned grammar can be reused directly with `GrammarLexer.tokenize/2`
  if you want to drive the shared lexer engine yourself.
  """

  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer

  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  @default_version "10"
  @valid_versions ~w(1.0 2.0 3.0 3.1 4.0 4.1 4.5 4.6 4.7 5 6 7 8 9 10)

  @doc """
  Return the default F# version used when no version is specified.
  """
  @spec default_version() :: String.t()
  def default_version, do: @default_version

  @doc """
  Return the list of supported F# version strings.
  """
  @spec supported_versions() :: [String.t()]
  def supported_versions, do: @valid_versions

  @doc """
  Tokenize F# source code and return `{:ok, tokens}` on success.

  ## Parameters

  - `source` -- F# source code as a string.
  - `version` -- Optional F# version string. Must be one of:
    `"1.0"`, `"2.0"`, `"3.0"`, `"3.1"`, `"4.0"`, `"4.1"`, `"4.5"`,
    `"4.6"`, `"4.7"`, `"5"`, `"6"`, `"7"`, `"8"`, `"9"`, `"10"`.
    Pass `nil` (default) to use the latest grammar (F# 10).

  ## Returns

  `{:ok, tokens}` or `{:error, message}`. Each token is a
  `%CodingAdventures.Lexer.Token{}` struct and the list always ends with EOF.

  ## Examples

      iex> {:ok, tokens} = CodingAdventures.FSharpLexer.tokenize("let value = 1")
      iex> Enum.map(tokens, & &1.type)
      ["KEYWORD", "NAME", "EQUALS", "NUMBER", "EOF"]

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised F# version identifier.
  """
  @spec tokenize(String.t(), String.t() | nil) ::
          {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source, version \\ nil) when is_binary(source) do
    grammar = get_grammar(resolve_version(version))
    GrammarLexer.tokenize(source, grammar)
  end

  @doc """
  Parse and return the `TokenGrammar` for the requested F# version.

  ## Parameters

  - `version` -- Optional F# version string (same values as `tokenize/2`).

  ## Returns

  A `TokenGrammar` struct cached per version.

  ## Examples

      iex> grammar = CodingAdventures.FSharpLexer.create_lexer()
      iex> is_map(grammar)
      true
      iex> Enum.member?(grammar.keywords, "let")
      true

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised F# version identifier.
  """
  @spec create_lexer(String.t() | nil) :: TokenGrammar.t()
  def create_lexer(version \\ nil) do
    get_grammar(resolve_version(version))
  end

  defp resolve_version(nil), do: @default_version
  defp resolve_version(""), do: @default_version

  defp resolve_version(version) when is_binary(version) do
    unless version in @valid_versions do
      raise ArgumentError,
            "Unknown F# version #{inspect(version)}. " <>
              "Valid values: #{Enum.join(@valid_versions, ", ")}"
    end

    version
  end

  defp resolve_version(version) do
    raise ArgumentError,
          "Unknown F# version #{inspect(version)}. " <>
            "Valid values: #{Enum.join(@valid_versions, ", ")}"
  end

  defp get_grammar(version) do
    case :persistent_term.get({__MODULE__, :grammar, version}, nil) do
      nil ->
        tokens_path = Path.join([@grammars_dir, "fsharp", "fsharp#{version}.tokens"])
        {:ok, grammar} = TokenGrammar.parse(File.read!(tokens_path))
        :persistent_term.put({__MODULE__, :grammar, version}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
