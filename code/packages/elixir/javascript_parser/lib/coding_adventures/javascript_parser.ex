defmodule CodingAdventures.JavascriptParser do
  @moduledoc """
  Parses JavaScript source code into ASTs using the grammar-driven parser approach.

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.

  ## Version Support

  This parser accepts the same version strings as `CodingAdventures.JavascriptLexer`:

  | Version string  | Grammar files                                        |
  |-----------------|------------------------------------------------------|
  | `"es1"`         | `grammars/ecmascript/es1.{tokens,grammar}`           |
  | `"es3"`         | `grammars/ecmascript/es3.{tokens,grammar}`           |
  | `"es5"`         | `grammars/ecmascript/es5.{tokens,grammar}`           |
  | `"es2015"`…     | `grammars/ecmascript/es2015.{tokens,grammar}` …      |
  | `"es2025"`      | `grammars/ecmascript/es2025.{tokens,grammar}`        |
  | `nil` (default) | `grammars/javascript.{tokens,grammar}` (generic)     |

  ## Usage

      # Generic (version-agnostic)
      ast = CodingAdventures.JavascriptParser.parse("let x = 1 + 2;")

      # Version-specific
      ast = CodingAdventures.JavascriptParser.parse("var x = 1 + 2;", "es5")
      ast = CodingAdventures.JavascriptParser.parse("let x = 1 + 2;", "es2015")

  ## Stub Note

  This is a stub implementation. Function signatures are stable and
  backwards-compatible.
  """

  @valid_versions ~w(es1 es3 es5 es2015 es2016 es2017 es2018 es2019 es2020 es2021 es2022 es2023 es2024 es2025)

  @doc """
  Parse JavaScript source code and return an AST node.

  ## Parameters

  - `source` — JavaScript source code as a string.
  - `version` — Optional ECMAScript edition string. Must be one of:
    `"es1"`, `"es3"`, `"es5"`, `"es2015"` … `"es2025"`.
    Pass `nil` (default) to use the generic grammar.

  ## Returns

  A map representing the root AST node. The root node has `:rule_name` of
  `"program"` when the full implementation is active.

  ## Examples

      iex> ast = CodingAdventures.JavascriptParser.parse("let x = 1;")
      iex> is_map(ast)
      true

      iex> ast = CodingAdventures.JavascriptParser.parse("var x = 1;", "es5")
      iex> is_map(ast)
      true

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised ECMAScript edition identifier.
  """
  @spec parse(String.t(), String.t() | nil) :: map()
  def parse(source, version \\ nil) when is_binary(source) do
    validate_version!(version)
    # Stub: return a minimal program node until the full grammar-driven
    # implementation is wired up. The function signature and version
    # validation are stable.
    _ = source
    %{rule_name: "program", children: [], version: version}
  end

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  defp validate_version!(nil), do: :ok

  defp validate_version!(version) when is_binary(version) do
    unless version in @valid_versions do
      raise ArgumentError,
            "Unknown JavaScript/ECMAScript version #{inspect(version)}. " <>
              "Valid values: #{Enum.join(@valid_versions, ", ")}"
    end

    :ok
  end
end
