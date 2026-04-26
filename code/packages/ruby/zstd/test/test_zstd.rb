# frozen_string_literal: true

# test_zstd.rb — CMP07 ZStd package tests (TC-1 through TC-9 + unit tests).
#
# Each test class covers a distinct scenario. The TC-N classes follow the
# test plan from the spec; the unit test classes verify internal helpers.

require "minitest/autorun"
require "coding_adventures_zstd"

# ── Helper ────────────────────────────────────────────────────────────────────

# Round-trip helper: compress then decompress, asserting no error.
def rt(data)
  compressed = CodingAdventures::Zstd.compress(data.b)
  CodingAdventures::Zstd.decompress(compressed)
end

# ── TC-1: Empty round-trip ────────────────────────────────────────────────────
#
# An empty input must produce a valid ZStd frame (magic + header + one empty
# raw block) and decompress back to empty bytes without panic or error.

class TestTC1Empty < Minitest::Test
  def test_empty_round_trip
    assert_equal "".b, rt("")
  end

  def test_empty_produces_valid_frame
    compressed = CodingAdventures::Zstd.compress("".b)
    # Minimum frame: 4 (magic) + 1 (FHD) + 8 (FCS) + 3 (block header) = 16
    assert compressed.bytesize >= 16, "empty frame must be at least 16 bytes"
  end

  def test_empty_frame_starts_with_magic
    compressed = CodingAdventures::Zstd.compress("".b)
    assert_equal CodingAdventures::Zstd::MAGIC, compressed.byteslice(0, 4).unpack1("V")
  end
end

# ── TC-2: Single byte round-trip ──────────────────────────────────────────────
#
# The smallest non-empty input: one byte. Exercises the raw-block path since
# a single byte cannot be LZ-compressed.

class TestTC2SingleByte < Minitest::Test
  def test_single_byte_ascii
    assert_equal "\x42".b, rt("\x42")
  end

  def test_single_byte_zero
    assert_equal "\x00".b, rt("\x00")
  end

  def test_single_byte_ff
    assert_equal "\xFF".b, rt("\xFF")
  end
end

# ── TC-3: All 256 byte values ─────────────────────────────────────────────────
#
# Every byte value 0x00..0xFF in ascending order. Exercises literal encoding
# of non-ASCII bytes and the zero byte. Since all 256 bytes appear exactly
# once, LZ77 finds no matches — should produce a raw block.

class TestTC3AllBytes < Minitest::Test
  def test_all_256_byte_values
    input = (0..255).map(&:chr).join.b
    assert_equal input, rt(input)
  end

  def test_all_bytes_in_reverse_order
    input = (255).downto(0).map(&:chr).join.b
    assert_equal input, rt(input)
  end
end

# ── TC-4: RLE block ───────────────────────────────────────────────────────────
#
# 1024 identical bytes should be detected and encoded as an RLE block.
# The resulting compressed size must be < 30 bytes:
#   4 (magic) + 1 (FHD) + 8 (FCS) + 3 (block header) + 1 (RLE byte) = 17.

class TestTC4RLE < Minitest::Test
  def test_rle_round_trip
    input = ("A" * 1024).b
    assert_equal input, rt(input)
  end

  def test_rle_compressed_size_under_30
    input = ("A" * 1024).b
    compressed = CodingAdventures::Zstd.compress(input)
    assert_operator compressed.bytesize, :<, 30,
      "RLE of 1024 bytes: got #{compressed.bytesize} bytes, expected < 30"
  end

  def test_rle_with_zero_byte
    input = ("\x00" * 512).b
    assert_equal input, rt(input)
  end

  def test_rle_with_byte_255
    input = ("\xFF" * 256).b
    compressed = CodingAdventures::Zstd.compress(input)
    assert_equal input, CodingAdventures::Zstd.decompress(compressed)
    assert_operator compressed.bytesize, :<, 30
  end
end

# ── TC-5: English prose ───────────────────────────────────────────────────────
#
# Repeated English text has strong LZ77 matches. Must round-trip and achieve
# at least 20% compression (compressed size < 80% of input size).

