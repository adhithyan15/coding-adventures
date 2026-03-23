defmodule CodingAdventures.FPGA.LUTTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.FPGA.LUT

  describe "new/1" do
    test "creates a 4-input LUT with 16 entries" do
      lut = LUT.new(4)
      assert lut.num_inputs == 4
      assert length(lut.truth_table) == 16
      assert Enum.all?(lut.truth_table, &(&1 == 0))
    end

    test "creates a 2-input LUT with 4 entries" do
      lut = LUT.new(2)
      assert length(lut.truth_table) == 4
    end

    test "creates a 6-input LUT with 64 entries" do
      lut = LUT.new(6)
      assert length(lut.truth_table) == 64
    end
  end

  describe "configure/2" do
    test "loads a truth table" do
      lut = LUT.new(2)
      lut = LUT.configure(lut, [0, 0, 0, 1])
      assert lut.truth_table == [0, 0, 0, 1]
    end

    test "raises on wrong table size" do
      lut = LUT.new(2)
      assert_raise ArgumentError, fn -> LUT.configure(lut, [0, 0, 0]) end
    end

    test "raises on non-binary values" do
      lut = LUT.new(2)
      assert_raise ArgumentError, fn -> LUT.configure(lut, [0, 0, 0, 2]) end
    end
  end

  describe "evaluate/2" do
    test "AND gate as LUT2" do
      # AND truth table: 00->0, 01->0, 10->0, 11->1
      lut = LUT.new(2) |> LUT.configure([0, 0, 0, 1])
      assert LUT.evaluate(lut, [0, 0]) == 0
      assert LUT.evaluate(lut, [0, 1]) == 0
      assert LUT.evaluate(lut, [1, 0]) == 0
      assert LUT.evaluate(lut, [1, 1]) == 1
    end

    test "XOR gate as LUT2" do
      lut = LUT.new(2) |> LUT.configure([0, 1, 1, 0])
      assert LUT.evaluate(lut, [0, 0]) == 0
      assert LUT.evaluate(lut, [0, 1]) == 1
      assert LUT.evaluate(lut, [1, 0]) == 1
      assert LUT.evaluate(lut, [1, 1]) == 0
    end

    test "OR gate as LUT2" do
      lut = LUT.new(2) |> LUT.configure([0, 1, 1, 1])
      assert LUT.evaluate(lut, [0, 0]) == 0
      assert LUT.evaluate(lut, [0, 1]) == 1
      assert LUT.evaluate(lut, [1, 0]) == 1
      assert LUT.evaluate(lut, [1, 1]) == 1
    end

    test "3-input majority gate" do
      # Majority: output 1 when 2 or more inputs are 1
      # Indices: 000->0 001->0 010->0 011->1 100->0 101->1 110->1 111->1
      lut = LUT.new(3) |> LUT.configure([0, 0, 0, 1, 0, 1, 1, 1])
      assert LUT.evaluate(lut, [0, 0, 0]) == 0
      assert LUT.evaluate(lut, [0, 1, 1]) == 1
      assert LUT.evaluate(lut, [1, 0, 1]) == 1
      assert LUT.evaluate(lut, [1, 1, 1]) == 1
    end

    test "raises on wrong number of inputs" do
      lut = LUT.new(2) |> LUT.configure([0, 0, 0, 1])
      assert_raise ArgumentError, fn -> LUT.evaluate(lut, [1]) end
      assert_raise ArgumentError, fn -> LUT.evaluate(lut, [1, 0, 0]) end
    end

    test "raises on non-binary input" do
      lut = LUT.new(2) |> LUT.configure([0, 0, 0, 1])
      assert_raise ArgumentError, fn -> LUT.evaluate(lut, [1, 2]) end
    end
  end
end
