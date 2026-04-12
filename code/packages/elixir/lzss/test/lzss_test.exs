defmodule CodingAdventures.LZSSTest do
  use ExUnit.Case, async: true
  alias CodingAdventures.LZSS

  def rt(data), do: LZSS.decompress(LZSS.compress(data))

  # ─── Spec vectors ───────────────────────────────────────────────────────────

  test "encode empty" do
    assert LZSS.encode("") == []
  end

  test "encode single byte" do
    assert LZSS.encode("A") == [LZSS.literal(65)]
  end

  test "encode no repetition" do
    tokens = LZSS.encode("ABCDE")
    assert length(tokens) == 5
    assert Enum.all?(tokens, fn t -> t.kind == :literal end)
  end

  test "encode AABCBBABC" do
    tokens = LZSS.encode("AABCBBABC")
    assert length(tokens) == 7
    assert List.last(tokens) == LZSS.match(5, 3)
  end

  test "encode ABABAB" do
    assert LZSS.encode("ABABAB") == [
      LZSS.literal(65), LZSS.literal(66), LZSS.match(2, 4)
    ]
  end

  test "encode AAAAAAA" do
    assert LZSS.encode("AAAAAAA") == [LZSS.literal(65), LZSS.match(1, 6)]
  end

  # ─── Encode properties ──────────────────────────────────────────────────────

  test "match offset >= 1" do
    for tok <- LZSS.encode("ABABABAB") do
      if tok.kind == :match, do: assert(tok.offset >= 1)
    end
  end

  test "match length >= min_match" do
    for tok <- LZSS.encode("ABABABABABAB") do
      if tok.kind == :match, do: assert(tok.length >= 3)
    end
  end

  test "min_match large forces all literals" do
    tokens = LZSS.encode("ABABAB", 4096, 255, 100)
    assert Enum.all?(tokens, fn t -> t.kind == :literal end)
  end

  # ─── Decode ─────────────────────────────────────────────────────────────────

  test "decode empty" do
    assert LZSS.decode([], 0) == ""
  end

  test "decode single literal" do
    assert LZSS.decode([LZSS.literal(65)], 1) == "A"
  end

  test "decode overlapping match AAAAAAA" do
    tokens = [LZSS.literal(65), LZSS.match(1, 6)]
    assert LZSS.decode(tokens, 7) == "AAAAAAA"
  end

  test "decode ABABAB" do
    tokens = [LZSS.literal(65), LZSS.literal(66), LZSS.match(2, 4)]
    assert LZSS.decode(tokens, 6) == "ABABAB"
  end

  # ─── Round-trip ─────────────────────────────────────────────────────────────

  test "rt empty"         do assert rt("") == ""                   end
  test "rt single"        do assert rt("A") == "A"                 end
  test "rt no repetition" do assert rt("ABCDE") == "ABCDE"         end
  test "rt all identical" do assert rt("AAAAAAA") == "AAAAAAA"     end
  test "rt ABABAB"        do assert rt("ABABAB") == "ABABAB"        end
  test "rt AABCBBABC"     do assert rt("AABCBBABC") == "AABCBBABC" end
  test "rt hello world"   do assert rt("hello world") == "hello world" end

  test "rt ABC*100" do
    data = String.duplicate("ABC", 100)
    assert rt(data) == data
  end

  test "rt binary nulls" do
    data = <<0, 0, 0, 255, 255>>
    assert rt(data) == data
  end

  test "rt full byte range" do
    data = :binary.list_to_bin(Enum.to_list(0..255))
    assert rt(data) == data
  end

  test "rt repeated pattern" do
    data = :binary.list_to_bin(Enum.map(0..299, fn i -> rem(i, 3) end))
    assert rt(data) == data
  end

  test "rt long ABCDEF" do
    data = String.duplicate("ABCDEF", 500)
    assert rt(data) == data
  end

  # ─── Wire format ────────────────────────────────────────────────────────────

  test "compress stores original length" do
    compressed = LZSS.compress("hello")
    <<orig_len::unsigned-big-integer-32, _::binary>> = compressed
    assert orig_len == 5
  end

  test "compress deterministic" do
    data = "hello world test"
    assert LZSS.compress(data) == LZSS.compress(data)
  end

  test "compress empty produces 8-byte header" do
    c = LZSS.compress("")
    <<orig_len::unsigned-big-integer-32, block_count::unsigned-big-integer-32>> = c
    assert orig_len   == 0
    assert block_count == 0
    assert byte_size(c) == 8
  end

  test "crafted large block_count is safe" do
    bad_header = <<4::unsigned-big-integer-32, 0x40000000::unsigned-big-integer-32>>
    payload    = bad_header <> <<0, 65, 66, 67, 68>>
    result     = LZSS.decompress(payload)
    assert is_binary(result)
  end

  # ─── Compression effectiveness ──────────────────────────────────────────────

  test "repetitive data compresses" do
    data = String.duplicate("ABC", 1000)
    assert byte_size(LZSS.compress(data)) < byte_size(data)
  end

  test "all same byte compresses" do
    data = String.duplicate(<<0x42>>, 10000)
    compressed = LZSS.compress(data)
    assert byte_size(compressed) < byte_size(data)
    assert LZSS.decompress(compressed) == data
  end
end
