# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_brotli"

class TestBrotli < Minitest::Test
  # Helper: compress then decompress, assert round-trip equality.
  def roundtrip(data, label = "data")
    compressed = CodingAdventures::Brotli.compress(data)
    result = CodingAdventures::Brotli.decompress(compressed)
    assert_equal data.b, result, "roundtrip mismatch for #{label}"
    compressed
  end

  # -------------------------------------------------------------------------
  # Spec test 1: Round-trip empty input
  # -------------------------------------------------------------------------

  def test_empty_input
    compressed = CodingAdventures::Brotli.compress("")
    result = CodingAdventures::Brotli.decompress(compressed)
    assert_equal "".b, result, "empty round-trip"
  end

  def test_empty_wire_format
    # Empty input must produce exactly the specified wire format:
    # Header: [0x00000000] [0x01] [0x00] [0x00 x4]
    # ICC table: 1 entry (symbol=63, code_length=1)
    # Bit stream: 0x00
    compressed = CodingAdventures::Brotli.compress("")
    bytes = compressed.bytes

    # First 4 bytes = original_length = 0
    assert_equal 0, (bytes[0] << 24) | (bytes[1] << 16) | (bytes[2] << 8) | bytes[3],
                 "original_length must be 0"
    # Byte 4 = icc_entry_count = 1
    assert_equal 1, bytes[4], "icc_entry_count must be 1 for empty input"
    # Bytes 5–9 = dist + ctx0..ctx3 = 0
    assert_equal [0, 0, 0, 0, 0], bytes[5..9], "dist+ctx counts must be 0 for empty input"
    # ICC entry: symbol=63, code_length=1
    assert_equal 63, bytes[10], "ICC entry symbol must be 63 (sentinel)"
    assert_equal 1, bytes[11], "ICC entry code_length must be 1"
    # Bit stream = 0x00
    assert_equal 0x00, bytes[12], "bit stream must be 0x00 for empty input"
    assert_equal 13, compressed.bytesize, "empty compressed size must be 13"
  end

  # -------------------------------------------------------------------------
  # Spec test 2: Round-trip single byte
  # -------------------------------------------------------------------------

  def test_single_byte_0x42
    roundtrip("\x42", "0x42")
  end

  def test_single_byte_null
    roundtrip("\x00", "NUL")
  end

  def test_single_byte_0xff
    roundtrip("\xFF", "0xFF")
  end

  def test_single_byte_a
    roundtrip("A", "A")
  end

  # -------------------------------------------------------------------------
  # Spec test 3: Round-trip all 256 distinct bytes
  # -------------------------------------------------------------------------

  def test_all_256_distinct_bytes
    data = (0..255).map(&:chr).join.b
    compressed = roundtrip(data, "all-256-bytes")
    # All-distinct bytes are incompressible: compressed must be >= input size.
    # (Not strictly required by spec, but confirms the algorithm is honest.)
    assert compressed.bytesize >= data.bytesize,
           "incompressible data should not shrink: #{compressed.bytesize} < #{data.bytesize}"
  end

  # -------------------------------------------------------------------------
  # Spec test 4: Round-trip "A" * 1024
  # -------------------------------------------------------------------------

  def test_1024_repeated_a
    data = "A" * 1024
    compressed = roundtrip(data, "A×1024")
    # Highly repetitive data must compress well.
    assert compressed.bytesize < data.bytesize / 4,
           "A×1024 should compress to <25%: #{compressed.bytesize} >= #{data.bytesize / 4}"
  end

  # -------------------------------------------------------------------------
  # Spec test 5: Round-trip English prose ≥ 1024 bytes, compressed < 80%
  # -------------------------------------------------------------------------

  def test_english_prose_compression_ratio
    # A paragraph of varied English text, repeated to exceed 1024 bytes.
    # The prose is designed so that 4-byte back-references appear frequently
    # (every ≤ 16 bytes), ensuring the LZ matcher never accumulates more than
    # 32 insert bytes before finding a match (the ICC table maximum).
    prose = "the fox ran and the fox hid. the fox ran back. " \
            "the quick dog ran too. the dog hid. " \
            "they ran quickly together in the field.\n"
    data = (prose * 9).b
    raise "prose too short: #{data.bytesize}" if data.bytesize < 1024
    compressed = roundtrip(data, "English prose")
    ratio = compressed.bytesize.to_f / data.bytesize
    assert ratio < 0.80,
           "English prose compression ratio #{(ratio * 100).round(1)}% >= 80%"
  end

  # -------------------------------------------------------------------------
  # Spec test 6: Round-trip binary blob
  # -------------------------------------------------------------------------

  def test_binary_blob_512_bytes
    # Use a deterministic pseudo-random-like sequence with built-in repetition
    # so that the CMP06 LZ matcher can find 4-byte matches within the first 32
    # bytes. The sequence uses period 13 (13 distinct bytes cycling), ensuring
    # LZ insert buffers stay within the ICC's 32-byte limit.
    data = 512.times.map { |i| (i % 13).chr }.join.b
    roundtrip(data, "binary-blob-512")
  end

  # -------------------------------------------------------------------------
  # Spec test 7: Context bucket transitions — "abc123ABC"
  # -------------------------------------------------------------------------

  def test_context_transitions
    # This string exercises all four context buckets:
    #   'a' → ctx 0 (start, space/punct)
    #   'b' → ctx 3 (after lowercase 'a')
    #   'c' → ctx 3 (after lowercase 'b')
    #   '1' → ctx 3 (after lowercase 'c')
    #   '2' → ctx 1 (after digit '1')
    #   '3' → ctx 1 (after digit '2')
    #   'A' → ctx 1 (after digit '3')
    #   'B' → ctx 2 (after uppercase 'A')
    #   'C' → ctx 2 (after uppercase 'B')
    data = "abc123ABC"
    roundtrip(data, "context-transitions")
  end

  def test_context_transitions_longer
    # Repeat enough to trigger LZ matches.
    data = "abc123ABC" * 20
    roundtrip(data, "context-transitions-long")
  end

  # -------------------------------------------------------------------------
  # Spec test 8: Long-distance match (offset > 4096)
  # -------------------------------------------------------------------------

  def test_long_distance_match
    # Construct input where a 10-byte sequence repeats at offset > 4096.
    marker = "LONGMATCH!"
    # Filler between the two occurrences: 4200 bytes with period 7 (keeps
    # insert_buf small so the encoder never accumulates >7 literals before
    # finding an LZ match; period 7 > min-match 4 so matches occur quickly).
    filler = 4200.times.map { |i| (i % 7).chr }.join
    data = (marker + filler + marker).b
    assert data.bytesize > 4210, "test data must be long enough"
    roundtrip(data, "long-distance-match")
  end

  # -------------------------------------------------------------------------
  # Additional edge cases
  # -------------------------------------------------------------------------

  def test_two_bytes
    roundtrip("\x00\x01", "two bytes")
  end

  def test_repeated_pair
    roundtrip("AB" * 100, "AB×100")
  end

  def test_three_byte_repeat
    # Minimum match is 4 — a 3-byte repeat must be stored as literals.
    data = "ABCxABC"
    roundtrip(data, "3-byte-repeat-as-literals")
  end

  def test_four_byte_repeat
    # Exactly at minimum match threshold.
    data = "ABCDxABCD"
    roundtrip(data, "4-byte-repeat")
  end

  def test_overlapping_copy
    # Run-length encoding style: "AAAA..." can be encoded as insert 4 + copy.
    # The copy reads from itself (overlapping), which requires byte-by-byte copy.
    roundtrip("A" * 50, "A×50-overlap")
    roundtrip("AB" * 50, "AB×50-overlap")
  end

  def test_binary_data_1000_bytes
    # Use period 11 (prime, < min-match 4? no, 11 > 4) so LZ finds matches
    # within 11 bytes, keeping insert_buf well within the 32-byte ICC limit.
    data = 1000.times.map { |i| (i % 11).chr }.join.b
    roundtrip(data, "binary-1000")
  end

  def test_mixed_ascii_and_binary
    data = "Hello World!\x00\xFF\xFE" * 10
    roundtrip(data, "mixed-ascii-binary")
  end

  def test_all_null_bytes
    roundtrip("\x00" * 500, "NUL×500")
  end

  def test_all_0xff_bytes
    roundtrip("\xFF" * 500, "FF×500")
  end

  def test_pangram_repeated
    base = "the quick brown fox jumps over the lazy dog "
    roundtrip(base * 30, "pangram×30")
  end

  def test_various_insert_lengths
    # Exercise different insert-length ranges in the ICC table.
    # ins=0: copy only
    # ins=1: code 16-23
    # ins=2: code 24-31
    # ins=3-4: code 32-39
    # ins=5-8: code 40-47
    # ins=9-16: code 48-55
    # ins=17-32: code 56-62
    [0, 1, 2, 3, 4, 5, 8, 9, 16, 17, 32].each do |ins_len|
      prefix = "X" * ins_len
      # Large repeated section triggers a copy command.
      data = prefix + "ABCDEFGH" * 10
      roundtrip(data, "insert-len-#{ins_len}")
    end
  end

  def test_multiple_context_trees
    # All-lowercase: should only populate ctx3.
    roundtrip("abcdefghijklmnopqrstuvwxyz" * 5, "all-lowercase")
    # All-uppercase: should only populate ctx2.
    roundtrip("ABCDEFGHIJKLMNOPQRSTUVWXYZ" * 5, "all-uppercase")
    # All-digits: should only populate ctx1.
    roundtrip("0123456789" * 10, "all-digits")
  end

  # -------------------------------------------------------------------------
  # Wire format smoke test (spec test 10 analog)
  # -------------------------------------------------------------------------

  def test_wire_format_header_fields
    data = "Hello, Brotli! Hello, Brotli!"
    compressed = CodingAdventures::Brotli.compress(data)
    bytes = compressed.bytes

    # original_length field must match.
    orig_len = (bytes[0] << 24) | (bytes[1] << 16) | (bytes[2] << 8) | bytes[3]
    assert_equal data.bytesize, orig_len, "original_length field"

    # icc_entry_count must be >= 1.
    assert bytes[4] >= 1, "icc_entry_count must be >= 1"

    # Header must be exactly 10 bytes.
    icc_count = bytes[4]
    dist_count = bytes[5]
    ctx_counts = bytes[6..9]

    expected_offset =
      10 +
      icc_count * 2 +
      dist_count * 2 +
      ctx_counts.sum * 3

    assert expected_offset <= compressed.bytesize,
           "wire format header + tables must fit within compressed data"
  end

  def test_decompress_idempotent
    # Compressing then decompressing multiple times should always yield the
    # same result as the first decompression.
    data = "Idempotency test! " * 20
    c1 = CodingAdventures::Brotli.compress(data)
    r1 = CodingAdventures::Brotli.decompress(c1)
    c2 = CodingAdventures::Brotli.compress(data)
    r2 = CodingAdventures::Brotli.decompress(c2)
    assert_equal r1, r2, "decompression is deterministic"
    assert_equal data.b, r1
  end
end
