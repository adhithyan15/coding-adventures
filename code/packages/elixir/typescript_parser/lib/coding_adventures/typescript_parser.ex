defmodule CodingAdventures.TypescriptParser do
  @moduledoc """
  Parses TypeScript source code into ASTs using the grammar-driven parser approach.

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.

  ## Version Support

  This parser accepts the same version strings as `CodingAdventures.TypescriptLexer`:

  | Version string  | Grammar files                                       |
  |-----------------|-----------------------------------------------------|
  | `"ts1.0"`       | `grammars/typescript/ts1.0.{tokens,grammar}`        |
  | `"ts2.0"`       | `grammars/typescript/ts2.0.{tokens,grammar}`        |
  | `"ts3.0"`       | `grammars/typescript/ts3.0.{tokens,grammar}`        |
  | `"ts4.0"`       | `grammars/typescript/ts4.0.{tokens,grammar}`        |
  | `"ts5.0"`       | `grammars/typescript/ts5.0.{tokens,grammar}`        |
  | `"ts5.8"`       | `grammars/typescript/ts5.8.{tokens,grammar}`        |
  | `nil` (default) | `grammars/typescript.{tokens,grammar}` (generic)    |

  ## Usage

      # Generic (version-agnostic)
      ast = CodingAdventures.TypescriptParser.parse("let x = 1 + 2;")

      # Version-specific
      ast = CodingAdventures.TypescriptParser.parse("let x: number = 1;", "ts5.8")

  ## Stub Note

  This is a stub implementation. Function signatures are stable and
  backwards-compatible.
  """

  @valid_versions ~w(ts1.0 ts2.0 ts3.0 ts4.0 ts5.0 ts5.8)

  @doc """
  Parse TypeScript source code and return an AST node.

  ## Parameters

  - `source` — TypeScript source code as a string.
  - `version` — Optional TypeScript version string. Must be one of:
    `"ts1.0"`, `"ts2.0"`, `"ts3.0"`, `"ts4.0"`, `"ts5.0"`, `"ts5.8"`.
    Pass `nil` (default) to use the generic grammar.

  ## Returns

  A map representing the root AST node. The root node has `:rule_name` of
  `"program"` when the full implementation is active.

  ## Examples

      iex> ast = CodingAdventures.TypescriptParser.parse("let x = 1;")
      iex> is_map(ast)
      true

      iex> ast = CodingAdventures.TypescriptParser.parse("let x = 1;", "ts5.8")
      iex> is_map(ast)
      true

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised TypeScript version identifier.
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
            "Unknown TypeScript version #{inspect(version)}. " <>
              "Valid values: #{Enum.join(@valid_versions, ", ")}"
    end

    :ok
  end
end
