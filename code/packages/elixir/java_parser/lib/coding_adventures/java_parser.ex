defmodule CodingAdventures.JavaParser do
  @moduledoc """
  Java parser backed by the shared grammar-driven parser engine.

  The parser reads `java<version>.grammar` from `code/grammars/java/`, parses
  the file into a `ParserGrammar`, caches the parsed grammar in
  `:persistent_term`, tokenizes Java source with `CodingAdventures.JavaLexer`,
  and delegates AST construction to `CodingAdventures.Parser.GrammarParser`.

  ## Version Support

  This parser accepts the same version strings as `CodingAdventures.JavaLexer`:

  | Version string  | Grammar files                                        |
  |-----------------|------------------------------------------------------|
  | `"1.0"`         | `grammars/java/java1.0.{tokens,grammar}`             |
  | `"1.1"`         | `grammars/java/java1.1.{tokens,grammar}`             |
  | `"1.4"`         | `grammars/java/java1.4.{tokens,grammar}`             |
  | `"5"`           | `grammars/java/java5.{tokens,grammar}`               |
  | `"7"`           | `grammars/java/java7.{tokens,grammar}`               |
  | `"8"`           | `grammars/java/java8.{tokens,grammar}`               |
  | `"10"`          | `grammars/java/java10.{tokens,grammar}`              |
  | `"14"`          | `grammars/java/java14.{tokens,grammar}`              |
  | `"17"`          | `grammars/java/java17.{tokens,grammar}`              |
  | `"21"`          | `grammars/java/java21.{tokens,grammar}`              |
  | `nil` / `""`    | `grammars/java/java21.{tokens,grammar}` (default)    |

  ## Usage

      {:ok, ast} = CodingAdventures.JavaParser.parse("public class Hello { }")
      {:ok, ast} = CodingAdventures.JavaParser.parse("public class Hello { }", "8")
      grammar = CodingAdventures.JavaParser.create_parser("1.0")
  """

  alias CodingAdventures.GrammarTools.ParserGrammar
  alias CodingAdventures.JavaLexer
  alias CodingAdventures.Parser.{GrammarParser, ASTNode}

  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  @default_version "21"
  @valid_versions ~w(1.0 1.1 1.4 5 7 8 10 14 17 21)

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
  Parse Java source code and return `{:ok, ast}` on success.

  ## Parameters

  - `source` -- Java source code as a string.
  - `version` -- Optional Java version string. Must be one of:
    `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"`.
    Pass `nil` (default) to use the default grammar (Java 21).

  ## Returns

  `{:ok, ast}` or `{:error, message}`. The root AST node has `rule_name`
  `"program"`.

  ## Examples

      iex> {:ok, ast} = CodingAdventures.JavaParser.parse("public class Hello { }")
      iex> ast.rule_name
      "program"

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised Java version identifier.
  """
  @spec parse(String.t(), String.t() | nil) ::
          {:ok, ASTNode.t()} | {:error, String.t()}
  def parse(source, version \\ nil) when is_binary(source) do
    version = resolve_version(version)

    case JavaLexer.tokenize(source, version) do
      {:ok, tokens} ->
        grammar = get_grammar(version)
        GrammarParser.parse(tokens, grammar)

      {:error, msg} ->
        {:error, msg}
    end
  end

  @doc """
  Parse and return the `ParserGrammar` for the requested Java version.

  ## Parameters

  - `version` -- Optional Java version string (same values as `parse/2`).

  ## Returns

  A `ParserGrammar` struct cached per version.

  ## Examples

      iex> grammar = CodingAdventures.JavaParser.create_parser()
      iex> is_map(grammar)
      true
      iex> grammar.rules |> hd() |> Map.fetch!(:name)
      "program"

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised Java version identifier.
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
        grammar_path = Path.join([@grammars_dir, "java", "java#{version}.grammar"])
        {:ok, grammar} = ParserGrammar.parse(File.read!(grammar_path))
        :persistent_term.put({__MODULE__, :grammar, version}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
