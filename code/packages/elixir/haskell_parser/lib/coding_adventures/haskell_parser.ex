defmodule CodingAdventures.HaskellParser do
  @moduledoc """
  Haskell parser backed by the shared grammar-driven parser engine.

  The parser reads `haskell<version>.grammar` from `code/grammars/haskell/`, parses
  the file into a `ParserGrammar`, caches the parsed grammar in
  `:persistent_term`, tokenizes Haskell source with `CodingAdventures.HaskellLexer`,
  and delegates AST construction to `CodingAdventures.Parser.GrammarParser`.

  ## Version Support

  This parser accepts the same version strings as `CodingAdventures.HaskellLexer`:

  | Version string  | Grammar files                                        |
  |-----------------|------------------------------------------------------|
  | `"1.0"`         | `grammars/haskell/haskell1.0.{tokens,grammar}`             |
  | `"1.1"`         | `grammars/haskell/haskell1.1.{tokens,grammar}`             |
  | `"1.4"`         | `grammars/haskell/haskell1.4.{tokens,grammar}`             |
  | `"5"`           | `grammars/haskell/haskell5.{tokens,grammar}`               |
  | `"7"`           | `grammars/haskell/haskell7.{tokens,grammar}`               |
  | `"8"`           | `grammars/haskell/haskell8.{tokens,grammar}`               |
  | `"10"`          | `grammars/haskell/haskell10.{tokens,grammar}`              |
  | `"14"`          | `grammars/haskell/haskell14.{tokens,grammar}`              |
  | `"17"`          | `grammars/haskell/haskell17.{tokens,grammar}`              |
  | `"21"`          | `grammars/haskell/haskell21.{tokens,grammar}`              |
  | `nil` / `""`    | `grammars/haskell/haskell21.{tokens,grammar}` (default)    |

  ## Usage

      {:ok, ast} = CodingAdventures.HaskellParser.parse("public class Hello { }")
      {:ok, ast} = CodingAdventures.HaskellParser.parse("public class Hello { }", "8")
      grammar = CodingAdventures.HaskellParser.create_parser("1.0")
  """

  alias CodingAdventures.GrammarTools.ParserGrammar
  alias CodingAdventures.HaskellLexer
  alias CodingAdventures.Parser.{GrammarParser, ASTNode}

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
  Parse Haskell source code and return `{:ok, ast}` on success.

  ## Parameters

  - `source` -- Haskell source code as a string.
  - `version` -- Optional Haskell version string. Must be one of:
    `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"`.
    Pass `nil` (default) to use the default grammar (Haskell 21).

  ## Returns

  `{:ok, ast}` or `{:error, message}`. The root AST node has `rule_name`
  `"program"`.

  ## Examples

      iex> {:ok, ast} = CodingAdventures.HaskellParser.parse("public class Hello { }")
      iex> ast.rule_name
      "program"

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised Haskell version identifier.
  """
  @spec parse(String.t(), String.t() | nil) ::
          {:ok, ASTNode.t()} | {:error, String.t()}
  def parse(source, version \\ nil) when is_binary(source) do
    version = resolve_version(version)

    case HaskellLexer.tokenize(source, version) do
      {:ok, tokens} ->
        grammar = get_grammar(version)
        GrammarParser.parse(tokens, grammar)

      {:error, msg} ->
        {:error, msg}
    end
  end

  @doc """
  Parse and return the `ParserGrammar` for the requested Haskell version.

  ## Parameters

  - `version` -- Optional Haskell version string (same values as `parse/2`).

  ## Returns

  A `ParserGrammar` struct cached per version.

  ## Examples

      iex> grammar = CodingAdventures.HaskellParser.create_parser()
      iex> is_map(grammar)
      true
      iex> grammar.rules |> hd() |> Map.fetch!(:name)
      "program"

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised Haskell version identifier.
  """
  @spec create_parser(String.t() | nil) :: ParserGrammar.t()
  def create_parser(version \\ nil) do
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
        grammar_path = Path.join([@grammars_dir, "haskell", "haskell#{version}.grammar"])
        {:ok, grammar} = ParserGrammar.parse(File.read!(grammar_path))
        :persistent_term.put({__MODULE__, :grammar, version}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
