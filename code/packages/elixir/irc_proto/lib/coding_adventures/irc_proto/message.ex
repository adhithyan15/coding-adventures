defmodule CodingAdventures.IrcProto.Message do
  @moduledoc """
  Structured representation of a single IRC protocol message.

  ## IRC message anatomy (RFC 1459 section 2.3)

  A raw IRC line looks like:

      [:prefix] command [param1 ... paramN] [\\r]\\n

  After parsing, each field maps to:

  | Wire field      | Struct field | Notes                                |
  |-----------------|--------------|--------------------------------------|
  | `:irc.local`    | `:prefix`    | Optional; nil when absent            |
  | `PRIVMSG`       | `:command`   | Always uppercase after parsing       |
  | `#chan :hello`  | `:params`    | List of strings; leading : stripped  |

  ## Usage

      iex> msg = %Message{command: "NICK", params: ["alice"]}
      iex> msg.command
      "NICK"
  """

  @enforce_keys [:command]
  defstruct [:prefix, :command, params: []]

  @typedoc """
  A parsed IRC message.

  - `:prefix`  -- optional server or nick!user@host prefix string.
  - `:command` -- the IRC verb (e.g. "NICK", "001"), always uppercase.
  - `:params`  -- list of parameter strings (trailing param already stripped of its leading ":").
  """
  @type t :: %__MODULE__{
          prefix: String.t() | nil,
          command: String.t(),
          params: [String.t()]
        }
end
