defmodule CodingAdventures.Brainfuck.Lexer do
  @moduledoc """
  Brainfuck Lexer — Thin wrapper around the grammar-driven lexer engine.

  This module reads `brainfuck.tokens` from the shared grammars directory
  and uses `GrammarLexer.tokenize/2` to tokenize Brainfuck source code.

  ## What is Brainfuck tokenization?

  Brainfuck has exactly 8 meaningful characters:

  | Character | Token type  | Meaning                            |
  |-----------|-------------|------------------------------------|
  | `>`       | RIGHT       | move data pointer right            |
  | `<`       | LEFT        | move data pointer left             |
  | `+`       | INC         | increment current cell             |
  | `-`       | DEC         | decrement current cell             |
  | `.`       | OUTPUT      | write current cell to stdout       |
  | `,`       | INPUT       | read one byte from stdin           |
  | `[`       | LOOP_START  | jump forward if cell is zero       |
  | `]`       | LOOP_END    | jump back if cell is nonzero       |

  Every other character — letters, digits, spaces, punctuation — is a
  comment. The `brainfuck.tokens` grammar file puts these in its `skip:`
  section, so they are consumed silently and never appear in the token
  stream.

  This design keeps the parser grammar clean: every token the parser
  sees is a command.

  ## Usage

      {:ok, tokens} = CodingAdventures.Brainfuck.Lexer.tokenize("++[>+<-]")

  ## How It Works

  1. On first call, `create_lexer/0` parses `brainfuck.tokens` into a
     `TokenGrammar` struct. This result is cached in `:persistent_term`
     so subsequent calls pay no file I/O cost.

  2. `tokenize/1` passes the source and grammar to `GrammarLexer.tokenize/2`,
     which does all the actual scanning work.
  """

  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer

  # Path to the shared grammars directory.
  #
  # The directory layout (from this file's location outward):
  #
  #   code/
  #     grammars/
  #       brainfuck.tokens   <-- we need this
  #     packages/
  #       elixir/
  #         brainfuck/
  #           lib/
  #             brainfuck/
  #               lexer.ex   <-- __DIR__ points here
  #
  # Navigating up from __DIR__:
  #   1. brainfuck/   (lib/brainfuck -> lib)
  #   2. lib/
  #   3. brainfuck/   (elixir/brainfuck -> elixir)
  #   4. elixir/
  #   (4 levels up from __DIR__ reaches code/)
  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  @doc """
  Tokenize Brainfuck source code into a flat token stream.

  Returns `{:ok, tokens}` on success, `{:error, message}` on failure.
  Each token is a `%Token{type, value, line, column}` struct.

  Comments (all non-command characters) are silently discarded by the
  lexer's `skip:` mechanism. The returned list contains only the 8
  command token types plus a terminal EOF token.

  ## Examples

      iex> {:ok, tokens} = CodingAdventures.Brainfuck.Lexer.tokenize("+>-<")
      iex> Enum.map(tokens, & &1.type)
      ["INC", "RIGHT", "DEC", "LEFT", :eof]

      iex> {:ok, tokens} = CodingAdventures.Brainfuck.Lexer.tokenize("hello world")
      iex> tokens
      [%Token{type: :eof, ...}]
  """
  @spec tokenize(String.t()) :: {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source) do
    grammar = get_grammar()
    GrammarLexer.tokenize(source, grammar)
  end

  @doc """
  Parse the `brainfuck.tokens` grammar file and return the `TokenGrammar`.

  This is useful for inspecting the grammar or reusing it directly.
  For most callers, `tokenize/1` is the right entry point.
  """
  @spec create_lexer() :: TokenGrammar.t()
  def create_lexer do
    tokens_path = Path.join(@grammars_dir, "brainfuck.tokens")
    {:ok, grammar} = TokenGrammar.parse(File.read!(tokens_path))
    grammar
  end

  # Retrieve the cached TokenGrammar, building and caching it on first access.
  #
  # `:persistent_term` gives O(1) read performance with no locking. The
  # grammar never changes at runtime, so we only pay the parse cost once.
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
