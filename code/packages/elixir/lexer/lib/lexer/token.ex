defmodule CodingAdventures.Lexer.Token do
  @moduledoc """
  A single token produced by the lexer.

  Every token records four things:

  1. **type** — What kind of token this is (e.g., "NUMBER", "STRING", "EOF").
     Unlike the Python lexer which uses a `TokenType` enum, Elixir tokens use
     plain strings. This is more natural in Elixir and avoids the need for
     enum-to-string mapping.

  2. **value** — The actual text that was matched from the source code.
     For STRING tokens, this is the content *after* removing quotes and
     processing escape sequences.

  3. **line** — The 1-based line number where the token starts.

  4. **column** — The 1-based column number where the token starts.
  """

  defstruct [:type, :value, :line, :column]

  @type t :: %__MODULE__{
          type: String.t(),
          value: String.t(),
          line: pos_integer(),
          column: pos_integer()
        }
end
