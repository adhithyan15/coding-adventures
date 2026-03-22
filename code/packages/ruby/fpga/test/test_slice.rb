# frozen_string_literal: true

require_relative "test_helper"

# ============================================================================
# Tests for Slice.
# ============================================================================

class TestSlice < Minitest::Test
  def setup
    @slice = CodingAdventures::FPGA::Slice.new(lut_inputs: 4)
    @and_tt = [0] * 16
    @and_tt[3] = 1
    @xor_tt = [0] * 16
    @xor_tt[1] = 1
    @xor_tt[2] = 1
  end

  def test_combinational_output
    @slice.configure(lut_a_table: @and_tt, lut_b_table: @xor_tt)

    out = @slice.evaluate([1, 1, 0, 0], [1, 0, 0, 0], clock: 0)
    assert_equal 1, out.output_a  # AND(1,1)=1
    assert_equal 1, out.output_b  # XOR(1,0)=1
    assert_equal 0, out.carry_out # carry disabled
  end

  def test_carry_chain
    @slice.configure(
      lut_a_table: @and_tt, lut_b_table: @and_tt,
      carry_enabled: true
    )

    # Both LUTs output 1 (AND(1,1)=1)
    # carry_out = (1 AND 1) OR (0 AND (1 XOR 1)) = 1 OR 0 = 1
    out = @slice.evaluate([1, 1, 0, 0], [1, 1, 0, 0], clock: 0, carry_in: 0)
    assert_equal 1, out.carry_out
  end

  def test_carry_chain_with_carry_in
    @slice.configure(
      lut_a_table: @xor_tt, lut_b_table: @xor_tt,
      carry_enabled: true
    )

    # LUT A: XOR(1,0)=1, LUT B: XOR(1,0)=1
    # carry_out = (1 AND 1) OR (1 AND (1 XOR 1)) = 1 OR 0 = 1
    out = @slice.evaluate([1, 0, 0, 0], [1, 0, 0, 0], clock: 0, carry_in: 1)
    assert_equal 1, out.carry_out
  end

  def test_registered_output
    @slice.configure(
      lut_a_table: @and_tt, lut_b_table: @xor_tt,
      ff_a_enabled: true
    )

    # Clock low: master captures data
    @slice.evaluate([1, 1, 0, 0], [1, 0, 0, 0], clock: 0)
    # Clock high: slave captures from master
    out = @slice.evaluate([1, 1, 0, 0], [1, 0, 0, 0], clock: 1)
    # After one full cycle, the FF should capture AND(1,1)=1
    assert_equal 1, out.output_a
    assert_equal 1, out.output_b  # combinational (no FF)
  end

  def test_lut_accessors
    assert_kind_of CodingAdventures::FPGA::LUT, @slice.lut_a
    assert_kind_of CodingAdventures::FPGA::LUT, @slice.lut_b
    assert_equal 4, @slice.k
  end
end
