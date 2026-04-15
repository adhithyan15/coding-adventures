defmodule CodingAdventures.HuffmanCompressionTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.HuffmanCompression

  # Convenience: compress then decompress.
  defp rt(data), do: HuffmanCompression.decompress(HuffmanCompression.compress(data))

  # ─── Basic round-trip ───────────────────────────────────────────────────────

  test "round-trip empty string" do
    assert rt("") == ""
  end

  test "round-trip single byte" do
    assert rt("A") == "A"
  end

  test "round-trip two distinct bytes" do
    assert rt("AB") == "AB"
  end

  test "round-trip all same byte" do
    assert rt("AAAAAAA") == "AAAAAAA"
  end

  test "round-trip hello world" do
    assert rt("hello world") == "hello world"
  end

  test "round-trip null bytes" do
    data = <<0, 0, 0, 255, 255>>
    assert rt(data) == data
  end

  test "round-trip long repetitive data" do
    data = String.duplicate("ABCDEF", 500)
    assert rt(data) == data
  end

  test "round-trip all 256 distinct bytes" do
    data = :binary.list_to_bin(Enum.to_list(0..255))
    assert rt(data) == data
  end

  test "round-trip cycling bytes" do
    data = :binary.list_to_bin(Enum.map(0..299, fn i -> rem(i, 4) end))
    assert rt(data) == data
  end

  test "round-trip mixed ascii and binary" do
    data = "prefix" <> <<0, 1, 2, 3, 255>> <> "suffix"
    assert rt(data) == data
  end

  test "round-trip highly repetitive single byte" do
    data = String.duplicate(<<0x42>>, 10000)
    assert rt(data) == data
  end

  test "round-trip repeated pattern of 3 bytes" do
    data = String.duplicate("ABC", 100)
    assert rt(data) == data
  end

  test "compress is deterministic" do
    data = "hello world test"
    assert HuffmanCompression.compress(data) == HuffmanCompression.compress(data)
  end

  # ─── Wire format verification for "AAABBC" ──────────────────────────────────
  #
  # "AAABBC" has 6 bytes. Symbol frequencies:
  #   A (65) → 3
  #   B (66) → 2
  #   C (67) → 1
  #
  # Huffman tree (canonical):
  #   A → "0"    (length 1)
  #   B → "10"   (length 2)
  #   C → "11"   (length 2)
  #
  # Sorted code-lengths by (length, symbol):
  #   A(65) → 1
  #   B(66) → 2
  #   C(67) → 2
  #
  # Encoded bits for "AAABBC":
  #   A="0", A="0", A="0", B="10", B="10", C="11"
  #   bits = "0" + "0" + "0" + "10" + "10" + "11" = "000101011"  (wait...)
  #
  # Actually canonical codes starting from code=0 for length 1:
  #   A(65) len=1: code=0   → "0"
  #   B(66) len=2: code=0<<(2-1)=0  wait, first code is 0 for len=1.
  #   Next code after A: code+1 = 1. B is len=2: 1 << (2-1) = 2 → "10"
  #   Next code after B: code+1 = 3. C is len=2: no shift needed → "11"
  #
  # Encoded bits for "AAABBC":
  #   A="0", A="0", A="0", B="10", B="10", C="11"
  #   All bits: "000" + "10" + "10" + "11" = "00010" + "1011" = "000101011"
  #   9 bits → packed into 2 bytes (9 bits, padded with 7 zeros to 16 bits).
  #
  # Wire format header:
  #   original_length = 6   → <<0, 0, 0, 6>>
  #   symbol_count    = 3   → <<0, 0, 0, 3>>
  #   table entry A: <<65, 1>>
  #   table entry B: <<66, 2>>
  #   table entry C: <<67, 2>>
  #   bit_stream: pack "000101011" LSB-first
  #     byte 0: bit0=0,bit1=0,bit2=0,bit3=1,bit4=0,bit5=1,bit6=0,bit7=1 → 0b10101000 = 168
  #     byte 1: bit0=1 (remaining), padded → 0b00000001 = 1
  #   So bit_stream bytes: <<168, 1>>

  test "wire format: original_length for AAABBC is 6" do
    compressed = HuffmanCompression.compress("AAABBC")
    <<orig_len::32-big, _rest::binary>> = compressed
    assert orig_len == 6
  end

  test "wire format: symbol_count for AAABBC is 3" do
    compressed = HuffmanCompression.compress("AAABBC")
    <<_orig_len::32-big, sym_count::32-big, _rest::binary>> = compressed
    assert sym_count == 3
  end

  test "wire format: code-lengths table for AAABBC is sorted by (len, sym)" do
    compressed = HuffmanCompression.compress("AAABBC")
    <<_orig_len::32-big, _sym_count::32-big, table_bytes::binary-size(6), _bits::binary>> =
      compressed

    # Table: [A(65)→1, B(66)→2, C(67)→2]
    <<s0::8, l0::8, s1::8, l1::8, s2::8, l2::8>> = table_bytes
    assert {s0, l0} == {65, 1}
    assert {s1, l1} == {66, 2}
    assert {s2, l2} == {67, 2}
  end

  test "wire format: AAABBC round-trips correctly" do
    assert rt("AAABBC") == "AAABBC"
  end

  test "wire format: total compressed size for AAABBC" do
    compressed = HuffmanCompression.compress("AAABBC")
    # Header (8) + table (3*2=6) + bit_stream (2 bytes for 9 bits) = 16
    assert byte_size(compressed) == 16
  end

  # ─── Edge cases ──────────────────────────────────────────────────────────────

  test "compress empty returns 8-byte header" do
    compressed = HuffmanCompression.compress("")
    assert byte_size(compressed) == 8
    <<orig_len::32-big, sym_count::32-big>> = compressed
    assert orig_len == 0
    assert sym_count == 0
  end

  test "decompress empty-compressed data returns empty string" do
    assert HuffmanCompression.decompress(HuffmanCompression.compress("")) == ""
  end

  test "decompress with fewer than 8 bytes returns error" do
    assert HuffmanCompression.decompress(<<1, 2>>) == {:error, :too_short}
    assert HuffmanCompression.decompress(<<0, 0, 0, 1, 0, 0, 0>>) == {:error, :too_short}
  end

  test "compress single byte: all frequencies equal, one symbol" do
    compressed = HuffmanCompression.compress("Z")
    <<orig_len::32-big, sym_count::32-big, _rest::binary>> = compressed
    assert orig_len == 1
    assert sym_count == 1
    assert rt("Z") == "Z"
  end

  test "all 256 distinct bytes round-trip" do
    data = :binary.list_to_bin(Enum.to_list(0..255))
    result = rt(data)
    assert result == data
  end

  test "compress all 256 bytes: symbol_count is 256" do
    data = :binary.list_to_bin(Enum.to_list(0..255))
    compressed = HuffmanCompression.compress(data)
    <<_orig_len::32-big, sym_count::32-big, _rest::binary>> = compressed
    assert sym_count == 256
  end

  # ─── Compression effectiveness ───────────────────────────────────────────────

  test "highly repetitive data compresses smaller than original" do
    data = String.duplicate("ABC", 1000)
    compressed = HuffmanCompression.compress(data)
    assert byte_size(compressed) < byte_size(data)
  end

  test "single repeated byte compresses smaller" do
    data = String.duplicate(<<0x42>>, 10000)
    compressed = HuffmanCompression.compress(data)
    assert byte_size(compressed) < byte_size(data)
  end

  # ─── Bit packing helpers ─────────────────────────────────────────────────────

  test "pack_bits_lsb_first packs empty string to empty binary" do
    assert HuffmanCompression.pack_bits_lsb_first("") == <<>>
  end

  test "pack_bits_lsb_first packs 8 bits into 1 byte" do
    # "10000000" → bit0=1, bit1=0, ..., bit7=0 → byte = 0b00000001 = 1
    assert HuffmanCompression.pack_bits_lsb_first("10000000") == <<1>>
  end

  test "pack_bits_lsb_first packs 9 bits into 2 bytes with padding" do
    # "100000001" → byte0 = 0b00000001 = 1, byte1 = 0b00000001 = 1
    # bit0=1 of first byte, bits 1-7 = 0; second byte bit0=1
    assert byte_size(HuffmanCompression.pack_bits_lsb_first("100000001")) == 2
  end

  test "unpack_bits_lsb_first unpacks single byte" do
    # byte = 1 (0b00000001) → bits: "10000000" (bit0=1, bit1..7=0)
    result = HuffmanCompression.unpack_bits_lsb_first(<<1>>)
    assert result == "10000000"
  end

  test "pack and unpack are inverse" do
    # "000101011" — 9 bits
    bits_in = "000101011"
    packed = HuffmanCompression.pack_bits_lsb_first(bits_in)
    # Unpack gives 16 bits (2 bytes), first 9 should match
    unpacked = HuffmanCompression.unpack_bits_lsb_first(packed)
    assert String.slice(unpacked, 0, String.length(bits_in)) == bits_in
  end

  # ─── Security / robustness ──────────────────────────────────────────────────

  test "decompress with too-short input returns error tuple" do
    assert HuffmanCompression.decompress(<<1, 2>>) == {:error, :too_short}
  end

  test "decompress random bytes does not crash" do
    random = :crypto.strong_rand_bytes(100)
    result = HuffmanCompression.decompress(random)
    # Either returns a binary or an error tuple
    assert is_binary(result) or match?({:error, _}, result)
  end

  test "decompress crafted all-zeros payload does not crash" do
    payload = <<0, 0, 0, 0, 0, 0, 0, 0>> <> :binary.copy(<<0>>, 20)
    result = HuffmanCompression.decompress(payload)
    assert is_binary(result)
  end

  test "decompress truncated compressed data does not crash" do
    compressed = HuffmanCompression.compress("hello world hello world")
    truncated = binary_part(compressed, 0, div(byte_size(compressed), 2))

    case HuffmanCompression.decompress(truncated) do
      result when is_binary(result) -> assert true
      {:error, _} -> assert true
    end
  end

  # ─── Spec vectors ────────────────────────────────────────────────────────────

  test "spec vector: rt empty" do
    assert rt("") == ""
  end

  test "spec vector: rt single byte A" do
    assert rt("A") == "A"
  end

  test "spec vector: rt AAABBC" do
    assert rt("AAABBC") == "AAABBC"
  end

  test "spec vector: rt hello hello hello" do
    assert rt("hello hello hello") == "hello hello hello"
  end
end
