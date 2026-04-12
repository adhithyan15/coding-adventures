defmodule CodingAdventures.FSharpParser do
  @moduledoc """
  F# parser backed by the shared grammar-driven parser engine.

  The parser reads `fsharp<version>.grammar` from `code/grammars/fsharp/`,
  parses the file into a `ParserGrammar`, caches the parsed grammar in
  `:persistent_term`, tokenizes F# source with `CodingAdventures.FSharpLexer`,
  and delegates AST construction to `CodingAdventures.Parser.GrammarParser`.

  ## Version Support

  This parser accepts the same version strings as `CodingAdventures.FSharpLexer`:

  | Version string | Grammar files                                   |
  |----------------|-------------------------------------------------|
  | `"1.0"`        | `grammars/fsharp/fsharp1.0.{tokens,grammar}`    |
  | `"2.0"`        | `grammars/fsharp/fsharp2.0.{tokens,grammar}`    |
  | `"3.0"`        | `grammars/fsharp/fsharp3.0.{tokens,grammar}`    |
  | `"3.1"`        | `grammars/fsharp/fsharp3.1.{tokens,grammar}`    |
  | `"4.0"`        | `grammars/fsharp/fsharp4.0.{tokens,grammar}`    |
  | `"4.1"`        | `grammars/fsharp/fsharp4.1.{tokens,grammar}`    |
  | `"4.5"`        | `grammars/fsharp/fsharp4.5.{tokens,grammar}`    |
  | `"4.6"`        | `grammars/fsharp/fsharp4.6.{tokens,grammar}`    |
  | `"4.7"`        | `grammars/fsharp/fsharp4.7.{tokens,grammar}`    |
  | `"5"`          | `grammars/fsharp/fsharp5.{tokens,grammar}`      |
  | `"6"`          | `grammars/fsharp/fsharp6.{tokens,grammar}`      |
  | `"7"`          | `grammars/fsharp/fsharp7.{tokens,grammar}`      |
  | `"8"`          | `grammars/fsharp/fsharp8.{tokens,grammar}`      |
  | `"9"`          | `grammars/fsharp/fsharp9.{tokens,grammar}`      |
  | `"10"`         | `grammars/fsharp/fsharp10.{tokens,grammar}`     |
  | `nil` / `""`   | `grammars/fsharp/fsharp10.{tokens,grammar}` (default) |

  ## Usage

      {:ok, ast} = CodingAdventures.FSharpParser.parse("let value = 1")
      {:ok, ast} = CodingAdventures.FSharpParser.parse("let value = 1", "4.0")
      grammar = CodingAdventures.FSharpParser.create_parser("1.0")
  """

  alias CodingAdventures.GrammarTools.ParserGrammar
  alias CodingAdventures.FSharpLexer
  alias CodingAdventures.Parser.{GrammarParser, ASTNode}

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
  Parse F# source code and return `{:ok, ast}` on success.

  ## Parameters

  - `source` -- F# source code as a string.
  - `version` -- Optional F# version string. Must be one of the supported
    release identifiers.

  ## Returns

  `{:ok, ast}` or `{:error, message}`. The root AST node has `rule_name`
  `"compilation_unit"`.

  ## Examples

      iex> {:ok, ast} = CodingAdventures.FSharpParser.parse("let value = 1")
      iex> ast.rule_name
      "compilation_unit"

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised F# version identifier.
  """
  @spec parse(String.t(), String.t() | nil) ::
          {:ok, ASTNode.t()} | {:error, String.t()}
  def parse(source, version \\ nil) when is_binary(source) do
    version = resolve_version(version)

    case FSharpLexer.tokenize(source, version) do
      {:ok, tokens} ->
        grammar = get_grammar(version)
        GrammarParser.parse(tokens, grammar)

      {:error, msg} ->
        {:error, msg}
    end
  end

  @doc """
  Parse and return the `ParserGrammar` for the requested F# version.
  """
  @spec create_parser(String.t() | nil) :: ParserGrammar.t()
  def create_parser(version \\ nil) do
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
        grammar_path = Path.join([@grammars_dir, "fsharp", "fsharp#{version}.grammar"])
        {:ok, grammar} = ParserGrammar.parse(File.read!(grammar_path))
        :persistent_term.put({__MODULE__, :grammar, version}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
