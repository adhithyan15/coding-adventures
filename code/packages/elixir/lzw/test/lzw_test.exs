defmodule CodingAdventures.LZWTest do
  use ExUnit.Case, async: true
  alias CodingAdventures.LZW

  # ---- Constants -------------------------------------------------------------

  test "constants" do
    assert LZW.clear_code() == 256
    assert LZW.stop_code() == 257
    assert LZW.initial_next_code() == 258
    assert LZW.initial_code_size() == 9
    assert LZW.max_code_size() == 16
  end

  # ---- encode_codes ----------------------------------------------------------

  test "encode_codes empty" do
    {codes, orig} = LZW.encode_codes(<<>>)
    assert orig == 0
    assert hd(codes) == LZW.clear_code()
    assert List.last(codes) == LZW.stop_code()
    assert length(codes) == 2
  end

  test "encode_codes single byte" do
    {codes, orig} = LZW.encode_codes(<<"A">>)
    assert orig == 1
    assert hd(codes) == LZW.clear_code()
    assert List.last(codes) == LZW.stop_code()
    assert 65 in codes
  end

  test "encode_codes two distinct" do
    {codes, _orig} = LZW.encode_codes(<<"AB">>)
    assert codes == [LZW.clear_code(), 65, 66, LZW.stop_code()]
  end

  test "encode_codes ABABAB" do
    {codes, _orig} = LZW.encode_codes(<<"ABABAB">>)
    assert codes == [LZW.clear_code(), 65, 66, 258, 258, LZW.stop_code()]
  end

  test "encode_codes AAAAAAA" do
    {codes, _orig} = LZW.encode_codes(<<"AAAAAAA">>)
    assert codes == [LZW.clear_code(), 65, 258, 259, 65, LZW.stop_code()]
  end

  # ---- decode_codes ----------------------------------------------------------

  test "decode_codes empty stream" do
    assert LZW.decode_codes([LZW.clear_code(), LZW.stop_code()]) == <<>>
  end

  test "decode_codes single byte" do
    assert LZW.decode_codes([LZW.clear_code(), 65, LZW.stop_code()]) == <<"A">>
  end

  test "decode_codes two distinct" do
    assert LZW.decode_codes([LZW.clear_code(), 65, 66, LZW.stop_code()]) == <<"AB">>
  end

  test "decode_codes ABABAB" do
    result = LZW.decode_codes([LZW.clear_code(), 65, 66, 258, 258, LZW.stop_code()])
    assert result == <<"ABABAB">>
  end

  test "decode_codes AAAAAAA tricky token" do
    result = LZW.decode_codes([LZW.clear_code(), 65, 258, 259, 65, LZW.stop_code()])
    assert result == <<"AAAAAAA">>
  end

  test "decode_codes clear mid stream" do
    result = LZW.decode_codes([LZW.clear_code(), 65, LZW.clear_code(), 66, LZW.stop_code()])
    assert result == <<"AB">>
  end

  test "decode_codes invalid code skipped" do
    result = LZW.decode_codes([LZW.clear_code(), 9999, 65, LZW.stop_code()])
    assert result == <<"A">>
  end

  # ---- pack / unpack ---------------------------------------------------------

  test "pack_codes stores original_length in header" do
    packed = LZW.pack_codes([LZW.clear_code(), LZW.stop_code()], 42)
    <<stored::big-unsigned-integer-size(32), _rest::binary>> = packed
    assert stored == 42
  end

  test "pack/unpack roundtrip ABABAB" do
    codes = [LZW.clear_code(), 65, 66, 258, 258, LZW.stop_code()]
    packed = LZW.pack_codes(codes, 6)
    {unpacked, orig} = LZW.unpack_codes(packed)
    assert orig == 6
    assert unpacked == codes
  end

  test "pack/unpack roundtrip AAAAAAA" do
    codes = [LZW.clear_code(), 65, 258, 259, 65, LZW.stop_code()]
    packed = LZW.pack_codes(codes, 7)
    {unpacked, orig} = LZW.unpack_codes(packed)
    assert orig == 7
    assert unpacked == codes
  end

  test "unpack_codes short data" do
    {codes, orig} = LZW.unpack_codes(<<0, 0>>)
    assert is_list(codes)
    assert is_integer(orig)
  end

  # ---- compress / decompress -------------------------------------------------

  defp rt(data), do: LZW.decompress(LZW.compress(data))

  test "compress empty" do
    assert rt(<<>>) == <<>>
  end

  test "compress single byte" do
    assert rt(<<"A">>) == <<"A">>
  end

  test "compress two distinct" do
    assert rt(<<"AB">>) == <<"AB">>
  end

  test "compress ABABAB" do
    assert rt(<<"ABABAB">>) == <<"ABABAB">>
  end

  test "compress AAAAAAA tricky token" do
    assert rt(<<"AAAAAAA">>) == <<"AAAAAAA">>
  end

  test "compress AABABC" do
    assert rt(<<"AABABC">>) == <<"AABABC">>
  end

  test "compress long string" do
    data = String.duplicate("the quick brown fox jumps over the lazy dog ", 20)
    assert rt(data) == data
  end

  test "compress binary data" do
    data = :binary.list_to_bin(Enum.to_list(0..255) ++ Enum.to_list(0..255))
    assert rt(data) == data
  end

  test "compress all zeros" do
    data = :binary.copy(<<0>>, 100)
    assert rt(data) == data
  end

  test "compress all 0xFF" do
    data = :binary.copy(<<0xFF>>, 100)
    assert rt(data) == data
  end

  test "compresses repetitive data" do
    data = String.duplicate("ABCABC", 100)
    compressed = LZW.compress(data)
    assert byte_size(compressed) < byte_size(data)
  end

  test "header contains original_length" do
    data = "hello world"
    compressed = LZW.compress(data)
    <<stored::big-unsigned-integer-size(32), _rest::binary>> = compressed
    assert stored == byte_size(data)
  end
end
