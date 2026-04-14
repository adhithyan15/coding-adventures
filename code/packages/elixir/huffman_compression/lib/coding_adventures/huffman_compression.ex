defmodule CodingAdventures.HuffmanCompression do
  import Bitwise

  @moduledoc """
  Huffman Compression — CMP04 wire-format compress/decompress.

  ## What Is Huffman Coding?

  Huffman coding (1952) is a lossless entropy compression algorithm: it assigns
  the shortest bit codes to the most frequent symbols and the longest codes to
  the rarest ones. The result is that on average fewer bits are needed to
  represent the data than a flat 8-bit-per-byte encoding.

  Think of it like a Morse code that is automatically designed from your data.
  In Morse, 'E' (the most common English letter) is a single dot. Huffman does
  the same thing, but it derives the code lengths by measuring actual symbol
  frequencies in the input.

  ## CMP04 Wire Format

  ```
  Bytes 0–3:    original_length  (big-endian uint32)
                The exact number of bytes in the original (decompressed) data.

  Bytes 4–7:    symbol_count     (big-endian uint32)
                The number of distinct byte values present in the input.
                This tells the decoder how many (symbol, code_length) pairs to read.

  Bytes 8–8+2N: code-lengths table — N entries, each 2 bytes:
                  [0] symbol value  (uint8, 0–255)
                  [1] code length   (uint8, 1–16)
                Sorted by (code_length, symbol_value) ascending.
                This is the canonical form: given only lengths, the decoder can
                reconstruct the exact bit codes without transmitting the tree.

  Bytes 8+2N+:  bit stream — packed LSB-first, zero-padded to byte boundary.
                Each original byte is replaced by its Huffman code. Codes are
                emitted from the LSB of each byte upward.
  ```

  ## Canonical Huffman Codes

  Canonical codes are a standardised form of Huffman codes where, given only
  the code *lengths*, you can reconstruct the exact codes without transmitting
  the tree structure. This is how DEFLATE (ZIP, gzip, PNG) works.

  Reconstruction algorithm:
    1. Collect `{symbol, code_length}` pairs from the tree.
    2. Sort by `{code_length, symbol_value}`.
    3. Assign codes numerically:
         code[0] = 0 (zero-padded to length[0] bits)
         code[i] = (code[i-1] + 1) << (length[i] - length[i-1])

  ## LSB-First Bit Packing

  The bit stream uses LSB-first packing (the same convention as LZW/GIF):
    - Bit 0 of the first code goes into bit 0 of the first byte.
    - Bit 1 of the first code goes into bit 1 of the first byte.
    - When a byte fills up, we move to the next byte.

  Example: code "101" (3 bits) packs into the bottom 3 bits of a byte:
    byte = 0b????_?101  (bit 0 = 1, bit 1 = 0, bit 2 = 1)

  ## Series

      CMP00 (LZ77,    1977) — Sliding-window backreferences.
      CMP01 (LZ78,    1978) — Explicit dictionary (trie).
      CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
      CMP03 (LZW,     1984) — LZ78 + pre-initialised dict; GIF.
      CMP04 (Huffman, 1952) — Entropy coding. ← this module
      CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.

  ## Examples

      iex> data = "hello hello hello"
      iex> CodingAdventures.HuffmanCompression.decompress(CodingAdventures.HuffmanCompression.compress(data)) == data
      true

      iex> CodingAdventures.HuffmanCompression.decompress(CodingAdventures.HuffmanCompression.compress("AAABBC")) == "AAABBC"
      true

      iex> CodingAdventures.HuffmanCompression.decompress(CodingAdventures.HuffmanCompression.compress("")) == ""
      true
  """

  alias CodingAdventures.HuffmanTree

  # ─── compress/1 ──────────────────────────────────────────────────────────────

  @doc """
  Compress a binary using CMP04 Huffman wire format.

  Returns a binary whose first 8 bytes are the header (original_length,
  symbol_count), followed by N*2 bytes of code-length table, followed by the
  LSB-first bit-packed codestream.

  The empty binary is handled as a special case: returns an 8-byte header with
  `original_length = 0` and `symbol_count = 0` and an empty bit stream.

  ## Examples

      iex> data = "AAABBC"
      iex> compressed = CodingAdventures.HuffmanCompression.compress(data)
      iex> is_binary(compressed)
      true
      iex> <<orig_len::32-big, sym_count::32-big, _rest::binary>> = compressed
      iex> orig_len
      6
      iex> sym_count
      3
  """
  def compress(data) when is_binary(data) and byte_size(data) == 0 do
    # Empty input: 8-byte header, no table, no bits.
    <<0::32-big, 0::32-big>>
  end

  def compress(data) when is_binary(data) do
    original_length = byte_size(data)

    # Step 1: count symbol frequencies.
    # Enum.frequencies/1 returns %{byte_value => count}
    freq_map = Enum.frequencies(:binary.bin_to_list(data))

    # Step 2: build the Huffman tree from the frequency map.
    # HuffmanTree.build/1 expects a list of {symbol, frequency} pairs.
    tree = HuffmanTree.build(Enum.to_list(freq_map))

    # Step 3: compute canonical codes.
    # canonical_code_table/1 returns %{symbol => bit_string}
    # where bit_string is a string like "01" or "110".
    code_table = HuffmanTree.canonical_code_table(tree)

    # Step 4: sort code lengths by (code_length, symbol_value).
    # This sorted list is what goes into the wire-format table.
    sorted_lengths =
      code_table
      |> Enum.map(fn {sym, bits} -> {sym, String.length(bits)} end)
      |> Enum.sort_by(fn {sym, len} -> {len, sym} end)

    symbol_count = length(sorted_lengths)

    # Step 5: encode the input — replace each byte with its canonical code bits.
    encoded_bits =
      :binary.bin_to_list(data)
      |> Enum.map(fn byte -> Map.fetch!(code_table, byte) end)
      |> Enum.join()

    # Step 6: pack bits LSB-first into bytes.
    bit_bytes = pack_bits_lsb_first(encoded_bits)

    # Step 7: assemble wire format.
    # Header: original_length (4 bytes) + symbol_count (4 bytes)
    header = <<original_length::32-big, symbol_count::32-big>>

    # Code-lengths table: N * 2 bytes, each entry is {symbol, length}
    table_bytes =
      sorted_lengths
      |> Enum.map(fn {sym, len} -> <<sym::8, len::8>> end)
      |> Enum.join()

    header <> table_bytes <> bit_bytes
  end

  # ─── decompress/1 ────────────────────────────────────────────────────────────

  @doc """
  Decompress CMP04 wire-format bytes back to the original binary.

  Returns `{:error, :too_short}` if the input is fewer than 8 bytes.

  ## Examples

      iex> CodingAdventures.HuffmanCompression.decompress(CodingAdventures.HuffmanCompression.compress("AAABBC")) == "AAABBC"
      true

      iex> CodingAdventures.HuffmanCompression.decompress(<<1, 2>>) == {:error, :too_short}
      true
  """
  def decompress(data) when is_binary(data) and byte_size(data) < 8 do
    {:error, :too_short}
  end

  def decompress(data) when is_binary(data) do
    # Step 1: parse the 8-byte header.
    <<original_length::32-big, symbol_count::32-big, rest_data::binary>> = data

    # Fast path: empty input
    if original_length == 0 do
      ""
    else
      # Step 2: parse the code-lengths table (symbol_count * 2 bytes).
      table_byte_count = symbol_count * 2

      if byte_size(rest_data) < table_byte_count do
        {:error, :too_short}
      else
        <<table_bytes::binary-size(table_byte_count), bit_stream_bytes::binary>> = rest_data

        # Parse N entries, each 2 bytes: {symbol, code_length}
        lengths = parse_lengths_table(table_bytes, symbol_count, [])

        # Step 3: reconstruct canonical codes from the lengths table.
        # The lengths list is already sorted by (code_length, symbol_value).
        # We reverse the map: bit_string → symbol, for lookup during decoding.
        decode_table = canonical_codes_from_lengths(lengths)

        # Step 4: unpack the bit stream (LSB-first).
        all_bits = unpack_bits_lsb_first(bit_stream_bytes)

        # Step 5: decode exactly original_length symbols.
        # We rebuild the tree from the decode table for use with HuffmanTree.decode_all/3.
        # Alternatively, decode directly using the code table.
        symbols = decode_with_table(all_bits, decode_table, original_length)

        :binary.list_to_bin(symbols)
      end
    end
  end

  # ─── Wire-format helpers ─────────────────────────────────────────────────────

  # Parse the code-lengths table from a binary.
  # Each entry is 2 bytes: <<symbol::8, code_length::8>>.
  # Returns a list of {symbol, code_length} pairs in wire order (already sorted).
  defp parse_lengths_table(_bytes, 0, acc), do: Enum.reverse(acc)

  defp parse_lengths_table(<<sym::8, len::8, rest_data::binary>>, n, acc) do
    parse_lengths_table(rest_data, n - 1, [{sym, len} | acc])
  end

  # ─── Canonical code reconstruction ──────────────────────────────────────────
  #
  # Given a list of {symbol, code_length} pairs sorted by (length, symbol),
  # reconstruct the canonical bit strings and return a map %{bit_string => symbol}.
  #
  # Algorithm (same as DEFLATE):
  #   Start with code = 0 and prev_len = first_len.
  #   For each entry:
  #     If len > prev_len: code = code <<< (len - prev_len)
  #     Emit code as a zero-padded binary string of `len` bits.
  #     Increment code by 1 for the next entry.
  #
  # This produces the canonical codes — the same codes the encoder produced
  # because both sides sort by (length, symbol) and apply the same formula.

  defp canonical_codes_from_lengths([]) do
    %{}
  end

  defp canonical_codes_from_lengths(lengths) do
    [{_first_sym, first_len} | _rest] = lengths

    {table, _, _} =
      Enum.reduce(lengths, {%{}, 0, first_len}, fn {sym, len}, {acc, code, prev_len} ->
        # Shift left when moving to a longer code length.
        # This is the canonical code assignment formula.
        shifted = if len > prev_len, do: code <<< (len - prev_len), else: code
        bits = Integer.to_string(shifted, 2) |> String.pad_leading(len, "0")
        {Map.put(acc, bits, sym), shifted + 1, len}
      end)

    table
  end

  # ─── Symbol decoding ─────────────────────────────────────────────────────────
  #
  # We decode by scanning a bit string and trying increasingly longer prefixes
  # until we find a match in the decode table.
  #
  # This is a greedy prefix scan — it works because Huffman codes are
  # prefix-free: no valid code is a prefix of another valid code. So the first
  # match is always the correct one.
  #
  # For large alphabets, a trie would be faster (O(max_code_len) per symbol
  # instead of O(n * max_code_len)). For educational purposes we use a map.

  defp decode_with_table(bits, decode_table, count) do
    do_decode_symbols(bits, decode_table, count, 0, [])
  end

  defp do_decode_symbols(_bits, _table, 0, _pos, acc) do
    Enum.reverse(acc)
  end

  defp do_decode_symbols(bits, table, remaining, pos, acc) do
    # Try prefixes of increasing length starting at `pos`.
    bits_remaining = String.length(bits) - pos

    if bits_remaining <= 0 do
      # Bit stream exhausted early (truncated input) — return what we have.
      Enum.reverse(acc)
    else
      case scan_prefix(bits, table, pos, pos + 1) do
        {:ok, sym, new_pos} ->
          do_decode_symbols(bits, table, remaining - 1, new_pos, [sym | acc])

        :not_found ->
          # Could not match any code — malformed stream, stop.
          Enum.reverse(acc)
      end
    end
  end

  # Scan for the longest matching prefix starting at `start_pos`, ending at
  # `end_pos` (exclusive). Tries end_pos from start_pos+1 up to
  # start_pos + max_possible_code_length.
  defp scan_prefix(bits, table, start_pos, end_pos) do
    total_bits = String.length(bits)

    if end_pos > total_bits + 1 or end_pos - start_pos > 16 do
      :not_found
    else
      snippet = String.slice(bits, start_pos, end_pos - start_pos)

      case Map.get(table, snippet) do
        nil ->
          # No match yet, try a longer prefix.
          if end_pos > total_bits do
            :not_found
          else
            scan_prefix(bits, table, start_pos, end_pos + 1)
          end

        sym ->
          {:ok, sym, end_pos}
      end
    end
  end

  # ─── Bit packing (LSB-first) ─────────────────────────────────────────────────
  #
  # LSB-first means the first code bit is placed at bit position 0 of the first
  # byte, the second code bit at position 1, and so on. When a byte is full (8
  # bits consumed) we move to the next byte starting at position 0 again.
  #
  # Example: encoding "A" (3-bit code "011")
  #   byte 0: bit0 = 0, bit1 = 1, bit2 = 1 → 0b00000110 = 6
  #
  # Why LSB-first? It matches the convention used by LZW/GIF and is natural for
  # the bitwise OR/shift operations we use here. Both encoder and decoder agree
  # on the same convention, so data round-trips correctly.

  @doc false
  def pack_bits_lsb_first(bits) when is_binary(bits) do
    bits
    |> String.graphemes()
    |> Enum.map(&String.to_integer/1)
    |> pack_bits_into_bytes()
  end

  defp pack_bits_into_bytes([]) do
    <<>>
  end

  defp pack_bits_into_bytes(bits) do
    # Chunk the bit list into groups of 8, padding the last chunk with zeros.
    bits
    |> Enum.chunk_every(8, 8, List.duplicate(0, 7))
    |> Enum.map(fn chunk ->
      # Pack 8 bits (or fewer with padding) into one byte, LSB-first.
      # Bit at index i (0-based) goes to bit position i in the byte.
      chunk
      |> Enum.with_index()
      |> Enum.reduce(0, fn {bit, i}, acc -> acc ||| (bit <<< i) end)
    end)
    |> :binary.list_to_bin()
  end

  # ─── Bit unpacking (LSB-first) ───────────────────────────────────────────────
  #
  # To unpack, we reverse the packing process: for each byte, emit its 8 bits
  # from LSB to MSB as "0" and "1" characters. The decoder will then parse this
  # string left-to-right, matching prefixes against the canonical code table.

  @doc false
  def unpack_bits_lsb_first(data) when is_binary(data) do
    data
    |> :binary.bin_to_list()
    |> Enum.flat_map(fn byte ->
      # Emit bits 0 through 7 from the byte (LSB first).
      Enum.map(0..7, fn i -> (byte >>> i) &&& 1 end)
    end)
    |> Enum.map(&Integer.to_string/1)
    |> Enum.join()
  end
end
