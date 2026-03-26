defmodule CodingAdventures.FPGA.SliceTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.FPGA.Slice

  describe "new/1" do
    test "creates slice with default 4-input LUTs" do
      slice = Slice.new()
      assert slice.lut_a.num_inputs == 4
      assert slice.lut_b.num_inputs == 4
      assert slice.use_ff_a == false
      assert slice.use_ff_b == false
      assert slice.carry_enable == false
    end

    test "creates slice with custom options" do
      slice = Slice.new(lut_inputs: 2, use_ff_a: true, carry_enable: true)
      assert slice.lut_a.num_inputs == 2
      assert slice.use_ff_a == true
      assert slice.carry_enable == true
    end
  end

  describe "configure/2" do
    test "configures both LUTs" do
      slice = Slice.new(lut_inputs: 2)
      slice = Slice.configure(slice, %{lut_a: [0, 0, 0, 1], lut_b: [0, 1, 1, 0]})
      assert slice.lut_a.truth_table == [0, 0, 0, 1]
      assert slice.lut_b.truth_table == [0, 1, 1, 0]
    end

    test "configures only one LUT" do
      slice = Slice.new(lut_inputs: 2)
      slice = Slice.configure(slice, %{lut_a: [1, 1, 1, 0]})
      assert slice.lut_a.truth_table == [1, 1, 1, 0]
      # LUT B should still be zeros
      assert slice.lut_b.truth_table == [0, 0, 0, 0]
    end
  end

  describe "evaluate/5" do
    test "combinational mode (no flip-flops)" do
      slice = Slice.new(lut_inputs: 2)
      slice = Slice.configure(slice, %{lut_a: [0, 0, 0, 1], lut_b: [0, 1, 1, 0]})

      # LUT A = AND(1,1) = 1, LUT B = XOR(0,1) = 1
      {out_a, out_b, _carry, _slice} = Slice.evaluate(slice, [1, 1], [0, 1], 0, 0)
      assert out_a == 1
      assert out_b == 1
    end

    test "combinational mode with different inputs" do
      slice = Slice.new(lut_inputs: 2)
      slice = Slice.configure(slice, %{lut_a: [0, 0, 0, 1], lut_b: [0, 1, 1, 0]})

      # LUT A = AND(1,0) = 0, LUT B = XOR(1,1) = 0
      {out_a, out_b, _carry, _slice} = Slice.evaluate(slice, [1, 0], [1, 1], 0, 0)
      assert out_a == 0
      assert out_b == 0
    end

    test "registered mode captures on clock" do
      slice = Slice.new(lut_inputs: 2, use_ff_a: true, use_ff_b: true)
      slice = Slice.configure(slice, %{lut_a: [0, 0, 0, 1], lut_b: [0, 0, 0, 1]})

      # Clock low: master captures data
      {_out_a, _out_b, _carry, slice} = Slice.evaluate(slice, [1, 1], [1, 1], 0, 0)

      # Clock high: slave presents data
      {out_a, out_b, _carry, _slice} = Slice.evaluate(slice, [1, 1], [1, 1], 1, 0)
      assert out_a == 1
      assert out_b == 1
    end

    test "carry chain propagation" do
      slice = Slice.new(lut_inputs: 2, carry_enable: true)
      # LUT A outputs 1 for input [1,1], LUT B outputs 1 for input [1,1]
      slice = Slice.configure(slice, %{lut_a: [0, 0, 0, 1], lut_b: [0, 0, 0, 1]})

      # carry_in=0, LUT_A=1: sum_a = 1 XOR 0 = 1, carry_mid = 1 AND 0 | 1 AND 1 = 1
      # carry_mid=1, LUT_B=1: sum_b = 1 XOR 1 = 0, carry_out = 1 AND 1 | 1 AND 1 = 1
      {out_a, out_b, carry_out, _slice} = Slice.evaluate(slice, [1, 1], [1, 1], 0, 0)
      assert out_a == 1
      assert out_b == 0
      assert carry_out == 1
    end

    test "carry chain with carry_in=1" do
      slice = Slice.new(lut_inputs: 2, carry_enable: true)
      # LUT A outputs 0 for [0,0]
      slice = Slice.configure(slice, %{lut_a: [0, 0, 0, 1], lut_b: [0, 0, 0, 1]})

      # carry_in=1, LUT_A=0: sum_a = 0 XOR 1 = 1, carry_mid = 0
      {out_a, _out_b, _carry, _slice} = Slice.evaluate(slice, [0, 0], [0, 0], 0, 1)
      assert out_a == 1
    end
  end
end
