# frozen_string_literal: true

require_relative "test_helper"

# ============================================================================
# Tests for logic gates -- exhaustive truth table verification.
#
# Every gate is tested against its complete truth table. Since each input
# can only be 0 or 1, we can verify every possible combination.
# ============================================================================

module CodingAdventures
  # Shorthand alias to keep test lines readable
  LG = LogicGates
end

# === Fundamental gates: truth table tests ===

class TestNOT < Minitest::Test
  def test_not_0
    assert_equal 1, CodingAdventures::LG.not_gate(0)
  end

  def test_not_1
    assert_equal 0, CodingAdventures::LG.not_gate(1)
  end
end

class TestAND < Minitest::Test
  def test_0_0
    assert_equal 0, CodingAdventures::LG.and_gate(0, 0)
  end

  def test_0_1
    assert_equal 0, CodingAdventures::LG.and_gate(0, 1)
  end

  def test_1_0
    assert_equal 0, CodingAdventures::LG.and_gate(1, 0)
  end

  def test_1_1
    assert_equal 1, CodingAdventures::LG.and_gate(1, 1)
  end
end

class TestOR < Minitest::Test
  def test_0_0
    assert_equal 0, CodingAdventures::LG.or_gate(0, 0)
  end

  def test_0_1
    assert_equal 1, CodingAdventures::LG.or_gate(0, 1)
  end

  def test_1_0
    assert_equal 1, CodingAdventures::LG.or_gate(1, 0)
  end

  def test_1_1
    assert_equal 1, CodingAdventures::LG.or_gate(1, 1)
  end
end

class TestXOR < Minitest::Test
  def test_0_0
    assert_equal 0, CodingAdventures::LG.xor_gate(0, 0)
  end

  def test_0_1
    assert_equal 1, CodingAdventures::LG.xor_gate(0, 1)
  end

  def test_1_0
    assert_equal 1, CodingAdventures::LG.xor_gate(1, 0)
  end

  def test_1_1
    assert_equal 0, CodingAdventures::LG.xor_gate(1, 1)
  end
end

class TestNAND < Minitest::Test
  def test_0_0
    assert_equal 1, CodingAdventures::LG.nand_gate(0, 0)
  end

  def test_0_1
    assert_equal 1, CodingAdventures::LG.nand_gate(0, 1)
  end

  def test_1_0
    assert_equal 1, CodingAdventures::LG.nand_gate(1, 0)
  end

  def test_1_1
    assert_equal 0, CodingAdventures::LG.nand_gate(1, 1)
  end
end

class TestNOR < Minitest::Test
  def test_0_0
    assert_equal 1, CodingAdventures::LG.nor_gate(0, 0)
  end

  def test_0_1
    assert_equal 0, CodingAdventures::LG.nor_gate(0, 1)
  end

  def test_1_0
    assert_equal 0, CodingAdventures::LG.nor_gate(1, 0)
  end

  def test_1_1
    assert_equal 0, CodingAdventures::LG.nor_gate(1, 1)
  end
end

class TestXNOR < Minitest::Test
  def test_0_0
    assert_equal 1, CodingAdventures::LG.xnor_gate(0, 0)
  end

  def test_0_1
    assert_equal 0, CodingAdventures::LG.xnor_gate(0, 1)
  end

  def test_1_0
    assert_equal 0, CodingAdventures::LG.xnor_gate(1, 0)
  end

  def test_1_1
    assert_equal 1, CodingAdventures::LG.xnor_gate(1, 1)
  end
end

# === NAND-derived gates: verify they match direct implementations ===

