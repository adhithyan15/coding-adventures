# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_wasm_execution"

# ==========================================================================
# Tests for WasmExecution Values
# ==========================================================================
#
# WASM values are typed numeric values. Each value carries a type tag and
# a raw payload. These tests verify:
#
#   1. Constructors produce correctly typed values
#   2. Wrapping semantics for i32 and i64
#   3. f32 round-trips through 32-bit IEEE 754
#   4. default_value returns zero-initialized values
#   5. Type extraction helpers (as_i32, as_i64, etc.) enforce types
#   6. Unsigned interpretation helpers (to_u32, to_u64)
# ==========================================================================

class TestValues < Minitest::Test
  WE = CodingAdventures::WasmExecution
  VT = CodingAdventures::WasmTypes::VALUE_TYPE

  # ── i32 Constructor ────────────────────────────────────────────────

  def test_i32_positive_value
    v = WE.i32(42)
    assert_equal VT[:i32], v.type
    assert_equal 42, v.value
  end

  def test_i32_zero
    v = WE.i32(0)
    assert_equal 0, v.value
  end

  def test_i32_negative_value
    v = WE.i32(-1)
    assert_equal(-1, v.value)
  end

  def test_i32_max_positive
    v = WE.i32(2_147_483_647)
    assert_equal 2_147_483_647, v.value
  end

  def test_i32_min_negative
    v = WE.i32(-2_147_483_648)
    assert_equal(-2_147_483_648, v.value)
  end

  def test_i32_wraps_unsigned_max_to_negative_one
    # 0xFFFFFFFF (2^32 - 1) wraps to -1 in signed i32
    v = WE.i32(0xFFFFFFFF)
    assert_equal(-1, v.value)
  end

  def test_i32_wraps_at_2_pow_32
    # 2^32 wraps to 0
    v = WE.i32(0x100000000)
    assert_equal 0, v.value
  end

  def test_i32_wraps_large_positive
    # 2^31 wraps to -2^31
    v = WE.i32(0x80000000)
    assert_equal(-2_147_483_648, v.value)
  end

  def test_i32_wraps_large_negative
    # Values below -2^31 wrap around
    v = WE.i32(-2_147_483_649)
    assert_equal 2_147_483_647, v.value
  end

  # ── i64 Constructor ────────────────────────────────────────────────

  def test_i64_positive_value
    v = WE.i64(42)
    assert_equal VT[:i64], v.type
    assert_equal 42, v.value
  end

  def test_i64_zero
    v = WE.i64(0)
    assert_equal 0, v.value
  end

  def test_i64_negative_value
    v = WE.i64(-1)
    assert_equal(-1, v.value)
  end

  def test_i64_wraps_unsigned_max
    v = WE.i64(0xFFFFFFFFFFFFFFFF)
    assert_equal(-1, v.value)
  end

  def test_i64_wraps_at_2_pow_64
    v = WE.i64(0x10000000000000000)
    assert_equal 0, v.value
  end

  # ── f32 Constructor ────────────────────────────────────────────────

  def test_f32_basic_value
    v = WE.f32(3.14)
    assert_equal VT[:f32], v.type
    # f32 rounds to single precision
    expected = [3.14].pack("e").unpack1("e")
    assert_in_delta expected, v.value, 1e-6
  end

  def test_f32_zero
    v = WE.f32(0)
    assert_equal 0.0, v.value
  end

  def test_f32_integer_converts_to_float
    v = WE.f32(42)
    assert_kind_of Float, v.value
    assert_in_delta 42.0, v.value, 1e-6
  end

  def test_f32_preserves_nan
    v = WE.f32(Float::NAN)
    assert_predicate v.value, :nan?
  end

  def test_f32_preserves_infinity
    v = WE.f32(Float::INFINITY)
    assert_equal Float::INFINITY, v.value
  end

  def test_f32_negative_infinity
    v = WE.f32(-Float::INFINITY)
    assert_equal(-Float::INFINITY, v.value)
  end

  # ── f64 Constructor ────────────────────────────────────────────────

  def test_f64_basic_value
    v = WE.f64(3.14159265358979)
    assert_equal VT[:f64], v.type
    assert_in_delta 3.14159265358979, v.value, 1e-14
  end

  def test_f64_zero
    v = WE.f64(0)
    assert_equal 0.0, v.value
  end

  def test_f64_integer_converts_to_float
    v = WE.f64(100)
    assert_kind_of Float, v.value
  end

  def test_f64_preserves_nan
    v = WE.f64(Float::NAN)
    assert_predicate v.value, :nan?
  end

  # ── default_value ──────────────────────────────────────────────────

  def test_default_value_i32
    v = WE.default_value(VT[:i32])
    assert_equal VT[:i32], v.type
    assert_equal 0, v.value
  end

  def test_default_value_i64
    v = WE.default_value(VT[:i64])
    assert_equal VT[:i64], v.type
    assert_equal 0, v.value
  end

  def test_default_value_f32
    v = WE.default_value(VT[:f32])
    assert_equal VT[:f32], v.type
    assert_equal 0.0, v.value
  end

  def test_default_value_f64
    v = WE.default_value(VT[:f64])
    assert_equal VT[:f64], v.type
    assert_equal 0.0, v.value
  end

  def test_default_value_unknown_type_raises
    assert_raises(WE::TrapError) { WE.default_value(0x00) }
  end

  # ── Type Extraction Helpers ────────────────────────────────────────

  def test_as_i32_returns_value
    v = WE.i32(42)
    assert_equal 42, WE.as_i32(v)
  end

  def test_as_i32_rejects_i64
    v = WE.i64(42)
    assert_raises(WE::TrapError) { WE.as_i32(v) }
  end

  def test_as_i32_rejects_f32
    v = WE.f32(42.0)
    assert_raises(WE::TrapError) { WE.as_i32(v) }
  end

  def test_as_i64_returns_value
    v = WE.i64(999)
    assert_equal 999, WE.as_i64(v)
  end

  def test_as_i64_rejects_i32
    v = WE.i32(999)
    assert_raises(WE::TrapError) { WE.as_i64(v) }
  end

  def test_as_f32_returns_value
    v = WE.f32(1.5)
    assert_in_delta 1.5, WE.as_f32(v), 1e-6
  end

  def test_as_f32_rejects_f64
    v = WE.f64(1.5)
    assert_raises(WE::TrapError) { WE.as_f32(v) }
  end

  def test_as_f64_returns_value
    v = WE.f64(2.5)
    assert_in_delta 2.5, WE.as_f64(v), 1e-14
  end

  def test_as_f64_rejects_i32
    v = WE.i32(5)
    assert_raises(WE::TrapError) { WE.as_f64(v) }
  end

  # ── Unsigned Interpretation Helpers ─────────────────────────────────

  def test_to_u32_positive_unchanged
    assert_equal 42, WE.to_u32(42)
  end

  def test_to_u32_negative_one_becomes_max
    assert_equal 0xFFFFFFFF, WE.to_u32(-1)
  end

  def test_to_u32_min_signed
    assert_equal 0x80000000, WE.to_u32(-2_147_483_648)
  end

  def test_to_u64_positive_unchanged
    assert_equal 42, WE.to_u64(42)
  end

  def test_to_u64_negative_one_becomes_max
    assert_equal 0xFFFFFFFFFFFFFFFF, WE.to_u64(-1)
  end

  # ── TYPE_NAMES ─────────────────────────────────────────────────────

  def test_type_names_has_all_four_types
    names = WE::TYPE_NAMES
    assert_equal "i32", names[VT[:i32]]
    assert_equal "i64", names[VT[:i64]]
    assert_equal "f32", names[VT[:f32]]
    assert_equal "f64", names[VT[:f64]]
  end
end
