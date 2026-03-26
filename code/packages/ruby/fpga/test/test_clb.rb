# frozen_string_literal: true

require_relative "test_helper"

# ============================================================================
# Tests for CLB (Configurable Logic Block).
# ============================================================================

class TestCLB < Minitest::Test
  def setup
    @clb = CodingAdventures::FPGA::CLB.new(lut_inputs: 4)
    @and_tt = [0] * 16
    @and_tt[3] = 1
    @xor_tt = [0] * 16
    @xor_tt[1] = 1
    @xor_tt[2] = 1
  end

  def test_two_slices
    assert_kind_of CodingAdventures::FPGA::Slice, @clb.slice0
    assert_kind_of CodingAdventures::FPGA::Slice, @clb.slice1
    assert_equal 4, @clb.k
  end

  def test_independent_slices
    @clb.slice0.configure(lut_a_table: @and_tt, lut_b_table: @and_tt)
    @clb.slice1.configure(lut_a_table: @xor_tt, lut_b_table: @xor_tt)

    out = @clb.evaluate(
      [1, 1, 0, 0], [1, 1, 0, 0],
      [1, 0, 0, 0], [1, 0, 0, 0],
      clock: 0
    )

    assert_equal 1, out.slice0.output_a  # AND(1,1)
    assert_equal 1, out.slice0.output_b  # AND(1,1)
    assert_equal 1, out.slice1.output_a  # XOR(1,0)
    assert_equal 1, out.slice1.output_b  # XOR(1,0)
  end

  def test_carry_chain_between_slices
    @clb.slice0.configure(
      lut_a_table: @and_tt, lut_b_table: @and_tt,
      carry_enabled: true
    )
    @clb.slice1.configure(
      lut_a_table: @xor_tt, lut_b_table: @xor_tt,
      carry_enabled: true
    )

    # Slice 0: both LUTs output AND(1,1)=1
    # carry_out_0 = (1 AND 1) OR (0 AND (1 XOR 1)) = 1
    # Slice 1: both LUTs output XOR(1,0)=1
    # carry_out_1 = (1 AND 1) OR (1 AND (1 XOR 1)) = 1
    out = @clb.evaluate(
      [1, 1, 0, 0], [1, 1, 0, 0],
      [1, 0, 0, 0], [1, 0, 0, 0],
      clock: 0, carry_in: 0
    )

    assert_equal 1, out.slice0.carry_out
    assert_equal 1, out.slice1.carry_out
  end
end