class TestNandDerived < Minitest::Test
  # Every NAND-derived gate must produce identical output to its
  # direct version. We test all input combinations.

  def test_nand_not_matches_not
    [0, 1].each do |a|
      assert_equal CodingAdventures::LG.not_gate(a),
        CodingAdventures::LG.nand_not(a),
        "nand_not(#{a}) should equal not_gate(#{a})"
    end
  end

  def test_nand_and_matches_and
    [0, 1].repeated_permutation(2).each do |a, b|
      assert_equal CodingAdventures::LG.and_gate(a, b),
        CodingAdventures::LG.nand_and(a, b),
        "nand_and(#{a}, #{b}) should equal and_gate(#{a}, #{b})"
    end
  end

  def test_nand_or_matches_or
    [0, 1].repeated_permutation(2).each do |a, b|
      assert_equal CodingAdventures::LG.or_gate(a, b),
        CodingAdventures::LG.nand_or(a, b),
        "nand_or(#{a}, #{b}) should equal or_gate(#{a}, #{b})"
    end
  end

  def test_nand_xor_matches_xor
    [0, 1].repeated_permutation(2).each do |a, b|
      assert_equal CodingAdventures::LG.xor_gate(a, b),
        CodingAdventures::LG.nand_xor(a, b),
        "nand_xor(#{a}, #{b}) should equal xor_gate(#{a}, #{b})"
    end
  end
end

# === NAND-derived gates: direct truth table tests ===

class TestNandNot < Minitest::Test
  def test_0
    assert_equal 1, CodingAdventures::LG.nand_not(0)
  end

  def test_1
    assert_equal 0, CodingAdventures::LG.nand_not(1)
  end
end

class TestNandAnd < Minitest::Test
  def test_0_0
    assert_equal 0, CodingAdventures::LG.nand_and(0, 0)
  end

  def test_0_1
    assert_equal 0, CodingAdventures::LG.nand_and(0, 1)
  end

  def test_1_0
    assert_equal 0, CodingAdventures::LG.nand_and(1, 0)
  end

  def test_1_1
    assert_equal 1, CodingAdventures::LG.nand_and(1, 1)
  end
end

class TestNandOr < Minitest::Test
  def test_0_0
    assert_equal 0, CodingAdventures::LG.nand_or(0, 0)
  end

  def test_0_1
    assert_equal 1, CodingAdventures::LG.nand_or(0, 1)
  end

  def test_1_0
    assert_equal 1, CodingAdventures::LG.nand_or(1, 0)
  end

  def test_1_1
    assert_equal 1, CodingAdventures::LG.nand_or(1, 1)
  end
end

class TestNandXor < Minitest::Test
  def test_0_0
    assert_equal 0, CodingAdventures::LG.nand_xor(0, 0)
  end

  def test_0_1
    assert_equal 1, CodingAdventures::LG.nand_xor(0, 1)
  end

  def test_1_0
    assert_equal 1, CodingAdventures::LG.nand_xor(1, 0)
  end

  def test_1_1
    assert_equal 0, CodingAdventures::LG.nand_xor(1, 1)
  end
end

# === Multi-input variants ===

class TestAndN < Minitest::Test
  def test_all_ones
    assert_equal 1, CodingAdventures::LG.and_n(1, 1, 1, 1)
  end

  def test_one_zero
    assert_equal 0, CodingAdventures::LG.and_n(1, 1, 0, 1)
  end

  def test_all_zeros
    assert_equal 0, CodingAdventures::LG.and_n(0, 0, 0)
  end

  def test_two_inputs
    assert_equal 1, CodingAdventures::LG.and_n(1, 1)
    assert_equal 0, CodingAdventures::LG.and_n(1, 0)
  end

  def test_five_inputs
    assert_equal 1, CodingAdventures::LG.and_n(1, 1, 1, 1, 1)
    assert_equal 0, CodingAdventures::LG.and_n(1, 1, 1, 1, 0)
  end

  def test_too_few_inputs
    assert_raises(ArgumentError) do
      CodingAdventures::LG.and_n(1)
    end
  end
end

class TestOrN < Minitest::Test
  def test_all_zeros
    assert_equal 0, CodingAdventures::LG.or_n(0, 0, 0, 0)
  end

  def test_one_one
    assert_equal 1, CodingAdventures::LG.or_n(0, 0, 1, 0)
  end

  def test_all_ones
    assert_equal 1, CodingAdventures::LG.or_n(1, 1, 1)
  end

  def test_two_inputs
    assert_equal 0, CodingAdventures::LG.or_n(0, 0)
    assert_equal 1, CodingAdventures::LG.or_n(0, 1)
  end

  def test_five_inputs
    assert_equal 0, CodingAdventures::LG.or_n(0, 0, 0, 0, 0)
    assert_equal 1, CodingAdventures::LG.or_n(0, 0, 0, 0, 1)
  end

  def test_too_few_inputs
    assert_raises(ArgumentError) do
      CodingAdventures::LG.or_n(0)
    end
  end
