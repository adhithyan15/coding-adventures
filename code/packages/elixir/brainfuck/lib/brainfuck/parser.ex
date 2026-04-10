defmodule CodingAdventures.Brainfuck.Parser do
  @moduledoc """
  Brainfuck Parser — Thin wrapper around the grammar-driven parser engine.

  This module combines `Brainfuck.Lexer.tokenize/1` with `GrammarParser.parse/2`
  to parse Brainfuck source code into an Abstract Syntax Tree. It reads
  `brainfuck.grammar` from the shared grammars directory.

  ## Why a Parser?

  The Brainfuck VM already works without a parser — the `Translator` module
  compiles source directly to bytecode. The parser's value is compositional:
  it produces a tree that other tools (optimizers, visualizers, transpilers)
  can consume without reimplementing recursive descent.

  ## Grammar Overview

  Brainfuck's grammar has just four rules:

  ```
  program     = { instruction } ;
  instruction = loop | command ;
  loop        = LOOP_START { instruction } LOOP_END ;
  command     = RIGHT | LEFT | INC | DEC | OUTPUT | INPUT ;
  ```

  The key property is that `loop` references `instruction`, which references
  `loop` — mutual recursion. This allows arbitrarily deep nesting like
  `+[+[+[+]]]`.

  ## Usage

      {:ok, ast} = CodingAdventures.Brainfuck.Parser.parse("++[>+<-]")

  The returned AST is a tree of `ASTNode` structs where `rule_name`
  indicates the grammar rule (`"program"`, `"instruction"`, `"loop"`,
  `"command"`) and `children` contains sub-nodes and tokens.
  """

  alias CodingAdventures.GrammarTools.ParserGrammar
  alias CodingAdventures.Brainfuck.Lexer
  alias CodingAdventures.Parser.{GrammarParser, ASTNode}

  # Path to the shared grammars directory.
  # Same four-level-up navigation as lexer.ex.
  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  @doc """
  Parse Brainfuck source code into an Abstract Syntax Tree.

  Returns `{:ok, ast_node}` on success, `{:error, message}` on failure.
  The root node has `rule_name: "program"` and zero or more instruction
  children.

  If the source contains unmatched brackets, parsing will return
  `{:error, message}` because the grammar requires a `LOOP_END` for
  every `LOOP_START`.

  ## Examples

      iex> {:ok, ast} = CodingAdventures.Brainfuck.Parser.parse("++[>+<-]")
      iex> ast.rule_name
      "program"

      iex> CodingAdventures.Brainfuck.Parser.parse("[+")
      {:error, _}
  """
  @spec parse(String.t()) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def parse(source) do
    grammar = get_grammar()

    # Step 1: Tokenize using the Brainfuck lexer.
    # Comments (non-command characters) are stripped by the lexer's skip:
    # mechanism. Only command tokens and EOF reach the parser.
    case Lexer.tokenize(source) do
      {:ok, tokens} -> GrammarParser.parse(tokens, grammar)
      {:error, msg} -> {:error, msg}
    end
  end

  @doc """
  Parse the `brainfuck.grammar` file and return the `ParserGrammar`.

  This is useful for inspecting the grammar or reusing it directly.
  """
  @spec create_parser() :: ParserGrammar.t()
  def create_parser do
    grammar_path = Path.join(@grammars_dir, "brainfuck.grammar")
    {:ok, grammar} = ParserGrammar.parse(File.read!(grammar_path))
    grammar
  end

  # Retrieve the cached ParserGrammar, building and caching it on first access.
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
