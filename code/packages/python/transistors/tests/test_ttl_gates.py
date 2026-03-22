"""Tests for TTL logic gates (historical BJT-based)."""

import pytest

from transistors.ttl_gates import RTLInverter, TTLNand


class TestTTLNand:
    """Test TTL NAND gate truth table and power characteristics."""

    def test_truth_table(self) -> None:
        nand = TTLNand()
        assert nand.evaluate_digital(0, 0) == 1
        assert nand.evaluate_digital(0, 1) == 1
        assert nand.evaluate_digital(1, 0) == 1
        assert nand.evaluate_digital(1, 1) == 0

    def test_static_power_milliwatts(self) -> None:
        """TTL gates dissipate milliwatts even when idle."""
        nand = TTLNand()
        assert nand.static_power > 1e-3  # More than 1 milliwatt

    def test_output_voltage_low(self) -> None:
        """Output LOW should be near Vce_sat (~0.2V)."""
        nand = TTLNand()
        result = nand.evaluate(5.0, 5.0)
        assert result.voltage < 0.5
        assert result.logic_value == 0

    def test_output_voltage_high(self) -> None:
        """Output HIGH should be near Vcc - 0.7V."""
        nand = TTLNand()
        result = nand.evaluate(0.0, 0.0)
        assert result.voltage > 3.0
        assert result.logic_value == 1

    def test_propagation_delay(self) -> None:
        """TTL should have propagation delay in nanosecond range."""
        nand = TTLNand()
        result = nand.evaluate(5.0, 5.0)
        assert 1e-9 < result.propagation_delay < 100e-9

    def test_rejects_invalid_input(self) -> None:
        nand = TTLNand()
        with pytest.raises((ValueError, TypeError)):
            nand.evaluate_digital(2, 0)

    def test_custom_vcc(self) -> None:
        """Custom Vcc should be respected."""
        nand = TTLNand(vcc=3.3)
        assert nand.vcc == 3.3


class TestRTLInverter:
    """Test RTL inverter truth table and behavior."""

    def test_truth_table(self) -> None:
        inv = RTLInverter()
        assert inv.evaluate_digital(0) == 1
        assert inv.evaluate_digital(1) == 0

    def test_output_voltage_high(self) -> None:
        """Input LOW -> output near Vcc."""
        inv = RTLInverter()
        result = inv.evaluate(0.0)
        assert result.voltage > 4.0
        assert result.logic_value == 1

    def test_output_voltage_low(self) -> None:
        """Input HIGH -> output near GND."""
        inv = RTLInverter()
        result = inv.evaluate(5.0)
        assert result.voltage < 1.0
        assert result.logic_value == 0

    def test_propagation_delay(self) -> None:
        """RTL should be slower than TTL."""
        inv = RTLInverter()
        result = inv.evaluate(5.0)
        assert result.propagation_delay > 10e-9  # RTL is slow

    def test_rejects_invalid_input(self) -> None:
        inv = RTLInverter()
        with pytest.raises((ValueError, TypeError)):
            inv.evaluate_digital(True)

    def test_custom_resistors(self) -> None:
        """Custom resistor values should be respected."""
        inv = RTLInverter(r_base=5000, r_collector=2000)
        assert inv.r_base == 5000
        assert inv.r_collector == 2000
