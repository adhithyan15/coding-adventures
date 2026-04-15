defmodule CodingAdventures.VhdlLexer do
  @moduledoc """
  VHDL lexer backed by compiled, versioned grammars.
  """

  alias CodingAdventures.Lexer.GrammarLexer
  alias CodingAdventures.VhdlLexer.Grammar.{V1987, V1993, V2002, V2008, V2019}

  @default_version "2008"
  @supported_versions ~w(1987 1993 2002 2008 2019)
  @token_grammars %{
    "1987" => &V1987.token_grammar/0,
    "1993" => &V1993.token_grammar/0,
    "2002" => &V2002.token_grammar/0,
    "2008" => &V2008.token_grammar/0,
    "2019" => &V2019.token_grammar/0
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

  @spec tokenize(String.t(), keyword()) ::
          {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source, opts \\ []) do
    version = resolve_version!(Keyword.get(opts, :version))
    grammar = create_lexer(version)

    case GrammarLexer.tokenize(source, grammar) do
      {:ok, tokens} ->
        keyword_set = MapSet.new(grammar.keywords)
        {:ok, Enum.map(tokens, &normalize_case(&1, keyword_set))}

      {:error, _msg} = err ->
        err
    end
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

  defp normalize_case(%{type: "NAME"} = token, keyword_set) do
    downcased = String.downcase(token.value)

    if MapSet.member?(keyword_set, downcased) do
      %{token | type: "KEYWORD", value: downcased}
    else
      %{token | value: downcased}
    end
  end

  defp normalize_case(%{type: "KEYWORD"} = token, _keyword_set) do
    %{token | value: String.downcase(token.value)}
  end

  defp normalize_case(token, _keyword_set), do: token
end
