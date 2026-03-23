defmodule CodingAdventures.LatticeTranspiler do
  @moduledoc """
  Lattice-to-CSS transpiler — end-to-end pipeline.

  This module wires together three packages into a single `transpile/2` function:

  1. **`LatticeLexer`** + **`LatticeParser`** — Source text → Lattice AST
  2. **`LatticeAstToCss.Transformer`** — Lattice AST → Clean CSS AST
  3. **`LatticeAstToCss.Emitter`** — Clean CSS AST → CSS text

  Each step is tested separately in its own package. This module just connects
  them in sequence.

  ## Pipeline Diagram

      Lattice Source
           │
           ▼
      ┌─────────────┐
      │ Lattice Lexer│  ← lattice.tokens
      └──────┬──────┘
             │ tokens
             ▼
      ┌─────────────┐
      │Lattice Parser│  ← lattice.grammar
      └──────┬──────┘
             │ AST (CSS + Lattice nodes)
             ▼
      ┌─────────────┐
      │ Transformer  │  ← scope, evaluator
      └──────┬──────┘
             │ AST (CSS nodes only)
             ▼
      ┌─────────────┐
      │  CSS Emitter │
      └──────┬──────┘
             │
             ▼
        CSS Text

  ## Usage

      # Simple transpilation
      {:ok, css} = CodingAdventures.LatticeTranspiler.transpile(source)

      # With options
      {:ok, css} = CodingAdventures.LatticeTranspiler.transpile(source,
        minified: true,
        indent: "    "
      )

  ## Error Handling

  The pipeline returns `{:error, message}` for any of:

  - Lexer errors (unknown characters)
  - Parser errors (syntax errors)
  - Lattice errors (undefined variable, wrong arity, circular mixin, etc.)

  ## Examples

      iex> CodingAdventures.LatticeTranspiler.transpile("h1 { color: red; }")
      {:ok, "h1 {\\n  color: red;\\n}\\n"}

      iex> CodingAdventures.LatticeTranspiler.transpile(\"\"\"
      ...>   $primary: #4a90d9;
      ...>   h1 { color: $primary; }
      ...> \"\"\")
      {:ok, "h1 {\\n  color: #4a90d9;\\n}\\n"}

  """

  alias CodingAdventures.LatticeParser
  alias CodingAdventures.LatticeAstToCss.{Transformer, Emitter}

  @doc """
  Transpile Lattice source text to CSS.

  This is the main entry point for the Lattice transpiler. Pass in a string
  of Lattice source, get back CSS text.

  ## Parameters

  - `source` — the Lattice source text to transpile
  - `opts` — keyword options:
    - `:minified` — if `true`, emit minified CSS with no extra whitespace
      (default: `false`)
    - `:indent` — the indentation string per nesting level (default: `"  "`)

  ## Returns

  - `{:ok, css_text}` — the transpiled CSS string
  - `{:error, message}` — if any step failed

  ## Examples

      {:ok, css} = CodingAdventures.LatticeTranspiler.transpile(\"\"\"
        $primary: #4a90d9;

        @mixin button($bg, $fg: white) {
          background: $bg;
          color: $fg;
          padding: 8px 16px;
        }

        .btn {
          @include button($primary);
        }
      \"\"\")
      # css is:
      # .btn {
      #   background: #4a90d9;
      #   color: white;
      #   padding: 8px 16px;
      # }
  """
  @spec transpile(String.t(), keyword()) :: {:ok, String.t()} | {:error, String.t()}
  def transpile(source, opts \\ []) do
    minified = Keyword.get(opts, :minified, false)
    indent = Keyword.get(opts, :indent, "  ")

    # Step 1: Parse (lex + parse)
    with {:ok, ast} <- LatticeParser.parse(source),

         # Step 2: Transform (Lattice AST → clean CSS AST)
         {:ok, css_ast} <- Transformer.transform(ast) do

      # Step 3: Emit (clean CSS AST → CSS text)
      css = Emitter.emit(css_ast, minified: minified, indent: indent)
      {:ok, css}
    end
  end
end
