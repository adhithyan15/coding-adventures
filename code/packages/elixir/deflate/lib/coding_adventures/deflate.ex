defmodule CodingAdventures.Deflate do
  import Bitwise

  @moduledoc """
  DEFLATE Compression — CMP05 wire-format compress/decompress.

  ## What Is DEFLATE?

  DEFLATE (1996, RFC 1951) is the dominant general-purpose lossless compression
  algorithm. It combines two complementary techniques:

  1. **LZSS tokenization** (CMP02) — replaces repeated substrings with
     back-references into a 4096-byte sliding window.

  2. **Dual canonical Huffman coding** (DT27) — entropy-codes the resulting
     token stream using TWO separate Huffman trees:
     - LL tree: literals (0-255), end-of-data (256), length codes (257-284)
     - Dist tree: distance codes (0-23, for offsets 1-4096)

  ## The Expanded LL Alphabet

  DEFLATE merges literal bytes and match lengths into one alphabet (285 symbols):

  ```
  Symbols 0-255:   literal byte values
  Symbol  256:     end-of-data marker
  Symbols 257-284: length codes (each covers a range via extra bits)
  ```

  ## Wire Format (CMP05)

  ```
  [4B] original_length    big-endian uint32
  [2B] ll_entry_count     big-endian uint16
  [2B] dist_entry_count   big-endian uint16 (0 if no matches)
  [ll_entry_count × 3B]   (symbol uint16 BE, code_length uint8)
  [dist_entry_count × 3B] same format
  [remaining bytes]       LSB-first packed bit stream
  ```

  ## Examples

      iex> data = "hello hello hello"
      iex> CodingAdventures.Deflate.decompress(CodingAdventures.Deflate.compress(data)) == data
      true

  """

  alias CodingAdventures.LZSS
  alias CodingAdventures.HuffmanTree

  # ---------------------------------------------------------------------------
  # Length code table (LL symbols 257-284)
  # ---------------------------------------------------------------------------
  #
  # Each length symbol covers a range of match lengths. The exact length is
  # encoded as extra_bits raw bits after the Huffman code.

  @length_table [
    # {symbol, base_length, extra_bits}
    {257,   3, 0},
    {258,   4, 0},
    {259,   5, 0},
    {260,   6, 0},
    {261,   7, 0},
    {262,   8, 0},
    {263,   9, 0},
    {264,  10, 0},
    {265,  11, 1},
    {266,  13, 1},
    {267,  15, 1},
    {268,  17, 1},
    {269,  19, 2},
    {270,  23, 2},
    {271,  27, 2},
    {272,  31, 2},
    {273,  35, 3},
    {274,  43, 3},
    {275,  51, 3},
    {276,  59, 3},
    {277,  67, 4},
    {278,  83, 4},
    {279,  99, 4},
    {280, 115, 4},
    {281, 131, 5},
    {282, 163, 5},
    {283, 195, 5},
    {284, 227, 5},
  ]

  # Build fast-lookup maps from the length table.
  @length_base @length_table |> Enum.map(fn {sym, base, _} -> {sym, base} end) |> Map.new()
  @length_extra @length_table |> Enum.map(fn {sym, _, extra} -> {sym, extra} end) |> Map.new()

  # ---------------------------------------------------------------------------
  # Distance code table (codes 0-23)
  # ---------------------------------------------------------------------------

  @dist_table [
    # {code, base_dist, extra_bits}
    { 0,    1,  0},
    { 1,    2,  0},
    { 2,    3,  0},
    { 3,    4,  0},
    { 4,    5,  1},
    { 5,    7,  1},
    { 6,    9,  2},
    { 7,   13,  2},
    { 8,   17,  3},
    { 9,   25,  3},
    {10,   33,  4},
    {11,   49,  4},
    {12,   65,  5},
    {13,   97,  5},
    {14,  129,  6},
    {15,  193,  6},
    {16,  257,  7},
    {17,  385,  7},
    {18,  513,  8},
    {19,  769,  8},
    {20, 1025,  9},
    {21, 1537,  9},
    {22, 2049, 10},
    {23, 3073, 10},
  ]

  @dist_base @dist_table |> Enum.map(fn {code, base, _} -> {code, base} end) |> Map.new()
  @dist_extra @dist_table |> Enum.map(fn {code, _, extra} -> {code, extra} end) |> Map.new()

  # ---------------------------------------------------------------------------
  # Helper: length_symbol(length) → LL symbol (257-284)
  # ---------------------------------------------------------------------------

  defp length_symbol(length) do
    Enum.find_value(@length_table, 284, fn {sym, base, extra} ->
      max_len = base + (1 <<< extra) - 1
      if length <= max_len, do: sym, else: nil
    end)
  end

  # ---------------------------------------------------------------------------
  # Helper: dist_code(offset) → distance code (0-23)
  # ---------------------------------------------------------------------------

  defp dist_code(offset) do
    Enum.find_value(@dist_table, 23, fn {code, base, extra} ->
      max_dist = base + (1 <<< extra) - 1
      if offset <= max_dist, do: code, else: nil
    end)
  end

  # ---------------------------------------------------------------------------
  # Bit I/O helpers
  # ---------------------------------------------------------------------------

  defp pack_bits_lsb_first(bits) when is_binary(bits) do
    bit_list = for <<b::1 <- bits>>, do: b
    pack_bits_lsb_first(bit_list)
  end

  defp pack_bits_lsb_first(bits) when is_list(bits) do
    bits
    |> Enum.chunk_every(8, 8, List.duplicate(0, 7))
    |> Enum.map(fn byte_bits ->
      byte_bits
      |> Enum.with_index()
      |> Enum.reduce(0, fn {bit, pos}, acc -> acc ||| (bit <<< pos) end)
    end)
    |> :erlang.list_to_binary()
  end

  defp unpack_bits_lsb_first(data) when is_binary(data) do
    for <<byte::8 <- data>>,
        i <- 0..7,
        do: (byte >>> i) &&& 1
  end

  # ---------------------------------------------------------------------------
  # Canonical code reconstruction (for decompress)
  # ---------------------------------------------------------------------------

  defp reconstruct_canonical_codes([]) do
    %{}
  end

  defp reconstruct_canonical_codes([{sym, _len}]) do
    %{"0" => sym}
  end

  defp reconstruct_canonical_codes(lengths) do
    {result, _, _} =
      Enum.reduce(lengths, {%{}, 0, nil}, fn {sym, code_len}, {acc, code, prev_len} ->
        code2 =
          if prev_len != nil and code_len > prev_len do
            code <<< (code_len - prev_len)
          else
            code
          end
        bit_str = Integer.to_string(code2, 2) |> String.pad_leading(code_len, "0")
        {Map.put(acc, bit_str, sym), code2 + 1, code_len}
      end)
    result
  end

  # ---------------------------------------------------------------------------
  # LSB-first extra bits encoding
  # ---------------------------------------------------------------------------

  # Emit `n` bits of `val`, LSB first, as a list of 0/1 integers.
  defp extra_bits_lsb(val, n) when n == 0, do: []
  defp extra_bits_lsb(val, n) do
    for i <- 0..(n - 1), do: (val >>> i) &&& 1
  end

  # ---------------------------------------------------------------------------
  # Public API: compress/1
  # ---------------------------------------------------------------------------

  @doc """
  Compress `data` using DEFLATE (CMP05) and return wire-format bytes.

  Accepts a binary or iolist.

  ## Algorithm

  1. LZSS tokenization (window=4096, max_match=255, min_match=3).
  2. Tally LL and distance symbol frequencies.
  3. Build canonical Huffman trees via DT27.
  4. Encode token stream to LSB-first bit stream.
  5. Assemble header + LL table + dist table + bit stream.
  """
  def compress(data) when is_binary(data) or is_list(data) do
    data = if is_list(data), do: IO.iodata_to_binary(data), else: data
    original_length = byte_size(data)

    if original_length == 0 do
      # Empty input: LL tree has only symbol 256 (end-of-data).
      # Single-symbol tree → code "0".
      ll_table = <<0::16, 256::16, 1::8>>
      bit_stream = <<0::8>>
      <<0::32, 1::16, 0::16>> <> ll_table <> bit_stream
    else
      # Pass 1: LZSS tokenization.
      tokens = LZSS.encode(data)

      # Pass 2a: Tally frequencies.
      {ll_freq, dist_freq} =
        Enum.reduce(tokens, {%{}, %{}}, fn tok, {llf, df} ->
          case tok do
            %{kind: :literal, byte: b} ->
              {Map.update(llf, b, 1, &(&1 + 1)), df}
            %{kind: :match, length: len, offset: off} ->
              sym = length_symbol(len)
              dc = dist_code(off)
              {Map.update(llf, sym, 1, &(&1 + 1)),
               Map.update(df, dc, 1, &(&1 + 1))}
          end
        end)

      # Always include the end-of-data symbol.
      ll_freq = Map.update(ll_freq, 256, 1, &(&1 + 1))

      # Pass 2b: Build canonical Huffman trees.
      ll_tree = HuffmanTree.build(Map.to_list(ll_freq))
      ll_code_table = HuffmanTree.canonical_code_table(ll_tree)

      dist_code_table =
        if map_size(dist_freq) > 0 do
          dist_tree = HuffmanTree.build(Map.to_list(dist_freq))
          HuffmanTree.canonical_code_table(dist_tree)
        else
          %{}
        end

      # Pass 2c: Encode token stream.
      bit_list =
        Enum.flat_map(tokens, fn tok ->
          case tok do
            %{kind: :literal, byte: b} ->
              Map.get(ll_code_table, b) |> String.graphemes() |> Enum.map(&String.to_integer/1)

            %{kind: :match, length: len, offset: off} ->
              sym = length_symbol(len)
              extra_bits_count = Map.get(@length_extra, sym, 0)
              extra_val = len - Map.get(@length_base, sym, 0)

              dc = dist_code(off)
              dist_extra_bits = Map.get(@dist_extra, dc, 0)
              dist_extra_val = off - Map.get(@dist_base, dc, 0)

              ll_bits = Map.get(ll_code_table, sym) |> String.graphemes() |> Enum.map(&String.to_integer/1)
              len_extra = extra_bits_lsb(extra_val, extra_bits_count)
              dist_bits = Map.get(dist_code_table, dc) |> String.graphemes() |> Enum.map(&String.to_integer/1)
              dist_extra = extra_bits_lsb(dist_extra_val, dist_extra_bits)

              ll_bits ++ len_extra ++ dist_bits ++ dist_extra
          end
        end)

      # Append end-of-data symbol.
      eod_bits = Map.get(ll_code_table, 256) |> String.graphemes() |> Enum.map(&String.to_integer/1)
      all_bits = bit_list ++ eod_bits

      bit_stream = pack_bits_lsb_first(all_bits)

      # Assemble wire format.
      ll_lengths =
        ll_code_table
        |> Enum.map(fn {sym, code} -> {sym, String.length(code)} end)
        |> Enum.sort(fn {sa, la}, {sb, lb} ->
          if la != lb, do: la < lb, else: sa < sb
        end)

      dist_lengths =
        dist_code_table
        |> Enum.map(fn {sym, code} -> {sym, String.length(code)} end)
        |> Enum.sort(fn {sa, la}, {sb, lb} ->
          if la != lb, do: la < lb, else: sa < sb
        end)

      ll_count = length(ll_lengths)
      dist_count = length(dist_lengths)

      header = <<original_length::32, ll_count::16, dist_count::16>>

      ll_bytes = for {sym, len} <- ll_lengths, into: <<>>, do: <<sym::16, len::8>>
      dist_bytes = for {sym, len} <- dist_lengths, into: <<>>, do: <<sym::16, len::8>>

      header <> ll_bytes <> dist_bytes <> bit_stream
    end
  end

  # ---------------------------------------------------------------------------
  # Public API: decompress/1
  # ---------------------------------------------------------------------------

  @doc """
  Decompress CMP05 wire-format `data` and return the original binary.

  ## Algorithm

  1. Parse 8-byte header (original_length, ll_entry_count, dist_entry_count).
  2. Parse LL and dist code-length tables.
  3. Reconstruct canonical codes.
  4. Unpack LSB-first bit stream.
  5. Decode tokens until end-of-data symbol (256).
  """
  def decompress(data) when is_binary(data) and byte_size(data) < 8, do: ""

  def decompress(<<original_length::32, ll_entry_count::16, dist_entry_count::16, rest::binary>>) do
    if original_length == 0 do
      ""
    else
      # Parse LL table.
      {ll_lengths, rest2} = parse_table(rest, ll_entry_count)
      # Parse dist table.
      {dist_lengths, rest3} = parse_table(rest2, dist_entry_count)

      # Reconstruct canonical code tables (bit_string → symbol).
      ll_rev_map = reconstruct_canonical_codes(ll_lengths)
      dist_rev_map = reconstruct_canonical_codes(dist_lengths)

      # Unpack bit stream.
      bits = unpack_bits_lsb_first(rest3)

      # Decode token stream.
      output = decode_tokens(bits, 0, ll_rev_map, dist_rev_map, [])
      :erlang.list_to_binary(output)
    end
  end

  defp parse_table(data, 0), do: {[], data}
  defp parse_table(data, n) do
    <<sym::16, code_len::8, rest::binary>> = data
    {tail, rest2} = parse_table(rest, n - 1)
    {[{sym, code_len} | tail], rest2}
  end

  # Decode tokens from bits until end-of-data symbol (256).
  defp decode_tokens(bits, bit_pos, ll_rev_map, dist_rev_map, output) do
    {ll_sym, bit_pos2} = next_huffman_symbol(bits, bit_pos, ll_rev_map)

    cond do
      ll_sym == 256 ->
        Enum.reverse(output)

      ll_sym < 256 ->
        decode_tokens(bits, bit_pos2, ll_rev_map, dist_rev_map, [ll_sym | output])

      true ->
        # Length code 257-284.
        extra = Map.get(@length_extra, ll_sym, 0)
        {extra_val, bit_pos3} = read_bits(bits, bit_pos2, extra)
        length_val = Map.get(@length_base, ll_sym, 0) + extra_val

        {dist_sym, bit_pos4} = next_huffman_symbol(bits, bit_pos3, dist_rev_map)
        dextra = Map.get(@dist_extra, dist_sym, 0)
        {dextra_val, bit_pos5} = read_bits(bits, bit_pos4, dextra)
        offset_val = Map.get(@dist_base, dist_sym, 0) + dextra_val

        # Copy length bytes from offset positions back (byte-by-byte for overlap).
        reversed_output = output  # currently reversed
        start = length(reversed_output) - offset_val
        copied = copy_from_reversed(reversed_output, start, length_val, [])
        new_output = Enum.reverse(copied) ++ reversed_output

        decode_tokens(bits, bit_pos5, ll_rev_map, dist_rev_map, new_output)
    end
  end

  # Copy `count` bytes starting at `start` (0-indexed from the front of the
  # unreversed output) from the reversed output list.
  # Since output is reversed, index `start` in the forward direction corresponds
  # to index (length - 1 - start) in the reversed list — but we need byte-by-byte
  # copying for overlap. We'll work with the forward output.
  defp copy_from_reversed(reversed_output, start, count, acc) do
    # Convert reversed to forward, copy, then reverse acc.
    fwd = Enum.reverse(reversed_output)
    do_copy(fwd, start, count, acc)
  end

  defp do_copy(_fwd, _start, 0, acc), do: Enum.reverse(acc)
  defp do_copy(fwd, start, count, acc) do
    byte = Enum.at(fwd, start)
    do_copy(fwd ++ [byte], start + 1, count - 1, [byte | acc])
  end

  # Read n raw bits from the bit list (as a list of 0/1 integers), LSB-first.
  defp read_bits(_bits, pos, 0), do: {0, pos}
  defp read_bits(bits, pos, n) do
    {val, new_pos} =
      Enum.reduce(0..(n - 1), {0, pos}, fn i, {acc, p} ->
        bit = Enum.at(bits, p, 0)
        {acc ||| (bit <<< i), p + 1}
      end)
    {val, new_pos}
  end

  # Decode one Huffman symbol by reading bits until a prefix match.
  defp next_huffman_symbol(bits, pos, rev_map) do
    do_next_huffman(bits, pos, rev_map, "")
  end

  defp do_next_huffman(bits, pos, rev_map, acc) do
    bit = Enum.at(bits, pos, 0)
    new_acc = acc <> Integer.to_string(bit)
    case Map.get(rev_map, new_acc) do
      nil -> do_next_huffman(bits, pos + 1, rev_map, new_acc)
      sym -> {sym, pos + 1}
    end
  end
end
