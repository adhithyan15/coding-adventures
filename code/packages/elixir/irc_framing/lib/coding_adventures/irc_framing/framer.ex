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

  The struct has three fields:

  - `:buf` — a binary accumulating bytes received so far.
  - `:tail_len` — how many bytes at the end of `:buf` follow the last LF, that
    is, the line still waiting for its terminator.
  - `:discarding` — `true` while the rest of an overlong line is being skipped.

  ## Usage

      framer = Framer.new()
      framer = Framer.feed(framer, raw_bytes_from_tcp)
      {framer, lines} = Framer.frames(framer)
      # lines is a list of complete IRC lines, stripped of their terminators.

  ## RFC 1459 length limit

  IRC messages (including the CRLF) must not exceed 512 bytes (RFC 1459 and
  RFC 2812 section 2.3). Lines longer than 510 bytes (the content limit,
  excluding CRLF) are silently discarded.

  ## Memory bound (SEC-011, CWE-400/770)

  The 510-byte rule above is only applied to lines that have already ended in
  LF, so on its own it does **not** stop a peer that never sends LF from growing
  the buffer without limit. `feed/2` therefore also caps the *unterminated
  tail* (the bytes after the last LF) at 511 bytes: 510 content bytes plus a CR
  whose LF has not arrived yet. One more byte with no LF and the line cannot be
  legal, so the partial line is dropped and the framer discards input up to and
  including the next LF (the `:discarding` state, which keeps no data). Memory
  stays constant however much is fed. This is a fail-closed, silent discard like
  the one for complete overlong lines, so the API is unchanged. Complete lines
  already buffered are kept; callers bound those by calling `frames/1` after
  each read. `reset/1` clears the discard state.
  """

  @max_line_bytes 510

  # The most bytes we hold for one line that has not yet seen its LF. A legal
  # line can be 510 content bytes plus a CR whose LF has not arrived yet, i.e.
  # 511 bytes. One more byte with no LF and the line can no longer be legal
  # (SEC-011), so we stop remembering it.
  @max_pending_bytes @max_line_bytes + 1

  @enforce_keys [:buf]
  defstruct [:buf, tail_len: 0, discarding: false]

  @typedoc """
  A Framer struct.

  - `:buf` — binary accumulating unprocessed bytes.
  - `:tail_len` — length of the unterminated tail of `:buf` (bytes after the
    last LF). Invariant: at most 511 after every `feed/2`.
  - `:discarding` — `true` while skipping the rest of an overlong line.
  """
  @type t :: %__MODULE__{buf: binary(), tail_len: non_neg_integer(), discarding: boolean()}

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

  The unterminated tail (bytes after the last LF) never exceeds 511 bytes; see
  the module documentation (SEC-011). Past that the partial line is dropped and
  input is skipped up to the next LF.

  ## Example

      iex> framer = Framer.new()
      iex> framer = Framer.feed(framer, "NICK alice")
      iex> framer.buf
      "NICK alice"
  """
  @spec feed(t(), binary()) :: t()
  def feed(%__MODULE__{} = framer, bytes) when is_binary(bytes) do
    cond do
      byte_size(bytes) == 0 ->
        framer

      # Step 1: while discarding an overlong line, skip to just past its LF.
      framer.discarding ->
        case :binary.match(bytes, "\n") do
          # Still inside the overlong line: keep nothing.
          :nomatch ->
            framer

          {pos, 1} ->
            rest = binary_part(bytes, pos + 1, byte_size(bytes) - pos - 1)
            store(%{framer | discarding: false}, rest)
        end

      true ->
        store(framer, bytes)
    end
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
    # Only the unterminated tail remains in the buffer.
    {%{framer | buf: remaining, tail_len: byte_size(remaining)}, lines}
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
  def reset(%__MODULE__{} = framer), do: %{framer | buf: <<>>, tail_len: 0, discarding: false}

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

  # Step 2 and 3 of feed/2. Split *data* at its last LF: the head (through that
  # LF) completes lines and is stored; only the tail can be flooded. The tail is
  # stored only if the pending line still fits, and the check happens *before*
  # the append so a huge single feed is never copied.
  #
  #     data = [ ...complete lines... LF ][ unterminated tail ]
  #                                       ^ only this part can be flooded
  defp store(%__MODULE__{buf: buf, tail_len: tail_len} = framer, data) do
    head_len = last_lf_end(data)
    tail_size = byte_size(data) - head_len

    {buf, tail_len} =
      if head_len > 0 do
        {buf <> binary_part(data, 0, head_len), 0}
      else
        {buf, tail_len}
      end

    if tail_len + tail_size > @max_pending_bytes do
      # Drop the partial line and skip to the next LF.
      kept = binary_part(buf, 0, byte_size(buf) - tail_len)
      %{framer | buf: kept, tail_len: 0, discarding: true}
    else
      tail = binary_part(data, head_len, tail_size)
      %{framer | buf: buf <> tail, tail_len: tail_len + tail_size}
    end
  end

  # Number of bytes of *data* up to and including its last LF (0 if none).
  defp last_lf_end(data) do
    case :binary.match(data, "\n") do
      :nomatch -> 0
      {_, 1} -> scan_back(data, byte_size(data))
    end
  end

  defp scan_back(_data, 0), do: 0

  defp scan_back(data, n) do
    if :binary.at(data, n - 1) == ?\n, do: n, else: scan_back(data, n - 1)
  end

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
