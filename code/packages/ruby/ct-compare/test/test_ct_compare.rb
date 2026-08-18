# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_ct_compare"

class TestCtCompare < Minitest::Test
  CT = CodingAdventures::CtCompare

  def test_ct_eq_matches_only_identical_byte_strings
    assert CT.ct_eq("", "")
    assert CT.ct_eq("abc".b, "abc".b)
    assert CT.ct_eq(("\x00".b * 32), ("\x00".b * 32))
    refute CT.ct_eq("abc".b, "abd".b)
    refute CT.ct_eq("abc".b, "abc ".b)
    refute CT.ct_eq("".b, "a".b)
  end

  # The point of the accumulator is that a difference anywhere is caught. A
  # naive loop that forgot to OR would report equality whenever the LAST byte
  # matched, so differences are exercised at the first, middle, and last
  # positions rather than only at one end.
  def test_a_difference_at_any_position_is_detected
    base = ("\x41".b * 32)
    [0, 15, 31].each do |index|
      mutated = base.dup
      mutated.setbyte(index, 0x42)
      refute CT.ct_eq(base, mutated), "difference at byte #{index} must be detected"
      refute CT.ct_eq(mutated, base), "difference at byte #{index} must be symmetric"
    end
  end

  def test_ct_eq_accepts_byte_arrays_and_rejects_other_types
    assert CT.ct_eq([0x61, 0x62], "ab".b)
    assert CT.ct_eq([1, 2, 3], [1, 2, 3])
    assert_raises(ArgumentError) { CT.ct_eq(nil, "a".b) }
    assert_raises(ArgumentError) { CT.ct_eq(42, "a".b) }
  end

  def test_ct_eq_ignores_encoding_and_compares_bytes
    # Same bytes, different encodings, must compare equal: these are byte
    # strings, not text.
    utf8 = "abc"
    binary = "abc".b
    assert CT.ct_eq(utf8, binary)
  end

  def test_ct_eq_fixed_agrees_with_ct_eq_everywhere
    [
      ["", ""],
      ["abc".b, "abc".b],
      ["abc".b, "abd".b],
      ["abc".b, "ab".b]
    ].each do |left, right|
      assert_equal CT.ct_eq(left, right), CT.ct_eq_fixed(left, right),
        "ct_eq_fixed must not diverge from ct_eq"
    end
  end

  def test_ct_select_bytes_picks_a_side_without_branching
    left = "\x01\x02\x03".b
    right = "\xfd\xfe\xff".b
    assert_equal left, CT.ct_select_bytes(left, right, true)
    assert_equal right, CT.ct_select_bytes(left, right, false)
    assert_equal Encoding::BINARY, CT.ct_select_bytes(left, right, true).encoding
  end

  def test_ct_select_bytes_requires_equal_lengths
    assert_raises(ArgumentError) { CT.ct_select_bytes("ab".b, "abc".b, true) }
    assert_equal "".b, CT.ct_select_bytes("".b, "".b, true)
  end

  def test_ct_select_bytes_covers_every_byte_value
    # The XOR-mask identity must hold across the full 0..255 range, not just the
    # low bytes -- an implementation using a signed mask would break near 0x80.
    left = (0..255).to_a.pack("C*")
    right = (0..255).to_a.reverse.pack("C*")
    assert_equal left, CT.ct_select_bytes(left, right, true)
    assert_equal right, CT.ct_select_bytes(left, right, false)
  end

  def test_ct_eq_u64_compares_unsigned_counters
    assert CT.ct_eq_u64(0, 0)
    assert CT.ct_eq_u64(CT::U64_MAX, CT::U64_MAX)
    refute CT.ct_eq_u64(0, 1)
    refute CT.ct_eq_u64(CT::U64_MAX, CT::U64_MAX - 1)
    # A difference confined to the high bit must still be caught -- the fold
    # shifts by 63, so an off-by-one there would report equality here.
    refute CT.ct_eq_u64(1 << 63, 0)
    assert CT.ct_eq_u64(1 << 63, 1 << 63)
  end

  def test_ct_eq_u64_rejects_out_of_range_and_non_integers
    [-1, CT::U64_MAX + 1, 1.0, "1", nil].each do |bad|
      assert_raises(ArgumentError, "#{bad.inspect} must be rejected") { CT.ct_eq_u64(bad, 0) }
      assert_raises(ArgumentError, "#{bad.inspect} must be rejected") { CT.ct_eq_u64(0, bad) }
    end
  end

  # An equality primitive that answers true for unequal inputs is the worst
  # failure it can have. pack("C*") reduces mod 256 and truncates Floats, so
  # without validation ct_eq([256], "\x00") would be true. The typed ports
  # cannot express these inputs at all and Python raises; match that.
  def test_out_of_range_array_elements_are_rejected_not_truncated
    [[256], [257, 258], [-1], [97.9], [nil], ["a"], [1 << 64]].each do |bad|
      assert_raises(ArgumentError, "#{bad.inspect} must be rejected") { CT.ct_eq(bad, "\x00".b) }
      assert_raises(ArgumentError, "#{bad.inspect} must be rejected") { CT.ct_eq("\x00".b, bad) }
    end
    # The boundary values themselves remain valid.
    assert CT.ct_eq([0, 255], "\x00\xff".b)
  end

  # Ruby truthiness would make 0 select LEFT; Python treats 0 as falsy and
  # selects RIGHT. A call site ported across languages would silently take the
  # opposite branch, which in a constant-time select means the wrong key.
  def test_ct_select_bytes_requires_a_real_boolean
    left = "\x01".b
    right = "\x02".b
    [0, 1, nil, "true", :true].each do |bad|
      assert_raises(ArgumentError, "#{bad.inspect} must be rejected") do
        CT.ct_select_bytes(left, right, bad)
      end
    end
    assert_equal left, CT.ct_select_bytes(left, right, true)
    assert_equal right, CT.ct_select_bytes(left, right, false)
  end

  def test_internal_helpers_are_not_public
    refute_respond_to CT, :as_bytes
    refute_respond_to CT, :validate_u64!
  end
end
