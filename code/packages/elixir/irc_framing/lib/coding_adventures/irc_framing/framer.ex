defmodule CodingAdventures.IrcFraming.Framer do
  @moduledoc """
  Reassembles a raw TCP byte stream into complete IRC lines.

  ## The framing problem

  IRC uses CRLF-terminated text lines (RFC 1459 section 2.3). TCP, however,
  delivers an arbitrary byte stream — a single recv() call may return half a
  line, one complete line, or five lines concatenated together. We need to
  buffer incoming bytes and extract complete lines on demand.

  ## Design

  `Framer` is a pure value struct. State comes in, new state + lines go out.
  There are no processes, no GenServers, no side effects.

  The struct has a single field:

  - `:buf` — a binary accumulating bytes received so far.

  ## Usage

      framer = Framer.new()
      framer = Framer.feed(framer, raw_bytes_from_tcp)
      {framer, lines} = Framer.frames(framer)
      # lines is a list of complete IRC lines, stripped of their terminators.

  ## RFC 1459 length limit

  IRC messages (including the CRLF) must not exceed 512 bytes. Lines longer
  than 510 bytes (the content limit, excluding CRLF) are silently discarded.
  This prevents memory exhaustion from a slow-drip flood of non-terminated data.
  """

  @max_line_bytes 510

  @enforce_keys [:buf]
  defstruct [:buf]

  @typedoc """
  A Framer struct.

  - `:buf` — binary accumulating unprocessed bytes.
  """
  @type t :: %__MODULE__{buf: binary()}

  @doc """
  Create a new, empty Framer.

  ## Example

      iex> framer = Framer.new()
      iex> framer.buf
      <<>>
  """
  @spec new() :: t()
  def new, do: %__MODULE__{buf: <<>>}

  @doc """
  Append *bytes* to the framer's internal buffer.

  This is a no-op if *bytes* is empty.

  ## Example

      iex> framer = Framer.new()
      iex> framer = Framer.feed(framer, "NICK alice")
      iex> framer.buf
      "NICK alice"
  """
  @spec feed(t(), binary()) :: t()
  def feed(%__MODULE__{buf: buf} = framer, bytes) when is_binary(bytes) do
    if byte_size(bytes) == 0, do: framer, else: %{framer | buf: buf <> bytes}
  end

  @doc """
  Extract all complete lines from the buffer.

  Returns `{updated_framer, lines}` where:
  - `updated_framer` has the remaining partial line (if any) in `:buf`.
  - `lines` is a list of strings with the CRLF terminator stripped.

  Both CRLF (`\\r\\n`) and bare LF (`\\n`) terminators are accepted. Lines
  longer than 510 bytes are silently discarded.

  ## Example

      iex> framer = Framer.new()
      iex> framer = Framer.feed(framer, "NICK alice\\r\\nJOIN #chan\\r\\n")
      iex> {_framer, lines} = Framer.frames(framer)
      iex> lines
      ["NICK alice", "JOIN #chan"]
  """
  @spec frames(t()) :: {t(), [String.t()]}
  def frames(%__MODULE__{buf: buf} = framer) do
    {remaining, lines} = extract_frames(buf, [])
    {%{framer | buf: remaining}, lines}
  end

  @doc """
  Clear the framer's buffer.

  Discards all buffered data. Useful when a connection resets.

  ## Example

      iex> framer = Framer.new() |> Framer.feed("partial")
      iex> framer = Framer.reset(framer)
      iex> framer.buf
      <<>>
  """
  @spec reset(t()) :: t()
  def reset(%__MODULE__{} = framer), do: %{framer | buf: <<>>}

  @doc """
  Return the current buffer size in bytes.

  ## Example

      iex> framer = Framer.new() |> Framer.feed("NICK alice")
      iex> Framer.buffer_size(framer)
      10
  """
  @spec buffer_size(t()) :: non_neg_integer()
  def buffer_size(%__MODULE__{buf: buf}), do: byte_size(buf)

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  # Recursively scan *buf* for LF characters and extract complete lines.
  # Returns {remaining_buf, reversed_list_of_lines}.
  defp extract_frames(buf, acc) do
    case :binary.match(buf, "\n") do
      :nomatch ->
        # No complete line yet — return the buffer as-is.
        {buf, Enum.reverse(acc)}

      {lf_pos, 1} ->
        # Found a LF at position lf_pos. The line is everything before it.
        raw_line = binary_part(buf, 0, lf_pos)

        # Strip optional trailing CR to handle both CRLF and LF terminators.
        line = if String.ends_with?(raw_line, "\r"), do: String.slice(raw_line, 0..-2//1), else: raw_line

        # Discard lines exceeding the RFC 1459 length limit.
        rest = binary_part(buf, lf_pos + 1, byte_size(buf) - lf_pos - 1)

        if byte_size(line) > @max_line_bytes do
          # Silently drop the overlong line and continue processing.
          extract_frames(rest, acc)
        else
          extract_frames(rest, [line | acc])
        end
    end
  end
end
