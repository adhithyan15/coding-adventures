defmodule CodingAdventures.ArithmeticTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Arithmetic, as: Arith

  describe "half_adder" do
    test "all truth table entries" do
      assert Arith.half_adder(0, 0) == {0, 0}
      assert Arith.half_adder(0, 1) == {1, 0}
      assert Arith.half_adder(1, 0) == {1, 0}
      assert Arith.half_adder(1, 1) == {0, 1}
    end
  end

  describe "full_adder" do
    test "all truth table entries" do
      assert Arith.full_adder(0, 0, 0) == {0, 0}
      assert Arith.full_adder(0, 0, 1) == {1, 0}
      assert Arith.full_adder(0, 1, 0) == {1, 0}
      assert Arith.full_adder(0, 1, 1) == {0, 1}
      assert Arith.full_adder(1, 0, 0) == {1, 0}
      assert Arith.full_adder(1, 0, 1) == {0, 1}
      assert Arith.full_adder(1, 1, 0) == {0, 1}
      assert Arith.full_adder(1, 1, 1) == {1, 1}
    end
  end

  describe "ripple_carry_adder" do
    test "5 + 3 = 8 in 4-bit" do
      # 5 = [1,0,1,0], 3 = [1,1,0,0] (LSB first)
      {sum, carry} = Arith.ripple_carry_adder([1, 0, 1, 0], [1, 1, 0, 0])
      assert sum == [0, 0, 0, 1]
      assert carry == 0
    end

    test "15 + 1 = 0 with carry in 4-bit" do
      {sum, carry} = Arith.ripple_carry_adder([1, 1, 1, 1], [1, 0, 0, 0])
      assert sum == [0, 0, 0, 0]
      assert carry == 1
    end
  end

  describe "ALU add" do
    test "basic addition" do
      result = Arith.alu_execute(:add, [1, 0, 1, 0], [1, 1, 0, 0])
      assert result.value == [0, 0, 0, 1]
      assert result.carry == false
    end

    test "overflow" do
      result = Arith.alu_execute(:add, [1, 1, 1, 1], [1, 0, 0, 0])
      assert result.value == [0, 0, 0, 0]
      assert result.carry == true
      assert result.zero == true
    end
  end

  describe "ALU sub" do
    test "basic subtraction" do
      # 5 - 3 = 2 → [0,1,0,0]
      result = Arith.alu_execute(:sub, [1, 0, 1, 0], [1, 1, 0, 0])
      assert result.value == [0, 1, 0, 0]
    end
  end

  describe "ALU bitwise" do
    test "AND" do
      result = Arith.alu_execute(:and_op, [1, 1, 0, 0], [1, 0, 1, 0])
      assert result.value == [1, 0, 0, 0]
    end

    test "OR" do
      result = Arith.alu_execute(:or_op, [1, 1, 0, 0], [1, 0, 1, 0])
      assert result.value == [1, 1, 1, 0]
    end

    test "XOR" do
      result = Arith.alu_execute(:xor_op, [1, 1, 0, 0], [1, 0, 1, 0])
      assert result.value == [0, 1, 1, 0]
    end

    test "NOT" do
      result = Arith.alu_execute(:not_op, [1, 0, 1, 0], [0, 0, 0, 0])
      assert result.value == [0, 1, 0, 1]
    end
  end
end
