defmodule CodingAdventures.LZW do
  @moduledoc """
  LZW (Lempel-Ziv-Welch, 1984) lossless compression algorithm.
  Part of the CMP compression series in the coding-adventures monorepo.

  ## What Is LZW?

  LZW is LZ78 with a pre-seeded dictionary: all 256 single-byte sequences are
  added before encoding begins (codes 0–255). This eliminates LZ78's mandatory
  `next_char` byte — every symbol is already in the dictionary, so the encoder
  can emit pure codes.

  With only codes to transmit, LZW uses variable-width bit-packing: codes start
  at 9 bits and grow as the dictionary expands. This is exactly how GIF works.

  ## Reserved Codes

      0–255:  Pre-seeded single-byte entries.
      256:    CLEAR_CODE — reset to initial 256-entry state.
      257:    STOP_CODE  — end of code stream.
      258+:   Dynamically added entries.

  ## Wire Format (CMP03)

      Bytes 0–3:  original_length (big-endian uint32)
      Bytes 4+:   bit-packed variable-width codes, LSB-first

  ## The Tricky Token

  During decoding the decoder may receive code C == next_code (not yet added).
  This happens when the input has the form xyx...x. The fix:

      entry = dict[prev_code] ++ [hd(dict[prev_code])]

  ## The Series

      CMP00 (LZ77,    1977) — Sliding-window backreferences.
      CMP01 (LZ78,    1978) — Explicit dictionary (trie).
      CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
      CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; GIF. (this module)
      CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
      CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
  """

  import Bitwise

  # ---------------------------------------------------------------------------
  # Constants
  # ---------------------------------------------------------------------------

  @clear_code 256
  @stop_code 257
  @initial_next 258
  @initial_code_size 9
  @max_code_size 16

  @doc "Reset code value (256)."
  def clear_code, do: @clear_code

  @doc "Stop code value (257)."
  def stop_code, do: @stop_code

  @doc "First dynamically assigned code (258)."
  def initial_next_code, do: @initial_next

  @doc "Starting bit-width for codes (9)."
  def initial_code_size, do: @initial_code_size

  @doc "Maximum bit-width (16)."
  def max_code_size, do: @max_code_size

  # ---------------------------------------------------------------------------
  # BitWriter helpers
  # ---------------------------------------------------------------------------

  # A bit-writer is represented as {buf, bit_pos, output_iolist}.
  # Bits are accumulated in `buf` (integer) starting at the LSB.
  # When `bit_pos` reaches 8 or more, whole bytes are extracted and
  # appended to `output_iolist`.

  defp bw_new(), do: {0, 0, []}

  defp bw_write({buf, bit_pos, out}, code, code_size) do
    new_buf = buf ||| (code <<< bit_pos)
    new_bit_pos = bit_pos + code_size
    bw_drain({new_buf, new_bit_pos, out})
  end

  defp bw_drain({buf, bit_pos, out}) when bit_pos >= 8 do
    bw_drain({buf >>> 8, bit_pos - 8, [out | [buf &&& 0xFF]]})
  end

  defp bw_drain(state), do: state

  defp bw_flush({buf, bit_pos, out}) when bit_pos > 0 do
    :erlang.iolist_to_binary([out | [buf &&& 0xFF]])
  end

  defp bw_flush({_buf, 0, out}) do
    :erlang.iolist_to_binary(out)
  end

  # ---------------------------------------------------------------------------
  # BitReader helpers
  # ---------------------------------------------------------------------------

  # A bit-reader state is {binary_data, byte_pos, buf, bit_pos}.
  # We lazily consume bytes from `binary_data` into `buf` as needed.

  defp br_new(data), do: {data, 0, 0, 0}

  # Read `code_size` bits. Returns {code, new_state} or :eof.
  defp br_read({data, pos, buf, bit_pos}, code_size) do
    case br_fill({data, pos, buf, bit_pos}, code_size) do
      :eof ->
        :eof

      {data2, pos2, buf2, bit_pos2} ->
        mask = (1 <<< code_size) - 1
        code = buf2 &&& mask
        {code, {data2, pos2, buf2 >>> code_size, bit_pos2 - code_size}}
    end
  end

  defp br_fill({data, pos, buf, bit_pos}, needed) when bit_pos >= needed do
    {data, pos, buf, bit_pos}
  end

  defp br_fill({data, pos, buf, bit_pos}, needed) do
    if pos >= byte_size(data) do
      :eof
    else
      byte = :binary.at(data, pos)
      br_fill({data, pos + 1, buf ||| (byte <<< bit_pos), bit_pos + 8}, needed)
    end
  end

  defp br_exhausted?({data, pos, _buf, bit_pos}) do
    pos >= byte_size(data) and bit_pos == 0
  end

  # ---------------------------------------------------------------------------
  # Encoder
  # ---------------------------------------------------------------------------

  @doc """
  Encode `data` (binary) into a list of LZW codes including CLEAR and STOP.

  Returns `{codes, original_length}`.
  """
  def encode_codes(data) when is_binary(data) do
    original_length = byte_size(data)

    # Seed encode dictionary: sequence (charlist) → code (integer).
    enc_dict =
      Enum.reduce(0..255, %{}, fn b, acc -> Map.put(acc, [b], b) end)

    bytes = :binary.bin_to_list(data)
    {codes_rev, enc_dict2, _next, w} = encode_loop(bytes, enc_dict, @initial_next, [], [])

    # Flush remaining prefix and emit STOP.
    final_code = Map.get(enc_dict2, w)
    codes = Enum.reverse([@stop_code, final_code | codes_rev])
    codes = if w == [], do: Enum.reverse([@stop_code | codes_rev]), else: codes
    {[@clear_code | codes], original_length}
  end

  defp encode_loop([], enc_dict, next_code, codes_acc, w) do
    {codes_acc, enc_dict, next_code, w}
  end

  defp encode_loop([byte | rest], enc_dict, next_code, codes_acc, w) do
    wb = w ++ [byte]
    max_entries = 1 <<< @max_code_size

    if Map.has_key?(enc_dict, wb) do
      encode_loop(rest, enc_dict, next_code, codes_acc, wb)
    else
      # Emit code for w.
      code_for_w = Map.get(enc_dict, w)
      {enc_dict2, next_code2, extra_codes} =
        if next_code < max_entries do
          {Map.put(enc_dict, wb, next_code), next_code + 1, []}
        else
          # Reset dictionary when full.
          fresh = Enum.reduce(0..255, %{}, fn b, acc -> Map.put(acc, [b], b) end)
          {fresh, @initial_next, [@clear_code]}
        end

      new_codes = extra_codes ++ [code_for_w | codes_acc]
      encode_loop(rest, enc_dict2, next_code2, new_codes, [byte])
    end
  end

  # ---------------------------------------------------------------------------
  # Decoder
  # ---------------------------------------------------------------------------

  @doc """
  Decode a list of LZW codes back to a binary.

  Handles CLEAR_CODE (reset), STOP_CODE (done), and the tricky-token
  edge case (code == next_code).
  """
  def decode_codes(codes) when is_list(codes) do
    # Seed decode dictionary indexed 0..257.
    dec_dict = build_initial_dec_dict()
    {output_rev, _} = decode_loop(codes, dec_dict, @initial_next, nil, [])
    :binary.list_to_bin(Enum.reverse(output_rev) |> List.flatten())
  end

  defp build_initial_dec_dict do
    base = Enum.reduce(0..255, %{}, fn b, acc -> Map.put(acc, b, [b]) end)
    base |> Map.put(@clear_code, []) |> Map.put(@stop_code, [])
  end

  defp decode_loop([], dec_dict, next_code, prev_code, out) do
    {out, {dec_dict, next_code, prev_code}}
  end

  defp decode_loop([@clear_code | rest], _dec_dict, _next_code, _prev_code, out) do
    decode_loop(rest, build_initial_dec_dict(), @initial_next, nil, out)
  end

  defp decode_loop([@stop_code | _rest], dec_dict, next_code, prev_code, out) do
    {out, {dec_dict, next_code, prev_code}}
  end

  defp decode_loop([code | rest], dec_dict, next_code, prev_code, out) do
    max_entries = 1 <<< @max_code_size

    # Resolve entry for this code.
    entry =
      cond do
        Map.has_key?(dec_dict, code) ->
          Map.get(dec_dict, code)

        code == next_code and not is_nil(prev_code) ->
          # Tricky token: code not yet in dict.
          prev_entry = Map.get(dec_dict, prev_code)
          prev_entry ++ [hd(prev_entry)]

        true ->
          # Invalid or malformed — skip.
          nil
      end

    if is_nil(entry) do
      decode_loop(rest, dec_dict, next_code, prev_code, out)
    else
      # Add new dictionary entry.
      {dec_dict2, next_code2} =
        if not is_nil(prev_code) and next_code < max_entries do
          prev_entry = Map.get(dec_dict, prev_code)
          new_entry = prev_entry ++ [hd(entry)]
          {Map.put(dec_dict, next_code, new_entry), next_code + 1}
        else
          {dec_dict, next_code}
        end

      decode_loop(rest, dec_dict2, next_code2, code, [entry | out])
    end
  end

  # ---------------------------------------------------------------------------
  # Serialisation
  # ---------------------------------------------------------------------------

  @doc """
  Pack a list of LZW codes into the CMP03 wire format.

  Header: 4-byte big-endian original_length.
  Body:   LSB-first variable-width bit-packed codes.
  """
  def pack_codes(codes, original_length) do
    {packed, _, _} =
      Enum.reduce(codes, {bw_new(), @initial_code_size, @initial_next}, fn code,
                                                                            {writer, code_size,
                                                                             next_code} ->
        writer2 = bw_write(writer, code, code_size)
        max = 1 <<< @max_code_size

        {code_size2, next_code2} =
          cond do
            code == @clear_code ->
              {@initial_code_size, @initial_next}

            code == @stop_code ->
              {code_size, next_code}

            next_code < max ->
              nc = next_code + 1
              cs = if nc > (1 <<< code_size) and code_size < @max_code_size, do: code_size + 1, else: code_size
              {cs, nc}

            true ->
              {code_size, next_code}
          end

        {writer2, code_size2, next_code2}
      end)

    body = bw_flush(packed)
    header = <<original_length::big-unsigned-integer-size(32)>>
    header <> body
  end

  @doc """
  Unpack CMP03 wire-format bytes into a list of LZW codes.

  Returns `{codes, original_length}`. Stops at STOP_CODE or stream exhaustion.
  """
  def unpack_codes(data) when is_binary(data) and byte_size(data) < 4 do
    {[@clear_code, @stop_code], 0}
  end

  def unpack_codes(data) when is_binary(data) do
    <<original_length::big-unsigned-integer-size(32), rest::binary>> = data
    reader = br_new(rest)
    {codes, _} = unpack_loop(reader, @initial_code_size, @initial_next, [])
    {Enum.reverse(codes), original_length}
  end

  defp unpack_loop(reader, code_size, next_code, codes_acc) do
    if br_exhausted?(reader) do
      {codes_acc, reader}
    else
      case br_read(reader, code_size) do
        :eof ->
          {codes_acc, reader}

        {code, reader2} ->
          max = 1 <<< @max_code_size

          {code_size2, next_code2} =
            cond do
              code == @stop_code ->
                {code_size, next_code}

              code == @clear_code ->
                {@initial_code_size, @initial_next}

              next_code < max ->
                nc = next_code + 1
                cs = if nc > (1 <<< code_size) and code_size < @max_code_size, do: code_size + 1, else: code_size
                {cs, nc}

              true ->
                {code_size, next_code}
            end

          if code == @stop_code do
            {[code | codes_acc], reader2}
          else
            unpack_loop(reader2, code_size2, next_code2, [code | codes_acc])
          end
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Compress `data` (binary) using LZW and return CMP03 wire-format bytes.
  """
  def compress(data) when is_binary(data) do
    {codes, original_length} = encode_codes(data)
    pack_codes(codes, original_length)
  end

  @doc """
  Decompress CMP03 wire-format `data` and return the original binary.
  """
  def decompress(data) when is_binary(data) do
    {codes, original_length} = unpack_codes(data)
    result = decode_codes(codes)
    binary_part(result, 0, min(byte_size(result), original_length))
  end
end
