defmodule CodingAdventures.LatticeAstToCss do
  @moduledoc """
  Three-pass Lattice AST → CSS compiler.

  This package transforms a Lattice AST (produced by `LatticeParser`) into
  a clean CSS AST, then emits formatted CSS text. It is the core semantic
  layer of the Lattice transpiler.

  ## Architecture

  The package contains five modules:

  - `Errors` — structured error types (undefined variable, wrong arity, etc.)
  - `Scope` — lexical scope chain for variable/mixin/function lookup
  - `Values` — typed Lattice values (number, dimension, string, etc.)
  - `Evaluator` — compile-time expression evaluator
  - `Transformer` — three-pass AST transformation (symbol collection,
                    expansion, cleanup)
  - `Emitter` — CSS text generator from a clean CSS AST

  ## Three-Pass Pipeline

  **Pass 1 — Symbol Collection:**
  Scan the top-level stylesheet for variable, mixin, and function definitions.
  Collect them into registries for use during expansion. Remove definition
  nodes from the AST (they produce no CSS output).

  **Pass 2 — Expansion:**
  Recursively walk the AST, substituting variables, expanding `@include`
  directives, evaluating `@if`/`@for`/`@each` control flow, and evaluating
  Lattice function calls. After this pass, the AST is pure CSS.

  **Pass 3 — Cleanup:**
  Remove empty blocks and nil children that resulted from Lattice nodes
  being expanded away.

  ## Convenience Functions

  The top-level module provides two convenience functions that chain the
  entire pipeline:

      # Get a clean CSS AST
      {:ok, css_ast} = CodingAdventures.LatticeAstToCss.transform(ast)

      # Get CSS text directly
      {:ok, css_text} = CodingAdventures.LatticeAstToCss.transform_to_css(ast)

  ## Usage

      alias CodingAdventures.LatticeAstToCss
      alias CodingAdventures.LatticeAstToCss.{Transformer, Emitter}

      {:ok, ast} = CodingAdventures.LatticeParser.parse(source)
      {:ok, css_ast} = Transformer.transform(ast)
      css_text = Emitter.emit(css_ast)
  """

  alias CodingAdventures.LatticeAstToCss.{Transformer, Emitter}
  alias CodingAdventures.Parser.ASTNode

  @doc """
  Transform a Lattice AST into a clean CSS AST.

  Convenience wrapper around `Transformer.transform/1`.

  ## Returns

  - `{:ok, css_ast}` — clean CSS AST with no Lattice nodes
  - `{:error, message}` — if a Lattice error occurred
  """
  @spec transform(ASTNode.t()) :: {:ok, ASTNode.t()} | {:error, String.t()}
  defdelegate transform(ast), to: Transformer

  @doc """
  Transform a Lattice AST into CSS text.

  Chains `Transformer.transform/1` and `Emitter.emit/2` for convenience.

  ## Parameters

  - `ast` — Lattice AST from the parser
  - `opts` — options passed to `Emitter.emit/2`:
    - `:minified` — if `true`, emit minified CSS (default: `false`)
    - `:indent` — indentation string (default: `"  "`)

  ## Returns

  - `{:ok, css_text}` — the transpiled CSS string
  - `{:error, message}` — if a Lattice error occurred
  """
  @spec transform_to_css(ASTNode.t(), keyword()) :: {:ok, String.t()} | {:error, String.t()}
  def transform_to_css(ast, opts \\ []) do
    case Transformer.transform(ast) do
      {:ok, css_ast} -> {:ok, Emitter.emit(css_ast, opts)}
      {:error, _} = err -> err
    end
  end
end
