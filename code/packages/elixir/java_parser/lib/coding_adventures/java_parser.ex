defmodule CodingAdventures.JavaParser do
  @moduledoc """
  Parses Java source code into ASTs using the grammar-driven parser approach.

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.

  ## Version Support

  This parser accepts the same version strings as `CodingAdventures.JavaLexer`:

  | Version string  | Grammar files                                        |
  |-----------------|------------------------------------------------------|
  | `"1.0"`         | `grammars/java/java1.0.{tokens,grammar}`             |
  | `"1.1"`         | `grammars/java/java1.1.{tokens,grammar}`             |
  | `"1.4"`         | `grammars/java/java1.4.{tokens,grammar}`             |
  | `"5"`           | `grammars/java/java5.{tokens,grammar}`               |
  | `"7"`           | `grammars/java/java7.{tokens,grammar}`               |
  | `"8"`           | `grammars/java/java8.{tokens,grammar}`               |
  | `"10"`          | `grammars/java/java10.{tokens,grammar}`              |
  | `"14"`          | `grammars/java/java14.{tokens,grammar}`              |
  | `"17"`          | `grammars/java/java17.{tokens,grammar}`              |
  | `"21"`          | `grammars/java/java21.{tokens,grammar}`              |
  | `nil` (default) | `grammars/java/java21.{tokens,grammar}` (default)    |

  ## Usage

      # Default (Java 21)
      ast = CodingAdventures.JavaParser.parse("int x = 1 + 2;")

      # Version-specific
      ast = CodingAdventures.JavaParser.parse("int x = 1 + 2;", "8")
      ast = CodingAdventures.JavaParser.parse("int x = 1 + 2;", "1.0")

  ## Stub Note

  This is a stub implementation. Function signatures are stable and
  backwards-compatible.
  """

  @valid_versions ~w(1.0 1.1 1.4 5 7 8 10 14 17 21)

  @doc """
  Parse Java source code and return an AST node.

  ## Parameters

  - `source` -- Java source code as a string.
  - `version` -- Optional Java version string. Must be one of:
    `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"`.
    Pass `nil` (default) to use the default grammar (Java 21).

  ## Returns

  A map representing the root AST node. The root node has `:rule_name` of
  `"program"` when the full implementation is active.

  ## Examples

      iex> ast = CodingAdventures.JavaParser.parse("int x = 1;")
      iex> is_map(ast)
      true

      iex> ast = CodingAdventures.JavaParser.parse("int x = 1;", "8")
      iex> is_map(ast)
      true

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised Java version identifier.
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

  @doc """
  Create a parser context for Java source code.

  Unlike `parse/2`, which eagerly produces the full AST,
  `create_parser/2` returns a map describing the configured parser state.
  This is useful when building pipelines or deferred parsing workflows.

  ## Parameters

  - `source` -- Java source code as a string.
  - `version` -- Optional Java version string (same values as `parse/2`).

  ## Returns

  A map with at minimum `:source` and `:version` keys.

  ## Examples

      iex> parser = CodingAdventures.JavaParser.create_parser("int x = 1;")
      iex> is_map(parser)
      true

      iex> parser = CodingAdventures.JavaParser.create_parser("int x = 1;", "8")
      iex> parser.version
      "8"

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised Java version identifier.
  """
  @spec create_parser(String.t(), String.t() | nil) :: map()
  def create_parser(source, version \\ nil) when is_binary(source) do
    validate_version!(version)
    %{source: source, version: version, language: :java}
  end

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  defp validate_version!(nil), do: :ok

  defp validate_version!(version) when is_binary(version) do
    unless version in @valid_versions do
      raise ArgumentError,
            "Unknown Java version #{inspect(version)}. " <>
              "Valid values: #{Enum.join(@valid_versions, ", ")}"
    end

    :ok
  end
end
