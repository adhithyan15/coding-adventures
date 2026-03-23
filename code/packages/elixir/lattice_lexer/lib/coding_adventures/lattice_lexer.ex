defmodule CodingAdventures.LatticeLexer do
  @moduledoc """
  Lattice CSS Superset Tokenizer

  This module tokenizes Lattice source code — a CSS superset language that
  extends CSS with variables, mixins, control flow, functions, and modules.

  ## What Is Lattice?

  Lattice is to CSS what Sass is to stylesheets: it adds programming constructs
  on top of CSS syntax. A Lattice stylesheet looks like CSS but can contain:

  - **Variables**: `$primary-color: #4a90d9;`
  - **Mixins**: `@mixin button($bg) { background: $bg; }`
  - **Control flow**: `@if $theme == dark { ... } @else { ... }`
  - **Iteration**: `@for $i from 1 through 12 { ... }`
  - **Functions**: `@function spacing($n) { @return $n * 8px; }`
  - **Modules**: `@use "colors";`

  At the token level, Lattice is mostly CSS. Only 5 new token types are added:

  | Token           | Example  | Description                           |
  |-----------------|----------|---------------------------------------|
  | `VARIABLE`      | `$color` | Dollar-sign identifier                |
  | `EQUALS_EQUALS` | `==`     | Equality comparison for `@if`         |
  | `NOT_EQUALS`    | `!=`     | Inequality comparison for `@if`       |
  | `GREATER_EQUALS`| `>=`     | Greater-or-equal comparison           |
  | `LESS_EQUALS`   | `<=`     | Less-or-equal comparison              |

  CSS at-keywords (`@media`, `@import`) and Lattice at-keywords (`@mixin`,
  `@if`) share the same `AT_KEYWORD` token type. The grammar (not the lexer)
  distinguishes them via literal text matching.

  ## Locating the Grammar File

  The `lattice.tokens` grammar lives at `code/grammars/lattice.tokens` in the
  repository root. We navigate there relative to this source file:

      lattice_lexer.ex
      └── coding_adventures/    (lib subdirectory)
          └── lib/              (lib)
              └── lattice_lexer/ (package root)
                  └── elixir/   (language dir)
                      └── packages/ (packages dir)
                          └── code/ (repo root dir)
                              └── grammars/
                                  └── lattice.tokens

  That is 6 levels up from the file's `__DIR__`.

  ## Usage

      {:ok, tokens} = CodingAdventures.LatticeLexer.tokenize("$color: red;")
      # tokens is a list of %Token{type, value, line, column}

  ## How It Works

  1. On first call, reads `lattice.tokens` and parses it into a `TokenGrammar`
     struct via `GrammarTools.parse_token_grammar/1`.
  2. Caches the grammar in `:persistent_term` for subsequent calls.
  3. Passes the source and grammar to `GrammarLexer.tokenize/2`, which does
     the real tokenization work using compiled regex patterns.

  The returned token list always ends with an `EOF` token.
  """

  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer

  # ---------------------------------------------------------------------------
  # Grammar File Location
  # ---------------------------------------------------------------------------
  #
  # __DIR__ is the directory containing THIS file:
  #   code/packages/elixir/lattice_lexer/lib/coding_adventures/
  #
  # We need to reach: code/grammars/
  #
  # Path traversal (counting ".." segments from __DIR__):
  #   1: lib/coding_adventures/ → lib/
  #   2: lib/ → lattice_lexer/
  #   3: lattice_lexer/ → elixir/
  #   4: elixir/ → packages/
  #   5: packages/ → code/  ← here we append "grammars"
  #
  # So: 5 ".." segments from __DIR__, then "grammars".

  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  @doc """
  Tokenize Lattice source code.

  This is the main entry point. Pass a string of Lattice source, get back
  a list of `%Token{}` structs. The list always ends with an `EOF` token.

  ## Parameters

  - `source` — the Lattice source text to tokenize

  ## Returns

  - `{:ok, tokens}` on success, where `tokens` is a list of `%Token{}`
  - `{:error, message}` on failure (unknown character, etc.)

  ## Examples

      {:ok, tokens} = CodingAdventures.LatticeLexer.tokenize("$color: red;")
      # [%Token{type: "VARIABLE", value: "$color"},
      #  %Token{type: "COLON", value: ":"},
      #  %Token{type: "IDENT", value: "red"},
      #  %Token{type: "SEMICOLON", value: ";"},
      #  %Token{type: "EOF", value: ""}]

  Lattice-specific tokens:

      {:ok, tokens} = CodingAdventures.LatticeLexer.tokenize("@if $x == 10")
      # includes EQUALS_EQUALS token

  Single-line comments are skipped:

      {:ok, tokens} = CodingAdventures.LatticeLexer.tokenize("// comment\\n$x: 1;")
      # comment tokens are suppressed; returns VARIABLE COLON NUMBER SEMICOLON EOF
  """
  @spec tokenize(String.t()) :: {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source) do
    grammar = get_grammar()
    GrammarLexer.tokenize(source, grammar)
  end

  @doc """
  Parse the `lattice.tokens` grammar file and return the `TokenGrammar`.

  This is the grammar struct used internally by `tokenize/1`. Exposed
  publicly for introspection, testing, and advanced use cases.

  ## Returns

  A `%TokenGrammar{}` struct, or raises if the grammar file cannot be read.
  """
  @spec create_lexer() :: TokenGrammar.t()
  def create_lexer do
    tokens_path = Path.join(@grammars_dir, "lattice.tokens")
    content = File.read!(tokens_path)
    # The lattice.tokens file contains an `errors:` section at the end
    # (for malformed strings / URLs). The Elixir TokenGrammar parser does not
    # support that directive, so we strip everything from the `errors:` line
    # onward before handing the text to TokenGrammar.parse/1.
    stripped = strip_errors_section(content)
    {:ok, grammar} = TokenGrammar.parse(stripped)
    grammar
  end

  # Strip the `errors:` block (and everything after it) from a .tokens file.
  # We truncate at the first line whose trimmed content equals "errors:" so
  # that BAD_STRING / BAD_URL patterns don't cause a parse failure.
  defp strip_errors_section(content) do
    content
    |> String.split("\n")
    |> Enum.take_while(fn line -> String.trim(line) != "errors:" end)
    |> Enum.join("\n")
  end

  # ---------------------------------------------------------------------------
  # Grammar Caching
  # ---------------------------------------------------------------------------
  #
  # Parsing the grammar file (reading, regex compilation, etc.) takes a small
  # but non-trivial amount of time. Since the grammar never changes during a
  # process lifetime, we cache it in :persistent_term — a BEAM-level key-value
  # store that's faster than ETS and doesn't require a GenServer.
  #
  # The cache key is {__MODULE__, :grammar} — scoped to this module to avoid
  # collisions with other modules using the same pattern.

  defp get_grammar do
    case :persistent_term.get({__MODULE__, :grammar}, nil) do
      nil ->
        grammar = create_lexer()
        :persistent_term.put({__MODULE__, :grammar}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
