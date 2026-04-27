defmodule CodingAdventures.CSharpParser do
  @moduledoc """
  Parses C# source code into ASTs using the grammar-driven parser approach.

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.

  ## What is a Parser?

  Once a **lexer** has converted raw source text into a flat list of tokens, a
  **parser** takes that token stream and builds a tree that reflects the
  *grammatical structure* of the program. This tree is called an
  **Abstract Syntax Tree (AST)**.

  Consider the C# expression:

      1 + 2 * 3

  The lexer sees four tokens: `1`, `+`, `2`, `*`, `3`. But the parser knows
  that `*` binds more tightly than `+`, so it builds:

        +
       / \\
      1   *
         / \\
        2   3

  The AST captures this hierarchy. Later compiler stages—type checkers, code
  generators—walk the AST rather than the flat token list, because the tree
  encodes the *meaning* of the code rather than just its spelling.

  ## Parser vs. Lexer: a Pipeline

  Compilation typically follows this pipeline:

      Source text
          │
          ▼
      [Lexer]  ─── produces ──▶  token stream
          │
          ▼
      [Parser] ─── produces ──▶  AST
          │
          ▼
      [Semantic analysis, code generation, …]

  This module sits at the **Parser** stage. It depends on
  `CodingAdventures.CSharpLexer` to first produce tokens, then applies the
  C#-version-specific context-free grammar to construct the AST.

  ## Version Support

  This parser accepts the same version strings as `CodingAdventures.CSharpLexer`:

  | Version string  | Grammar files                                          |
  |-----------------|--------------------------------------------------------|
  | `"1.0"`         | `grammars/csharp/csharp1.0.{tokens,grammar}`           |
  | `"2.0"`         | `grammars/csharp/csharp2.0.{tokens,grammar}`           |
  | `"3.0"`         | `grammars/csharp/csharp3.0.{tokens,grammar}`           |
  | `"4.0"`         | `grammars/csharp/csharp4.0.{tokens,grammar}`           |
  | `"5.0"`         | `grammars/csharp/csharp5.0.{tokens,grammar}`           |
  | `"6.0"`         | `grammars/csharp/csharp6.0.{tokens,grammar}`           |
  | `"7.0"`         | `grammars/csharp/csharp7.0.{tokens,grammar}`           |
  | `"8.0"`         | `grammars/csharp/csharp8.0.{tokens,grammar}`           |
  | `"9.0"`         | `grammars/csharp/csharp9.0.{tokens,grammar}`           |
  | `"10.0"`        | `grammars/csharp/csharp10.0.{tokens,grammar}`          |
  | `"11.0"`        | `grammars/csharp/csharp11.0.{tokens,grammar}`          |
  | `"12.0"`        | `grammars/csharp/csharp12.0.{tokens,grammar}`          |
  | `nil` (default) | `grammars/csharp/csharp12.0.{tokens,grammar}`          |

  ## Usage

      # Default (C# 12.0)
      ast = CodingAdventures.CSharpParser.parse_csharp("int x = 1 + 2;")

      # Version-specific
      ast = CodingAdventures.CSharpParser.parse_csharp("int x = 1 + 2;", "8.0")
      ast = CodingAdventures.CSharpParser.parse_csharp("int x = 1 + 2;", "1.0")

  ## Stub Note

  This is a stub implementation. Function signatures are stable and
  backwards-compatible.
  """

  # The complete list of supported C# version identifiers. Must stay in sync
  # with `CodingAdventures.CSharpLexer.@valid_versions` and the grammar files
  # under `code/grammars/csharp/`.
  @valid_versions ~w(1.0 2.0 3.0 4.0 5.0 6.0 7.0 8.0 9.0 10.0 11.0 12.0)

  @doc """
  Parse C# source code and return an AST node.

  This is the primary entry point for parsing. It accepts a source string and
  an optional version identifier, validates the version, and returns the root
  AST node produced by the underlying grammar-driven engine.

  ## Parameters

  - `source` -- C# source code as a string.
  - `version` -- Optional C# version string. Must be one of:
    `"1.0"`, `"2.0"`, `"3.0"`, `"4.0"`, `"5.0"`, `"6.0"`, `"7.0"`,
    `"8.0"`, `"9.0"`, `"10.0"`, `"11.0"`, `"12.0"`.
    Pass `nil` (default) to use the default grammar (C# 12.0).

  ## Returns

  A map representing the root AST node. The root node has `:rule_name` of
  `"compilation_unit"` when the full implementation is active (matching the
  top-level rule in the C# formal grammar).

  ## Examples

      iex> ast = CodingAdventures.CSharpParser.parse_csharp("int x = 1;")
      iex> is_map(ast)
      true

      iex> ast = CodingAdventures.CSharpParser.parse_csharp("int x = 1;", "8.0")
      iex> is_map(ast)
      true

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised C# version identifier.
  """
  @spec parse_csharp(String.t(), String.t() | nil) :: map()
  def parse_csharp(source, version \\ nil) when is_binary(source) do
    validate_version!(version)
    # Stub: return a minimal compilation-unit node until the full grammar-driven
    # implementation is wired up. The function signature and version
    # validation are stable.
    _ = source
    %{rule_name: "compilation_unit", children: [], version: version}
  end

  @doc """
  Create a parser context for C# source code.

  Unlike `parse_csharp/2`, which eagerly produces the full AST,
  `create_csharp_parser/2` returns a map describing the configured parser
  state. This is useful when building pipelines or deferred parsing workflows
  where you want to configure the parser once and run it later.

  ## Parameters

  - `source` -- C# source code as a string.
  - `version` -- Optional C# version string (same values as `parse_csharp/2`).

  ## Returns

  A map with at minimum `:source`, `:version`, and `:language` keys. Callers
  should treat this as an opaque handle.

  ## Examples

      iex> parser = CodingAdventures.CSharpParser.create_csharp_parser("int x = 1;")
      iex> is_map(parser)
      true

      iex> parser = CodingAdventures.CSharpParser.create_csharp_parser("int x = 1;", "8.0")
      iex> parser.version
      "8.0"

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised C# version identifier.
  """
  @spec create_csharp_parser(String.t(), String.t() | nil) :: map()
  def create_csharp_parser(source, version \\ nil) when is_binary(source) do
    validate_version!(version)
    %{source: source, version: version, language: :csharp}
  end

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  # `nil` is always valid — it means "use the default (12.0) grammar".
  defp validate_version!(nil), do: :ok

  # For any non-nil binary we check it against the known version list and raise
  # a descriptive `ArgumentError` if it isn't found. The error message lists all
  # valid values so the caller knows exactly what to pass.
  defp validate_version!(ver) when is_binary(ver) do
    unless ver in @valid_versions do
      raise ArgumentError,
            "Unknown C# version #{inspect(ver)}. " <>
              "Valid values: #{Enum.join(@valid_versions, ", ")}"
    end

    :ok
  end
end
