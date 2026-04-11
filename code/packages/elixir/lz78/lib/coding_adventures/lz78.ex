defmodule CodingAdventures.LZ78 do
  @moduledoc """
  LZ78 lossless compression algorithm (1978) — CMP01.

  LZ78 (Lempel & Ziv, 1978) builds an explicit trie-based dictionary of byte
  sequences as it encodes. Both encoder and decoder build the same dictionary
  independently — no dictionary is transmitted.

  ## How it differs from LZ77

  LZ77 uses a sliding window: references are (offset, length, next_char) into
  the last N bytes. LZ78 grows an explicit dictionary: references are
  (dict_index, next_char) into an ever-growing trie.

  ## Token

  Each token is a `%{dict_index: integer, next_char: integer}` map:
  - `dict_index`: ID of the longest dictionary prefix (0 = literal).
  - `next_char`:  Byte following the match. 0 is the flush sentinel when
    input ends mid-match.

  ## Wire Format

      Bytes 0-3:  original length (big-endian uint32)
      Bytes 4-7:  token count (big-endian uint32)
      Bytes 8+:   token_count x 4 bytes each:
                    [0..1]  dict_index (big-endian uint16)
                    [2]     next_char (uint8)
                    [3]     reserved (0x00)

  ## Series

      CMP00 (LZ77, 1977) - Sliding-window backreferences.
      CMP01 (LZ78, 1978) - Explicit dictionary (trie). <- this module
      CMP02 (LZSS, 1982) - LZ77 + flag bits.
      CMP03 (LZW,  1984) - LZ78 + pre-initialised alphabet; GIF.
      CMP04 (Huffman, 1952) - Entropy coding.
      CMP05 (DEFLATE, 1996) - LZ77 + Huffman; ZIP/gzip/PNG.

  ## Examples

      iex> data = "hello hello hello"
      iex> CodingAdventures.LZ78.decompress(CodingAdventures.LZ78.compress(data)) == data
      true
  """

  alias CodingAdventures.LZ78.TrieCursor

  # Token constructor

  @doc "Create a %{dict_index, next_char} token map."
  def token(dict_index, next_char),
    do: %{dict_index: dict_index, next_char: next_char}

  # Encoder

  @doc """
  Encode a binary into an LZ78 token list.

  Uses TrieCursor to walk the dictionary one byte at a time.
  When step/2 returns :miss (no child edge), emits a token for the current
  dict_id plus byte, records the new sequence, and resets to root.

  If the input ends mid-match, a flush token with next_char: 0 is emitted.

  ## Parameters

  - `data`          - binary input.
  - `max_dict_size` - maximum dictionary entries (default 65536).

  ## Examples

      iex> tokens = CodingAdventures.LZ78.encode("ABCDE")
      iex> length(tokens)
      5
      iex> Enum.all?(tokens, fn t -> t.dict_index == 0 end)
      true
  """
  def encode(data, max_dict_size \\ 65536) when is_binary(data) do
    cursor = TrieCursor.new()
    bytes  = :binary.bin_to_list(data)
    encode_loop(bytes, cursor, 1, max_dict_size, [])
  end

  defp encode_loop([], cursor, _next_id, _max, acc) do
    if TrieCursor.at_root?(cursor) do
      Enum.reverse(acc)
    else
      flush = token(TrieCursor.dict_id(cursor), 0)
      Enum.reverse([flush | acc])
    end
  end

  defp encode_loop([byte | rest], cursor, next_id, max, acc) do
    case TrieCursor.step(cursor, byte) do
      {:ok, new_cursor} ->
        encode_loop(rest, new_cursor, next_id, max, acc)

      :miss ->
        tok = token(TrieCursor.dict_id(cursor), byte)

        {updated_cursor, new_next_id} =
          if next_id < max do
            {TrieCursor.insert(cursor, byte, next_id), next_id + 1}
          else
            {cursor, next_id}
          end

        encode_loop(rest, TrieCursor.reset(updated_cursor), new_next_id, max, [tok | acc])
    end
  end

  # Decoder

  @doc """
  Decode an LZ78 token list back into the original bytes.

  ## Parameters

  - `tokens`          - token list from encode/2.
  - `original_length` - if a non-negative integer, truncates output to that many
    bytes (strips flush sentinel). Pass :all (default) to return all bytes.

  ## Examples

      iex> tokens = CodingAdventures.LZ78.encode("hello")
      iex> CodingAdventures.LZ78.decode(tokens, 5) == "hello"
      true
  """
  def decode(tokens, original_length \\ :all) do
    dict_table = [{0, 0}]
    output = decode_loop(tokens, dict_table, [], original_length)
    :erlang.list_to_binary(output)
  end

  defp decode_loop([], _table, output, _orig_len), do: Enum.reverse(output)

  defp decode_loop([tok | rest], table, output, orig_len) do
    seq = reconstruct(table, tok.dict_index)
    output_after_seq = Enum.reduce(seq, output, fn b, acc -> [b | acc] end)

    output_after_char =
      cond do
        orig_len == :all                    -> [tok.next_char | output_after_seq]
        length(output_after_seq) < orig_len -> [tok.next_char | output_after_seq]
        true                                -> output_after_seq
      end

    new_table = table ++ [{tok.dict_index, tok.next_char}]

    case orig_len do
      :all ->
        decode_loop(rest, new_table, output_after_char, orig_len)

      n when length(output_after_char) >= n ->
        output_after_char |> Enum.reverse() |> Enum.take(n)

      _ ->
        decode_loop(rest, new_table, output_after_char, orig_len)
    end
  end

  defp reconstruct(_table, 0), do: []

  defp reconstruct(table, index) do
    collect_chain(index, table, [])
  end

  defp collect_chain(0, _table, acc), do: acc

  defp collect_chain(index, table, acc) do
    {parent_id, byte} = Enum.at(table, index)
    collect_chain(parent_id, table, [byte | acc])
  end

  # Serialisation

  @doc false
  def serialise_tokens(tokens, original_length) do
    header = <<original_length::unsigned-big-integer-32, length(tokens)::unsigned-big-integer-32>>

    body =
      tokens
      |> Enum.map(fn tok ->
        <<tok.dict_index::unsigned-big-integer-16, tok.next_char::unsigned-8, 0::8>>
      end)
      |> IO.iodata_to_binary()

    header <> body
  end

  @doc false
  def deserialise_tokens(data) when byte_size(data) < 8, do: {[], 0}

  def deserialise_tokens(data) do
    <<original_length::unsigned-big-integer-32,
      token_count::unsigned-big-integer-32,
      rest::binary>> = data

    tokens = parse_tokens(rest, token_count, [])
    {tokens, original_length}
  end

  defp parse_tokens(_data, 0, acc), do: Enum.reverse(acc)
  defp parse_tokens(data, _count, acc) when byte_size(data) < 4, do: Enum.reverse(acc)

  defp parse_tokens(
    <<dict_index::unsigned-big-integer-16, next_char::8, _reserved::8, rest::binary>>,
    count,
    acc
  ) do
    parse_tokens(rest, count - 1, [token(dict_index, next_char) | acc])
  end

  # One-shot API

  @doc """
  Compress a binary using LZ78, returning the CMP01 wire format.

  ## Examples

      iex> CodingAdventures.LZ78.decompress(CodingAdventures.LZ78.compress("AAAAAAA")) == "AAAAAAA"
      true
  """
  def compress(data, max_dict_size \\ 65536) when is_binary(data) do
    tokens = encode(data, max_dict_size)
    serialise_tokens(tokens, byte_size(data))
  end

  @doc """
  Decompress bytes that were compressed with compress/2.

  ## Examples

      iex> original = "hello hello hello"
      iex> CodingAdventures.LZ78.decompress(CodingAdventures.LZ78.compress(original)) == original
      true
  """
  def decompress(data) when is_binary(data) do
    {tokens, original_length} = deserialise_tokens(data)
    decode(tokens, original_length)
  end
end
