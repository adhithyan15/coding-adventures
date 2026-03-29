# frozen_string_literal: true

require "test_helper"

module CodingAdventures
  module Transistors
    class TestCMOSInverter < Minitest::Test
      # Test CMOS NOT gate truth table and electrical properties.

      def test_truth_table
        # NOT gate: 0->1, 1->0.
        inv = CMOSInverter.new
        assert_equal 1, inv.evaluate_digital(0)
        assert_equal 0, inv.evaluate_digital(1)
      end

      def test_voltage_swing_high_input
        # Input HIGH -> output near GND.
        inv = CMOSInverter.new(CircuitParams.new(vdd: 3.3))
        result = inv.evaluate(3.3)
        assert result.voltage < 0.1
      end

      def test_voltage_swing_low_input
        # Input LOW -> output near Vdd.
        inv = CMOSInverter.new(CircuitParams.new(vdd: 3.3))
        result = inv.evaluate(0.0)
        assert result.voltage > 3.2
      end

      def test_static_power_zero
        # CMOS should have near-zero static power.
        inv = CMOSInverter.new
        assert inv.static_power < 1e-9
      end

      def test_dynamic_power
        # Dynamic power should be positive and scale with V^2.
        inv = CMOSInverter.new(CircuitParams.new(vdd: 3.3))
        p = inv.dynamic_power(frequency: 1e9, c_load: 1e-12)
        assert p > 0
      end

      def test_dynamic_power_scales_with_v_squared
        # Halving Vdd should reduce dynamic power by ~4x.
        inv_high = CMOSInverter.new(CircuitParams.new(vdd: 3.3))
        inv_low = CMOSInverter.new(CircuitParams.new(vdd: 1.65))
        p_high = inv_high.dynamic_power(frequency: 1e9, c_load: 1e-12)
        p_low = inv_low.dynamic_power(frequency: 1e9, c_load: 1e-12)
        ratio = p_high / p_low
        assert ratio > 3.5
        assert ratio < 4.5
      end

      def test_vtc_has_sharp_transition
        # VTC should show output snap from HIGH to LOW.
        inv = CMOSInverter.new(CircuitParams.new(vdd: 3.3))
        vtc = inv.voltage_transfer_characteristic(steps: 10)
        assert_equal 11, vtc.length
        # First point: input=0, output should be HIGH
        assert vtc[0][1] > 3.0
        # Last point: input=Vdd, output should be LOW
        assert vtc[-1][1] < 0.5
      end

      def test_rejects_invalid_input
        # evaluate_digital should reject non-binary inputs.
        inv = CMOSInverter.new
        assert_raises(ArgumentError) { inv.evaluate_digital(2) }
        assert_raises(TypeError) { inv.evaluate_digital(true) }
      end

      def test_transistor_count
        # Inverter uses 2 transistors.
        inv = CMOSInverter.new
        result = inv.evaluate(0.0)
        assert_equal 2, result.transistor_count
      end
    end

    class TestCMOSNand < Minitest::Test
      # Test CMOS NAND gate truth table.

      def test_truth_table
        nand = CMOSNand.new
        assert_equal 1, nand.evaluate_digital(0, 0)
        assert_equal 1, nand.evaluate_digital(0, 1)
        assert_equal 1, nand.evaluate_digital(1, 0)
        assert_equal 0, nand.evaluate_digital(1, 1)
      end

      def test_transistor_count
        nand = CMOSNand.new
        assert_equal 4, nand.transistor_count
      end

      def test_voltage_output_high
        nand = CMOSNand.new(CircuitParams.new(vdd: 3.3))
        result = nand.evaluate(0.0, 0.0)
        assert result.voltage > 3.0
      end

      def test_voltage_output_low
        nand = CMOSNand.new(CircuitParams.new(vdd: 3.3))
        result = nand.evaluate(3.3, 3.3)
        assert result.voltage < 0.5
      end

      def test_rejects_invalid_input
        nand = CMOSNand.new
        assert_raises(ArgumentError) { nand.evaluate_digital(2, 0) }
      end
    end

    class TestCMOSNor < Minitest::Test
      # Test CMOS NOR gate truth table.

      def test_truth_table
        nor = CMOSNor.new
        assert_equal 1, nor.evaluate_digital(0, 0)
        assert_equal 0, nor.evaluate_digital(0, 1)
        assert_equal 0, nor.evaluate_digital(1, 0)
        assert_equal 0, nor.evaluate_digital(1, 1)
      end

      def test_rejects_invalid_input
        nor = CMOSNor.new
        assert_raises(ArgumentError) { nor.evaluate_digital(0, 2) }
      end
    end

    class TestCMOSAnd < Minitest::Test
      # Test CMOS AND gate truth table.

      def test_truth_table
        and_gate = CMOSAnd.new
        assert_equal 0, and_gate.evaluate_digital(0, 0)
        assert_equal 0, and_gate.evaluate_digital(0, 1)
        assert_equal 0, and_gate.evaluate_digital(1, 0)
        assert_equal 1, and_gate.evaluate_digital(1, 1)
      end

      def test_rejects_invalid_input
        and_gate = CMOSAnd.new
        assert_raises(TypeError) { and_gate.evaluate_digital(true, 0) }
      end
    end

    class TestCMOSOr < Minitest::Test
      # Test CMOS OR gate truth table.

      def test_truth_table
        or_gate = CMOSOr.new
        assert_equal 0, or_gate.evaluate_digital(0, 0)
        assert_equal 1, or_gate.evaluate_digital(0, 1)
        assert_equal 1, or_gate.evaluate_digital(1, 0)
        assert_equal 1, or_gate.evaluate_digital(1, 1)
      end

      def test_rejects_invalid_input
        or_gate = CMOSOr.new
        assert_raises(ArgumentError) { or_gate.evaluate_digital(-1, 0) }
      end
    end

    class TestCMOSXor < Minitest::Test
      # Test CMOS XOR gate truth table.

      def test_truth_table
        xor_gate = CMOSXor.new
        assert_equal 0, xor_gate.evaluate_digital(0, 0)
        assert_equal 1, xor_gate.evaluate_digital(0, 1)
        assert_equal 1, xor_gate.evaluate_digital(1, 0)
        assert_equal 0, xor_gate.evaluate_digital(1, 1)
      end

      def test_evaluate_from_nands
        # NAND-based XOR should match direct XOR.
        xor_gate = CMOSXor.new
        [0, 1].each do |a|
          [0, 1].each do |b|
            assert_equal xor_gate.evaluate_digital(a, b), xor_gate.evaluate_from_nands(a, b)
          end
        end
      end

      def test_rejects_invalid_input
        xor_gate = CMOSXor.new
        assert_raises(ArgumentError) { xor_gate.evaluate_digital(0, 2) }
      end
    end

    class TestCMOSXnor < Minitest::Test
      # Test CMOS XNOR gate truth table (XNOR = NOT XOR = equivalence gate).

      def test_truth_table
        # XNOR outputs 1 when inputs are the SAME, 0 when different.
        xnor_gate = CMOSXnor.new
        assert_equal 1, xnor_gate.evaluate_digital(0, 0)
        assert_equal 0, xnor_gate.evaluate_digital(0, 1)
        assert_equal 0, xnor_gate.evaluate_digital(1, 0)
        assert_equal 1, xnor_gate.evaluate_digital(1, 1)
      end

      def test_xnor_is_inverse_of_xor
        # XNOR(a, b) = NOT(XOR(a, b)) for all input combinations.
        xnor_gate = CMOSXnor.new
        xor_gate = CMOSXor.new
        inv = CMOSInverter.new
        [0, 1].each do |a|
          [0, 1].each do |b|
            assert_equal inv.evaluate_digital(xor_gate.evaluate_digital(a, b)),
                         xnor_gate.evaluate_digital(a, b)
          end
        end
      end

      def test_transistor_count
        # XNOR = XOR (6) + Inverter (2) = 8 transistors.
        xnor_gate = CMOSXnor.new
        result = xnor_gate.evaluate(0.0, 0.0)
        assert_equal 8, result.transistor_count
      end

      def test_rejects_invalid_input
        xnor_gate = CMOSXnor.new
        assert_raises(ArgumentError) { xnor_gate.evaluate_digital(0, 2) }
        assert_raises(TypeError) { xnor_gate.evaluate_digital(true, 0) }
      end
    end
  end
end
