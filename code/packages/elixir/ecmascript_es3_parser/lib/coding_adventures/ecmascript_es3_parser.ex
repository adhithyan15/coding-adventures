defmodule CodingAdventures.EcmascriptEs3Parser do
  @moduledoc """
  Parses ECMAScript 3 (1999) source code into ASTs using the grammar-driven parser.

  This module combines the ES3 lexer with the grammar-driven parser engine.
  The pipeline is:

  1. Source code --> `es3.tokens` --> `GrammarLexer` --> token list
  2. Token list --> `es3.grammar` --> `GrammarParser` --> AST

  ## What ES3 adds over ES1:

  - `try`/`catch`/`finally`/`throw` statements
  - `===` and `!==` strict equality in expressions
  - `instanceof` in relational expressions
  - `REGEX` as a primary expression

  ## Usage

      {:ok, ast} = CodingAdventures.EcmascriptEs3Parser.parse("try { x(); } catch(e) { }")

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.
  """

  alias CodingAdventures.GrammarTools
  alias CodingAdventures.Parser
  alias CodingAdventures.EcmascriptEs3Lexer

  @grammar_dir Path.expand("../../../../../grammars/ecmascript", __DIR__)

  @doc """
  Return the path to the ES3 parser grammar file.
  """
  @spec grammar_path() :: String.t()
  def grammar_path do
    Path.join(@grammar_dir, "es3.grammar")
  end

  @doc """
  Load and parse the ES3 parser grammar.

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
  Parse ECMAScript 3 source code into an AST.

  Takes a string of ES3 JavaScript source code, tokenizes it using the ES3
  lexer, then parses the token stream using the ES3 grammar.

  Returns `{:ok, ast}` on success, `{:error, message}` on failure.

  ## Examples

      iex> {:ok, ast} = CodingAdventures.EcmascriptEs3Parser.parse("try { x(); } catch(e) { }")
      iex> ast.rule_name
      "program"
  """
  @spec parse(String.t()) :: {:ok, CodingAdventures.Parser.ASTNode.t()} | {:error, String.t()}
  def parse(source) do
    with {:ok, tokens} <- EcmascriptEs3Lexer.tokenize(source),
         {:ok, grammar} <- load_grammar() do
      Parser.parse(tokens, grammar)
    end
  end
end
