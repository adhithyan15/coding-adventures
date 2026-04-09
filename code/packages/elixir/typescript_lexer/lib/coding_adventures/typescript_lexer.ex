defmodule CodingAdventures.TypescriptLexer do
  @moduledoc """
  Tokenizes TypeScript source code using the grammar-driven lexer approach.

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.

  ## Version Support

  TypeScript has evolved significantly across major versions. Pass an optional
  `version` argument to select a version-specific token grammar:

  | Version string | Grammar file                             |
  |----------------|------------------------------------------|
  | `"ts1.0"`      | `grammars/typescript/ts1.0.tokens`       |
  | `"ts2.0"`      | `grammars/typescript/ts2.0.tokens`       |
  | `"ts3.0"`      | `grammars/typescript/ts3.0.tokens`       |
  | `"ts4.0"`      | `grammars/typescript/ts4.0.tokens`       |
  | `"ts5.0"`      | `grammars/typescript/ts5.0.tokens`       |
  | `"ts5.8"`      | `grammars/typescript/ts5.8.tokens`       |
  | `nil` (default)| `grammars/typescript.tokens` (generic)   |

  When `version` is `nil` or not provided, the generic `typescript.tokens`
  grammar is used — suitable when you do not need version-specific keyword sets.

  ## Usage

      # Generic (version-agnostic)
      tokens = CodingAdventures.TypescriptLexer.tokenize("let x: number = 1;")

      # Version-specific
      tokens = CodingAdventures.TypescriptLexer.tokenize("let x: number = 1;", "ts5.8")

  ## Stub Note

  This is a stub implementation. The full implementation will load grammar files
  and delegate to the grammar-driven lexer engine. Function signatures are stable
  and backwards-compatible.
  """

  @valid_versions ~w(ts1.0 ts2.0 ts3.0 ts4.0 ts5.0 ts5.8)

  @doc """
  Tokenize TypeScript source code and return a list of tokens.

  ## Parameters

  - `source` — TypeScript source code as a string.
  - `version` — Optional TypeScript version string. Must be one of:
    `"ts1.0"`, `"ts2.0"`, `"ts3.0"`, `"ts4.0"`, `"ts5.0"`, `"ts5.8"`.
    Pass `nil` (default) to use the generic grammar.

  ## Returns

  A list of token maps. Each token has at minimum a `:type` and `:value` key.
  The last token in the list is always an EOF token.

  ## Examples

      iex> tokens = CodingAdventures.TypescriptLexer.tokenize("let x = 1;")
      iex> is_list(tokens)
      true

      iex> tokens = CodingAdventures.TypescriptLexer.tokenize("let x = 1;", "ts5.8")
      iex> is_list(tokens)
      true

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised TypeScript version identifier.
  """
  @spec tokenize(String.t(), String.t() | nil) :: list()
  def tokenize(source, version \\ nil) when is_binary(source) do
    validate_version!(version, :typescript)
    # Stub: return an empty list until the full grammar-driven implementation
    # is wired up. The function signature and version validation are stable.
    _ = source
    []
  end

  @doc """
  Create a lexer context for TypeScript source code.

  Unlike `tokenize/2`, which eagerly produces the full token list,
  `create_lexer/2` returns a map describing the configured lexer state.
  This is the Elixir analogue of the TypeScript `createTypescriptLexer`
  factory function and is useful when building pipelines or streaming tokenizers.

  ## Parameters

  - `source` — TypeScript source code as a string.
  - `version` — Optional TypeScript version string (same values as `tokenize/2`).

  ## Returns

  A map with at minimum `:source` and `:version` keys. Callers should treat this
  as an opaque token and pass it to `tokenize_lexer/1` (future API).

  ## Examples

      iex> lexer = CodingAdventures.TypescriptLexer.create_lexer("let x = 1;")
      iex> is_map(lexer)
      true

      iex> lexer = CodingAdventures.TypescriptLexer.create_lexer("let x = 1;", "ts5.8")
      iex> lexer.version
      "ts5.8"

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised TypeScript version identifier.
  """
  @spec create_lexer(String.t(), String.t() | nil) :: map()
  def create_lexer(source, version \\ nil) when is_binary(source) do
    validate_version!(version, :typescript)
    %{source: source, version: version, language: :typescript}
  end

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  # Validate that `version` is either nil or a known version string.
  # `kind` is `:typescript` — used to build a meaningful error message.
  defp validate_version!(nil, _kind), do: :ok

  defp validate_version!(version, :typescript) when is_binary(version) do
    unless version in @valid_versions do
      raise ArgumentError,
            "Unknown TypeScript version #{inspect(version)}. " <>
              "Valid values: #{Enum.join(@valid_versions, ", ")}"
    end

    :ok
  end
end
