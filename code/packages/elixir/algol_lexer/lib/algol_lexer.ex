defmodule CodingAdventures.AlgolLexer do
  @moduledoc """
  ALGOL 60 Lexer — Thin wrapper around the grammar-driven lexer engine.

  ## A Brief History

  ALGOL 60 (ALGOrithmic Language, 1960) was a landmark in programming language
  design. It was the first language formally specified using BNF (Backus-Naur
  Form), and it introduced ideas that every modern programmer takes for granted:

  - **Block structure** — `begin ... end` creates a nested lexical scope.
  - **Lexical scoping** — inner blocks can read variables from outer blocks.
  - **Recursion** — a procedure may call itself; the call stack was invented
    to support ALGOL 60.
  - **Free-format source** — whitespace is insignificant between tokens, unlike
    Fortran 77 and COBOL, which were tied to punched-card column conventions.

  ALGOL 60 never became commercially dominant (IBM backed Fortran instead), but
  it became the teaching language of choice and the ancestral template for Pascal,
  C, Ada, and Simula (the first object-oriented language). If you have ever
  written `if`, `else`, `for`, `while`, `begin`, or `end`, you are using
  vocabulary coined by ALGOL 60.

  ## Token Highlights

  | Token kind   | Lexeme example | Notes                                       |
  |-------------|----------------|---------------------------------------------|
  | `ASSIGN`    | `:=`           | Walrus — not `=`, which means equality      |
  | `POWER`     | `**`           | Also `^` (CARET); both mean exponentiation  |
  | `LEQ`       | `<=`           | ≤ in the original mathematical typesetting  |
  | `GEQ`       | `>=`           | ≥                                           |
  | `NEQ`       | `!=`           | ≠                                           |
  | `EQ`        | `=`            | Equality test, never assignment             |
  | `INTEGER_LIT` | `42`         | Plain decimal digits                        |
  | `REAL_LIT`  | `3.14`, `1.5E3` | Decimal or scientific notation             |
  | `STRING_LIT` | `'hello'`     | Single-quoted, no escape sequences          |
  | `IDENT`     | `myVar`        | Letter then letters/digits; no underscore   |
  | `comment`   | (skipped)      | `comment ... ;` consumed silently           |

  ## Usage

      {:ok, tokens} = CodingAdventures.AlgolLexer.tokenize("begin integer x; x := 42 end")
      # => [%Token{type: "begin"}, %Token{type: "integer"}, ...]

  ## How It Works

  1. `create_lexer/0` reads `algol.tokens` from the shared grammars directory
     and parses it into a `TokenGrammar` struct.

  2. `tokenize/1` passes the source and grammar to `GrammarLexer.tokenize/2`,
     which does the actual work: scanning the source character by character,
     matching token patterns in priority order, reclassifying `IDENT` tokens
     that match keywords, and skipping whitespace and comment blocks.

  3. The grammar is cached in a `persistent_term` so that repeated calls pay
     the file-read cost only once.
  """

  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer

  # Path to the shared grammars directory, resolved at compile time relative
  # to this source file. Walking up four levels: lib → algol_lexer → elixir
  # → packages → code, then into grammars.
  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  @doc """
  Tokenize ALGOL 60 source code.

  Returns `{:ok, tokens}` on success, where `tokens` is a list of
  `%Token{type, value, line, column}` structs terminated by an `EOF` token.
  Returns `{:error, message}` if an unexpected character is encountered.

  ## Examples

      iex> {:ok, tokens} = AlgolLexer.tokenize("x := 1")
      iex> Enum.map(tokens, & &1.type)
      ["NAME", "ASSIGN", "INTEGER_LIT", "EOF"]

  """
  @spec tokenize(String.t()) :: {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source) do
    grammar = get_grammar()
    # Post-tokenize hook: the generic GrammarLexer emits "KEYWORD" as the type
    # for all keyword tokens (begin, end, integer, etc.).  ALGOL 60 has many
    # distinct keywords and callers expect the specific keyword name as the type
    # (e.g. type "begin" not "KEYWORD").  We reclassify here so that the public
    # API of AlgolLexer matches the per-language convention of returning the
    # lowercase keyword word as the token type.
    GrammarLexer.tokenize(source, grammar,
      post_tokenize_hooks: [
        fn tokens ->
          Enum.map(tokens, fn
            %{type: "KEYWORD"} = tok -> %{tok | type: String.downcase(tok.value)}
            tok -> tok
          end)
        end
      ]
    )
  end

  @doc """
  Parse the `algol.tokens` grammar file and return the `TokenGrammar`.

  Useful for inspecting the set of token definitions or for passing the
  grammar directly to `GrammarLexer.tokenize/2` in performance-sensitive
  code that already holds a grammar reference.
  """
  @spec create_lexer() :: TokenGrammar.t()
  def create_lexer do
    tokens_path = Path.join(@grammars_dir, "algol.tokens")
    {:ok, grammar} = TokenGrammar.parse(File.read!(tokens_path))
    grammar
  end

  # Cache the parsed grammar in a persistent_term so the file is read and
  # parsed only once per BEAM node lifetime. persistent_term is faster than
  # ETS for read-heavy, write-once data because the value lives directly in
  # the process heap on read (no copy needed for immutable terms).
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
