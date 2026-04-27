defmodule CodingAdventures.VerilogParser do
  @moduledoc """
  Verilog parser backed by compiled, versioned grammars.
  """

  alias CodingAdventures.Parser.{ASTNode, GrammarParser}
  alias CodingAdventures.VerilogLexer
  alias CodingAdventures.VerilogParser.Grammar.{V1995, V2001, V2005}

  @default_version "2005"
  @supported_versions ~w(1995 2001 2005)
  @parser_grammars %{
    "1995" => &V1995.parser_grammar/0,
    "2001" => &V2001.parser_grammar/0,
    "2005" => &V2005.parser_grammar/0
  }

  @spec default_version() :: String.t()
  def default_version, do: @default_version

  @spec supported_versions() :: [String.t()]
  def supported_versions, do: @supported_versions

  @spec resolve_version!(String.t() | nil) :: String.t()
  def resolve_version!(version \\ nil)

  def resolve_version!(nil), do: @default_version
  def resolve_version!(""), do: @default_version

  def resolve_version!(version) when version in @supported_versions, do: version

  def resolve_version!(version) do
    raise ArgumentError,
          "Unknown Verilog version #{inspect(version)}. " <>
            "Valid values: #{Enum.join(@supported_versions, ", ")}"
  end

  @spec parse(String.t(), keyword()) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def parse(source, opts \\ []) do
    version = resolve_version!(Keyword.get(opts, :version))
    preprocess = Keyword.get(opts, :preprocess, false)
    parser_opts = Keyword.drop(opts, [:version, :preprocess])

    with {:ok, tokens} <- VerilogLexer.tokenize(source, version: version, preprocess: preprocess) do
      GrammarParser.parse(tokens, create_parser(version), parser_opts)
    end
  end

  @spec create_parser(String.t() | nil) :: CodingAdventures.GrammarTools.ParserGrammar.t()
  def create_parser(version \\ nil) do
    resolved = resolve_version!(version)

    case :persistent_term.get({__MODULE__, :grammar, resolved}, nil) do
      nil ->
        grammar = Map.fetch!(@parser_grammars, resolved).()
        :persistent_term.put({__MODULE__, :grammar, resolved}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