class TestTC5EnglishProse < Minitest::Test
  def test_prose_round_trip
    input = ("the quick brown fox jumps over the lazy dog " * 25).b
    assert_equal input, rt(input)
  end

  def test_prose_compression_ratio
    input = ("the quick brown fox jumps over the lazy dog " * 25).b
    compressed = CodingAdventures::Zstd.compress(input)
    threshold = input.bytesize * 80 / 100
    assert_operator compressed.bytesize, :<, threshold,
      "prose: #{compressed.bytesize} bytes (input #{input.bytesize}), expected < #{threshold} (80%)"
  end

  def test_longer_prose_round_trip
    input = ("Lorem ipsum dolor sit amet, consectetur adipiscing elit. " * 50).b
    assert_equal input, rt(input)
  end
end

# ── TC-6: LCG pseudo-random data round-trip ───────────────────────────────────
#
# LCG pseudo-random bytes have minimal LZ matches, exercising the raw-block
# fallback. Round-trip must be exact regardless of which block type is chosen.

class TestTC6Random < Minitest::Test
  def test_lcg_random_round_trip
    seed = 42
    input = 512.times.map do
      seed = (seed * 1664525 + 1013904223) & 0xFFFFFFFF
      seed & 0xFF
    end.pack("C*")
    assert_equal input, rt(input)
  end

  def test_lcg_different_seed
    seed = 0xDEADBEEF
    input = 1024.times.map do
      seed = (seed * 1664525 + 1013904223) & 0xFFFFFFFF
      seed & 0xFF
    end.pack("C*")
    assert_equal input, rt(input)
  end
end

# ── TC-7: 200 KB single-byte run (multi-block) ────────────────────────────────
#
# 200 KB > MAX_BLOCK_SIZE (128 KB), so this forces at least 2 blocks.
# All bytes are identical, so both blocks should be RLE blocks.
# Total compressed size should be tiny.

class TestTC7LargeSingleByteRun < Minitest::Test
  def test_200kb_single_byte_round_trip
    input = ("x" * (200 * 1024)).b
    assert_equal input, rt(input)
  end

  def test_200kb_rle_compresses_tiny
    input = ("x" * (200 * 1024)).b
    compressed = CodingAdventures::Zstd.compress(input)
    # Two RLE blocks: 2 × (3-byte header + 1 RLE byte) + 13-byte frame header = 21 + 13 = 34
    assert_operator compressed.bytesize, :<, 50,
      "200 KB single byte: compressed to #{compressed.bytesize} bytes, expected < 50"
  end

  def test_300kb_repetitive_text_multi_block
    # Exercises multi-block with compressed (not just RLE) blocks
    input = ("ABCDEFGHIJKLMNOP" * 20000).b  # ~320 KB
    assert_equal input, rt(input)
  end
end

# ── TC-8: 300 KB repetitive text (multi-block compression) ───────────────────
#
# Repetitive text longer than MAX_BLOCK_SIZE, exercising the compressed-block
# path across multiple blocks.

class TestTC8RepetitiveText < Minitest::Test
  def test_300kb_repetitive_round_trip
    input = ("the quick brown fox " * 16000).b  # ~320 KB
    assert_equal input, rt(input)
  end

  def test_repetitive_achieves_compression
    input = ("the quick brown fox " * 16000).b
    compressed = CodingAdventures::Zstd.compress(input)
    # Expect at least 50% compression on highly repetitive text
    assert_operator compressed.bytesize, :<, input.bytesize / 2,
      "repetitive text: #{compressed.bytesize} vs #{input.bytesize}, expected < 50%"
  end
end

# ── TC-9: Bad magic raises exception ─────────────────────────────────────────
#
# Feeding non-ZStd data to decompress must raise RuntimeError immediately.

class TestTC9BadMagic < Minitest::Test
  def test_random_bytes_raise
    assert_raises(RuntimeError) do
      CodingAdventures::Zstd.decompress("not zstd data at all!!".b)
    end
  end

  def test_gzip_magic_raises
    # gzip starts with 0x1F 0x8B — must be rejected
    assert_raises(RuntimeError) do
      CodingAdventures::Zstd.decompress("\x1F\x8B\x08\x00\x00\x00\x00\x00\x00\x03".b)
    end
  end

  def test_truncated_frame_raises
    assert_raises(RuntimeError) do
      CodingAdventures::Zstd.decompress("\x28\xB5\x2F".b)  # only 3 bytes
    end
  end