end

# === Input validation ===

class TestValidation < Minitest::Test
  # --- TypeError: non-Integer inputs ---

  def test_string_input
    assert_raises(TypeError) do
      CodingAdventures::LG.not_gate("a")
    end
  end

  def test_float_input
    assert_raises(TypeError) do
      CodingAdventures::LG.xor_gate(1.0, 0)
    end
  end

  def test_nil_input
    assert_raises(TypeError) do
      CodingAdventures::LG.and_gate(nil, 1)
    end
  end

  def test_boolean_true_input
    # In Ruby, true is NOT an Integer, so this should raise TypeError
    assert_raises(TypeError) do
      CodingAdventures::LG.and_gate(true, 0)
    end
  end

  def test_boolean_false_input
    assert_raises(TypeError) do
      CodingAdventures::LG.or_gate(false, 1)
    end
  end

  def test_symbol_input
    assert_raises(TypeError) do
      CodingAdventures::LG.not_gate(:one)
    end
  end

  def test_array_input
    assert_raises(TypeError) do
      CodingAdventures::LG.and_gate([1], 0)
    end
  end

  # --- ArgumentError: Integer but outside {0, 1} ---

  def test_invalid_int_value_2
    error = assert_raises(ArgumentError) do
      CodingAdventures::LG.and_gate(2, 1)
    end
    assert_match(/must be 0 or 1/, error.message)
  end

  def test_negative_value
    error = assert_raises(ArgumentError) do
      CodingAdventures::LG.or_gate(-1, 0)
    end
    assert_match(/must be 0 or 1/, error.message)
  end

  def test_large_value
    assert_raises(ArgumentError) do
      CodingAdventures::LG.not_gate(42)
    end
  end

  # --- TypeError message format ---

  def test_type_error_message_includes_class_name
    error = assert_raises(TypeError) do
      CodingAdventures::LG.not_gate("hello")
    end
    assert_match(/must be an Integer/, error.message)
    assert_match(/String/, error.message)
  end

  # --- Validation in composite gates ---

  def test_nand_validates_input
    assert_raises(TypeError) do
      CodingAdventures::LG.nand_gate("1", 0)
    end
  end

  def test_nor_validates_input
    assert_raises(ArgumentError) do
      CodingAdventures::LG.nor_gate(2, 0)
    end
  end

  def test_xnor_validates_input
    assert_raises(TypeError) do
      CodingAdventures::LG.xnor_gate(1.0, 0)
    end
  end

  # --- Validation in multi-input gates ---

  def test_and_n_validates_inputs
    assert_raises(TypeError) do
      CodingAdventures::LG.and_n(1, "1", 0)
    end
  end

  def test_or_n_validates_inputs
    assert_raises(ArgumentError) do
      CodingAdventures::LG.or_n(0, 2, 0)
    end
  end

  # --- Validation in NAND-derived gates ---

  def test_nand_not_validates_input
    assert_raises(TypeError) do
      CodingAdventures::LG.nand_not("0")
    end
  end

  def test_nand_and_validates_input
    assert_raises(ArgumentError) do
      CodingAdventures::LG.nand_and(3, 0)
    end
  end

  def test_nand_or_validates_input
    assert_raises(TypeError) do
      CodingAdventures::LG.nand_or(nil, 1)
    end
  end

  def test_nand_xor_validates_input
    assert_raises(TypeError) do
      CodingAdventures::LG.nand_xor(true, 0)
    end
  end
end

# === Version ===

class TestVersion < Minitest::Test
  def test_version_is_set
    refute_nil CodingAdventures::LogicGates::VERSION
  end

  def test_version_is_string
    assert_kind_of String, CodingAdventures::LogicGates::VERSION
  end

  def test_version_format
    assert_match(/\A\d+\.\d+\.\d+\z/, CodingAdventures::LogicGates::VERSION)
  end
end
