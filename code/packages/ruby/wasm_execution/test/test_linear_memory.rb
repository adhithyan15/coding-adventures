# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_wasm_execution"

# ==========================================================================
# Tests for LinearMemory
# ==========================================================================
#
# LinearMemory models WASM's flat, byte-addressable heap. It is measured
# in 64 KiB pages and uses little-endian byte order throughout.
#
# These tests cover:
#   1. Initialization (page count, byte length, zero-filled)
#   2. Full-width loads and stores (i32, i64, f32, f64)
#   3. Narrow loads (sign-extend and zero-extend variants)
#   4. Narrow stores (8-bit, 16-bit, 32-bit truncations)
#   5. Memory growth (grow, max_pages enforcement)
#   6. Out-of-bounds access detection
#   7. Raw byte writes (write_bytes)
# ==========================================================================

class TestLinearMemory < Minitest::Test
  LM = CodingAdventures::WasmExecution::LinearMemory
  TrapError = CodingAdventures::WasmExecution::TrapError

  # ── Initialization ─────────────────────────────────────────────────

  def test_initial_page_count
    mem = LM.new(2)
    assert_equal 2, mem.page_count
  end

  def test_initial_byte_length
    mem = LM.new(1)
    assert_equal 65536, mem.byte_length
  end

  def test_initial_memory_is_zero_filled
    mem = LM.new(1)
    assert_equal 0, mem.load_i32(0)
    assert_equal 0, mem.load_i32(100)
  end

  def test_zero_pages_creates_empty_memory
    mem = LM.new(0)
    assert_equal 0, mem.page_count
    assert_equal 0, mem.byte_length
  end

  # ── Full-Width i32 Load/Store ──────────────────────────────────────

  def test_store_and_load_i32_positive
    mem = LM.new(1)
    mem.store_i32(0, 42)
    assert_equal 42, mem.load_i32(0)
  end

  def test_store_and_load_i32_negative
    mem = LM.new(1)
    mem.store_i32(0, -1)
    assert_equal(-1, mem.load_i32(0))
  end

  def test_store_and_load_i32_max
    mem = LM.new(1)
    mem.store_i32(0, 2_147_483_647)
    assert_equal 2_147_483_647, mem.load_i32(0)
  end

  def test_store_and_load_i32_min
    mem = LM.new(1)
    mem.store_i32(0, -2_147_483_648)
    assert_equal(-2_147_483_648, mem.load_i32(0))
  end

  def test_i32_at_nonzero_offset
    mem = LM.new(1)
    mem.store_i32(100, 99)
    assert_equal 99, mem.load_i32(100)
    assert_equal 0, mem.load_i32(0) # other locations unaffected
  end

  # ── Full-Width i64 Load/Store ──────────────────────────────────────

  def test_store_and_load_i64
    mem = LM.new(1)
    mem.store_i64(0, 1_000_000_000_000)
    assert_equal 1_000_000_000_000, mem.load_i64(0)
  end

  def test_store_and_load_i64_negative
    mem = LM.new(1)
    mem.store_i64(0, -1)
    assert_equal(-1, mem.load_i64(0))
  end

  # ── Full-Width f32 Load/Store ──────────────────────────────────────

  def test_store_and_load_f32
    mem = LM.new(1)
    mem.store_f32(0, 3.14)
    assert_in_delta 3.14, mem.load_f32(0), 0.001
  end

  def test_store_and_load_f32_negative
    mem = LM.new(1)
    mem.store_f32(0, -2.5)
    assert_in_delta(-2.5, mem.load_f32(0), 1e-6)
  end

  # ── Full-Width f64 Load/Store ──────────────────────────────────────

  def test_store_and_load_f64
    mem = LM.new(1)
    mem.store_f64(0, 3.141592653589793)
    assert_in_delta 3.141592653589793, mem.load_f64(0), 1e-15
  end

  def test_store_and_load_f64_negative
    mem = LM.new(1)
    mem.store_f64(0, -1.23456789)
    assert_in_delta(-1.23456789, mem.load_f64(0), 1e-9)
  end

  # ── Narrow Loads for i32 ───────────────────────────────────────────

  def test_load_i32_8s_sign_extends
    mem = LM.new(1)
    mem.store_i32_8(0, -128) # 0x80
    assert_equal(-128, mem.load_i32_8s(0))
  end

  def test_load_i32_8u_zero_extends
    mem = LM.new(1)
    mem.store_i32_8(0, -128) # stored as 0x80
    assert_equal 128, mem.load_i32_8u(0)
  end

  def test_load_i32_8s_positive
    mem = LM.new(1)
    mem.store_i32_8(0, 127)
    assert_equal 127, mem.load_i32_8s(0)
  end

  def test_load_i32_16s_sign_extends
    mem = LM.new(1)
    mem.store_i32_16(0, -1000)
    assert_equal(-1000, mem.load_i32_16s(0))
  end

  def test_load_i32_16u_zero_extends
    mem = LM.new(1)
    mem.store_i32_16(0, -1) # stored as 0xFFFF
    assert_equal 65535, mem.load_i32_16u(0)
  end

  # ── Narrow Loads for i64 ───────────────────────────────────────────

  def test_load_i64_8s_sign_extends
    mem = LM.new(1)
    mem.store_i64_8(0, 0xFF) # -1 as signed byte
    assert_equal(-1, mem.load_i64_8s(0))
  end

  def test_load_i64_8u_zero_extends
    mem = LM.new(1)
    mem.store_i64_8(0, 0xFF)
    assert_equal 255, mem.load_i64_8u(0)
  end

  def test_load_i64_16s_sign_extends
    mem = LM.new(1)
    mem.store_i64_16(0, 0xFFFF)
    assert_equal(-1, mem.load_i64_16s(0))
  end

  def test_load_i64_16u_zero_extends
    mem = LM.new(1)
    mem.store_i64_16(0, 0xFFFF)
    assert_equal 65535, mem.load_i64_16u(0)
  end

  def test_load_i64_32s_sign_extends
    mem = LM.new(1)
    mem.store_i64_32(0, 0xFFFFFFFF)
    assert_equal(-1, mem.load_i64_32s(0))
  end

  def test_load_i64_32u_zero_extends
    mem = LM.new(1)
    mem.store_i64_32(0, 0xFFFFFFFF)
    assert_equal 4_294_967_295, mem.load_i64_32u(0)
  end

  # ── Narrow Stores ──────────────────────────────────────────────────

  def test_store_i32_8_truncates
    mem = LM.new(1)
    mem.store_i32_8(0, 42)
    assert_equal 42, mem.load_i32_8u(0)
  end

  def test_store_i32_16_truncates
    mem = LM.new(1)
    mem.store_i32_16(0, 1000)
    assert_equal 1000, mem.load_i32_16u(0)
  end

  def test_store_i64_8_truncates
    mem = LM.new(1)
    mem.store_i64_8(0, 0x1FF) # only low 8 bits stored
    assert_equal 0xFF, mem.load_i64_8u(0)
  end

  def test_store_i64_16_truncates
    mem = LM.new(1)
    mem.store_i64_16(0, 0x1FFFF) # only low 16 bits stored
    assert_equal 0xFFFF, mem.load_i64_16u(0)
  end

  def test_store_i64_32_truncates
    mem = LM.new(1)
    mem.store_i64_32(0, 0x1FFFFFFFF) # only low 32 bits stored
    assert_equal 0xFFFFFFFF, mem.load_i64_32u(0)
  end

  # ── Memory Growth ──────────────────────────────────────────────────

  def test_grow_returns_old_page_count
    mem = LM.new(1)
    old = mem.grow(2)
    assert_equal 1, old
    assert_equal 3, mem.page_count
  end

  def test_grow_increases_byte_length
    mem = LM.new(1)
    mem.grow(1)
    assert_equal 131072, mem.byte_length
  end

  def test_grow_zero_pages_succeeds
    mem = LM.new(1)
    old = mem.grow(0)
    assert_equal 1, old
    assert_equal 1, mem.page_count
  end

  def test_grow_beyond_max_returns_negative_one
    mem = LM.new(1, 2) # max 2 pages
    result = mem.grow(2) # would be 3, exceeds max
    assert_equal(-1, result)
    assert_equal 1, mem.page_count # unchanged
  end

  def test_grow_to_max_succeeds
    mem = LM.new(1, 3)
    result = mem.grow(2)
    assert_equal 1, result
    assert_equal 3, mem.page_count
  end

  def test_grow_beyond_absolute_max_returns_negative_one
    mem = LM.new(1)
    result = mem.grow(65536) # would be 65537, exceeds 65536
    assert_equal(-1, result)
  end

  def test_new_pages_are_zero_filled
    mem = LM.new(1)
    mem.grow(1)
    # Check a byte in the newly grown page
    assert_equal 0, mem.load_i32(65536)
  end

  # ── Out-of-Bounds Access ───────────────────────────────────────────

  def test_load_i32_oob_raises
    mem = LM.new(1)
    assert_raises(TrapError) { mem.load_i32(65536) }
  end

  def test_load_i32_partial_oob_raises
    mem = LM.new(1)
    # Offset 65534 + 4 bytes = 65538, past end of 65536
    assert_raises(TrapError) { mem.load_i32(65534) }
  end

  def test_store_i32_oob_raises
    mem = LM.new(1)
    assert_raises(TrapError) { mem.store_i32(65536, 0) }
  end

  def test_load_i64_oob_raises
    mem = LM.new(1)
    assert_raises(TrapError) { mem.load_i64(65536) }
  end

  def test_load_f32_oob_raises
    mem = LM.new(1)
    assert_raises(TrapError) { mem.load_f32(65536) }
  end

  def test_load_f64_oob_raises
    mem = LM.new(1)
    assert_raises(TrapError) { mem.load_f64(65536) }
  end

  def test_negative_offset_raises
    mem = LM.new(1)
    assert_raises(TrapError) { mem.load_i32(-1) }
  end

  def test_oob_on_zero_page_memory
    mem = LM.new(0)
    assert_raises(TrapError) { mem.load_i32(0) }
  end

  def test_load_i32_8s_oob_raises
    mem = LM.new(1)
    assert_raises(TrapError) { mem.load_i32_8s(65536) }
  end

  def test_load_i32_16s_oob_raises
    mem = LM.new(1)
    assert_raises(TrapError) { mem.load_i32_16s(65535) }
  end

  # ── Raw Byte Access ────────────────────────────────────────────────

  def test_write_bytes_from_string
    mem = LM.new(1)
    mem.write_bytes(0, "\x01\x02\x03\x04".b)
    assert_equal 1, mem.load_i32_8u(0)
    assert_equal 2, mem.load_i32_8u(1)
    assert_equal 3, mem.load_i32_8u(2)
    assert_equal 4, mem.load_i32_8u(3)
  end

  def test_write_bytes_from_array
    mem = LM.new(1)
    mem.write_bytes(10, [0xAB, 0xCD])
    assert_equal 0xAB, mem.load_i32_8u(10)
    assert_equal 0xCD, mem.load_i32_8u(11)
  end

  def test_write_bytes_oob_raises
    mem = LM.new(1)
    assert_raises(TrapError) { mem.write_bytes(65535, "\x01\x02".b) }
  end

  # ── Edge case: last valid offset ───────────────────────────────────

  def test_load_i32_at_last_valid_offset
    mem = LM.new(1)
    mem.store_i32(65532, 42) # last 4 bytes of page
    assert_equal 42, mem.load_i32(65532)
  end

  def test_load_i64_at_last_valid_offset
    mem = LM.new(1)
    mem.store_i64(65528, 99) # last 8 bytes of page
    assert_equal 99, mem.load_i64(65528)
  end
end
