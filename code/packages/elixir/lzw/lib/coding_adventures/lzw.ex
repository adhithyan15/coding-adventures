defmodule CodingAdventures.LZW do
  import Bitwise

  @moduledoc """
  LZW lossless compression algorithm (1984) — CMP03.

  ## What Is LZW?

  LZW (Lempel-Ziv-Welch) is LZ78 with a pre-seeded dictionary: all 256
  single-byte sequences are already in the dictionary before encoding begins
  (codes 0–255). This eliminates LZ78's mandatory `next_char` byte — every
  input byte is already a known code, so the encoder only emits codes, never
  raw bytes.

  With only codes to transmit, LZW uses variable-width bit-packing: codes start
  at 9 bits (covering codes 0–511) and grow automatically as the dictionary
  fills up. This is exactly how GIF compression works.

  ## The Pre-Seeded Dictionary

  Think of the dictionary as a table of translations: byte sequence → code.
  The key insight is that because every possible single byte is pre-loaded, the
  encoder never gets stuck. If the current prefix plus the next byte isn't in
  the dictionary yet, the encoder can always fall back to emitting the code for
  the current prefix (which IS in the dictionary), then start fresh with just
  the next byte.

  ## Reserved Codes

  ```
  0–255:  Single-byte entries (pre-seeded at startup).
  256:    CLEAR_CODE — reset encoder and decoder to the initial 258-entry state.
  257:    STOP_CODE  — signals end of the compressed code stream.
  258+:   Dynamically added dictionary entries.
  ```

  ## Wire Format (CMP03)

  ```
  Bytes 0–3:  original_length  (big-endian uint32)
  Bytes 4+:   variable-width bit-packed codes, LSB-first
  ```

  The LSB-first bit order is the GIF convention: the first code starts at bit 0
  of byte 4, continues into bit 1, …, and the second code immediately follows.

  ## Variable-Width Code Sizes

  | Dictionary size  | Code size |
  |------------------|-----------|
  | 258–512 entries  |  9 bits   |
  | 513–1024 entries | 10 bits   |
  | …                | …         |
  | 32769–65536      | 16 bits   |

  The encoder increments `code_size` whenever `next_code > (1 <<< code_size)`.
  The decoder mirrors this bump exactly by tracking `next_code` independently —
  both sides always agree on the current code width.

  ## The Tricky Token

  During decoding, it's possible to receive a code that hasn't been added to
  the decode dictionary yet (code == next_code). This happens with inputs of
  the form `xyx...x` where a sequence starts and ends with the same byte. The
  fix is well-known:

  ```
  entry = dict[prev_code] ++ [List.first(dict[prev_code])]
  ```

  i.e. the missing entry is the previous entry extended by its own first byte.

  ## Dictionary Reset (CLEAR_CODE)

  When the dictionary reaches 65536 entries (MAX_CODE_SIZE = 16 bits), the
  encoder emits CLEAR_CODE and both sides reset to the initial 258-entry state
  with code_size back to 9. This prevents code overflow and lets compression
  adapt to changing data patterns.

  ## Series

      CMP00 (LZ77,    1977) — Sliding-window backreferences.
      CMP01 (LZ78,    1978) — Explicit dictionary (trie).
      CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
      CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; GIF. ← this module
      CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
      CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.

  ## Examples

      iex> data = "hello hello hello"
      iex> CodingAdventures.LZW.decompress(CodingAdventures.LZW.compress(data)) == data
      true

      iex> CodingAdventures.LZW.decompress(CodingAdventures.LZW.compress("ABABAB")) == "ABABAB"
      true

      iex> CodingAdventures.LZW.decompress(CodingAdventures.LZW.compress("AAAAAAA")) == "AAAAAAA"
      true
  """

  # ─── Constants ──────────────────────────────────────────────────────────────

  # CLEAR_CODE (256): tells the decoder to discard its current dictionary and
  # restart from the 258-entry initial state with code_size = 9.
  @clear_code 256

  # STOP_CODE (257): signals the end of the compressed code stream.
  @stop_code 257

  # The first dynamically assigned code. Codes 0–255 are the pre-seeded
  # single-byte entries; 256 = CLEAR_CODE; 257 = STOP_CODE.
  @initial_next_code 258

  # Codes start at 9 bits so they can represent 0–511, which covers the
  # initial 258 entries with plenty of room for the first ~254 dynamic entries.
  @initial_code_size 9

  # Maximum bit width. At 16 bits the dictionary holds up to 65536 entries.
  # When next_code reaches 65536, the encoder emits CLEAR_CODE and resets.
  @max_code_size 16

  # ─── Encoder ────────────────────────────────────────────────────────────────

  @doc """
  Encode a binary into a list of LZW codes.

  The returned list starts with CLEAR_CODE (256) and ends with STOP_CODE (257).
  Intermediate codes are either pre-seeded single-byte codes (0–255) or dynamic
  dictionary codes (258+).

  ## Examples

      iex> codes = CodingAdventures.LZW.encode_codes("ABABAB")
      iex> List.first(codes) == 256  # CLEAR_CODE
      true
      iex> List.last(codes) == 257   # STOP_CODE
      true
  """
  def encode_codes(data) when is_binary(data) do
    bytes = :binary.bin_to_list(data)

    # Build the initial encode dictionary: byte sequence (list) → code.
    # Keys are charlists (lists of bytes) so multi-byte entries share the same
    # type as single-byte entries.
    init_dict = Enum.into(0..255, %{}, fn b -> {[b], b} end)

    # Emit the opening CLEAR_CODE to tell the decoder to start fresh.
    init_codes = [@clear_code]

    {final_codes, final_dict, _next_code, final_prefix} =
      do_encode(bytes, init_codes, init_dict, @initial_next_code, [])

    # Flush the last accumulated prefix. It is always in the dictionary
    # (either a pre-seeded single-byte entry or a previously added sequence).
    codes_flushed =
      if final_prefix != [] do
        final_codes ++ [Map.fetch!(final_dict, final_prefix)]
      else
        final_codes
      end

    codes_flushed ++ [@stop_code]
  end

  # Recursive encoder with explicit state.
  defp do_encode([], codes, dict, next_code, prefix) do
    {codes, dict, next_code, prefix}
  end

  defp do_encode([byte | rest], codes, dict, next_code, prefix) do
    candidate = prefix ++ [byte]

    if Map.has_key?(dict, candidate) do
      # Candidate is already known — keep extending the prefix.
      do_encode(rest, codes, dict, next_code, candidate)
    else
      # Emit the code for the current prefix.
      emitted_code = Map.fetch!(dict, prefix)
      new_codes = codes ++ [emitted_code]

      # Try to add the candidate (prefix ++ [byte]) to the dictionary.
      {new_dict, new_next, reset_codes} =
        cond do
          next_code < (1 <<< @max_code_size) ->
            # Normal case: add candidate.
            {Map.put(dict, candidate, next_code), next_code + 1, new_codes}

          next_code == (1 <<< @max_code_size) ->
            # Dictionary full. Emit CLEAR_CODE, reset dictionary.
            reset_dict = Enum.into(0..255, %{}, fn b -> {[b], b} end)
            {reset_dict, @initial_next_code, new_codes ++ [@clear_code]}

          true ->
            {dict, next_code, new_codes}
        end

      # Restart prefix with just the current byte.
      do_encode(rest, reset_codes, new_dict, new_next, [byte])
    end
  end

  # ─── Decoder ────────────────────────────────────────────────────────────────

  @doc """
  Decode a list of LZW codes back into a binary.

  Accepts the raw code list as produced by `encode_codes/1`. Handles
  CLEAR_CODE resets and the tricky-token edge case.

  ## Examples

      iex> codes = CodingAdventures.LZW.encode_codes("hello")
      iex> CodingAdventures.LZW.decode_codes(codes) == "hello"
      true
  """
  def decode_codes(codes) when is_list(codes) do
    # Build the initial decode dictionary: code → byte list.
    # We use an Erlang array for O(1) indexed lookup as the dictionary grows.
    init_dict = build_initial_decode_dict()

    {output, _dict, _next_code, _prev_code} =
      do_decode(codes, [], init_dict, @initial_next_code, :none)

    :erlang.list_to_binary(output)
  end

  # Build a map keyed by integer code for the initial 258 entries.
  # Entries 0–255: single byte. 256 (CLEAR) and 257 (STOP): nil placeholders.
  defp build_initial_decode_dict do
    base = Enum.into(0..255, %{}, fn b -> {b, [b]} end)
    base |> Map.put(@clear_code, nil) |> Map.put(@stop_code, nil)
  end

  defp do_decode([], output, dict, next_code, prev_code) do
    {Enum.reverse(output), dict, next_code, prev_code}
  end

  defp do_decode([code | rest], output, dict, next_code, prev_code) do
    cond do
      code == @clear_code ->
        # Reset: rebuild initial dictionary, restart code_size tracking.
        fresh_dict = build_initial_decode_dict()
        do_decode(rest, output, fresh_dict, @initial_next_code, :none)

      code == @stop_code ->
        # Stop: ignore the rest of the code stream.
        {Enum.reverse(output), dict, next_code, prev_code}

      true ->
        # Resolve the entry for this code.
        entry =
          cond do
            Map.has_key?(dict, code) ->
              # Normal case: code is already in the dictionary.
              Map.fetch!(dict, code)

            code == next_code and prev_code != :none ->
              # Tricky token: decoder receives code == next_code, meaning the
              # entry hasn't been added yet. The entry is dict[prev_code]
              # extended by its own first byte. This happens with input like
              # "AAAAAAA" where the encoder emits a code for a sequence the
              # decoder has just started to build.
              prev_entry = Map.fetch!(dict, prev_code)
              prev_entry ++ [List.first(prev_entry)]

            true ->
              # Malformed code — skip gracefully.
              nil
          end

        case entry do
          nil ->
            # Invalid code: skip and continue without updating the dictionary.
            do_decode(rest, output, dict, next_code, prev_code)

          _ ->
            # Append the entry bytes to output (reversed list for efficiency).
            new_output = Enum.reverse(entry) ++ output

            # Add a new entry to the dictionary if we have a previous code.
            # New entry = dict[prev_code] ++ [first byte of current entry].
            {new_dict, new_next} =
              if prev_code != :none and next_code < (1 <<< @max_code_size) do
                prev_entry = Map.fetch!(dict, prev_code)
                new_entry = prev_entry ++ [List.first(entry)]
                {Map.put(dict, next_code, new_entry), next_code + 1}
              else
                {dict, next_code}
              end

            do_decode(rest, new_output, new_dict, new_next, code)
        end
    end
  end

  # ─── Bit I/O ────────────────────────────────────────────────────────────────
  #
  # LZW uses LSB-first bit packing (GIF convention). Bit 0 of the first code
  # lands in bit 0 of the first byte; bit 8 of the first code lands in bit 0
  # of the second byte (if code_size == 9). This is the opposite of the
  # big-endian / MSB-first convention used in many protocols.
  #
  # Example for two 9-bit codes:
  #   Code A = 0b1_0000_0001 (257)
  #   Code B = 0b0_0000_0010 (2)
  #   Packed bytes: [0b0000_0001, 0b0000_0100, 0b0000_0000]
  #                              ^^^^^^^^^^^^  bit 0–7 of A
  #                              bit 8 of A → bit 0 of byte 1
  #                              bits 1–8 of B in byte 1 & 2

  @doc """
  Pack a list of LZW codes into CMP03 wire-format binary.

  Prepends a 4-byte big-endian `original_length` header, then appends
  LSB-first variable-width bit-packed codes.

  The code_size starts at `@initial_code_size` (9) and grows when
  `next_code > (1 <<< code_size)`.

  ## Examples

      iex> codes = CodingAdventures.LZW.encode_codes("A")
      iex> packed = CodingAdventures.LZW.pack_codes(codes, 1)
      iex> byte_size(packed) >= 4
      true
  """
  def pack_codes(codes, original_length) when is_list(codes) and is_integer(original_length) do
    # BitWriter state: {bit_buffer :: integer, bits_in_buffer :: integer, byte_list :: list}
    initial_writer = {0, 0, []}

    {{final_buf, final_bits, final_bytes}, _final_code_size, _final_next_code} =
      Enum.reduce(codes, {initial_writer, @initial_code_size, @initial_next_code},
        fn code, {{buf, bits, bytes}, code_size, next_code} ->
          # Write `code_size` bits of `code` into the buffer (LSB-first).
          {new_buf, new_bits, new_bytes} = bit_write(buf, bits, bytes, code, code_size)

          # Update code_size and next_code tracking.
          {new_code_size, new_next} =
            cond do
              code == @clear_code ->
                {@initial_code_size, @initial_next_code}

              code == @stop_code ->
                {code_size, next_code}

              next_code < (1 <<< @max_code_size) ->
                nn = next_code + 1
                ns =
                  if nn > (1 <<< code_size) and code_size < @max_code_size do
                    code_size + 1
                  else
                    code_size
                  end
                {ns, nn}

              true ->
                {code_size, next_code}
            end

          {{new_buf, new_bits, new_bytes}, new_code_size, new_next}
        end)

    # Flush any remaining bits in the buffer (pad with zeros to fill the last byte).
    flushed_bytes =
      if final_bits > 0 do
        [final_buf &&& 0xFF | final_bytes]
      else
        final_bytes
      end

    # Reverse the accumulated bytes (we prepended for O(1) append).
    body = :erlang.list_to_binary(Enum.reverse(flushed_bytes))

    # Prepend the 4-byte big-endian original_length header.
    <<original_length::unsigned-big-integer-32>> <> body
  end

  # Write `code_size` bits of `code` into the bit buffer, flushing complete
  # bytes to `bytes` as they fill up.
  #
  # The buffer accumulates bits from LSB upward. When `bits_in_buffer >= 8`
  # we extract the bottom 8 bits as a completed byte and shift the buffer down.
  defp bit_write(buf, bits, bytes, code, code_size) do
    # OR the code (shifted left by `bits`) into the buffer.
    new_buf  = buf ||| (code <<< bits)
    new_bits = bits + code_size
    flush_bytes(new_buf, new_bits, bytes)
  end

  defp flush_bytes(buf, bits, bytes) when bits >= 8 do
    flush_bytes(buf >>> 8, bits - 8, [buf &&& 0xFF | bytes])
  end

  defp flush_bytes(buf, bits, bytes), do: {buf, bits, bytes}

  @doc """
  Unpack CMP03 wire-format binary into a list of LZW codes.

  Returns `{codes, original_length}`. Stops reading at STOP_CODE or stream
  exhaustion.

  Short input (< 4 bytes) returns `{[CLEAR_CODE, STOP_CODE], 0}` — a safe
  empty stream that round-trips to the empty binary.

  ## Examples

      iex> packed = CodingAdventures.LZW.pack_codes([256, 257], 0)
      iex> {codes, orig_len} = CodingAdventures.LZW.unpack_codes(packed)
      iex> orig_len == 0
      true
  """
  def unpack_codes(data) when is_binary(data) and byte_size(data) < 4 do
    # Too short to contain a valid header — return a safe empty stream.
    {[@clear_code, @stop_code], 0}
  end

  def unpack_codes(data) when is_binary(data) do
    <<original_length::unsigned-big-integer-32, body::binary>> = data

    body_bytes = :binary.bin_to_list(body)

    # BitReader state: {byte_list, bit_buffer :: integer, bits_in_buffer :: integer}
    initial_reader = {body_bytes, 0, 0}

    {codes, _reader} =
      do_unpack(initial_reader, @initial_code_size, @initial_next_code, [])

    {codes, original_length}
  end

  defp do_unpack(reader, code_size, next_code, acc) do
    case bit_read(reader, code_size) do
      {:ok, code, new_reader} ->
        new_acc = [code | acc]

        cond do
          code == @stop_code ->
            {Enum.reverse(new_acc), new_reader}

          code == @clear_code ->
            do_unpack(new_reader, @initial_code_size, @initial_next_code, new_acc)

          true ->
            {new_next, new_size} =
              if next_code < (1 <<< @max_code_size) do
                nn = next_code + 1
                ns =
                  if nn > (1 <<< code_size) and code_size < @max_code_size do
                    code_size + 1
                  else
                    code_size
                  end
                {nn, ns}
              else
                {next_code, code_size}
              end

            do_unpack(new_reader, new_size, new_next, new_acc)
        end

      :eof ->
        {Enum.reverse(acc), reader}
    end
  end

  # Read `code_size` bits from the reader (LSB-first).
  # Returns {:ok, code, new_reader} or :eof.
  defp bit_read({bytes, buf, bits}, code_size) do
    {bytes2, buf2, bits2} = refill(bytes, buf, bits, code_size)

    if bits2 >= code_size do
      code = buf2 &&& ((1 <<< code_size) - 1)
      {:ok, code, {bytes2, buf2 >>> code_size, bits2 - code_size}}
    else
      :eof
    end
  end

  # Pull bytes from the byte list into the buffer until we have at least
  # `need` bits or run out of input.
  defp refill([], buf, bits, _need), do: {[], buf, bits}

  defp refill([byte | rest] = bytes, buf, bits, need) do
    if bits >= need do
      {bytes, buf, bits}
    else
      new_buf  = buf ||| (byte <<< bits)
      new_bits = bits + 8
      refill(rest, new_buf, new_bits, need)
    end
  end

  # ─── One-shot API ────────────────────────────────────────────────────────────

  @doc """
  Compress a binary using LZW, returning the CMP03 wire format.

  The returned binary begins with a 4-byte big-endian `original_length`
  header, followed by LSB-first variable-width bit-packed LZW codes.

  ## Examples

      iex> compressed = CodingAdventures.LZW.compress("hello world")
      iex> CodingAdventures.LZW.decompress(compressed) == "hello world"
      true

      iex> CodingAdventures.LZW.decompress(CodingAdventures.LZW.compress("")) == ""
      true
  """
  def compress(data) when is_binary(data) do
    codes = encode_codes(data)
    pack_codes(codes, byte_size(data))
  end

  @doc """
  Decompress CMP03 wire-format bytes back to the original binary.

  Returns `{:error, :too_short}` if the input is fewer than 4 bytes (no valid
  header), allowing callers to handle corrupt/truncated input gracefully.

  For convenience, the happy path returns the binary directly (not wrapped in
  `:ok`), mirroring the API style of LZSS in this series.

  ## Examples

      iex> CodingAdventures.LZW.decompress(CodingAdventures.LZW.compress("ABABAB")) == "ABABAB"
      true

      iex> CodingAdventures.LZW.decompress(<<1, 2>>) == {:error, :too_short}
      true
  """
  def decompress(data) when is_binary(data) and byte_size(data) < 4 do
    {:error, :too_short}
  end

  def decompress(data) when is_binary(data) do
    {codes, original_length} = unpack_codes(data)
    result = decode_codes(codes)

    # Truncate to original_length in case the decoder over-produces.
    if byte_size(result) > original_length do
      binary_part(result, 0, original_length)
    else
      result
    end
  end
end
