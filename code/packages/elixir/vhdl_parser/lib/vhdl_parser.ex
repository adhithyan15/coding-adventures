defmodule CodingAdventures.VhdlParser do
  @moduledoc """
  VHDL parser backed by compiled, versioned grammars.
  """

  alias CodingAdventures.Parser.{ASTNode, GrammarParser}
  alias CodingAdventures.VhdlLexer
  alias CodingAdventures.VhdlParser.Grammar.{V1987, V1993, V2002, V2008, V2019}

  @default_version "2008"
  @supported_versions ~w(1987 1993 2002 2008 2019)
  @parser_grammars %{
    "1987" => &V1987.parser_grammar/0,
    "1993" => &V1993.parser_grammar/0,
    "2002" => &V2002.parser_grammar/0,
    "2008" => &V2008.parser_grammar/0,
    "2019" => &V2019.parser_grammar/0
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
          "Unknown VHDL version #{inspect(version)}. " <>
            "Valid values: #{Enum.join(@supported_versions, ", ")}"
  end

  @spec parse(String.t(), keyword()) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def parse(source, opts \\ []) do
    version = resolve_version!(Keyword.get(opts, :version))
    parser_opts = Keyword.drop(opts, [:version])

    with {:ok, tokens} <- VhdlLexer.tokenize(source, version: version) do
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
