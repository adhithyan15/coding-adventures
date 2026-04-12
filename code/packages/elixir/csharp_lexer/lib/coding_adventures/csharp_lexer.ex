defmodule CodingAdventures.CSharpLexer do
  @moduledoc """
  Tokenizes C# source code using the grammar-driven lexer approach.

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.

  ## What is a Lexer?

  A **lexer** (also called a *tokenizer* or *scanner*) is the first stage of a
  compiler or interpreter pipeline. It reads raw source text—just a big string
  of characters—and breaks it into a flat sequence of **tokens**.

  Think of it like reading a sentence in English: before you can understand the
  grammar ("this is a noun phrase", "that is a verb phrase"), you first need to
  recognise individual words and punctuation marks. The lexer does exactly that
  job for programming languages.

  For example, the C# snippet:

      int x = 1 + 2;

  …would be turned into a stream of tokens like:

      [KEYWORD "int"] [IDENTIFIER "x"] [OPERATOR "="]
      [INTEGER_LITERAL "1"] [OPERATOR "+"] [INTEGER_LITERAL "2"]
      [PUNCTUATOR ";"] [EOF]

  ## Why Are There Multiple C# Versions?

  C# has evolved considerably since version 1.0 in 2002. Each major release
  added new keywords and syntactic constructs:

  - **C# 2.0** added generics (`List<T>`), nullable types (`int?`), and
    anonymous methods.
  - **C# 3.0** added LINQ, lambda expressions (`x => x * 2`), and `var`.
  - **C# 5.0** added `async` / `await` for asynchronous programming.
  - **C# 8.0** added nullable reference types and `switch` expressions.
  - **C# 12.0** (the default) is the most recent stable release, adding primary
    constructors, collection expressions, and more.

  Because each version introduced new reserved words and token types, we keep a
  separate token grammar file per version. Passing an older version string lets
  you parse legacy codebases without false positives from newer keywords.

  ## Version Support

  | Version string  | Grammar file                                 |
  |-----------------|----------------------------------------------|
  | `"1.0"`         | `grammars/csharp/csharp1.0.tokens`           |
  | `"2.0"`         | `grammars/csharp/csharp2.0.tokens`           |
  | `"3.0"`         | `grammars/csharp/csharp3.0.tokens`           |
  | `"4.0"`         | `grammars/csharp/csharp4.0.tokens`           |
  | `"5.0"`         | `grammars/csharp/csharp5.0.tokens`           |
  | `"6.0"`         | `grammars/csharp/csharp6.0.tokens`           |
  | `"7.0"`         | `grammars/csharp/csharp7.0.tokens`           |
  | `"8.0"`         | `grammars/csharp/csharp8.0.tokens`           |
  | `"9.0"`         | `grammars/csharp/csharp9.0.tokens`           |
  | `"10.0"`        | `grammars/csharp/csharp10.0.tokens`          |
  | `"11.0"`        | `grammars/csharp/csharp11.0.tokens`          |
  | `"12.0"`        | `grammars/csharp/csharp12.0.tokens`          |
  | `nil` (default) | `grammars/csharp/csharp12.0.tokens`          |

  When `version` is `nil` or not provided, C# 12.0 is used as the default—
  the latest stable release.

  ## Usage

      # Default (C# 12.0)
      tokens = CodingAdventures.CSharpLexer.tokenize_csharp("int x = 1 + 2;")

      # Version-specific
      tokens = CodingAdventures.CSharpLexer.tokenize_csharp("int x = 1;", "8.0")
      tokens = CodingAdventures.CSharpLexer.tokenize_csharp("int x = 1;", "1.0")

  ## Stub Note

  This is a stub implementation. The full implementation will load grammar files
  and delegate to the grammar-driven lexer engine. Function signatures are stable
  and backwards-compatible.
  """

  # The complete list of supported C# version identifiers. This list mirrors
  # the set of grammar files checked into `code/grammars/csharp/`. Any string
  # outside this list (and not nil) is invalid and will raise `ArgumentError`.
  @valid_versions ~w(1.0 2.0 3.0 4.0 5.0 6.0 7.0 8.0 9.0 10.0 11.0 12.0)

  @doc """
  Tokenize C# source code and return a list of tokens.

  This is the primary entry point for lexing. It accepts a source string and an
  optional version identifier, validates the version, and returns the token
  stream produced by the underlying grammar-driven engine.

  ## Parameters

  - `source` -- C# source code as a string.
  - `version` -- Optional C# version string. Must be one of:
    `"1.0"`, `"2.0"`, `"3.0"`, `"4.0"`, `"5.0"`, `"6.0"`, `"7.0"`,
    `"8.0"`, `"9.0"`, `"10.0"`, `"11.0"`, `"12.0"`.
    Pass `nil` (default) to use the default grammar (C# 12.0).

  ## Returns

  A list of token maps. Each token has at minimum a `:type` and `:value` key.
  The last token in the list is always an EOF token.

  ## Examples

      iex> tokens = CodingAdventures.CSharpLexer.tokenize_csharp("int x = 1;")
      iex> is_list(tokens)
      true

      iex> tokens = CodingAdventures.CSharpLexer.tokenize_csharp("int x = 1;", "8.0")
      iex> is_list(tokens)
      true

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised C# version identifier.
  """
  @spec tokenize_csharp(String.t(), String.t() | nil) :: list()
  def tokenize_csharp(source, version \\ nil) when is_binary(source) do
    validate_version!(version)
    # Stub: return an empty list until the full grammar-driven implementation
    # is wired up. The function signature and version validation are stable.
    _ = source
    []
  end

  @doc """
  Create a lexer context for C# source code.

  Unlike `tokenize_csharp/2`, which eagerly produces the full token list,
  `create_csharp_lexer/2` returns a map describing the configured lexer state.
  This is useful when building pipelines or streaming tokenizers where you want
  to defer the actual tokenization step.

  Think of this like configuring a machine before turning it on: you describe
  *what* you want to lex and *which grammar* to use, but the lexer hasn't
  started scanning yet.

  ## Parameters

  - `source` -- C# source code as a string.
  - `version` -- Optional C# version string (same values as `tokenize_csharp/2`).

  ## Returns

  A map with at minimum `:source`, `:version`, and `:language` keys. Callers
  should treat this as an opaque handle and pass it to `tokenize_lexer/1`
  (future API).

  ## Examples

      iex> lexer = CodingAdventures.CSharpLexer.create_csharp_lexer("int x = 1;")
      iex> is_map(lexer)
      true

      iex> lexer = CodingAdventures.CSharpLexer.create_csharp_lexer("int x = 1;", "8.0")
      iex> lexer.version
      "8.0"

  ## Errors

  Raises `ArgumentError` if `version` is a non-nil string that is not a
  recognised C# version identifier.
  """
  @spec create_csharp_lexer(String.t(), String.t() | nil) :: map()
  def create_csharp_lexer(source, version \\ nil) when is_binary(source) do
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
