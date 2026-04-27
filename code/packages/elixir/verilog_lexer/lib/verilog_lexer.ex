defmodule CodingAdventures.VerilogLexer do
  @moduledoc """
  Verilog lexer backed by compiled, versioned grammars.
  """

  alias CodingAdventures.Lexer.GrammarLexer
  alias CodingAdventures.VerilogLexer.Grammar.{V1995, V2001, V2005}
  alias CodingAdventures.VerilogLexer.Preprocessor

  @default_version "2005"
  @supported_versions ~w(1995 2001 2005)
  @token_grammars %{
    "1995" => &V1995.token_grammar/0,
    "2001" => &V2001.token_grammar/0,
    "2005" => &V2005.token_grammar/0
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

  @spec tokenize(String.t(), keyword()) ::
          {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source, opts \\ []) do
    version = resolve_version!(Keyword.get(opts, :version))

    processed_source =
      if Keyword.get(opts, :preprocess, false) do
        Preprocessor.process(source)
      else
        source
      end

    GrammarLexer.tokenize(processed_source, create_lexer(version))
  end

  @spec create_lexer(String.t() | nil) :: CodingAdventures.GrammarTools.TokenGrammar.t()
  def create_lexer(version \\ nil) do
    resolved = resolve_version!(version)

    case :persistent_term.get({__MODULE__, :grammar, resolved}, nil) do
      nil ->
        grammar = Map.fetch!(@token_grammars, resolved).()
        :persistent_term.put({__MODULE__, :grammar, resolved}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
