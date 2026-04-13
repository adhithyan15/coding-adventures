defmodule CodingAdventures.DeflateTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Deflate

  defp roundtrip(data) when is_binary(data) do
    compressed = Deflate.compress(data)
    result = Deflate.decompress(compressed)
    assert result == data, "roundtrip mismatch: expected #{inspect(data)}, got #{inspect(result)}"
  end

  # ---------------------------------------------------------------------------
  # Edge cases
  # ---------------------------------------------------------------------------

  test "empty input" do
    compressed = Deflate.compress("")
    result = Deflate.decompress(compressed)
    assert result == "" or result == <<>>
  end

  test "single byte 0x00" do
    roundtrip(<<0>>)
  end

  test "single byte 0xFF" do
    roundtrip(<<0xFF>>)
  end

  test "single byte 'A'" do
    roundtrip("A")
  end

  test "single byte repeated" do
    roundtrip(String.duplicate("A", 20))
    roundtrip(:binary.copy(<<0>>, 100))
  end

  # ---------------------------------------------------------------------------
  # Spec examples
  # ---------------------------------------------------------------------------

  test "AAABBC — all literals, no matches" do
    data = "AAABBC"
    roundtrip(data)
    compressed = Deflate.compress(data)
    <<_orig_len::32, _ll_count::16, dist_count::16, _rest::binary>> = compressed
    assert dist_count == 0, "expected dist_entry_count=0 for all-literals input"
  end

  test "AABCBBABC — one LZSS match" do
    data = "AABCBBABC"
    roundtrip(data)
    compressed = Deflate.compress(data)
    <<orig_len::32, _ll_count::16, dist_count::16, _rest::binary>> = compressed
    assert orig_len == 9
    assert dist_count > 0, "expected dist_entry_count>0 for input with a match"
  end

  # ---------------------------------------------------------------------------
  # Match tests
  # ---------------------------------------------------------------------------

  test "overlapping match (run encoding)" do
    roundtrip("AAAAAAA")
    roundtrip("ABABABABABAB")
  end

  test "multiple matches" do
    roundtrip("ABCABCABCABC")
    roundtrip("hello hello hello world")
  end

  # ---------------------------------------------------------------------------
  # Data variety
  # ---------------------------------------------------------------------------

  test "all 256 byte values" do
    data = :binary.list_to_bin(Enum.to_list(0..255))
    roundtrip(data)
  end

  test "binary data 1000 bytes" do
    data = :binary.list_to_bin(for i <- 0..999, do: rem(i, 256))
    roundtrip(data)
  end

  test "longer text with repetition" do
    base = "the quick brown fox jumps over the lazy dog "
    data = String.duplicate(base, 10)
    roundtrip(data)
  end

  # ---------------------------------------------------------------------------
  # Compression ratio
  # ---------------------------------------------------------------------------

  test "highly repetitive data compresses to < 50%" do
    data = String.duplicate("ABCABC", 100)
    compressed = Deflate.compress(data)
    assert byte_size(compressed) < byte_size(data) / 2,
           "expected significant compression: #{byte_size(compressed)} >= #{byte_size(data) / 2}"
  end

  # ---------------------------------------------------------------------------
  # Diverse round-trips
  # ---------------------------------------------------------------------------

  test "diverse inputs" do
    inputs = [
      :binary.copy(<<0>>, 100),
      :binary.copy(<<0xFF>>, 100),
      "abcdefghijklmnopqrstuvwxyz",
      String.duplicate("The quick brown fox ", 20),
    ]
    for data <- inputs, do: roundtrip(data)
  end

  test "max match length ~255" do
    data = String.duplicate("A", 300)
    roundtrip(data)
  end
end
