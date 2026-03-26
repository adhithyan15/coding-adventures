defmodule CodingAdventures.FPGA.CLBTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.FPGA.CLB

  describe "new/3" do
    test "creates CLB at given position" do
      clb = CLB.new(2, 3)
      assert clb.row == 2
      assert clb.col == 3
    end

    test "creates CLB with custom LUT inputs" do
      clb = CLB.new(0, 0, lut_inputs: 2)
      assert clb.slice_0.lut_a.num_inputs == 2
      assert clb.slice_1.lut_a.num_inputs == 2
    end
  end

  describe "configure/2" do
    test "configures both slices" do
      clb = CLB.new(0, 0, lut_inputs: 2)

      clb =
        CLB.configure(clb, %{
          slice_0: %{lut_a: [0, 0, 0, 1], lut_b: [0, 1, 1, 0]},
          slice_1: %{lut_a: [1, 1, 1, 0], lut_b: [1, 0, 0, 1]}
        })

      assert clb.slice_0.lut_a.truth_table == [0, 0, 0, 1]
      assert clb.slice_1.lut_a.truth_table == [1, 1, 1, 0]
    end

    test "configures one slice only" do
      clb = CLB.new(0, 0, lut_inputs: 2)
      clb = CLB.configure(clb, %{slice_0: %{lut_a: [1, 0, 0, 0]}})
      assert clb.slice_0.lut_a.truth_table == [1, 0, 0, 0]
      assert clb.slice_1.lut_a.truth_table == [0, 0, 0, 0]
    end
  end

  describe "evaluate/4" do
    test "evaluates all four LUTs" do
      clb = CLB.new(0, 0, lut_inputs: 2)

      clb =
        CLB.configure(clb, %{
          slice_0: %{lut_a: [0, 0, 0, 1], lut_b: [0, 1, 1, 0]},
          slice_1: %{lut_a: [1, 1, 1, 0], lut_b: [0, 0, 0, 1]}
        })

      inputs = %{s0_a: [1, 1], s0_b: [0, 1], s1_a: [1, 1], s1_b: [1, 1]}
      {outputs, _carry, _clb} = CLB.evaluate(clb, inputs, 0, 0)

      # slice0 LUT A: AND(1,1)=1, LUT B: XOR(0,1)=1
      # slice1 LUT A: NAND(1,1)=0, LUT B: AND(1,1)=1
      assert outputs == [1, 1, 0, 1]
    end

    test "carry propagates between slices" do
      clb =
        CLB.new(0, 0,
          lut_inputs: 2,
          slice_0_opts: [carry_enable: true],
          slice_1_opts: [carry_enable: true]
        )

      clb =
        CLB.configure(clb, %{
          slice_0: %{lut_a: [0, 0, 0, 1], lut_b: [0, 0, 0, 1]},
          slice_1: %{lut_a: [0, 0, 0, 1], lut_b: [0, 0, 0, 1]}
        })

      # All LUTs return 1, carry_in=1
      inputs = %{s0_a: [1, 1], s0_b: [1, 1], s1_a: [1, 1], s1_b: [1, 1]}
      {_outputs, carry_out, _clb} = CLB.evaluate(clb, inputs, 0, 1)
      # With carry chain, final carry_out should propagate through
      assert carry_out == 1
    end
  end
end
