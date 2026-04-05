defmodule CodingAdventures.EcmascriptEs1Parser do
  @moduledoc """
  Parses ECMAScript 1 (1997) source code into ASTs using the grammar-driven parser.

  This module combines the ES1 lexer with the grammar-driven parser engine.
  The pipeline is:

  1. Source code --> `es1.tokens` --> `GrammarLexer` --> token list
  2. Token list --> `es1.grammar` --> `GrammarParser` --> AST

  ## What is ECMAScript 1?

  ES1 (ECMA-262 1st Edition, June 1997) is the very first standardized version
  of JavaScript. The grammar covers:

  - Variable declarations (`var`)
  - Function declarations and expressions
  - All statement types: `if`, `while`, `do-while`, `for`, `for-in`, `switch`,
    `with`, `break`, `continue`, `return`, labelled statements
  - Full expression precedence chain from comma operator down to primary expressions
  - Object and array literals

  ### What ES1 does NOT have:

  - No `try`/`catch`/`finally`/`throw` (ES3)
  - No `===`/`!==` strict equality (ES3)
  - No regex literals (ES3)
  - No `debugger` statement (ES5)
  - No `let`/`const`/`class`/arrow functions (ES2015)

  ## Usage

      {:ok, ast} = CodingAdventures.EcmascriptEs1Parser.parse("var x = 1 + 2;")

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.
  """

  alias CodingAdventures.GrammarTools
  alias CodingAdventures.Parser
  alias CodingAdventures.EcmascriptEs1Lexer

  @grammar_dir Path.expand("../../../../../grammars/ecmascript", __DIR__)

  @doc """
  Return the path to the ES1 parser grammar file.
  """
  @spec grammar_path() :: String.t()
  def grammar_path do
    Path.join(@grammar_dir, "es1.grammar")
  end

  @doc """
  Load and parse the ES1 parser grammar.

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
  Parse ECMAScript 1 source code into an AST.

  Takes a string of ES1 JavaScript source code, tokenizes it using the ES1
  lexer, then parses the token stream using the ES1 grammar.

  Returns `{:ok, ast}` on success, `{:error, message}` on failure.

  ## Examples

      iex> {:ok, ast} = CodingAdventures.EcmascriptEs1Parser.parse("var x = 1;")
      iex> ast.rule_name
      "program"
  """
  @spec parse(String.t()) :: {:ok, CodingAdventures.Parser.ASTNode.t()} | {:error, String.t()}
  def parse(source) do
    with {:ok, tokens} <- EcmascriptEs1Lexer.tokenize(source),
         {:ok, grammar} <- load_grammar() do
      Parser.parse(tokens, grammar)
    end
  end
end