end

# ── Unit: RevBitWriter / RevBitReader ─────────────────────────────────────────

class TestRevBitRoundTrip < Minitest::Test
  def test_simple_roundtrip
    # Write order: A=0b101 (3 bits), B=0b11001100 (8 bits), C=0b1 (1 bit)
    # Read order (reversed): C, B, A
    bw = CodingAdventures::Zstd::RevBitWriter.new
    bw.add_bits(0b101, 3)       # A — written first → read last
    bw.add_bits(0b11001100, 8)  # B
    bw.add_bits(0b1, 1)         # C — written last → read first
    bw.flush
    buf = bw.finish

    br = CodingAdventures::Zstd::RevBitReader.new(buf)
    assert_equal 0b1, br.read_bits(1)         # C
    assert_equal 0b11001100, br.read_bits(8)  # B
    assert_equal 0b101, br.read_bits(3)       # A
  end

  def test_zero_bits_noop
    bw = CodingAdventures::Zstd::RevBitWriter.new
    bw.add_bits(0xFF, 0)  # should be a no-op
    bw.add_bits(0b110, 3)
    bw.flush
    buf = bw.finish

    br = CodingAdventures::Zstd::RevBitReader.new(buf)
    assert_equal 0b110, br.read_bits(3)
  end

  def test_large_bit_values
    bw = CodingAdventures::Zstd::RevBitWriter.new
    bw.add_bits(0xABCD, 16)
    bw.add_bits(0xFF, 8)
    bw.flush
    buf = bw.finish

    br = CodingAdventures::Zstd::RevBitReader.new(buf)
    assert_equal 0xFF, br.read_bits(8)
    assert_equal 0xABCD, br.read_bits(16)
  end

  def test_single_bit_sentinel
    # Writing 0 bits, just a flush — should produce a single sentinel byte 0x01
    bw = CodingAdventures::Zstd::RevBitWriter.new
    bw.flush
    buf = bw.finish
    assert_equal "\x01".b, buf
  end
end

# ── Unit: FSE decode table ─────────────────────────────────────────────────────

class TestFSEDecodeTable < Minitest::Test
  def test_ll_table_size
    dt = CodingAdventures::Zstd.build_decode_table(
      CodingAdventures::Zstd::LL_NORM,
      CodingAdventures::Zstd::LL_ACC_LOG
    )
    assert_equal 1 << CodingAdventures::Zstd::LL_ACC_LOG, dt.size
  end

  def test_ml_table_size
    dt = CodingAdventures::Zstd.build_decode_table(
      CodingAdventures::Zstd::ML_NORM,
      CodingAdventures::Zstd::ML_ACC_LOG
    )
    assert_equal 1 << CodingAdventures::Zstd::ML_ACC_LOG, dt.size
  end

  def test_of_table_size
    dt = CodingAdventures::Zstd.build_decode_table(
      CodingAdventures::Zstd::OF_NORM,
      CodingAdventures::Zstd::OF_ACC_LOG
    )
    assert_equal 1 << CodingAdventures::Zstd::OF_ACC_LOG, dt.size
  end

  def test_all_symbols_valid_in_ll_table
    dt = CodingAdventures::Zstd.build_decode_table(
      CodingAdventures::Zstd::LL_NORM,
      CodingAdventures::Zstd::LL_ACC_LOG
    )
    dt.each do |cell|
      assert_operator cell[:sym], :<, CodingAdventures::Zstd::LL_NORM.size,
        "LL table symbol #{cell[:sym]} out of range"
    end
  end
end

# ── Unit: LL / ML code mapping ────────────────────────────────────────────────

