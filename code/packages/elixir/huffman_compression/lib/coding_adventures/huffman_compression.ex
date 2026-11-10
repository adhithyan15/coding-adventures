defmodule CodingAdventures.HuffmanCompression do
  import Bitwise

  # Symbols are single bytes, so a valid table has at most 256 entries.
  @max_symbols 256

  # Longest code length the decoder accepts. Real codes over 256 symbols are far
  # shorter; the encoder never emits more.
  @max_code_len 56

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

  Returns `{:error, reason}` for malformed or hostile input instead of raising
  or looping. The reasons are:

    * `:too_short` — fewer than 8 bytes, or a truncated code-lengths table
    * `:too_many_symbols` — `symbol_count` above 256 (symbols are single bytes)
    * `:no_symbols` — non-empty output declared with an empty table
    * `:duplicate_symbol` — a byte value appears twice in the table
    * `:invalid_code_length` — a length of 0, or above #{56}
    * `:unsorted_table` — table not sorted by code length
    * `:oversubscribed` — the lengths need more code space than exists (Kraft > 1)
    * `:declared_length_too_large` — `original_length` exceeds 8 x payload bytes
    * `:bit_stream_exhausted` — payload ended before all symbols were decoded
    * `:invalid_code` — the bits read match no code in the table

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

    cond do
      # Fast path: empty input
      original_length == 0 ->
        ""

      # SECURITY: the header is attacker-controlled. Symbols are single bytes, so
      # a table can name at most 256 of them. Checking first also keeps the
      # table-size arithmetic below tiny.
      symbol_count > @max_symbols ->
        {:error, :too_many_symbols}

      symbol_count == 0 ->
        {:error, :no_symbols}

      # Step 2: the code-lengths table is symbol_count * 2 bytes.
      byte_size(rest_data) < symbol_count * 2 ->
        {:error, :too_short}

      true ->
        table_byte_count = symbol_count * 2
        <<table_bytes::binary-size(table_byte_count), payload::binary>> = rest_data
        decode_payload(original_length, parse_lengths_table(table_bytes, []), payload)
    end
  end

  # Validate the table, build the decode map, bound the declared length, then
  # decode. Each validation stage returns :ok / {:ok, value} or {:error, reason};
  # a successful decode returns the plain binary.
  defp decode_payload(original_length, lengths, payload) do
    with :ok <- check_unique_symbols(lengths),
         {:ok, decode_table, max_len} <- canonical_decode_table(lengths),
         :ok <- check_declared_length(original_length, payload) do
      # Step 5: decode exactly original_length symbols straight from the bytes.
      decode_bits(payload, byte_size(payload) * 8, decode_table, max_len, original_length, 0, 0, 0, [])
    end
  end

  # ─── Wire-format helpers ─────────────────────────────────────────────────────

  # Parse the code-lengths table from a binary.
  # Each entry is 2 bytes: <<symbol::8, code_length::8>>.
  # Returns a list of {symbol, code_length} pairs in wire order (already sorted).
  defp parse_lengths_table(<<>>, acc), do: Enum.reverse(acc)

  defp parse_lengths_table(<<sym::8, len::8, rest_data::binary>>, acc) do
    parse_lengths_table(rest_data, [{sym, len} | acc])
  end

  # A byte value can have only one code; a repeat makes the table ambiguous.
  defp check_unique_symbols(lengths) do
    if length(Enum.uniq_by(lengths, fn {sym, _len} -> sym end)) == length(lengths) do
      :ok
    else
      {:error, :duplicate_symbol}
    end
  end

  # SECURITY: the declared length is attacker-controlled. Every code is at least
  # one bit, so N payload bytes hold at most 8*N symbols; reject a larger
  # declaration before any work is sized from it.
  defp check_declared_length(original_length, payload) do
    if original_length > byte_size(payload) * 8 do
      {:error, :declared_length_too_large}
    else
      :ok
    end
  end

  # ─── Canonical code reconstruction ──────────────────────────────────────────
  #
  # Given a list of {symbol, code_length} pairs sorted by (length, symbol),
  # reconstruct the canonical codes and return {:ok, map, max_len} where map is
  # %{{length, code} => symbol}.
  #
  # Algorithm (same as DEFLATE):
  #   Start with code = 0 and prev_len = first_len.
  #   For each entry:
  #     If len > prev_len: code = code <<< (len - prev_len)
  #     Assign `code` (len bits) to the symbol.
  #     Increment code by 1 for the next entry.
  #
  # This produces the canonical codes — the same codes the encoder produced
  # because both sides sort by (length, symbol) and apply the same formula.
  #
  # SECURITY: the table is hostile input, so each entry is checked:
  #   * a length of 0 matches nothing, and one above @max_code_len is never
  #     emitted by the encoder;
  #   * a table not sorted by length breaks the canonical construction (the
  #     encoder always sorts);
  #   * an over-subscribed table (Kraft sum > 1) is detected by requiring the
  #     running code to still fit in `len` bits (code >>> len == 0).
  # Integers are arbitrary precision in Elixir, so nothing can wrap; the bounds
  # exist to keep the table small and the semantics identical to other ports.
  # Keys are {len, code} tuples so no per-symbol bit strings are built.

  defp canonical_decode_table([]), do: {:ok, %{}, 0}

  defp canonical_decode_table([{_first_sym, first_len} | _] = lengths) do
    result =
      Enum.reduce_while(lengths, {%{}, 0, first_len}, fn {sym, len}, {acc, code, prev_len} ->
        cond do
          len == 0 or len > @max_code_len ->
            {:halt, {:error, :invalid_code_length}}

          len < prev_len ->
            {:halt, {:error, :unsorted_table}}

          true ->
            # Shift left when moving to a longer code length.
            shifted = code <<< (len - prev_len)

            if (shifted >>> len) != 0 do
              {:halt, {:error, :oversubscribed}}
            else
              {:cont, {Map.put(acc, {len, shifted}, sym), shifted + 1, len}}
            end
        end
      end)

    case result do
      {:error, _reason} = error ->
        error

      {table, _next_code, max_len} ->
        {:ok, table, max_len}
    end
  end

  # ─── Symbol decoding ─────────────────────────────────────────────────────────
  #
  # Bits are LSB-first within each byte, so bit `pos` of the stream is
  # (byte[pos / 8] >>> (pos % 8)) &&& 1 — O(1) with :binary.at/2, no giant bit
  # string and no O(n) String.length per symbol.
  #
  # We build the candidate code MSB-first as an integer. Huffman codes are
  # prefix-free, so the first {len, code} hit in the table is the right symbol.
  # Once the accumulated length reaches the longest code with no match, no later
  # bit can match (an incomplete table fed unmatched bits), so we fail at once.

  defp decode_bits(_payload, _total, _table, _max, 0, _pos, _code, _len, out) do
    out |> Enum.reverse() |> :binary.list_to_bin()
  end

  defp decode_bits(payload, total, table, max_len, remaining, pos, code, len, out) do
    if pos >= total do
      {:error, :bit_stream_exhausted}
    else
      byte = :binary.at(payload, pos >>> 3)
      bit = (byte >>> (pos &&& 7)) &&& 1
      next_code = (code <<< 1) ||| bit
      next_len = len + 1

      case Map.fetch(table, {next_len, next_code}) do
        {:ok, sym} ->
          decode_bits(payload, total, table, max_len, remaining - 1, pos + 1, 0, 0, [sym | out])

        :error when next_len >= max_len ->
          {:error, :invalid_code}

        :error ->
          decode_bits(payload, total, table, max_len, remaining, pos + 1, next_code, next_len, out)
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
