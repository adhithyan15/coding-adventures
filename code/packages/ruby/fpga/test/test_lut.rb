# frozen_string_literal: true

require_relative "test_helper"

# ============================================================================
# Tests for LUT (Look-Up Table).
# ============================================================================

class TestLUT < Minitest::Test
  def test_default_k_is_4
    lut = CodingAdventures::FPGA::LUT.new
    assert_equal 4, lut.k
  end

  def test_truth_table_all_zeros_initially
    lut = CodingAdventures::FPGA::LUT.new(k: 4)
    assert_equal [0] * 16, lut.truth_table
  end

  def test_configure_and_gate
    # AND gate: output 1 only when I0=1 AND I1=1 (index 3)
    and_table = [0] * 16
    and_table[3] = 1
    lut = CodingAdventures::FPGA::LUT.new(k: 4, truth_table: and_table)

    assert_equal 0, lut.evaluate([0, 0, 0, 0])
    assert_equal 0, lut.evaluate([1, 0, 0, 0])
    assert_equal 0, lut.evaluate([0, 1, 0, 0])
    assert_equal 1, lut.evaluate([1, 1, 0, 0])
  end

  def test_configure_xor_gate
    # XOR: output 1 when inputs differ (index 1 and 2)
    xor_table = [0] * 16
    xor_table[1] = 1
    xor_table[2] = 1
    lut = CodingAdventures::FPGA::LUT.new(k: 4, truth_table: xor_table)

    assert_equal 0, lut.evaluate([0, 0, 0, 0])
    assert_equal 1, lut.evaluate([1, 0, 0, 0])
    assert_equal 1, lut.evaluate([0, 1, 0, 0])
    assert_equal 0, lut.evaluate([1, 1, 0, 0])
  end

  def test_reconfigure
    lut = CodingAdventures::FPGA::LUT.new(k: 4)

    and_table = [0] * 16
    and_table[3] = 1
    lut.configure(and_table)
    assert_equal 1, lut.evaluate([1, 1, 0, 0])

    # Reconfigure as OR
    or_table = [0] * 16
    or_table[1] = 1
    or_table[2] = 1
    or_table[3] = 1
    lut.configure(or_table)
    assert_equal 1, lut.evaluate([1, 0, 0, 0])
    assert_equal 0, lut.evaluate([0, 0, 0, 0])
  end

  def test_k_2_lut
    lut = CodingAdventures::FPGA::LUT.new(k: 2, truth_table: [0, 0, 0, 1])
    assert_equal 0, lut.evaluate([0, 0])
    assert_equal 1, lut.evaluate([1, 1])
  end

  def test_invalid_k_too_small
    assert_raises(ArgumentError) { CodingAdventures::FPGA::LUT.new(k: 1) }
  end

  def test_invalid_k_too_large
    assert_raises(ArgumentError) { CodingAdventures::FPGA::LUT.new(k: 7) }
  end

  def test_invalid_k_type
    assert_raises(TypeError) { CodingAdventures::FPGA::LUT.new(k: "4") }
  end

  def test_wrong_truth_table_length
    assert_raises(ArgumentError) do
      CodingAdventures::FPGA::LUT.new(k: 4, truth_table: [0, 1])
    end
  end

  def test_wrong_inputs_length
    lut = CodingAdventures::FPGA::LUT.new(k: 4)
    assert_raises(ArgumentError) { lut.evaluate([0, 0]) }
  end

  def test_non_array_truth_table
    lut = CodingAdventures::FPGA::LUT.new(k: 4)
    assert_raises(TypeError) { lut.configure(42) }
  end

  def test_non_array_inputs
    lut = CodingAdventures::FPGA::LUT.new(k: 4)
    assert_raises(TypeError) { lut.evaluate(42) }
  end
end
