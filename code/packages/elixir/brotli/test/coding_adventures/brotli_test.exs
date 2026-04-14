defmodule CodingAdventures.BrotliTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Brotli

  # ---------------------------------------------------------------------------
  # Helper
  # ---------------------------------------------------------------------------

  defp roundtrip(data) when is_binary(data) do
    compressed = Brotli.compress(data)
    result = Brotli.decompress(compressed)
    assert result == data,
      "roundtrip mismatch: expected #{inspect(data)}, got #{inspect(result)}"
    compressed
  end

  # ---------------------------------------------------------------------------
  # Spec test 1: Round-trip empty binary
  # ---------------------------------------------------------------------------

  test "round-trip: empty input" do
    compressed = Brotli.compress("")
    result = Brotli.decompress(compressed)
    assert result == "" or result == <<>>
  end

  test "empty input wire format is minimal" do
    compressed = Brotli.compress("")
    # Header (10 bytes) + ICC entry (63, 1) (2 bytes) + bit stream 0x00 (1 byte) = 13 bytes
    assert byte_size(compressed) == 13
    <<orig_len::32, icc_count::8, dist_count::8,
      c0::8, c1::8, c2::8, c3::8, _rest::binary>> = compressed
    assert orig_len == 0
    assert icc_count == 1
    assert dist_count == 0
    assert c0 == 0 and c1 == 0 and c2 == 0 and c3 == 0
  end

  # ---------------------------------------------------------------------------
  # Spec test 2: Round-trip single byte
  # ---------------------------------------------------------------------------

  test "round-trip: single byte 0x42" do
    roundtrip(<<0x42>>)
  end

  test "round-trip: single byte 0x00" do
    roundtrip(<<0>>)
  end

  test "round-trip: single byte 0xFF" do
    roundtrip(<<0xFF>>)
  end

  test "round-trip: single byte 'A'" do
    roundtrip("A")
  end

  # ---------------------------------------------------------------------------
  # Spec test 3: Round-trip all 256 distinct bytes
  # ---------------------------------------------------------------------------

  test "round-trip: all 256 distinct bytes (incompressible)" do
    data = :binary.list_to_bin(Enum.to_list(0..255))
    compressed = roundtrip(data)
    # All 256 distinct bytes: no repeated substrings of length >= 4, so no matches.
    # Compressed size will be LARGER than input (Huffman + header overhead).
    assert byte_size(compressed) > byte_size(data),
      "expected incompressible data to grow, got #{byte_size(compressed)} < #{byte_size(data)}"
  end

  # ---------------------------------------------------------------------------
  # Spec test 4: Round-trip 1024 × 'A' (heavy repetition)
  # ---------------------------------------------------------------------------

  test "round-trip: String.duplicate('A', 1024)" do
    data = String.duplicate("A", 1024)
    compressed = roundtrip(data)
    # Should compress significantly: 1024 bytes → much smaller.
    assert byte_size(compressed) < byte_size(data) * 0.5,
      "expected significant compression for repetitive data"
  end

  # ---------------------------------------------------------------------------
  # Spec test 5: Round-trip English prose >= 1024 bytes
  # ---------------------------------------------------------------------------

  @english_prose """
  The quick brown fox jumps over the lazy dog.
  Pack my box with five dozen liquor jugs.
  How vexingly quick daft zebras jump!
  The five boxing wizards jump quickly.
  Sphinx of black quartz, judge my vow.
  Two driven jocks help fax my big quiz.
  Five quacking zephyrs jolt my wax bed.
  The jay, pig, fox, zebra and my wolves quack!
  Blowzy red vixens fight for a quick jump.
  Joaquin Phoenix was gazed by MTV for luck.
  The quick brown fox jumps over the lazy dog.
  Pack my box with five dozen liquor jugs.
  How vexingly quick daft zebras jump!
  The five boxing wizards jump quickly.
  Sphinx of black quartz, judge my vow.
  Two driven jocks help fax my big quiz.
  Five quacking zephyrs jolt my wax bed.
  The jay, pig, fox, zebra and my wolves quack!
  Blowzy red vixens fight for a quick jump.
  Joaquin Phoenix was gazed by MTV for luck.
  The quick brown fox jumps over the lazy dog.
  Pack my box with five dozen liquor jugs.
  How vexingly quick daft zebras jump!
  The five boxing wizards jump quickly.
  """ |> String.duplicate(3)

  test "round-trip: English prose >= 1024 bytes" do
    data = @english_prose
    assert byte_size(data) >= 1024, "test prose must be at least 1024 bytes"
    compressed = roundtrip(data)
    ratio = byte_size(compressed) / byte_size(data)
    assert ratio < 0.80,
      "expected compressed size < 80% of input, got #{Float.round(ratio * 100, 1)}%"
  end

  # ---------------------------------------------------------------------------
  # Spec test 6: Round-trip deterministic binary blob
  # ---------------------------------------------------------------------------

  test "round-trip: deterministic binary blob (512 bytes)" do
    # Deterministic pseudo-random pattern — not truly random, but incompressible-ish.
    data = :binary.list_to_bin(for i <- 0..511, do: rem(i * 17 + 31, 256))
    roundtrip(data)
  end

  # ---------------------------------------------------------------------------
  # Spec test 7: Round-trip "abc123ABC" — cross-context literals
  # ---------------------------------------------------------------------------

  test "round-trip: abc123ABC (cross-context literal buckets)" do
    data = "abc123ABC"
    roundtrip(data)
  end

  test "context assignment verification for abc123ABC" do
    # Verify the context function:
    #   'a'      → ctx 0 (start of stream, no prev byte)
    #   'b','c'  → ctx 3 (after lowercase)
    #   '1'      → ctx 3 (after lowercase 'c')
    #   '2','3'  → ctx 1 (after digit)
    #   'A'      → ctx 1 (after digit '3')
    #   'B','C'  → ctx 2 (after uppercase)
    data = "abc123ABC"
    roundtrip(data)

    # Compressed output must have non-empty context tables for the contexts
    # that are actually used. We verify via roundtrip above and check structure.
    compressed = Brotli.compress(data)
    <<_orig_len::32, _icc::8, _dist::8,
      ctx0_count::8, ctx1_count::8, ctx2_count::8, ctx3_count::8,
      _rest::binary>> = compressed

    # With "abc123ABC": ctx 0 gets 'a', ctx 3 gets 'b','c', etc.
    # All four contexts should have at least one entry.
    assert ctx0_count > 0, "ctx0 should have at least one literal"
    assert ctx1_count > 0, "ctx1 should have at least one literal (digits after digit)"
    assert ctx2_count > 0, "ctx2 should have at least one literal (uppercase)"
    assert ctx3_count > 0, "ctx3 should have at least one literal (lowercase)"
  end

  # ---------------------------------------------------------------------------
  # Spec test 8: Long-distance match (offset > 4096)
  # ---------------------------------------------------------------------------

  test "round-trip: long-distance match (offset > 4096)" do
    # Build input where a 10-byte pattern repeats with distance > 4096.
    # We use 5000 filler bytes between two occurrences of the same pattern.
    pattern = "XYZPATTERN"
    filler = :binary.list_to_bin(for i <- 0..4999, do: rem(i, 97) + 32)
    data = pattern <> filler <> pattern
    compressed = roundtrip(data)
    # The second occurrence of `pattern` should be matched at distance > 4096.
    # This exercises distance codes 24-31 (base >= 4097).
    _ = compressed
  end

  # ---------------------------------------------------------------------------
  # Additional correctness tests
  # ---------------------------------------------------------------------------

  test "round-trip: single byte repeated many times" do
    roundtrip(String.duplicate("A", 100))
    roundtrip(:binary.copy(<<0>>, 200))
    roundtrip(:binary.copy(<<0xFF>>, 200))
  end

  test "round-trip: short strings" do
    roundtrip("hello")
    roundtrip("world")
    roundtrip("hello world")
    roundtrip("AAABBC")
    roundtrip("AABCBBABC")
  end

  test "round-trip: strings with repeated substrings" do
    roundtrip("hello hello hello")
    roundtrip("ABCABCABCABC")
    roundtrip(String.duplicate("the quick brown fox ", 20))
  end

  test "round-trip: overlapping matches (run-length encoding)" do
    roundtrip("AAAAAAA")
    roundtrip("ABABABABABAB")
  end

  test "round-trip: binary data 1000 bytes" do
    data = :binary.list_to_bin(for i <- 0..999, do: rem(i, 256))
    roundtrip(data)
  end

  test "round-trip: all lowercase letters" do
    roundtrip("abcdefghijklmnopqrstuvwxyz")
  end

  test "round-trip: all uppercase letters" do
    roundtrip("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
  end

  test "round-trip: all digits" do
    roundtrip("0123456789")
  end

  test "round-trip: mixed content" do
    roundtrip("Hello, World! 123 ABC abc")
  end

  # ---------------------------------------------------------------------------
  # Header structure tests
  # ---------------------------------------------------------------------------

  test "header contains correct original_length" do
    data = "Hello, World!"
    compressed = Brotli.compress(data)
    <<orig_len::32, _rest::binary>> = compressed
    assert orig_len == byte_size(data)
  end

  test "dist_entry_count is 0 for all-literal input (no matches)" do
    # Short random-ish input with no repeated 4-byte substrings.
    data = "ABCDEFGHIJKLMNOP"
    compressed = Brotli.compress(data)
    <<_orig_len::32, _icc::8, dist_count::8, _rest::binary>> = compressed
    assert dist_count == 0, "no matches expected, dist_entry_count should be 0"
  end

  test "dist_entry_count is > 0 for input with matches" do
    data = String.duplicate("ABCD", 100)
    compressed = Brotli.compress(data)
    <<_orig_len::32, _icc::8, dist_count::8, _rest::binary>> = compressed
    assert dist_count > 0, "matches expected, dist_entry_count should be > 0"
  end

  # ---------------------------------------------------------------------------
  # Compression ratio tests
  # ---------------------------------------------------------------------------

  test "highly repetitive data compresses to < 10%" do
    data = String.duplicate("A", 10_000)
    compressed = Brotli.compress(data)
    ratio = byte_size(compressed) / byte_size(data)
    assert ratio < 0.10,
      "expected < 10% for 10000 × 'A', got #{Float.round(ratio * 100, 1)}%"
  end

  test "repeated phrase compresses to < 50%" do
    data = String.duplicate("ABCABC", 500)
    compressed = Brotli.compress(data)
    ratio = byte_size(compressed) / byte_size(data)
    assert ratio < 0.50,
      "expected < 50% for repeated phrase, got #{Float.round(ratio * 100, 1)}%"
  end

  # ---------------------------------------------------------------------------
  # Decompress input shorter than header returns ""
  # ---------------------------------------------------------------------------

  test "decompress with too-short input returns empty string" do
    assert Brotli.decompress("") == ""
    assert Brotli.decompress(<<0, 1, 2>>) == ""
  end

  # ---------------------------------------------------------------------------
  # Edge: single-character input for each context bucket
  # ---------------------------------------------------------------------------

  test "round-trip: single space (context 0)" do
    roundtrip(" ")
  end

  test "round-trip: single digit (context 1 boundary)" do
    # The digit itself goes into ctx 0 (no prev byte → bucket 0).
    roundtrip("5")
  end

  test "round-trip: two digits (second goes into ctx 1)" do
    roundtrip("55")
  end

  test "round-trip: uppercase then uppercase (ctx 2)" do
    roundtrip("AB")
  end

  test "round-trip: lowercase then lowercase (ctx 3)" do
    roundtrip("ab")
  end
end
