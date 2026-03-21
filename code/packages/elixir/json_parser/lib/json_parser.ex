defmodule CodingAdventures.JsonParser do
  @moduledoc """
  JSON Parser — Thin wrapper around the grammar-driven parser engine.

  This module combines `JsonLexer.tokenize/1` with `GrammarParser.parse/2`
  to parse JSON source code into an AST. It reads `json.grammar` from the
  shared grammars directory.

  ## Usage

      {:ok, ast} = CodingAdventures.JsonParser.parse(~s({"key": [1, 2, 3]}))

  The returned AST is a tree of `ASTNode` structs where `rule_name`
  indicates the grammar rule ("value", "object", "pair", "array") and
  `children` contains sub-nodes and tokens.
  """

  alias CodingAdventures.GrammarTools.ParserGrammar
  alias CodingAdventures.JsonLexer
  alias CodingAdventures.Parser.{GrammarParser, ASTNode}

  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  @doc """
  Parse JSON source code into an AST.

  Returns `{:ok, ast_node}` on success, `{:error, message}` on failure.
  """
  @spec parse(String.t()) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def parse(source) do
    grammar = get_grammar()

    case JsonLexer.tokenize(source) do
      {:ok, tokens} -> GrammarParser.parse(tokens, grammar)
      {:error, msg} -> {:error, msg}
    end
  end

  @doc """
  Parse the json.grammar file and return the ParserGrammar.
  """
  @spec create_parser() :: ParserGrammar.t()
  def create_parser do
    grammar_path = Path.join(@grammars_dir, "json.grammar")
    {:ok, grammar} = ParserGrammar.parse(File.read!(grammar_path))
    grammar
  end

  defp get_grammar do
    case :persistent_term.get({__MODULE__, :grammar}, nil) do
      nil ->
        grammar = create_parser()
        :persistent_term.put({__MODULE__, :grammar}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
