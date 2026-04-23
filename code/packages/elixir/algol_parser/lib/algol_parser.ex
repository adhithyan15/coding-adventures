defmodule CodingAdventures.AlgolParser do
  @moduledoc """
  ALGOL 60 Parser — Thin wrapper around the grammar-driven parser engine.

  ## Historical Context

  ALGOL 60 holds a unique distinction in computer science: it was the first
  programming language whose complete, unambiguous syntax was specified using
  BNF (Backus-Naur Form). John Backus and Peter Naur published the ALGOL 60
  Report in 1960, and the notation they used — BNF — became the standard way
  to describe programming language grammars to this day.

  Before BNF, language specifications were written in English prose, leading to
  ambiguities and incompatible implementations. The ALGOL 60 Report changed that
  forever. Every language reference manual, RFC, and ISO standard that uses
  `<non-terminal> ::= ...` notation is following the tradition Backus and Naur
  established.

  ## Grammar Architecture

  The ALGOL 60 grammar (`algol.grammar`) is organized in four layers:

  1. **Top level** — `program` is a single `block`.
  2. **Declarations** — `type_decl`, `array_decl`, `switch_decl`, `procedure_decl`.
     All declarations must precede all statements within a block.
  3. **Statements** — `assign_stmt`, `goto_stmt`, `proc_stmt`, `cond_stmt`,
     `for_stmt`, `compound_stmt`, `empty_stmt`. Any statement may be labeled.
  4. **Expressions** — `arith_expr`, `bool_expr`, `desig_expr` with full
     operator precedence encoded in the grammar rules.

  ### Dangling Else Resolution

  ALGOL 60 resolves the dangling-else ambiguity at the grammar level, not by
  convention. The then-branch of a conditional is `unlabeled_stmt`, which
  deliberately excludes another conditional statement. To nest conditionals,
  you must use `begin ... end`:

      if a then begin if b then x := 1 end else x := 2

  In C and Java, the else associates with the nearest if by convention (not
  grammar), which is a frequent source of bugs.

  ### Exponentiation Associativity

  Per the ALGOL 60 Report, exponentiation is **left-associative**:
  `2^3^4 = (2^3)^4 = 4096`. This differs from most modern languages and
  from mathematical convention (which is right-associative). The grammar
  uses `{ (CARET | POWER) primary }` (left-recursive iteration) to achieve this.

  ## Usage

      {:ok, ast} = CodingAdventures.AlgolParser.parse("begin integer x; x := 42 end")

  The returned AST is a tree of `ASTNode` structs. Each node has:
  - `rule_name` — the grammar rule name (e.g., `"program"`, `"block"`, `"assign_stmt"`)
  - `children` — a list of child `ASTNode`s or `Token`s

  ## How It Works

  1. `AlgolLexer.tokenize/1` converts the source string into a token list.
  2. `GrammarParser.parse/2` uses `algol.grammar` to drive a recursive-descent
     parse of those tokens, building the AST bottom-up.
  3. Both the lexer grammar and the parser grammar are cached in `persistent_term`
     so the file reads happen only once per BEAM node lifetime.
  """

  alias CodingAdventures.GrammarTools.ParserGrammar
  alias CodingAdventures.AlgolLexer
  alias CodingAdventures.Parser.{GrammarParser, ASTNode}

  # Resolve the grammars directory at compile time relative to this source
  # file. Walking up: lib → algol_parser → elixir → packages → code → grammars.
  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "grammars"])
                |> Path.expand()
  @valid_versions ~w(algol60)

  @doc """
  Parse ALGOL 60 source code into an AST.

  Tokenizes the source with `AlgolLexer.tokenize/1`, then parses the token
  stream with `GrammarParser.parse/2` guided by `algol.grammar`.

  Returns `{:ok, ast_node}` on success, `{:error, message}` on failure.
  The root node has `rule_name: "program"`.

  ## Examples

      iex> {:ok, ast} = AlgolParser.parse("begin integer x; x := 42 end")
      iex> ast.rule_name
      "program"

  """
  @spec parse(String.t(), String.t()) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def parse(source, version \\ "algol60") do
    grammar = get_grammar(version)

    case AlgolLexer.tokenize(source, version) do
      {:ok, tokens} -> GrammarParser.parse(tokens, grammar)
      {:error, msg} -> {:error, msg}
    end
  end

  @doc """
  Parse the `algol.grammar` file and return the `ParserGrammar`.

  Useful for inspecting the grammar rules or for passing the grammar
  directly to `GrammarParser.parse/2` when you already hold a token list.
  """
  @spec create_parser(String.t()) :: ParserGrammar.t()
  def create_parser(version \\ "algol60") do
    grammar_path = resolve_grammar_path(version)
    {:ok, grammar} = ParserGrammar.parse(File.read!(grammar_path))
    grammar
  end

  defp resolve_grammar_path(version) when version in @valid_versions do
    Path.join([@grammars_dir, "algol", "#{version}.grammar"])
  end

  defp resolve_grammar_path(version) do
    raise ArgumentError,
          "Unknown ALGOL version #{inspect(version)}. Valid versions: #{Enum.join(@valid_versions, ", ")}"
  end

  # Cache the parsed ParserGrammar in a persistent_term. The grammar is
  # immutable once loaded, so persistent_term gives lock-free concurrent reads.
  defp get_grammar(version) do
    key = {__MODULE__, :grammar, version}

    case :persistent_term.get(key, nil) do
      nil ->
        grammar = create_parser(version)
        :persistent_term.put(key, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
