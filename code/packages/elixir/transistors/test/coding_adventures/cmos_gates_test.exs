defmodule CodingAdventures.Transistors.CMOSGatesTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Transistors.CMOSGates
  alias CodingAdventures.Transistors.Types.CircuitParams

  # ===========================================================================
  # CMOS Inverter Tests
  # ===========================================================================

  describe "CMOS Inverter truth table" do
    test "NOT 0 = 1" do
      assert CMOSGates.inverter_evaluate_digital(0) == 1
    end

    test "NOT 1 = 0" do
      assert CMOSGates.inverter_evaluate_digital(1) == 0
    end
  end

  describe "CMOS Inverter voltage swing" do
    test "input HIGH produces output near GND" do
      params = %CircuitParams{vdd: 3.3}
      result = CMOSGates.inverter_evaluate(3.3, params)
      assert result.voltage < 0.1
    end

    test "input LOW produces output near Vdd" do
      params = %CircuitParams{vdd: 3.3}
      result = CMOSGates.inverter_evaluate(0.0, params)
      assert result.voltage > 3.2
    end
  end

  describe "CMOS Inverter power" do
    test "static power is near zero" do
      assert CMOSGates.inverter_static_power() < 1.0e-9
    end

    test "dynamic power is positive" do
      params = %CircuitParams{vdd: 3.3}
      p = CMOSGates.inverter_dynamic_power(1.0e9, 1.0e-12, params)
      assert p > 0
    end

    test "dynamic power scales with V squared" do
      params_high = %CircuitParams{vdd: 3.3}
      params_low = %CircuitParams{vdd: 1.65}
      p_high = CMOSGates.inverter_dynamic_power(1.0e9, 1.0e-12, params_high)
      p_low = CMOSGates.inverter_dynamic_power(1.0e9, 1.0e-12, params_low)
      ratio = p_high / p_low
      assert ratio > 3.5 and ratio < 4.5
    end
  end

  describe "CMOS Inverter VTC" do
    test "VTC has sharp transition" do
      params = %CircuitParams{vdd: 3.3}
      vtc = CMOSGates.inverter_vtc(10, params)
      assert length(vtc) == 11
      # First point: input=0, output should be HIGH
      {_vin_first, vout_first} = hd(vtc)
      assert vout_first > 3.0
      # Last point: input=Vdd, output should be LOW
      {_vin_last, vout_last} = List.last(vtc)
      assert vout_last < 0.5
    end
  end

  describe "CMOS Inverter validation" do
    test "rejects invalid input value" do
      assert_raise ArgumentError, fn ->
        CMOSGates.inverter_evaluate_digital(2)
      end
    end

    test "rejects boolean input" do
      assert_raise ArgumentError, fn ->
        CMOSGates.inverter_evaluate_digital(true)
      end
    end
  end

  describe "CMOS Inverter transistor count" do
    test "uses 2 transistors" do
      result = CMOSGates.inverter_evaluate(0.0)
      assert result.transistor_count == 2
    end
  end

  # ===========================================================================
  # CMOS NAND Tests
  # ===========================================================================

  describe "CMOS NAND truth table" do
    test "NAND(0,0) = 1" do
      assert CMOSGates.nand_evaluate_digital(0, 0) == 1
    end

    test "NAND(0,1) = 1" do
      assert CMOSGates.nand_evaluate_digital(0, 1) == 1
    end

    test "NAND(1,0) = 1" do
      assert CMOSGates.nand_evaluate_digital(1, 0) == 1
    end

    test "NAND(1,1) = 0" do
      assert CMOSGates.nand_evaluate_digital(1, 1) == 0
    end
  end

  describe "CMOS NAND transistor count" do
    test "uses 4 transistors" do
      result = CMOSGates.nand_evaluate(0.0, 0.0)
      assert result.transistor_count == 4
    end
  end

  describe "CMOS NAND voltage" do
    test "output HIGH when both inputs low" do
      params = %CircuitParams{vdd: 3.3}
      result = CMOSGates.nand_evaluate(0.0, 0.0, params)
      assert result.voltage > 3.0
    end

    test "output LOW when both inputs high" do
      params = %CircuitParams{vdd: 3.3}
      result = CMOSGates.nand_evaluate(3.3, 3.3, params)
      assert result.voltage < 0.5
    end
  end

  describe "CMOS NAND validation" do
    test "rejects invalid input" do
      assert_raise ArgumentError, fn ->
        CMOSGates.nand_evaluate_digital(2, 0)
      end
    end
  end

  # ===========================================================================
  # CMOS NOR Tests
  # ===========================================================================

  describe "CMOS NOR truth table" do
    test "NOR(0,0) = 1" do
      assert CMOSGates.nor_evaluate_digital(0, 0) == 1
    end

    test "NOR(0,1) = 0" do
      assert CMOSGates.nor_evaluate_digital(0, 1) == 0
    end

    test "NOR(1,0) = 0" do
      assert CMOSGates.nor_evaluate_digital(1, 0) == 0
    end

    test "NOR(1,1) = 0" do
      assert CMOSGates.nor_evaluate_digital(1, 1) == 0
    end
  end

  describe "CMOS NOR validation" do
    test "rejects invalid input" do
      assert_raise ArgumentError, fn ->
        CMOSGates.nor_evaluate_digital(0, 2)
      end
    end
  end

  # ===========================================================================
  # CMOS AND Tests
  # ===========================================================================

  describe "CMOS AND truth table" do
    test "AND(0,0) = 0" do
      assert CMOSGates.and_evaluate_digital(0, 0) == 0
    end

    test "AND(0,1) = 0" do
      assert CMOSGates.and_evaluate_digital(0, 1) == 0
    end

    test "AND(1,0) = 0" do
      assert CMOSGates.and_evaluate_digital(1, 0) == 0
    end

    test "AND(1,1) = 1" do
      assert CMOSGates.and_evaluate_digital(1, 1) == 1
    end
  end

  describe "CMOS AND validation" do
    test "rejects boolean input" do
      assert_raise ArgumentError, fn ->
        CMOSGates.and_evaluate_digital(true, 0)
      end
    end
  end

  # ===========================================================================
  # CMOS OR Tests
  # ===========================================================================

  describe "CMOS OR truth table" do
    test "OR(0,0) = 0" do
      assert CMOSGates.or_evaluate_digital(0, 0) == 0
    end

    test "OR(0,1) = 1" do
      assert CMOSGates.or_evaluate_digital(0, 1) == 1
    end

    test "OR(1,0) = 1" do
      assert CMOSGates.or_evaluate_digital(1, 0) == 1
    end

    test "OR(1,1) = 1" do
      assert CMOSGates.or_evaluate_digital(1, 1) == 1
    end
  end

  describe "CMOS OR validation" do
    test "rejects negative input" do
      assert_raise ArgumentError, fn ->
        CMOSGates.or_evaluate_digital(-1, 0)
      end
    end
  end

  # ===========================================================================
  # CMOS XOR Tests
  # ===========================================================================

  describe "CMOS XOR truth table" do
    test "XOR(0,0) = 0" do
      assert CMOSGates.xor_evaluate_digital(0, 0) == 0
    end

    test "XOR(0,1) = 1" do
      assert CMOSGates.xor_evaluate_digital(0, 1) == 1
    end

    test "XOR(1,0) = 1" do
      assert CMOSGates.xor_evaluate_digital(1, 0) == 1
    end

    test "XOR(1,1) = 0" do
      assert CMOSGates.xor_evaluate_digital(1, 1) == 0
    end
  end

  describe "CMOS XOR from NANDs" do
    test "NAND-based XOR matches direct XOR" do
      for a <- [0, 1], b <- [0, 1] do
        assert CMOSGates.xor_evaluate_from_nands(a, b) == CMOSGates.xor_evaluate_digital(a, b)
      end
    end
  end

  describe "CMOS XOR validation" do
    test "rejects invalid input" do
      assert_raise ArgumentError, fn ->
        CMOSGates.xor_evaluate_digital(0, 2)
      end
    end
  end

  # ===========================================================================
  # CMOS XNOR Tests
  # ===========================================================================

  describe "CMOS XNOR truth table" do
    test "XNOR(0,0) = 1" do
      assert CMOSGates.xnor_evaluate_digital(0, 0) == 1
    end

    test "XNOR(0,1) = 0" do
      assert CMOSGates.xnor_evaluate_digital(0, 1) == 0
    end

    test "XNOR(1,0) = 0" do
      assert CMOSGates.xnor_evaluate_digital(1, 0) == 0
    end

    test "XNOR(1,1) = 1" do
      assert CMOSGates.xnor_evaluate_digital(1, 1) == 1
    end
  end

  describe "CMOS XNOR is inverse of XOR" do
    test "XNOR(a,b) = NOT(XOR(a,b)) for all inputs" do
      for a <- [0, 1], b <- [0, 1] do
        xor_result = CMOSGates.xor_evaluate_digital(a, b)
        xnor_result = CMOSGates.xnor_evaluate_digital(a, b)
        assert xnor_result == CMOSGates.inverter_evaluate_digital(xor_result)
      end
    end
  end

  describe "CMOS XNOR transistor count" do
    test "uses XOR transistors + 2 (inverter)" do
      params = %CircuitParams{}
      result = CMOSGates.xnor_evaluate(0.0, 0.0, params)
      xor_result = CMOSGates.xor_evaluate(0.0, 0.0, params)
      assert result.transistor_count == xor_result.transistor_count + 2
    end
  end

  describe "CMOS XNOR validation" do
    test "rejects invalid input for first argument" do
      assert_raise ArgumentError, fn ->
        CMOSGates.xnor_evaluate_digital(2, 0)
      end
    end

    test "rejects invalid input for second argument" do
      assert_raise ArgumentError, fn ->
        CMOSGates.xnor_evaluate_digital(0, 2)
      end
    end

    test "rejects boolean input" do
      assert_raise ArgumentError, fn ->
        CMOSGates.xnor_evaluate_digital(true, 0)
      end
    end
  end
end
