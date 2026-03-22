# frozen_string_literal: true

require "test_helper"

module CodingAdventures
  module Transistors
    class TestTTLNand < Minitest::Test
      # Test TTL NAND gate truth table and power characteristics.

      def test_truth_table
        nand = TTLNand.new
        assert_equal 1, nand.evaluate_digital(0, 0)
        assert_equal 1, nand.evaluate_digital(0, 1)
        assert_equal 1, nand.evaluate_digital(1, 0)
        assert_equal 0, nand.evaluate_digital(1, 1)
      end

      def test_static_power_milliwatts
        # TTL gates dissipate milliwatts even when idle.
        nand = TTLNand.new
        assert nand.static_power > 1e-3 # More than 1 milliwatt
      end

      def test_output_voltage_low
        # Output LOW should be near Vce_sat (~0.2V).
        nand = TTLNand.new
        result = nand.evaluate(5.0, 5.0)
        assert result.voltage < 0.5
        assert_equal 0, result.logic_value
      end

      def test_output_voltage_high
        # Output HIGH should be near Vcc - 0.7V.
        nand = TTLNand.new
        result = nand.evaluate(0.0, 0.0)
        assert result.voltage > 3.0
        assert_equal 1, result.logic_value
      end

      def test_propagation_delay
        # TTL should have propagation delay in nanosecond range.
        nand = TTLNand.new
        result = nand.evaluate(5.0, 5.0)
        assert result.propagation_delay > 1e-9
        assert result.propagation_delay < 100e-9
      end

      def test_rejects_invalid_input
        nand = TTLNand.new
        assert_raises(ArgumentError) { nand.evaluate_digital(2, 0) }
      end

      def test_custom_vcc
        # Custom Vcc should be respected.
        nand = TTLNand.new(vcc: 3.3)
        assert_equal 3.3, nand.vcc
      end
    end

    class TestRTLInverter < Minitest::Test
      # Test RTL inverter truth table and behavior.

      def test_truth_table
        inv = RTLInverter.new
        assert_equal 1, inv.evaluate_digital(0)
        assert_equal 0, inv.evaluate_digital(1)
      end

      def test_output_voltage_high
        # Input LOW -> output near Vcc.
        inv = RTLInverter.new
        result = inv.evaluate(0.0)
        assert result.voltage > 4.0
        assert_equal 1, result.logic_value
      end

      def test_output_voltage_low
        # Input HIGH -> output near GND.
        inv = RTLInverter.new
        result = inv.evaluate(5.0)
        assert result.voltage < 1.0
        assert_equal 0, result.logic_value
      end

      def test_propagation_delay
        # RTL should be slower than TTL.
        inv = RTLInverter.new
        result = inv.evaluate(5.0)
        assert result.propagation_delay > 10e-9
      end

      def test_rejects_invalid_input
        inv = RTLInverter.new
        assert_raises(TypeError) { inv.evaluate_digital(true) }
      end

      def test_custom_resistors
        # Custom resistor values should be respected.
        inv = RTLInverter.new(r_base: 5000, r_collector: 2000)
        assert_equal 5000, inv.r_base
        assert_equal 2000, inv.r_collector
      end
    end
  end
end
