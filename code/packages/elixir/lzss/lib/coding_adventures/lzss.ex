defmodule CodingAdventures.LZSS do
  import Bitwise

  @moduledoc """
  LZSS lossless compression algorithm (1982) — CMP02.

  LZSS (Lempel-Ziv-Storer-Szymanski) refines LZ77 by eliminating the mandatory
  `next_char` byte appended after every token. Instead, a flag-bit scheme
  distinguishes literals from back-references:

  - Literal → 1 byte (flag bit = 0)
  - Match   → 3 bytes (flag bit = 1: offset uint16 BE + length uint8)

  Tokens are grouped in blocks of 8. Each block starts with a flag byte
  (bit 0 = first token, bit 7 = eighth token).

  ## Wire Format (CMP02)

      Bytes 0-3:  original_length (big-endian uint32)
      Bytes 4-7:  block_count     (big-endian uint32)
      Bytes 8+:   blocks
        Each block: [1-byte flag] [1 or 3 bytes per symbol]

  ## Series

      CMP00 (LZ77, 1977) — Sliding-window backreferences.
      CMP01 (LZ78, 1978) — Explicit dictionary (trie).
      CMP02 (LZSS, 1982) — LZ77 + flag bits. ← this module
      CMP03 (LZW,  1984) — LZ78 + pre-initialised alphabet; GIF.
      CMP04 (Huffman, 1952) — Entropy coding.
      CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.

  ## Examples

      iex> data = "hello hello hello"
      iex> CodingAdventures.LZSS.decompress(CodingAdventures.LZSS.compress(data)) == data
      true
  """

  # ─── Token constructors ─────────────────────────────────────────────────────

  @doc "Create a %{kind: :literal, byte: byte} token."
  def literal(byte), do: %{kind: :literal, byte: byte}

  @doc "Create a %{kind: :match, offset: offset, length: length} token."
  def match(offset, length), do: %{kind: :match, offset: offset, length: length}

  # ─── Default parameters ─────────────────────────────────────────────────────

  @default_window_size 4096
  @default_max_match   255
  @default_min_match   3

  # ─── Encoder ────────────────────────────────────────────────────────────────

  @doc """
  Encode a binary into an LZSS token list.

  ## Parameters

  - `data`        — binary input.
  - `window_size` — max lookback distance (default 4096).
  - `max_match`   — max match length (default 255).
  - `min_match`   — min match length for a Match token (default 3).

  ## Examples

      iex> tokens = CodingAdventures.LZSS.encode("ABABAB")
      iex> length(tokens)
      3
  """
  def encode(data, window_size \\ @default_window_size,
                   max_match   \\ @default_max_match,
                   min_match   \\ @default_min_match)
      when is_binary(data) do
    bytes  = :binary.bin_to_list(data)
    encode_loop(bytes, 0, bytes, window_size, max_match, min_match, [])
  end

  defp encode_loop([], _cursor, _data, _ws, _mm, _min, acc), do: Enum.reverse(acc)

  defp encode_loop([_ | rest_bytes] = _remaining, cursor, data, ws, mm, min, acc) do
    win_start = max(0, cursor - ws)
    {off, len} = find_longest_match(data, cursor, win_start, mm)

    if len >= min do
      tok = match(off, len)
      # Advance cursor by len positions in the original byte list.
      new_remaining = Enum.drop(rest_bytes, len - 1)
      encode_loop(new_remaining, cursor + len, data, ws, mm, min, [tok | acc])
    else
      tok = literal(Enum.at(data, cursor))
      encode_loop(rest_bytes, cursor + 1, data, ws, mm, min, [tok | acc])
    end
  end

  # Find the longest match for data[cursor:] in data[win_start:cursor].
  # Returns {best_offset, best_length}. Matches may overlap.
  defp find_longest_match(data, cursor, win_start, max_match) do
    lookahead_end = min(cursor + max_match, length(data))
    data_arr = :array.from_list(data)

    Enum.reduce(win_start..(cursor - 1)//1, {0, 0}, fn pos, {best_off, best_len} ->
      len = count_match(data_arr, pos, cursor, lookahead_end, 0)
      if len > best_len, do: {cursor - pos, len}, else: {best_off, best_len}
    end)
  end

  defp count_match(_arr, _pos, cursor, lookahead_end, acc) when cursor + acc >= lookahead_end, do: acc
  defp count_match(arr, pos, cursor, lookahead_end, acc) do
    if :array.get(pos + acc, arr) == :array.get(cursor + acc, arr) do
      count_match(arr, pos, cursor, lookahead_end, acc + 1)
    else
      acc
    end
  end

  # ─── Decoder ────────────────────────────────────────────────────────────────

  @doc """
  Decode an LZSS token list back into the original bytes.

  ## Parameters

  - `tokens`          — token list from encode/1.
  - `original_length` — if an integer, truncates output; :all returns everything.

  ## Examples

      iex> tokens = CodingAdventures.LZSS.encode("hello")
      iex> CodingAdventures.LZSS.decode(tokens, 5) == "hello"
      true
  """
  def decode(tokens, original_length \\ :all) do
    output = decode_loop(tokens, [], original_length)
    :erlang.list_to_binary(output)
  end

  defp decode_loop([], output, _), do: Enum.reverse(output)

  defp decode_loop([tok | rest], output, orig_len) do
    new_output =
      case tok do
        %{kind: :literal, byte: b} ->
          [b | output]

        %{kind: :match, offset: off, length: len} ->
          # Guard against malformed tokens: offset=0 or offset > output length
          # would yield an invalid start_idx, causing nil reads and ArgumentError.
          current_len = length(output)
          if off < 1 or off > current_len do
            output  # skip invalid match token
          else
            start_idx = current_len - off
            # Output is reversed; index from the tail.
            copy_from_output(output, start_idx, current_len, len, output)
          end
      end

    case orig_len do
      :all -> decode_loop(rest, new_output, orig_len)
      n when length(new_output) >= n ->
        new_output |> Enum.reverse() |> Enum.take(n)
      _ -> decode_loop(rest, new_output, orig_len)
    end
  end

  # Copy `count` bytes starting at `start_idx` (0-based from front of original output list).
  # `output` is in reversed order; we track `base_len` as the length of output before this match.
  defp copy_from_output(_orig_output, _start_idx, _base_len, 0, acc), do: acc
  defp copy_from_output(orig_output, start_idx, base_len, count, acc) do
    # acc grows during copying; we need the i-th byte from the front of the ORIGINAL output.
    # The byte at position (start_idx + (base_len + match_written - base_len))
    # = start_idx + already_copied_in_this_match
    already_copied = base_len + (length(acc) - base_len)
    fetch_idx      = start_idx + (already_copied - base_len)
    # Retrieve from acc (reversed list): position fetch_idx from front = reversed[len-1-fetch_idx].
    byte = Enum.at(Enum.reverse(acc), fetch_idx)
    copy_from_output(orig_output, start_idx, base_len, count - 1, [byte | acc])
  end

  # ─── Serialisation ──────────────────────────────────────────────────────────

  @doc false
  def serialise_tokens(tokens, original_length) do
    blocks =
      tokens
      |> Enum.chunk_every(8)
      |> Enum.map(&encode_block/1)

    header = <<original_length::unsigned-big-integer-32, length(blocks)::unsigned-big-integer-32>>
    body   = IO.iodata_to_binary(blocks)
    header <> body
  end

  defp encode_block(chunk) do
    {flag, symbol_data} =
      chunk
      |> Enum.with_index()
      |> Enum.reduce({0, []}, fn {tok, bit}, {flag, parts} ->
        case tok do
          %{kind: :match, offset: off, length: len} ->
            {flag ||| (1 <<< bit), [parts, <<off::unsigned-big-integer-16, len::unsigned-8>>]}
          %{kind: :literal, byte: b} ->
            {flag, [parts, <<b::unsigned-8>>]}
        end
      end)

    [<<flag::unsigned-8>>, symbol_data]
  end

  @doc false
  def deserialise_tokens(data) when byte_size(data) < 8, do: {[], 0}

  def deserialise_tokens(data) do
    <<original_length::unsigned-big-integer-32,
      block_count::unsigned-big-integer-32,
      rest::binary>> = data

    # Cap block_count against actual payload size to prevent DoS.
    # Each block needs at least 2 bytes (1 flag + 1 payload byte minimum).
    max_possible = div(byte_size(rest), 2)
    safe_count   = min(block_count, max_possible)
    tokens = parse_blocks(rest, safe_count, [])
    {tokens, original_length}
  end

  defp parse_blocks(_data, 0, acc), do: Enum.reverse(acc)
  defp parse_blocks(data, _count, acc) when byte_size(data) == 0, do: Enum.reverse(acc)

  defp parse_blocks(<<flag::unsigned-8, rest::binary>>, count, acc) do
    {tokens, remaining} = parse_symbols(rest, flag, 0, [])
    parse_blocks(remaining, count - 1, tokens ++ acc)
  end

  defp parse_symbols(data, _flag, 8, acc), do: {acc, data}
  defp parse_symbols(<<>>, _flag, _bit, acc), do: {acc, <<>>}

  defp parse_symbols(data, flag, bit, acc) do
    if (flag &&& (1 <<< bit)) != 0 do
      # Match: 3 bytes
      case data do
        <<offset::unsigned-big-integer-16, length::unsigned-8, rest::binary>> ->
          parse_symbols(rest, flag, bit + 1, [match(offset, length) | acc])
        _ -> {acc, data}
      end
    else
      # Literal: 1 byte
      case data do
        <<b::unsigned-8, rest::binary>> ->
          parse_symbols(rest, flag, bit + 1, [literal(b) | acc])
        _ -> {acc, data}
      end
    end
  end

  # ─── One-shot API ────────────────────────────────────────────────────────────

  @doc """
  Compress a binary using LZSS, returning the CMP02 wire format.

  ## Examples

      iex> CodingAdventures.LZSS.decompress(CodingAdventures.LZSS.compress("AAAAAAA")) == "AAAAAAA"
      true
  """
  def compress(data, window_size \\ @default_window_size,
                     max_match   \\ @default_max_match,
                     min_match   \\ @default_min_match)
      when is_binary(data) do
    tokens = encode(data, window_size, max_match, min_match)
    serialise_tokens(tokens, byte_size(data))
  end

  @doc """
  Decompress bytes produced by compress/1.

  ## Examples

      iex> original = "hello hello hello"
      iex> CodingAdventures.LZSS.decompress(CodingAdventures.LZSS.compress(original)) == original
      true
  """
  def decompress(data) when is_binary(data) do
    {tokens, original_length} = deserialise_tokens(data)
    decode(tokens, original_length)
  end
end
