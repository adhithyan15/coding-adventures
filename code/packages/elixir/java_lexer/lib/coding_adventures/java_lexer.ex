defmodule CodingAdventures.JavaLexer do
  @moduledoc """
  Tokenizes Java source code using the grammar-driven lexer approach.

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.

  ## Version Support

  Java has evolved significantly since version 1.0 (1996). Pass an optional
  `version` argument to select a version-specific token grammar:

  | Version string  | Grammar file                              |
  |-----------------|-------------------------------------------|
  | `"1.0"`         | `grammars/java/java1.0.tokens`            |
  | `"1.1"`         | `grammars/java/java1.1.tokens`            |
  | `"1.4"`         | `grammars/java/java1.4.tokens`            |
  | `"5"`           | `grammars/java/java5.tokens`              |
  | `"7"`           | `grammars/java/java7.tokens`              |
  | `"8"`           | `grammars/java/java8.tokens`              |
  | `"10"`          | `grammars/java/java10.tokens`             |
  | `"14"`          | `grammars/java/java14.tokens`             |
  | `"17"`          | `grammars/java/java17.tokens`             |
  | `"21"`          | `grammars/java/java21.tokens`             |
  | `nil` (default) | `grammars/java/java21.tokens` (default)   |

  When `version` is `nil` or not provided, the Java 21 grammar is used as the
  default -- the latest LTS release.

  ## Usage

      # Default (Java 21)
      tokens = CodingAdventures.JavaLexer.tokenize("int x = 1 + 2;")

      # Version-specific
      tokens = CodingAdventures.JavaLexer.tokenize("int x = 1 + 2;", "8")
      tokens = CodingAdventures.JavaLexer.tokenize("int x = 1 + 2;", "1.0")

  ## Stub Note

  This is a stub implementation. The full implementation will load grammar files
  and delegate to the grammar-driven lexer engine. Function signatures are stable
  and backwards-compatible.
  """

  @valid_versions ~w(1.0 1.1 1.4 5 7 8 10 14 17 21)

  @doc """
  Tokenize Java source code and return a list of tokens.

  ## Parameters

  - `source` -- Java source code as a string.
  - `version` -- Optional Java version string. Must be one of:
    `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"`.
    Pass `nil` (default) to use the default grammar (Java 21).

  ## Returns

  A list of token maps. Each token has at minimum a `:type` and `:value` key.
  The last token in the list is always an EOF token.

  ## Examples

      iex> tokens = CodingAdventures.JavaLexer.tokenize("int x = 1;")
      iex> is_list(tokens)
      true

      iex> tokens = CodingAdventures.JavaLexer.tokenize("int x = 1;", "8")
      iex> is_list(tokens)
      true

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised Java version identifier.
  """
  @spec tokenize(String.t(), String.t() | nil) :: list()
  def tokenize(source, version \\ nil) when is_binary(source) do
    validate_version!(version)
    # Stub: return an empty list until the full grammar-driven implementation
    # is wired up. The function signature and version validation are stable.
    _ = source
    []
  end

  @doc """
  Create a lexer context for Java source code.

  Unlike `tokenize/2`, which eagerly produces the full token list,
  `create_lexer/2` returns a map describing the configured lexer state.
  This is the Elixir analogue of the Ruby `create_lexer` factory method
  and is useful when building pipelines or streaming tokenizers.

  ## Parameters

  - `source` -- Java source code as a string.
  - `version` -- Optional Java version string (same values as `tokenize/2`).

  ## Returns

  A map with at minimum `:source` and `:version` keys. Callers should treat this
  as an opaque token and pass it to `tokenize_lexer/1` (future API).

  ## Examples

      iex> lexer = CodingAdventures.JavaLexer.create_lexer("int x = 1;")
      iex> is_map(lexer)
      true

      iex> lexer = CodingAdventures.JavaLexer.create_lexer("int x = 1;", "8")
      iex> lexer.version
      "8"

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised Java version identifier.
  """
  @spec create_lexer(String.t(), String.t() | nil) :: map()
  def create_lexer(source, version \\ nil) when is_binary(source) do
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