class TestCodeMapping < Minitest::Test
  def test_ll_codes_0_to_15_are_identity
    0.upto(15) do |i|
      assert_equal i, CodingAdventures::Zstd.ll_to_code(i),
        "ll_to_code(#{i}) should be #{i}"
    end
  end

  def test_ml_codes_3_to_34_are_sequential
    3.upto(34) do |i|
      expected = i - 3
      assert_equal expected, CodingAdventures::Zstd.ml_to_code(i),
        "ml_to_code(#{i}) should be #{expected}"
    end
  end

  def test_ll_code_16_covers_16_to_17
    assert_equal 16, CodingAdventures::Zstd.ll_to_code(16)
    assert_equal 16, CodingAdventures::Zstd.ll_to_code(17)
  end

  def test_ll_code_17_covers_18_to_19
    assert_equal 17, CodingAdventures::Zstd.ll_to_code(18)
    assert_equal 17, CodingAdventures::Zstd.ll_to_code(19)
  end
end

# ── Unit: Literals section encode/decode ─────────────────────────────────────

class TestLiteralsSection < Minitest::Test
  def test_short_roundtrip
    # Short (≤ 31 bytes): 1-byte header
    lits = (0...20).to_a
    encoded = CodingAdventures::Zstd.encode_literals_section(lits).b
    decoded, consumed = CodingAdventures::Zstd.decode_literals_section(encoded)
    assert_equal lits, decoded
    assert_equal encoded.bytesize, consumed
  end

  def test_medium_roundtrip
    # Medium (32..4095 bytes): 2-byte header
    lits = (0...200).map { |i| i % 256 }
    encoded = CodingAdventures::Zstd.encode_literals_section(lits).b
    decoded, consumed = CodingAdventures::Zstd.decode_literals_section(encoded)
    assert_equal lits, decoded
    assert_equal encoded.bytesize, consumed
  end

  def test_large_roundtrip
    # Large (≥ 4096 bytes): 3-byte header
    lits = (0...5000).map { |i| i % 256 }
    encoded = CodingAdventures::Zstd.encode_literals_section(lits).b
    decoded, consumed = CodingAdventures::Zstd.decode_literals_section(encoded)
    assert_equal lits, decoded
    assert_equal encoded.bytesize, consumed
  end

  def test_empty_literals
    lits = []
    encoded = CodingAdventures::Zstd.encode_literals_section(lits).b
    decoded, consumed = CodingAdventures::Zstd.decode_literals_section(encoded)
    assert_equal lits, decoded
    assert_equal 1, consumed  # 1-byte header for size 0
  end
end

# ── Unit: Sequence count encode/decode ───────────────────────────────────────

class TestSeqCount < Minitest::Test
  # ── Endianness regression: low-byte-< 128 values ───────────────────────────
  #
  # The 2-byte form must place the format-flag byte (with bit 7 set) FIRST.
  # An earlier broken pattern in TS+Go wrote `[count & 0xFF, (count >> 8) | 0x80]`
  # — low byte first. For any count ≥ 128 whose low byte happens to be < 128
  # (e.g. 515 = 0x0203 → byte0 = 0x03), the decoder mis-takes the 1-byte path
  # and silently returns a tiny garbage count. The values below all have
  # low byte < 128, so they'd round-trip wrong if Ruby ever regressed.
  REGRESSION_VALUES = [
    128,       # 0x0080 — boundary, low byte = 0x80 (high bit borderline)
    256,       # 0x0100 — low byte 0x00, would decode as 1-byte 0
    300,       # 0x012C — low byte 0x2C, would decode as 1-byte 44
    515,       # 0x0203 — Lua's TC-8 case, low byte 0x03 = decode as 3
    768,       # 0x0300 — low byte 0x00
    1024,      # 0x0400 — low byte 0x00
    32258      # 0x7E02 — near upper end of 2-byte range, low 0x02
  ].freeze

  def test_roundtrip_values
    [0, 1, 50, 127, 128, 1000, 0x7FFE].each do |n|
      encoded = CodingAdventures::Zstd.encode_seq_count(n).b
      decoded, = CodingAdventures::Zstd.decode_seq_count(encoded)
      assert_equal n, decoded, "seq count #{n}"
    end
  end

  def test_low_byte_lt_128_regression
    REGRESSION_VALUES.each do |n|
      encoded = CodingAdventures::Zstd.encode_seq_count(n).b
      decoded, consumed = CodingAdventures::Zstd.decode_seq_count(encoded)
      assert_equal n, decoded, "seq count #{n} (low byte = 0x#{(n & 0xFF).to_s(16)})"
      assert_equal 2, consumed, "seq count #{n} should consume 2 bytes"
      # Wire format check: byte0 must have bit 7 set so the decoder picks
      # the 2-byte branch unambiguously.
      assert_operator encoded.getbyte(0), :>=, 128,
        "byte0 for count=#{n} must have bit 7 set; got 0x#{encoded.getbyte(0).to_s(16)}"
    end
  end

  def test_zero_is_one_byte
    assert_equal 1, CodingAdventures::Zstd.encode_seq_count(0).bytesize
  end

  def test_127_is_one_byte
    assert_equal 1, CodingAdventures::Zstd.encode_seq_count(127).bytesize
  end

  def test_128_is_two_bytes
    assert_equal 2, CodingAdventures::Zstd.encode_seq_count(128).bytesize
  end
