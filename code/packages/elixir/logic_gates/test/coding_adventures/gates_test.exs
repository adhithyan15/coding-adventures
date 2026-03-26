defmodule CodingAdventures.LogicGates.GatesTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.LogicGates.Gates

  # ===========================================================================
  # NOT GATE
  # ===========================================================================

  describe "not_gate/1" do
    test "NOT 0 = 1" do
      assert Gates.not_gate(0) == 1
    end

    test "NOT 1 = 0" do
      assert Gates.not_gate(1) == 0
    end

    test "rejects boolean" do
      assert_raise ArgumentError, fn -> Gates.not_gate(true) end
      assert_raise ArgumentError, fn -> Gates.not_gate(false) end
    end

    test "rejects non-integer" do
      assert_raise ArgumentError, fn -> Gates.not_gate(1.0) end
      assert_raise ArgumentError, fn -> Gates.not_gate("1") end
    end

    test "rejects out-of-range integer" do
      assert_raise ArgumentError, fn -> Gates.not_gate(2) end
      assert_raise ArgumentError, fn -> Gates.not_gate(-1) end
    end
  end

  # ===========================================================================
  # AND GATE
  # ===========================================================================

  describe "and_gate/2" do
    test "0 AND 0 = 0" do
      assert Gates.and_gate(0, 0) == 0
    end

    test "0 AND 1 = 0" do
      assert Gates.and_gate(0, 1) == 0
    end

    test "1 AND 0 = 0" do
      assert Gates.and_gate(1, 0) == 0
    end

    test "1 AND 1 = 1" do
      assert Gates.and_gate(1, 1) == 1
    end

    test "rejects invalid first input" do
      assert_raise ArgumentError, fn -> Gates.and_gate(2, 0) end
      assert_raise ArgumentError, fn -> Gates.and_gate(true, 0) end
    end

    test "rejects invalid second input" do
      assert_raise ArgumentError, fn -> Gates.and_gate(0, 2) end
      assert_raise ArgumentError, fn -> Gates.and_gate(0, true) end
      assert_raise ArgumentError, fn -> Gates.and_gate(1, -1) end
    end
  end

  # ===========================================================================
  # OR GATE
  # ===========================================================================

  describe "or_gate/2" do
    test "0 OR 0 = 0" do
      assert Gates.or_gate(0, 0) == 0
    end

    test "0 OR 1 = 1" do
      assert Gates.or_gate(0, 1) == 1
    end

    test "1 OR 0 = 1" do
      assert Gates.or_gate(1, 0) == 1
    end

    test "1 OR 1 = 1" do
      assert Gates.or_gate(1, 1) == 1
    end

    test "rejects invalid first input" do
      assert_raise ArgumentError, fn -> Gates.or_gate(-1, 0) end
    end

    test "rejects invalid second input" do
      assert_raise ArgumentError, fn -> Gates.or_gate(0, 2) end
    end
  end

  # ===========================================================================
  # XOR GATE
  # ===========================================================================

  describe "xor_gate/2" do
    test "0 XOR 0 = 0" do
      assert Gates.xor_gate(0, 0) == 0
    end

    test "0 XOR 1 = 1" do
      assert Gates.xor_gate(0, 1) == 1
    end

    test "1 XOR 0 = 1" do
      assert Gates.xor_gate(1, 0) == 1
    end

    test "1 XOR 1 = 0" do
      assert Gates.xor_gate(1, 1) == 0
    end

    test "rejects invalid inputs" do
      assert_raise ArgumentError, fn -> Gates.xor_gate(2, 0) end
      assert_raise ArgumentError, fn -> Gates.xor_gate(0, 2) end
    end
  end

  # ===========================================================================
  # NAND GATE
  # ===========================================================================

  describe "nand_gate/2" do
    test "0 NAND 0 = 1" do
      assert Gates.nand_gate(0, 0) == 1
    end

    test "0 NAND 1 = 1" do
      assert Gates.nand_gate(0, 1) == 1
    end

    test "1 NAND 0 = 1" do
      assert Gates.nand_gate(1, 0) == 1
    end

    test "1 NAND 1 = 0" do
      assert Gates.nand_gate(1, 1) == 0
    end

    test "rejects invalid inputs" do
      assert_raise ArgumentError, fn -> Gates.nand_gate(2, 0) end
      assert_raise ArgumentError, fn -> Gates.nand_gate(0, 2) end
    end
  end

  # ===========================================================================
  # NOR GATE
  # ===========================================================================

  describe "nor_gate/2" do
    test "0 NOR 0 = 1" do
      assert Gates.nor_gate(0, 0) == 1
    end

    test "0 NOR 1 = 0" do
      assert Gates.nor_gate(0, 1) == 0
    end

    test "1 NOR 0 = 0" do
      assert Gates.nor_gate(1, 0) == 0
    end

    test "1 NOR 1 = 0" do
      assert Gates.nor_gate(1, 1) == 0
    end

    test "rejects invalid inputs" do
      assert_raise ArgumentError, fn -> Gates.nor_gate(2, 0) end
      assert_raise ArgumentError, fn -> Gates.nor_gate(0, 2) end
    end
  end

  # ===========================================================================
  # XNOR GATE
  # ===========================================================================

  describe "xnor_gate/2" do
    test "0 XNOR 0 = 1" do
      assert Gates.xnor_gate(0, 0) == 1
    end

    test "0 XNOR 1 = 0" do
      assert Gates.xnor_gate(0, 1) == 0
    end

    test "1 XNOR 0 = 0" do
      assert Gates.xnor_gate(1, 0) == 0
    end

    test "1 XNOR 1 = 1" do
      assert Gates.xnor_gate(1, 1) == 1
    end

    test "rejects invalid inputs" do
      assert_raise ArgumentError, fn -> Gates.xnor_gate(2, 0) end
      assert_raise ArgumentError, fn -> Gates.xnor_gate(0, 2) end
    end
  end

  # ===========================================================================
  # NAND-DERIVED GATES
  # ===========================================================================

  describe "nand_not/1" do
    test "matches NOT for all inputs" do
      for a <- [0, 1] do
        assert Gates.nand_not(a) == Gates.not_gate(a)
      end
    end

    test "rejects invalid inputs" do
      assert_raise ArgumentError, fn -> Gates.nand_not(2) end
      assert_raise ArgumentError, fn -> Gates.nand_not(true) end
    end
  end

  describe "nand_and/2" do
    test "matches AND for all input combinations" do
      for a <- [0, 1], b <- [0, 1] do
        assert Gates.nand_and(a, b) == Gates.and_gate(a, b),
               "nand_and(#{a}, #{b}) should equal and_gate(#{a}, #{b})"
      end
    end

    test "rejects invalid inputs" do
      assert_raise ArgumentError, fn -> Gates.nand_and(2, 0) end
      assert_raise ArgumentError, fn -> Gates.nand_and(0, 2) end
    end
  end

  describe "nand_or/2" do
    test "matches OR for all input combinations" do
      for a <- [0, 1], b <- [0, 1] do
        assert Gates.nand_or(a, b) == Gates.or_gate(a, b),
               "nand_or(#{a}, #{b}) should equal or_gate(#{a}, #{b})"
      end
    end

    test "rejects invalid inputs" do
      assert_raise ArgumentError, fn -> Gates.nand_or(2, 0) end
      assert_raise ArgumentError, fn -> Gates.nand_or(0, 2) end
    end
  end

  describe "nand_xor/2" do
    test "matches XOR for all input combinations" do
      for a <- [0, 1], b <- [0, 1] do
        assert Gates.nand_xor(a, b) == Gates.xor_gate(a, b),
               "nand_xor(#{a}, #{b}) should equal xor_gate(#{a}, #{b})"
      end
    end

    test "rejects invalid inputs" do
      assert_raise ArgumentError, fn -> Gates.nand_xor(2, 0) end
      assert_raise ArgumentError, fn -> Gates.nand_xor(0, 2) end
    end
  end

  # ===========================================================================
  # MULTI-INPUT VARIANTS
  # ===========================================================================

  describe "and_n/1" do
    test "2 inputs" do
      assert Gates.and_n([1, 1]) == 1
      assert Gates.and_n([1, 0]) == 0
      assert Gates.and_n([0, 1]) == 0
      assert Gates.and_n([0, 0]) == 0
    end

    test "3 inputs" do
      assert Gates.and_n([1, 1, 1]) == 1
      assert Gates.and_n([1, 1, 0]) == 0
      assert Gates.and_n([0, 1, 1]) == 0
    end

    test "4 inputs" do
      assert Gates.and_n([1, 1, 1, 1]) == 1
      assert Gates.and_n([1, 1, 1, 0]) == 0
    end

    test "rejects fewer than 2 inputs" do
      assert_raise ArgumentError, fn -> Gates.and_n([1]) end
      assert_raise ArgumentError, fn -> Gates.and_n([]) end
    end

    test "rejects invalid values in list" do
      assert_raise ArgumentError, fn -> Gates.and_n([1, 2]) end
    end
  end

  describe "or_n/1" do
    test "2 inputs" do
      assert Gates.or_n([0, 0]) == 0
      assert Gates.or_n([0, 1]) == 1
      assert Gates.or_n([1, 0]) == 1
      assert Gates.or_n([1, 1]) == 1
    end

    test "3 inputs" do
      assert Gates.or_n([0, 0, 0]) == 0
      assert Gates.or_n([0, 0, 1]) == 1
    end

    test "4 inputs" do
      assert Gates.or_n([0, 0, 0, 0]) == 0
      assert Gates.or_n([0, 0, 0, 1]) == 1
    end

    test "rejects fewer than 2 inputs" do
      assert_raise ArgumentError, fn -> Gates.or_n([1]) end
      assert_raise ArgumentError, fn -> Gates.or_n([]) end
    end

    test "rejects invalid values in list" do
      assert_raise ArgumentError, fn -> Gates.or_n([0, 2]) end
    end
  end
end
