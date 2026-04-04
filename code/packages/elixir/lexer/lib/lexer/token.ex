defmodule CodingAdventures.Lexer.Token do
  @moduledoc """
  A single token produced by the lexer.

  Every token records four things (plus an optional fifth):

  1. **type** — What kind of token this is (e.g., "NUMBER", "STRING", "EOF").
     Unlike the Python lexer which uses a `TokenType` enum, Elixir tokens use
     plain strings. This is more natural in Elixir and avoids the need for
     enum-to-string mapping.

  2. **value** — The actual text that was matched from the source code.
     For STRING tokens, this is the content *after* removing quotes and
     processing escape sequences.

  3. **line** — The 1-based line number where the token starts.

  4. **column** — The 1-based column number where the token starts.

  5. **flags** — Optional bitmask carrying metadata about the token. When
     `nil`, all flags are off. Use bitwise AND to test individual flags:

         import Bitwise
         if (token.flags || 0) &&& Token.preceded_by_newline() != 0, do: ...

  ## Token Flag Constants

  Flags carry information that is neither type nor value but affects how
  downstream consumers (parsers, formatters, linters) interpret a token.

  - `preceded_by_newline/0` (bit 0, value 1) — Set when a line break
    appeared between this token and the previous one. Languages with
    automatic semicolon insertion (JavaScript, Go) use this to decide
    whether an implicit semicolon should be inserted.

  - `context_keyword/0` (bit 1, value 2) — Set for context-sensitive
    keywords: words that are keywords in some syntactic positions but
    identifiers in others. For example, JavaScript's `async`, `yield`,
    `await`, `get`, `set`. The lexer emits these as NAME tokens with
    this flag set, leaving the final keyword-vs-identifier decision to
    the language-specific parser.
  """

  defstruct [:type, :value, :line, :column, :flags]

  @type t :: %__MODULE__{
          type: String.t(),
          value: String.t(),
          line: pos_integer(),
          column: pos_integer(),
          flags: non_neg_integer() | nil
        }

  @doc """
  Flag bit set when a line break appeared between this token and the
  previous one.
  """
  @spec preceded_by_newline() :: 1
  def preceded_by_newline, do: 1

  @doc """
  Flag bit set for context-sensitive keywords — words that are keywords
  in some syntactic positions but identifiers in others.
  """
  @spec context_keyword() :: 2
  def context_keyword, do: 2
end