end

# ── Integration: deterministic output ────────────────────────────────────────

class TestDeterministic < Minitest::Test
  def test_same_input_produces_same_output
    data = ("hello zstd world! " * 50).b
    assert_equal CodingAdventures::Zstd.compress(data),
      CodingAdventures::Zstd.compress(data),
      "compress must be deterministic"
  end
end

# ── Integration: various data patterns ───────────────────────────────────────

class TestVariousPatterns < Minitest::Test
  def test_all_zeros
    assert_equal ("\x00" * 1000).b, rt("\x00" * 1000)
  end

  def test_all_ff
    assert_equal ("\xFF" * 1000).b, rt("\xFF" * 1000)
  end

  def test_hello_world
    assert_equal "hello world".b, rt("hello world")
  end

  def test_repeated_pattern
    data = (b"ABCDEF" * 500)
    assert_equal data, rt(data)
  end

  def test_binary_data_with_zeros_and_ff
    input = (0...300).map { |i| (i % 256) }.pack("C*")
    assert_equal input, rt(input)
  end

  def test_two_byte_pattern_repeated
    input = ("AB" * 2000).b
    assert_equal input, rt(input)
  end

  private

  def b(str)
    str.b
  end
end

# ── Integration: hand-crafted wire format ────────────────────────────────────
#
# Manually constructed minimal ZStd frame, verifying the decoder reads the
# wire format correctly without depending on our encoder.
#
# Frame layout:
#   [0..3]  Magic = 0xFD2FB528 LE = [0x28, 0xB5, 0x2F, 0xFD]
#   [4]     FHD = 0x20:
#             bits [7:6] = 00 → FCS flag 0
#             bit  [5]   = 1  → Single_Segment = 1
#             bits [4:0] = 0  → no checksum, no dict
#           With Single_Segment=1 and FCS_flag=00, FCS is 1 byte.
#   [5]     FCS = 0x05 (content size = 5)
#   [6..8]  Block header: Last=1, Type=Raw, Size=5
#             = (5 << 3) | (0 << 1) | 1 = 41 = 0x29
#             = [0x29, 0x00, 0x00]
#   [9..13] b"hello"

class TestWireFormat < Minitest::Test
  def test_minimal_raw_block_frame
    frame = [
      0x28, 0xB5, 0x2F, 0xFD,  # magic
      0x20,                     # FHD: Single_Segment=1, FCS=1byte
      0x05,                     # FCS = 5
      0x29, 0x00, 0x00,         # block header: last=1, raw, size=5
      "h".ord, "e".ord, "l".ord, "l".ord, "o".ord
    ].pack("C*")
    assert_equal "hello".b, CodingAdventures::Zstd.decompress(frame)
  end

  def test_rle_block_decodes_correctly
    # Minimal frame with one RLE block: 10 × byte 0x41 ('A')
    # Block header: last=1, type=RLE(01), size=10
    #   = (10 << 3) | (0b01 << 1) | 1 = 80 | 2 | 1 = 83 = 0x53
    frame = [
      0x28, 0xB5, 0x2F, 0xFD,  # magic
      0x20,                     # FHD: Single_Segment=1, FCS=1byte
      0x0A,                     # FCS = 10
      0x53, 0x00, 0x00,         # block header: last=1, rle, size=10
      0x41                      # RLE byte = 'A'
    ].pack("C*")
    assert_equal ("A" * 10).b, CodingAdventures::Zstd.decompress(frame)
  end
end
