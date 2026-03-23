defmodule CodingAdventures.LatticeParser do
  @moduledoc """
  Lattice CSS Superset Parser

  This module parses Lattice source code into an abstract syntax tree (AST).
  It is a thin wrapper combining two steps:

  1. **Lexing** — `LatticeLexer.tokenize/1` tokenizes the source into tokens.
  2. **Parsing** — `GrammarParser.parse/2` applies the grammar rules to produce
     an AST tree of `%ASTNode{}` structs.

  ## What the AST Contains

  The returned AST has `rule_name: "stylesheet"` at the root. Its children
  are `rule` nodes, each of which is one of:

  - **Lattice constructs** (produce no direct CSS output — they define values):
    - `variable_declaration` — `$color: red;`
    - `mixin_definition` — `@mixin button($bg) { ... }`
    - `function_definition` — `@function spacing($n) { ... }`
    - `use_directive` — `@use "colors";`

  - **CSS constructs** (produce CSS output):
    - `qualified_rule` — `h1 { color: red; }`
    - `at_rule` — `@media (max-width: 768px) { ... }`

  The AST-to-CSS compiler (`LatticeAstToCss`) then expands Lattice constructs
  and produces a clean CSS AST with only CSS nodes remaining.

  ## Grammar File Location

  The `lattice.grammar` file lives at `code/grammars/lattice.grammar` in the
  repository root. We navigate there relative to this source file:

      lattice_parser.ex
      └── coding_adventures/    (2 levels below the package root)
          └── lib/
              └── lattice_parser/
                  └── elixir/
                      └── packages/
                          └── code/
                              └── grammars/
                                  └── lattice.grammar

  6 levels up from `__DIR__`, then into `grammars/`.

  ## Usage

      {:ok, ast} = CodingAdventures.LatticeParser.parse("$color: red;")
      # ast is a %ASTNode{rule_name: "stylesheet", children: [...]}

  The AST can be passed to `CodingAdventures.LatticeAstToCss.Transformer.transform/1`
  for expansion.
  """

  alias CodingAdventures.GrammarTools.ParserGrammar
  alias CodingAdventures.LatticeLexer
  alias CodingAdventures.Parser.{GrammarParser, ASTNode}

  # ---------------------------------------------------------------------------
  # Grammar File Location
  # ---------------------------------------------------------------------------
  #
  # Same path strategy as the lexer. 6 ".." levels from __DIR__ to reach
  # the code/ directory, then into grammars/.

  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  @doc """
  Parse Lattice source code into an AST.

  This is the main entry point. Pass a string of Lattice source, get back
  an `%ASTNode{}` tree representing the complete parse.

  ## Parameters

  - `source` — the Lattice source text to parse

  ## Returns

  - `{:ok, ast_node}` on success, where `ast_node` is an `%ASTNode{rule_name: "stylesheet"}`
  - `{:error, message}` on failure (lexer error or syntax error)

  ## Examples

      {:ok, ast} = CodingAdventures.LatticeParser.parse("h1 { color: red; }")
      # ast.rule_name == "stylesheet"

      {:ok, ast} = CodingAdventures.LatticeParser.parse(\"\"\"
        $primary: #4a90d9;
        h1 { color: $primary; }
      \"\"\")

  ## Errors

      {:error, msg} = CodingAdventures.LatticeParser.parse("{ bad syntax")

  """
  @spec parse(String.t()) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def parse(source) do
    grammar = get_grammar()

    case LatticeLexer.tokenize(source) do
      {:ok, tokens} -> GrammarParser.parse(tokens, grammar)
      {:error, msg} -> {:error, msg}
    end
  end

  @doc """
  Parse the `lattice.grammar` file and return the `ParserGrammar`.

  This is the grammar struct used internally by `parse/1`. Exposed publicly
  for introspection, testing, and grammar validation purposes.

  ## Returns

  A `%ParserGrammar{}` struct, or raises if the grammar file cannot be read.
  """
  @spec create_parser() :: ParserGrammar.t()
  def create_parser do
    grammar_path = Path.join(@grammars_dir, "lattice.grammar")
    {:ok, grammar} = ParserGrammar.parse(File.read!(grammar_path))
    grammar
  end

  # ---------------------------------------------------------------------------
  # Grammar Caching
  # ---------------------------------------------------------------------------
  #
  # Like the lexer, we cache the parsed grammar in :persistent_term to avoid
  # re-parsing the grammar file on every parse/1 call.

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
