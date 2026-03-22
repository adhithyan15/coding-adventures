defmodule CodingAdventures.Transistors.TTLGatesTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Transistors.TTLGates

  # ===========================================================================
  # TTL NAND Tests
  # ===========================================================================

  describe "TTL NAND truth table" do
    test "NAND(0,0) = 1" do
      assert TTLGates.ttl_nand_evaluate_digital(0, 0) == 1
    end

    test "NAND(0,1) = 1" do
      assert TTLGates.ttl_nand_evaluate_digital(0, 1) == 1
    end

    test "NAND(1,0) = 1" do
      assert TTLGates.ttl_nand_evaluate_digital(1, 0) == 1
    end

    test "NAND(1,1) = 0" do
      assert TTLGates.ttl_nand_evaluate_digital(1, 1) == 0
    end
  end

  describe "TTL NAND static power" do
    test "dissipates milliwatts when idle" do
      power = TTLGates.ttl_nand_static_power()
      assert power > 1.0e-3
    end
  end

  describe "TTL NAND voltage output" do
    test "output LOW near Vce_sat" do
      result = TTLGates.ttl_nand_evaluate(5.0, 5.0)
      assert result.voltage < 0.5
      assert result.logic_value == 0
    end

    test "output HIGH near Vcc - 0.7V" do
      result = TTLGates.ttl_nand_evaluate(0.0, 0.0)
      assert result.voltage > 3.0
      assert result.logic_value == 1
    end
  end

  describe "TTL NAND propagation delay" do
    test "delay in nanosecond range" do
      result = TTLGates.ttl_nand_evaluate(5.0, 5.0)
      assert result.propagation_delay > 1.0e-9
      assert result.propagation_delay < 100.0e-9
    end
  end

  describe "TTL NAND validation" do
    test "rejects invalid input" do
      assert_raise ArgumentError, fn ->
        TTLGates.ttl_nand_evaluate_digital(2, 0)
      end
    end
  end

  describe "TTL NAND custom Vcc" do
    test "custom Vcc is respected" do
      # With Vcc=3.3, the gate should still function
      result = TTLGates.ttl_nand_evaluate_digital(0, 0, 3.3)
      assert result == 1
    end
  end

  # ===========================================================================
  # RTL Inverter Tests
  # ===========================================================================

  describe "RTL Inverter truth table" do
    test "NOT 0 = 1" do
      assert TTLGates.rtl_inverter_evaluate_digital(0) == 1
    end

    test "NOT 1 = 0" do
      assert TTLGates.rtl_inverter_evaluate_digital(1) == 0
    end
  end

  describe "RTL Inverter voltage" do
    test "input LOW produces output near Vcc" do
      result = TTLGates.rtl_inverter_evaluate(0.0)
      assert result.voltage > 4.0
      assert result.logic_value == 1
    end

    test "input HIGH produces output near GND" do
      result = TTLGates.rtl_inverter_evaluate(5.0)
      assert result.voltage < 1.0
      assert result.logic_value == 0
    end
  end

  describe "RTL Inverter propagation delay" do
    test "slower than TTL" do
      result = TTLGates.rtl_inverter_evaluate(5.0)
      assert result.propagation_delay > 10.0e-9
    end
  end

  describe "RTL Inverter validation" do
    test "rejects boolean input" do
      assert_raise ArgumentError, fn ->
        TTLGates.rtl_inverter_evaluate_digital(true)
      end
    end
  end

  describe "RTL Inverter custom resistors" do
    test "custom resistors produce valid output" do
      # With custom resistors, the inverter should still function
      result_0 = TTLGates.rtl_inverter_evaluate(0.0, 5.0, 5000.0, 2000.0)
      assert result_0.logic_value == 1

      result_1 = TTLGates.rtl_inverter_evaluate(5.0, 5.0, 5000.0, 2000.0)
      assert result_1.logic_value == 0
    end
  end
end
