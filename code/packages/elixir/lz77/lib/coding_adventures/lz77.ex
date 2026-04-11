defmodule CodingAdventures.LZ77 do
  @moduledoc """
  LZ77 lossless compression algorithm (Lempel & Ziv, 1977).

  ## What Is LZ77?

  LZ77 replaces repeated byte sequences with compact backreferences into a
  sliding window of recently seen data. It is the foundation of DEFLATE,
  gzip, PNG, and zlib.

  ## The Sliding Window Model

      ┌─────────────────────────────────┬──────────────────┐
      │         SEARCH BUFFER           │ LOOKAHEAD BUFFER  │
      │  (already processed — the       │  (not yet seen —  │
      │   last window_size bytes)       │  next max_match)  │
      └─────────────────────────────────┴──────────────────┘
                                         ↑
                                     cursor (current position)

  At each step the encoder finds the longest match in the search buffer. If
  found and long enough (≥ min_match), emit a backreference token. Otherwise
  emit a literal token.

  ## Token: {offset, length, next_char}

  - `offset`:    distance back the match starts (1..window_size), or 0.
  - `length`:    number of bytes the match covers (0 = literal).
  - `next_char`: literal byte immediately after the match (0..255).

  ## Overlapping Matches

  When `offset < length`, the match extends into bytes not yet decoded. The
  decoder must copy byte-by-byte (not bulk copy) to handle this correctly.

  ## The Series: CMP00 → CMP05

  - CMP00 (LZ77, 1977) — Sliding-window backreferences. This module.
  - CMP01 (LZ78, 1978) — Explicit dictionary (trie), no sliding window.
  - CMP02 (LZSS, 1982) — LZ77 + flag bits; eliminates wasted literals.
  - CMP03 (LZW,  1984) — Pre-initialized dictionary; powers GIF.
  - CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
  - CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
  """

  @typedoc "A single LZ77 token: {offset, length, next_char}"
  @type token :: %{offset: non_neg_integer(), length: non_neg_integer(), next_char: byte()}

  @doc """
  Creates a token map.

  ## Examples

      iex> CodingAdventures.LZ77.token(0, 0, 65)
      %{offset: 0, length: 0, next_char: 65}
  """
  def token(offset, length, next_char) do
    %{offset: offset, length: length, next_char: next_char}
  end

  # Finds the longest match in the search buffer.
  #
  # Scans the last window_size bytes before cursor for the longest substring
  # that matches the start of the lookahead buffer. Returns {offset, length}.
  defp find_longest_match(data, cursor, window_size, max_match) do
    data_len = byte_size(data)
    search_start = max(0, cursor - window_size)
    # Reserve 1 byte for next_char.
    lookahead_end = min(cursor + max_match, data_len - 1)

    # Try every position in the search buffer.
    Enum.reduce(search_start..(cursor - 1)//1, {0, 0}, fn pos, {best_off, best_len} ->
      # Count matching bytes from pos and cursor, allowing overlap.
      len = count_match(data, pos, cursor, lookahead_end)
      if len > best_len do
        {cursor - pos, len}
      else
        {best_off, best_len}
      end
    end)
  end

  # Counts matching bytes between positions pos and cursor, up to lookahead_end.
  # Matches may overlap (the match from cursor can extend past cursor into
  # bytes that will be decoded).
  defp count_match(data, pos, cursor, lookahead_end) do
    count_match_loop(data, pos, cursor, lookahead_end, 0)
  end

  defp count_match_loop(data, pos, cursor, lookahead_end, length) do
    if cursor + length < lookahead_end and
         binary_part(data, pos + length, 1) == binary_part(data, cursor + length, 1) do
      count_match_loop(data, pos, cursor, lookahead_end, length + 1)
    else
      length
    end
  end

  @doc """
  Encodes binary data into an LZ77 token stream.

  ## Parameters

  - `data`        — input binary.
  - `window_size` — maximum lookback distance (default 4096).
  - `max_match`   — maximum match length (default 255).
  - `min_match`   — minimum length for a backreference (default 3).

  ## Examples

      iex> tokens = CodingAdventures.LZ77.encode("ABCDE")
      iex> length(tokens)
      5
      iex> Enum.all?(tokens, fn t -> t.offset == 0 and t.length == 0 end)
      true

      iex> CodingAdventures.LZ77.decode(CodingAdventures.LZ77.encode("ABABABAB"))
      "ABABABAB"
  """
  def encode(data, window_size \\ 4096, max_match \\ 255, min_match \\ 3) do
    data_len = byte_size(data)
    encode_loop(data, data_len, 0, window_size, max_match, min_match, [])
  end

  defp encode_loop(_data, data_len, cursor, _window_size, _max_match, _min_match, tokens)
       when cursor >= data_len do
    Enum.reverse(tokens)
  end

  defp encode_loop(data, data_len, cursor, window_size, max_match, min_match, tokens) do
    # Edge case: last byte has no room for next_char after a match.
    if cursor == data_len - 1 do
      byte = :binary.at(data, cursor)
      encode_loop(data, data_len, cursor + 1, window_size, max_match, min_match, [
        token(0, 0, byte) | tokens
      ])
    else
      {offset, length} = find_longest_match(data, cursor, window_size, max_match)

      if length >= min_match do
        # Emit a backreference token.
        next_char = :binary.at(data, cursor + length)
        new_token = token(offset, length, next_char)
        encode_loop(data, data_len, cursor + length + 1, window_size, max_match, min_match, [
          new_token | tokens
        ])
      else
        # Emit a literal token.
        byte = :binary.at(data, cursor)
        encode_loop(data, data_len, cursor + 1, window_size, max_match, min_match, [
          token(0, 0, byte) | tokens
        ])
      end
    end
  end

  @doc """
  Decodes an LZ77 token stream back into the original data.

  ## Parameters

  - `tokens`         — the token stream (output of `encode/4`).
  - `initial_buffer` — optional seed for the search buffer (default `""`).

  ## Examples

      iex> CodingAdventures.LZ77.decode([])
      ""

      iex> tokens = [
      ...>   %{offset: 0, length: 0, next_char: 65},
      ...>   %{offset: 0, length: 0, next_char: 66},
      ...>   %{offset: 2, length: 5, next_char: 90}
      ...> ]
      iex> CodingAdventures.LZ77.decode(tokens)
      "ABABABAZ"
  """
  def decode(tokens, initial_buffer \\ "") do
    output = :binary.bin_to_list(initial_buffer)
    result = Enum.reduce(tokens, output, &decode_token/2)
    :binary.list_to_bin(result)
  end

  # Applies one token to the output accumulator.
  defp decode_token(%{offset: 0, length: 0, next_char: byte}, output) do
    output ++ [byte]
  end

  defp decode_token(%{offset: offset, length: length, next_char: next_char}, output) do
    start = length(output) - offset
    # Copy length bytes byte-by-byte to handle overlapping matches.
    copied = copy_bytes(output, start, length, output)
    copied ++ [next_char]
  end

  # Copies `count` bytes starting at position `start` in the buffer.
  # The buffer is passed as `acc` and grows as we copy (overlapping support).
  defp copy_bytes(_original, _start, 0, acc), do: acc

  defp copy_bytes(original, start, count, acc) do
    # Use the growing `acc` list so that overlapping reads work.
    byte = Enum.at(acc, start)
    copy_bytes(original, start + 1, count - 1, acc ++ [byte])
  end

  @doc """
  Serialises a token list to bytes using a fixed-width format.

  Format:
  - 4 bytes: token count (big-endian uint32)
  - N × 4 bytes: each token as `(offset: uint16 BE, length: uint8, next_char: uint8)`
  """
  def serialise_tokens(tokens) do
    count = length(tokens)
    header = <<count::unsigned-big-32>>

    body =
      Enum.reduce(tokens, <<>>, fn t, acc ->
        acc <> <<t.offset::unsigned-big-16, t.length::unsigned-8, t.next_char::unsigned-8>>
      end)

    header <> body
  end

  @doc """
  Deserialises bytes back into a token list.

  Inverse of `serialise_tokens/1`.
  """
  def deserialise_tokens(<<count::unsigned-big-32, rest::binary>>) do
    deserialise_tokens_loop(rest, count, [])
  end

  def deserialise_tokens(_), do: []

  defp deserialise_tokens_loop(_data, 0, acc), do: Enum.reverse(acc)

  defp deserialise_tokens_loop(
         <<offset::unsigned-big-16, length::unsigned-8, next_char::unsigned-8, rest::binary>>,
         remaining,
         acc
       ) do
    deserialise_tokens_loop(rest, remaining - 1, [token(offset, length, next_char) | acc])
  end

  defp deserialise_tokens_loop(_, _, acc), do: Enum.reverse(acc)

  @doc """
  Compresses data using LZ77.

  One-shot API: `encode/4` then serialise the token stream to bytes.

  ## Examples

      iex> CodingAdventures.LZ77.decompress(CodingAdventures.LZ77.compress("AAAAAAA"))
      "AAAAAAA"
  """
  def compress(data, window_size \\ 4096, max_match \\ 255, min_match \\ 3) do
    tokens = encode(data, window_size, max_match, min_match)
    serialise_tokens(tokens)
  end

  @doc """
  Decompresses data that was compressed with `compress/4`.

  Deserialises the byte stream into tokens, then decodes.
  """
  def decompress(data) do
    tokens = deserialise_tokens(data)
    decode(tokens)
  end
end
