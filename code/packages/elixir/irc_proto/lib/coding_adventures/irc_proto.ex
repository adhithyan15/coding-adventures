defmodule CodingAdventures.IrcProto do
  @moduledoc """
  Pure IRC message parsing and serialisation.

  This module converts between raw IRC text lines (RFC 1459 section 2.3) and
  structured `Message` values. It has no I/O, no processes, and no side effects.

  ## IRC message format

      [:prefix] command [param1 param2 ... [:trailing]]\\r\\n

  ### Prefix

  An optional sender tag starting with ":". After stripping the leading ":",
  the prefix is the server name or a nick!user@host mask.

  ### Command

  A verb like NICK, JOIN, PRIVMSG, or a 3-digit numeric like 001. Always
  normalised to uppercase by the parser.

  ### Parameters

  Up to 15 space-separated tokens. The last parameter may start with ":" on
  the wire (the "trailing" parameter), allowing it to contain spaces. The
  parser strips the ":" and returns the value as-is.

  ## Example

      iex> {:ok, msg} = IrcProto.parse("NICK alice")
      iex> msg.command
      "NICK"
      iex> msg.params
      ["alice"]

      iex> {:ok, msg} = IrcProto.parse(":irc.local 001 alice :Welcome to IRC!")
      iex> msg.prefix
      "irc.local"
      iex> msg.params
      ["alice", "Welcome to IRC!"]
  """

  alias CodingAdventures.IrcProto.Message

  @doc """
  Parse a raw IRC line into a `Message` struct.

  The input should be a single line without its trailing CRLF (the Framer
  in irc_framing strips the line terminator before calling here).

  ## Returns

  - `{:ok, Message.t()}` on success.
  - `{:error, reason}` if the line is empty or missing a command.

  ## Examples

      iex> IrcProto.parse("NICK alice")
      {:ok, %Message{prefix: nil, command: "NICK", params: ["alice"]}}

      iex> IrcProto.parse(":server 001 alice :Welcome!")
      {:ok, %Message{prefix: "server", command: "001", params: ["alice", "Welcome!"]}}

      iex> IrcProto.parse("")
      {:error, "empty line"}
  """
  @spec parse(String.t()) :: {:ok, Message.t()} | {:error, String.t()}
  def parse(line) when is_binary(line) do
    line = String.trim_trailing(line)

    if line == "" do
      {:error, "empty line"}
    else
      {prefix, rest} = extract_prefix(line)
      {command, rest2} = extract_command(rest)

      if command == "" do
        {:error, "missing command"}
      else
        params = collect_params(rest2, [])
        {:ok, %Message{prefix: prefix, command: String.upcase(command), params: params}}
      end
    end
  end

  @doc """
  Serialize a `Message` struct to a wire-format IRC line.

  The output always ends with \\r\\n (CRLF) as required by RFC 1459.
  The last parameter is prefixed with ":" on the wire only if it contains
  a space — otherwise it is written bare.

  ## Examples

      iex> IrcProto.serialize(%Message{command: "NICK", params: ["alice"]})
      "NICK alice\\r\\n"

      iex> IrcProto.serialize(%Message{prefix: "irc.local", command: "001",
      ...>   params: ["alice", "Welcome to IRC!"]})
      ":irc.local 001 alice :Welcome to IRC!\\r\\n"
  """
  @spec serialize(Message.t()) :: String.t()
  def serialize(%Message{prefix: prefix, command: command, params: params}) do
    prefix_part = if prefix, do: [":#{prefix}"], else: []

    param_parts =
      case params do
        [] ->
          []

        _ ->
          {init, [last]} = Enum.split(params, length(params) - 1)
          trailing = if String.contains?(last, " "), do: ":#{last}", else: last
          init ++ [trailing]
      end

    all_parts = prefix_part ++ [command] ++ param_parts
    Enum.join(all_parts, " ") <> "\r\n"
  end

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  # Extract optional prefix: if line starts with ":", everything before
  # the first space is the prefix. Returns {prefix_or_nil, rest}.
  defp extract_prefix(":" <> rest) do
    case :binary.match(rest, " ") do
      {pos, _len} ->
        prefix = binary_part(rest, 0, pos)
        remainder = binary_part(rest, pos + 1, byte_size(rest) - pos - 1)
        {prefix, String.trim_leading(remainder)}

      :nomatch ->
        # Entire line is the prefix, no command follows.
        {rest, ""}
    end
  end

  defp extract_prefix(line), do: {nil, line}

  # Extract the command token — everything up to the first space.
  # Returns {command, rest}.
  defp extract_command(""), do: {"", ""}

  defp extract_command(line) do
    case :binary.match(line, " ") do
      {pos, _len} ->
        cmd = binary_part(line, 0, pos)
        rest = String.trim_leading(binary_part(line, pos + 1, byte_size(line) - pos - 1))
        {cmd, rest}

      :nomatch ->
        {line, ""}
    end
  end

  # Recursively collect parameters. A ":" prefix means consume the rest
  # of the line as a single (trailing) parameter. Maximum 15 params (RFC 1459).
  defp collect_params("", acc), do: Enum.reverse(acc)
  defp collect_params(_, acc) when length(acc) >= 15, do: Enum.reverse(acc)

  defp collect_params(":" <> trailing, acc) do
    Enum.reverse([trailing | acc])
  end

  defp collect_params(line, acc) do
    case :binary.match(line, " ") do
      {pos, _len} ->
        param = binary_part(line, 0, pos)
        rest = String.trim_leading(binary_part(line, pos + 1, byte_size(line) - pos - 1))
        collect_params(rest, [param | acc])

      :nomatch ->
        Enum.reverse([line | acc])
    end
  end
end
