defmodule CodingAdventures.EcmascriptEs5Parser do
  @moduledoc """
  Parses ECMAScript 5 (2009) source code into ASTs using the grammar-driven parser.

  This module combines the ES5 lexer with the grammar-driven parser engine.
  The pipeline is:

  1. Source code --> `es5.tokens` --> `GrammarLexer` --> token list
  2. Token list --> `es5.grammar` --> `GrammarParser` --> AST

  ## What ES5 adds over ES3:

  - `debugger` statement
  - Getter/setter property definitions in object literals
  - Same operators and expression grammar as ES3

  ## Usage

      {:ok, ast} = CodingAdventures.EcmascriptEs5Parser.parse("debugger;")

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.
  """

  alias CodingAdventures.GrammarTools
  alias CodingAdventures.Parser
  alias CodingAdventures.EcmascriptEs5Lexer

  @grammar_dir Path.expand("../../../../../grammars/ecmascript", __DIR__)

  @doc """
  Return the path to the ES5 parser grammar file.
  """
  @spec grammar_path() :: String.t()
  def grammar_path do
    Path.join(@grammar_dir, "es5.grammar")
  end

  @doc """
  Load and parse the ES5 parser grammar.

  Returns `{:ok, parser_grammar}` on success, `{:error, message}` on failure.
  """
  @spec load_grammar() :: {:ok, CodingAdventures.GrammarTools.ParserGrammar.t()} | {:error, String.t()}
  def load_grammar do
    path = grammar_path()

    case File.read(path) do
      {:ok, source} -> GrammarTools.parse_parser_grammar(source)
      {:error, reason} -> {:error, "Failed to read #{path}: #{inspect(reason)}"}
    end
  end

  @doc """
  Parse ECMAScript 5 source code into an AST.

  Takes a string of ES5 JavaScript source code, tokenizes it using the ES5
  lexer, then parses the token stream using the ES5 grammar.

  Returns `{:ok, ast}` on success, `{:error, message}` on failure.

  ## Examples

      iex> {:ok, ast} = CodingAdventures.EcmascriptEs5Parser.parse("debugger;")
      iex> ast.rule_name
      "program"
  """
  @spec parse(String.t()) :: {:ok, CodingAdventures.Parser.ASTNode.t()} | {:error, String.t()}
  def parse(source) do
    with {:ok, tokens} <- EcmascriptEs5Lexer.tokenize(source),
         {:ok, grammar} <- load_grammar() do
      Parser.parse(tokens, grammar)
    end
  end
end
