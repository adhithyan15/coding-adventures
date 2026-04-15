defmodule CodingAdventures.LZWTest do
  use ExUnit.Case, async: true
  alias CodingAdventures.LZW

  # Convenience: compress then decompress.
  defp rt(data), do: LZW.decompress(LZW.compress(data))

  # ─── Encode codes ───────────────────────────────────────────────────────────

  test "encode_codes empty: starts with CLEAR, ends with STOP" do
    codes = LZW.encode_codes("")
    assert List.first(codes) == 256
    assert List.last(codes) == 257
  end

  test "encode_codes single byte: CLEAR, <byte>, STOP" do
    codes = LZW.encode_codes("A")
    assert codes == [256, 65, 257]
  end

  test "encode_codes AB: CLEAR, 65, 66, STOP" do
    codes = LZW.encode_codes("AB")
    assert codes == [256, 65, 66, 257]
  end

  test "encode_codes ABABAB: has fewer codes than bytes (compression)" do
    codes = LZW.encode_codes("ABABAB")
    # Expect: CLEAR, A, B, AB (258), AB (258), STOP — 6 total is possible,
    # but at minimum the output must be shorter than 6 plain codes.
    data_codes = codes -- [256, 257]
    assert length(data_codes) < 6
  end

  test "encode_codes AAAAAAA: first CLEAR then single A code then multi-A codes" do
    codes = LZW.encode_codes("AAAAAAA")
    assert List.first(codes) == 256
    assert List.last(codes) == 257
    # At minimum: CLEAR, A(65), AA(258), AAAA(259 or similar), STOP
    assert length(codes) < 9
  end

  test "encode_codes first element is always CLEAR_CODE" do
    for input <- ["", "A", "hello", "ABCABC"] do
      assert List.first(LZW.encode_codes(input)) == 256
    end
  end

  test "encode_codes last element is always STOP_CODE" do
    for input <- ["", "A", "hello", "ABCABC"] do
      assert List.last(LZW.encode_codes(input)) == 257
    end
  end

  # ─── Decode codes ───────────────────────────────────────────────────────────

  test "decode_codes empty code stream" do
    assert LZW.decode_codes([256, 257]) == ""
  end

  test "decode_codes single byte" do
    assert LZW.decode_codes([256, 65, 257]) == "A"
  end

  test "decode_codes two different bytes" do
    assert LZW.decode_codes([256, 65, 66, 257]) == "AB"
  end

  test "decode_codes round-trips encode_codes for ABABAB" do
    codes = LZW.encode_codes("ABABAB")
    assert LZW.decode_codes(codes) == "ABABAB"
  end

  test "decode_codes round-trips encode_codes for AAAAAAA (tricky token)" do
    # This is the classic tricky-token case: the decoder encounters
    # code == next_code because the encoder built an entry from the
    # same repeated byte.
    codes = LZW.encode_codes("AAAAAAA")
    assert LZW.decode_codes(codes) == "AAAAAAA"
  end

  test "decode_codes ignores trailing data after STOP_CODE" do
    # Extra data after STOP should not affect the output.
    codes = [256, 65, 257, 66, 67]
    assert LZW.decode_codes(codes) == "A"
  end

  test "decode_codes handles CLEAR_CODE mid-stream" do
    # CLEAR resets the dictionary; subsequent codes are interpreted from scratch.
    codes = [256, 65, 256, 66, 257]
    assert LZW.decode_codes(codes) == "AB"
  end

  # ─── Bit packing ────────────────────────────────────────────────────────────

  test "pack/unpack round-trip for empty code stream" do
    codes_in = [256, 257]
    packed = LZW.pack_codes(codes_in, 0)
    {codes_out, orig_len} = LZW.unpack_codes(packed)
    assert orig_len == 0
    assert codes_out == [256, 257]
  end

  test "pack_codes produces 4-byte header" do
    packed = LZW.pack_codes([256, 257], 42)
    <<orig_len::unsigned-big-integer-32, _::binary>> = packed
    assert orig_len == 42
  end

  test "pack/unpack round-trip preserves original_length" do
    codes = LZW.encode_codes("hello world")
    packed = LZW.pack_codes(codes, 11)
    {_codes_out, orig_len} = LZW.unpack_codes(packed)
    assert orig_len == 11
  end

  test "pack/unpack round-trip for ABABAB" do
    codes = LZW.encode_codes("ABABAB")
    packed = LZW.pack_codes(codes, 6)
    {codes_out, _orig_len} = LZW.unpack_codes(packed)
    assert codes_out == codes
  end

  # ─── Compress / Decompress API ──────────────────────────────────────────────

  test "compress returns binary" do
    assert is_binary(LZW.compress("hello"))
  end

  test "compress stores original_length in header" do
    compressed = LZW.compress("hello")
    <<orig_len::unsigned-big-integer-32, _::binary>> = compressed
    assert orig_len == 5
  end

  test "compress is deterministic" do
    data = "hello world test"
    assert LZW.compress(data) == LZW.compress(data)
  end

  test "compress empty produces valid header" do
    compressed = LZW.compress("")
    <<orig_len::unsigned-big-integer-32, _::binary>> = compressed
    assert orig_len == 0
    assert byte_size(compressed) >= 4
  end

  # ─── Spec vectors (must pass per CMP03 spec) ────────────────────────────────

  test "rt empty" do
    assert rt("") == ""
  end

  test "rt single byte A" do
    assert rt("A") == "A"
  end

  test "rt AB" do
    assert rt("AB") == "AB"
  end

  test "rt ABABAB (spec vector 4)" do
    assert rt("ABABAB") == "ABABAB"
  end

  test "rt AAAAAAA (spec vector 5 — tricky token)" do
    assert rt("AAAAAAA") == "AAAAAAA"
  end

  test "rt all 256 bytes" do
    data = :binary.list_to_bin(Enum.to_list(0..255))
    assert rt(data) == data
  end

  # ─── Round-trip breadth ─────────────────────────────────────────────────────

  test "rt single character" do
    assert rt("Z") == "Z"
  end

  test "rt no repetition" do
    assert rt("ABCDE") == "ABCDE"
  end

  test "rt hello world" do
    assert rt("hello world") == "hello world"
  end

  test "rt null bytes" do
    data = <<0, 0, 0, 255, 255>>
    assert rt(data) == data
  end

  test "rt repeated pattern ABC*100" do
    data = String.duplicate("ABC", 100)
    assert rt(data) == data
  end

  test "rt long ABCDEF*500" do
    data = String.duplicate("ABCDEF", 500)
    assert rt(data) == data
  end

  test "rt repeated single byte *10000" do
    data = String.duplicate(<<0x42>>, 10000)
    assert rt(data) == data
  end

  test "rt cycling bytes 0..299 mod 3" do
    data = :binary.list_to_bin(Enum.map(0..299, fn i -> rem(i, 3) end))
    assert rt(data) == data
  end

  test "rt mixed ascii and binary" do
    data = "prefix" <> <<0, 1, 2, 3, 255>> <> "suffix"
    assert rt(data) == data
  end

  # ─── Compression effectiveness ──────────────────────────────────────────────

  test "repetitive data compresses smaller" do
    data = String.duplicate("ABC", 1000)
    assert byte_size(LZW.compress(data)) < byte_size(data)
  end

  test "highly repetitive single byte compresses significantly" do
    data = String.duplicate(<<0x42>>, 10000)
    compressed = LZW.compress(data)
    assert byte_size(compressed) < byte_size(data)
    assert rt(data) == data
  end

  # ─── Security / robustness ──────────────────────────────────────────────────

  test "decompress with too-short input returns error tuple" do
    assert LZW.decompress(<<1, 2>>) == {:error, :too_short}
  end

  test "decompress with 3-byte input returns error tuple" do
    assert LZW.decompress(<<0, 0, 0>>) == {:error, :too_short}
  end

  test "decompress with exactly 4-byte input does not crash" do
    result = LZW.decompress(<<0, 0, 0, 5>>)
    assert is_binary(result)
  end

  test "decompress random bytes does not crash" do
    random = :crypto.strong_rand_bytes(100)
    result = LZW.decompress(random)
    assert is_binary(result)
  end

  test "decompress crafted all-zeros payload does not crash" do
    payload = <<0, 0, 0, 0>> <> :binary.copy(<<0>>, 20)
    result = LZW.decompress(payload)
    assert is_binary(result)
  end

  test "decompress truncated compressed data does not crash" do
    compressed = LZW.compress("hello world hello world")
    # Chop the compressed stream to simulate truncation.
    truncated = binary_part(compressed, 0, div(byte_size(compressed), 2))
    result = LZW.decompress(truncated)
    assert is_binary(result)
  end
end
