defmodule CodingAdventures.TomlParser do
  @moduledoc """
  TOML Parser — Thin wrapper around the grammar-driven parser engine.

  This module combines `TomlLexer.tokenize/1` with `GrammarParser.parse/2`
  to parse TOML source code into an AST. It reads `toml.grammar` from the
  shared grammars directory.

  ## Usage

      {:ok, ast} = CodingAdventures.TomlParser.parse("title = \\"TOML Example\\"")

  The returned AST is a tree of `ASTNode` structs where `rule_name`
  indicates the grammar rule ("document", "expression", "keyval", "key",
  "value", "array", "inline_table", etc.) and `children` contains
  sub-nodes and tokens.

  ## TOML-Specific Notes

  TOML is newline-sensitive — key-value pairs are separated by newlines.
  The grammar references NEWLINE tokens explicitly, which the parser uses
  to delimit expressions within a document.

  The grammar handles [[array-of-tables]] as two consecutive LBRACKET
  tokens, disambiguating from nested arrays by context (array_table_header
  appears at expression level, arrays appear in value position).

  Escape processing for strings is NOT done at the parser level. The
  `escapes: none` mode in toml.tokens means string values retain their
  raw escape sequences. A semantic layer on top of the AST would handle
  type-specific escape processing.
  """

  alias CodingAdventures.GrammarTools.ParserGrammar
  alias CodingAdventures.TomlLexer
  alias CodingAdventures.Parser.{GrammarParser, ASTNode}

  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  @doc """
  Parse TOML source code into an AST.

  Returns `{:ok, ast_node}` on success, `{:error, message}` on failure.
  """
  @spec parse(String.t()) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def parse(source) do
    grammar = get_grammar()

    case TomlLexer.tokenize(source) do
      {:ok, tokens} -> GrammarParser.parse(tokens, grammar)
      {:error, msg} -> {:error, msg}
    end
  end

  @doc """
  Parse the toml.grammar file and return the ParserGrammar.
  """
  @spec create_parser() :: ParserGrammar.t()
  def create_parser do
    grammar_path = Path.join(@grammars_dir, "toml.grammar")
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
