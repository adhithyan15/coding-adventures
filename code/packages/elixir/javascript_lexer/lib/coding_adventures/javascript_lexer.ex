defmodule CodingAdventures.JavascriptLexer do
  @moduledoc """
  Tokenizes JavaScript source code using the grammar-driven lexer approach.

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.

  ## Version Support

  ECMAScript has gone through many editions since ES1 (1997). Pass an optional
  `version` argument to select a version-specific token grammar:

  | Version string  | Grammar file                              |
  |-----------------|-------------------------------------------|
  | `"es1"`         | `grammars/ecmascript/es1.tokens`          |
  | `"es3"`         | `grammars/ecmascript/es3.tokens`          |
  | `"es5"`         | `grammars/ecmascript/es5.tokens`          |
  | `"es2015"`      | `grammars/ecmascript/es2015.tokens`       |
  | `"es2016"`      | `grammars/ecmascript/es2016.tokens`       |
  | `"es2017"`      | `grammars/ecmascript/es2017.tokens`       |
  | `"es2018"`      | `grammars/ecmascript/es2018.tokens`       |
  | `"es2019"`      | `grammars/ecmascript/es2019.tokens`       |
  | `"es2020"`      | `grammars/ecmascript/es2020.tokens`       |
  | `"es2021"`      | `grammars/ecmascript/es2021.tokens`       |
  | `"es2022"`      | `grammars/ecmascript/es2022.tokens`       |
  | `"es2023"`      | `grammars/ecmascript/es2023.tokens`       |
  | `"es2024"`      | `grammars/ecmascript/es2024.tokens`       |
  | `"es2025"`      | `grammars/ecmascript/es2025.tokens`       |
  | `nil` (default) | `grammars/javascript.tokens` (generic)    |

  When `version` is `nil` or not provided, the generic `javascript.tokens`
  grammar is used — suitable when you do not need edition-specific keyword sets.

  ## Usage

      # Generic (version-agnostic)
      tokens = CodingAdventures.JavascriptLexer.tokenize("let x = 1 + 2;")

      # Version-specific
      tokens = CodingAdventures.JavascriptLexer.tokenize("var x = 1 + 2;", "es5")
      tokens = CodingAdventures.JavascriptLexer.tokenize("let x = 1 + 2;", "es2015")

  ## Stub Note

  This is a stub implementation. The full implementation will load grammar files
  and delegate to the grammar-driven lexer engine. Function signatures are stable
  and backwards-compatible.
  """

  @valid_versions ~w(es1 es3 es5 es2015 es2016 es2017 es2018 es2019 es2020 es2021 es2022 es2023 es2024 es2025)

  @doc """
  Tokenize JavaScript source code and return a list of tokens.

  ## Parameters

  - `source` — JavaScript source code as a string.
  - `version` — Optional ECMAScript edition string. Must be one of:
    `"es1"`, `"es3"`, `"es5"`, `"es2015"` … `"es2025"`.
    Pass `nil` (default) to use the generic grammar.

  ## Returns

  A list of token maps. Each token has at minimum a `:type` and `:value` key.
  The last token in the list is always an EOF token.

  ## Examples

      iex> tokens = CodingAdventures.JavascriptLexer.tokenize("let x = 1;")
      iex> is_list(tokens)
      true

      iex> tokens = CodingAdventures.JavascriptLexer.tokenize("var x = 1;", "es5")
      iex> is_list(tokens)
      true

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised ECMAScript edition identifier.
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
  Create a lexer context for JavaScript source code.

  Unlike `tokenize/2`, which eagerly produces the full token list,
  `create_lexer/2` returns a map describing the configured lexer state.
  This is the Elixir analogue of the TypeScript `createJavascriptLexer`
  factory function and is useful when building pipelines or streaming tokenizers.

  ## Parameters

  - `source` — JavaScript source code as a string.
  - `version` — Optional ECMAScript edition string (same values as `tokenize/2`).

  ## Returns

  A map with at minimum `:source` and `:version` keys. Callers should treat this
  as an opaque token and pass it to `tokenize_lexer/1` (future API).

  ## Examples

      iex> lexer = CodingAdventures.JavascriptLexer.create_lexer("let x = 1;")
      iex> is_map(lexer)
      true

      iex> lexer = CodingAdventures.JavascriptLexer.create_lexer("var x = 1;", "es5")
      iex> lexer.version
      "es5"

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised ECMAScript edition identifier.
  """
  @spec create_lexer(String.t(), String.t() | nil) :: map()
  def create_lexer(source, version \\ nil) when is_binary(source) do
    validate_version!(version)
    %{source: source, version: version, language: :javascript}
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
