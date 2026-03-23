# frozen_string_literal: true

# test_wasm_leb128.rb — Minitest test suite for CodingAdventures::WasmLeb128
#
# Each test case maps to a specific aspect of the LEB128 specification.
# Test vectors are derived from the WebAssembly Core Specification and verified
# by hand calculation.

require "minitest/autorun"
require "coding_adventures_wasm_leb128"

# Convenience alias for the module under test
LEB128 = CodingAdventures::WasmLeb128

class TestWasmLeb128 < Minitest::Test
  # ─────────────────────────────────────────────────────────────────────────
  # Metadata
  # ─────────────────────────────────────────────────────────────────────────

  def test_version_exists
    refute_nil LEB128::VERSION
  end

  def test_leb128_error_is_standard_error_subclass
    err = LEB128::LEB128Error.new("test")
    assert_instance_of LEB128::LEB128Error, err
    assert_kind_of StandardError, err
    assert_equal "test", err.message
  end

  # ─────────────────────────────────────────────────────────────────────────
  # Test case 1: Zero
  # ─────────────────────────────────────────────────────────────────────────

  # [0x00] → unsigned=0, signed=0, bytes_consumed=1
  # 0x00 has no continuation bit, payload = 0.
  def test_decode_unsigned_zero
    value, bytes = LEB128.decode_unsigned("\x00".b)
    assert_equal 0, value
    assert_equal 1, bytes
  end

  def test_decode_signed_zero
    value, bytes = LEB128.decode_signed("\x00".b)
    assert_equal 0, value
    assert_equal 1, bytes
  end

  def test_encode_unsigned_zero
    assert_equal "\x00".b, LEB128.encode_unsigned(0)
  end

  def test_encode_signed_zero
    assert_equal "\x00".b, LEB128.encode_signed(0)
  end

  # ─────────────────────────────────────────────────────────────────────────
  # Test case 2: One-byte unsigned
  # ─────────────────────────────────────────────────────────────────────────

  # [0x03] → 3, bytes_consumed=1
  def test_decode_unsigned_one_byte
    value, bytes = LEB128.decode_unsigned("\x03".b)
    assert_equal 3, value
    assert_equal 1, bytes
  end

  def test_encode_unsigned_three
    assert_equal "\x03".b, LEB128.encode_unsigned(3)
  end

  # ─────────────────────────────────────────────────────────────────────────
  # Test case 3: One-byte signed negative
  # ─────────────────────────────────────────────────────────────────────────

  # [0x7E] → -2, bytes_consumed=1
  #
  #   0x7E = 0b0111_1110
  #   continuation bit = 0 → last byte
  #   payload = 0x7E & 0x7F = 126 = 0b111_1110
  #   bit 6 = 1 → sign extend: 126 - 128 = -2  ✓
  def test_decode_signed_negative_one_byte
    value, bytes = LEB128.decode_signed("\x7E".b)
    assert_equal(-2, value)
    assert_equal 1, bytes
  end

  def test_encode_signed_negative_two
    assert_equal "\x7E".b, LEB128.encode_signed(-2)
  end

  # ─────────────────────────────────────────────────────────────────────────
  # Test case 4: Multi-byte unsigned (624485)
  # ─────────────────────────────────────────────────────────────────────────

  # 624485 → [0xE5, 0x8E, 0x26], bytes_consumed=3
  #
  #   624485 in binary: 0b0001_0011_0000_0111_0110_0101
  #   7-bit groups (LE):
  #     bits 0–6:   110_0101 = 0x65 = 101  → byte: 0xE5 (+ continuation)
  #     bits 7–13:  000_1110 = 0x0E = 14   → byte: 0x8E (+ continuation)
  #     bits 14–19: 010_0110 = 0x26 = 38   → byte: 0x26 (last)
  #
  #   Decode check: 101 | (14 << 7) | (38 << 14) = 101 + 1792 + 622592 = 624485 ✓
  def test_decode_unsigned_multi_byte
    data = "\xE5\x8E\x26".b
    value, bytes = LEB128.decode_unsigned(data)
    assert_equal 624485, value
    assert_equal 3, bytes
  end

  def test_encode_unsigned_624485
    assert_equal "\xE5\x8E\x26".b, LEB128.encode_unsigned(624485)
  end

  # ─────────────────────────────────────────────────────────────────────────
  # Test case 5: Max u32 (4294967295 = 0xFFFFFFFF)
  # ─────────────────────────────────────────────────────────────────────────

  # [0xFF, 0xFF, 0xFF, 0xFF, 0x0F] → 4294967295, bytes_consumed=5
  #
  #   byte 0: 0xFF → payload=0x7F=127, more follows
  #   byte 1: 0xFF → payload=0x7F=127, more follows
  #   byte 2: 0xFF → payload=0x7F=127, more follows
  #   byte 3: 0xFF → payload=0x7F=127, more follows
  #   byte 4: 0x0F → payload=0x0F=15,  last byte
  #
  #   result = 127 | (127<<7) | (127<<14) | (127<<21) | (15<<28)
  #          = 0x7F | 0x3F80 | 0x1FC000 | 0xFE00000 | 0xF0000000
  #          = 0xFFFFFFFF = 4294967295  ✓
  def test_decode_unsigned_max_u32
    data = "\xFF\xFF\xFF\xFF\x0F".b
    value, bytes = LEB128.decode_unsigned(data)
    assert_equal 4294967295, value
    assert_equal 5, bytes
  end

  def test_encode_unsigned_max_u32
    assert_equal "\xFF\xFF\xFF\xFF\x0F".b, LEB128.encode_unsigned(4294967295)
  end

  # ─────────────────────────────────────────────────────────────────────────
  # Test case 6: Max i32 (2147483647 = 0x7FFFFFFF)
  # ─────────────────────────────────────────────────────────────────────────

  # [0xFF, 0xFF, 0xFF, 0xFF, 0x07] → 2147483647, bytes_consumed=5
  def test_decode_signed_max_i32
    data = "\xFF\xFF\xFF\xFF\x07".b
    value, bytes = LEB128.decode_signed(data)
    assert_equal 2147483647, value
    assert_equal 5, bytes
  end

  def test_encode_signed_max_i32
    assert_equal "\xFF\xFF\xFF\xFF\x07".b, LEB128.encode_signed(2147483647)
  end

  # ─────────────────────────────────────────────────────────────────────────
  # Test case 7: Min i32 (-2147483648 = 0x80000000 as u32)
  # ─────────────────────────────────────────────────────────────────────────

  # [0x80, 0x80, 0x80, 0x80, 0x78] → -2147483648, bytes_consumed=5
  #
  #   byte 4: 0x78 = 0b0111_1000
  #   payload of byte 4 = 0x78 & 0x7F = 0x78 = 120 = 0b111_1000
  #   result after accumulation = 120 << 28 (as unsigned Bignum in Ruby)
  #   = 120 * 268435456 = 32212254720 (bigger than 32 bits!)
  #   After truncation to 32 bits: 32212254720 & 0xFFFFFFFF = 2147483648
  #   Sign bit set → 2147483648 - 2^32 = -2147483648  ✓
  def test_decode_signed_min_i32
    data = "\x80\x80\x80\x80\x78".b
    value, bytes = LEB128.decode_signed(data)
    assert_equal(-2147483648, value)
    assert_equal 5, bytes
  end

  def test_encode_signed_min_i32
    assert_equal "\x80\x80\x80\x80\x78".b, LEB128.encode_signed(-2147483648)
  end

  # ─────────────────────────────────────────────────────────────────────────
  # Test case 8: Round-trip encode → decode (multiple unsigned values)
  # ─────────────────────────────────────────────────────────────────────────

  UNSIGNED_VALUES = [
    0, 1, 63, 64, 127, 128, 255, 256, 16383, 16384,
    624485, 1_000_000, 0x7FFFFFFF, 0xFFFFFFFF
  ].freeze

  def test_unsigned_round_trips
    UNSIGNED_VALUES.each do |v|
      encoded = LEB128.encode_unsigned(v)
      decoded, bytes_consumed = LEB128.decode_unsigned(encoded)
      assert_equal v, decoded, "round-trip failed for unsigned #{v}"
      assert_equal encoded.bytesize, bytes_consumed, "bytes_consumed mismatch for unsigned #{v}"
    end
  end

  # ─────────────────────────────────────────────────────────────────────────
  # Test case 11: Round-trip for signed values (including negatives)
  # ─────────────────────────────────────────────────────────────────────────

  SIGNED_VALUES = [
    0, 1, -1, 63, -64, 64, -65, 127, -128, 128, -129,
    2147483647, -2147483648, -1_000_000, -2
  ].freeze

  def test_signed_round_trips
    SIGNED_VALUES.each do |v|
      encoded = LEB128.encode_signed(v)
      decoded, bytes_consumed = LEB128.decode_signed(encoded)
      assert_equal v, decoded, "round-trip failed for signed #{v}"
      assert_equal encoded.bytesize, bytes_consumed, "bytes_consumed mismatch for signed #{v}"
    end
  end

  # ─────────────────────────────────────────────────────────────────────────
  # Test case 9: Unterminated sequences raise LEB128Error
  # ─────────────────────────────────────────────────────────────────────────

  # [0x80, 0x80] — both bytes have continuation bit set, no terminator
  def test_decode_unsigned_unterminated_raises
    assert_raises(LEB128::LEB128Error) do
      LEB128.decode_unsigned("\x80\x80".b)
    end
  end

  def test_decode_signed_unterminated_raises
    assert_raises(LEB128::LEB128Error) do
      LEB128.decode_signed("\x80\x80".b)
    end
  end

  # Empty input should also raise
  def test_decode_unsigned_empty_raises
    assert_raises(LEB128::LEB128Error) do
      LEB128.decode_unsigned("".b)
    end
  end

  def test_decode_signed_empty_raises
    assert_raises(LEB128::LEB128Error) do
      LEB128.decode_signed("".b)
    end
  end

  # Sequence too long (more than 5 bytes with continuation bits all set)
  def test_decode_unsigned_too_many_bytes_raises
    assert_raises(LEB128::LEB128Error) do
      LEB128.decode_unsigned("\x80\x80\x80\x80\x80\x01".b)
    end
  end

  # ─────────────────────────────────────────────────────────────────────────
  # Test case 10: Non-zero offset works correctly
  # ─────────────────────────────────────────────────────────────────────────

  def test_decode_unsigned_at_nonzero_offset
    # Byte layout: [0xAA, 0xE5, 0x8E, 0x26, 0xBB]
    #               skip  ←── 624485 ───→    skip
    data = "\xAA\xE5\x8E\x26\xBB".b
    value, bytes = LEB128.decode_unsigned(data, 1)
    assert_equal 624485, value
    assert_equal 3, bytes
  end

  def test_decode_signed_at_nonzero_offset
    # Byte layout: [0xFF, 0x7E, 0x00]
    #               skip  ← -2 →
    data = "\xFF\x7E\x00".b
    value, bytes = LEB128.decode_signed(data, 1)
    assert_equal(-2, value)
    assert_equal 1, bytes
  end

  # ─────────────────────────────────────────────────────────────────────────
  # Array input (alternate API)
  # ─────────────────────────────────────────────────────────────────────────

  def test_decode_unsigned_accepts_array
    value, bytes = LEB128.decode_unsigned([0xE5, 0x8E, 0x26])
    assert_equal 624485, value
    assert_equal 3, bytes
  end

  def test_decode_signed_accepts_array
    value, bytes = LEB128.decode_signed([0x7E])
    assert_equal(-2, value)
    assert_equal 1, bytes
  end

  # ─────────────────────────────────────────────────────────────────────────
  # Additional signed encoding tests
  # ─────────────────────────────────────────────────────────────────────────

  def test_encode_signed_positive_one
    # 1 encodes as [0x01] (single byte, no sign issues)
    assert_equal "\x01".b, LEB128.encode_signed(1)
  end

  def test_encode_signed_negative_one
    # -1 encodes as [0x7F]
    # -1 & 0x7F = 0x7F; -1 >> 7 = -1; bit 6 of 0x7F = 1 → done
    assert_equal "\x7F".b, LEB128.encode_signed(-1)
  end

  def test_decode_signed_negative_one
    value, bytes = LEB128.decode_signed("\x7F".b)
    assert_equal(-1, value)
    assert_equal 1, bytes
  end
end
