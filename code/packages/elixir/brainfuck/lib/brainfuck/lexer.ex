defmodule CodingAdventures.Brainfuck.Lexer do
  @moduledoc """
  Brainfuck Lexer — Thin wrapper around the grammar-driven lexer engine.

  This module uses the pre-compiled `brainfuck.tokens` grammar (see
  `CodingAdventures.Brainfuck.Grammar.Tokens`) and `GrammarLexer.tokenize/2`
  to tokenize Brainfuck source code.

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

  1. On first call, `create_lexer/0` fetches the pre-compiled
     `TokenGrammar` struct from `CodingAdventures.Brainfuck.Grammar.Tokens`.
     This result is cached in `:persistent_term` so subsequent calls pay
     no recomputation cost.

  2. `tokenize/1` passes the source and grammar to `GrammarLexer.tokenize/2`,
     which does all the actual scanning work.
  """

  alias CodingAdventures.Brainfuck.Grammar.Tokens
  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer

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
      ["INC", "RIGHT", "DEC", "LEFT", "EOF"]

      iex> {:ok, tokens} = CodingAdventures.Brainfuck.Lexer.tokenize("hello world")
      iex> tokens
      [%Token{type: "EOF", ...}]
  """
  @spec tokenize(String.t()) :: {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source) do
    grammar = get_grammar()
    GrammarLexer.tokenize(source, grammar)
  end

  @doc """
  Return the pre-compiled `brainfuck.tokens` `TokenGrammar`.

  This is useful for inspecting the grammar or reusing it directly.
  For most callers, `tokenize/1` is the right entry point.
  """
  @spec create_lexer() :: TokenGrammar.t()
  def create_lexer do
    Tokens.token_grammar()
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
