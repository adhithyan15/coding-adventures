"""Tests for CMOS logic gates built from transistors."""

import pytest

from transistors.cmos_gates import (
    CMOSAnd,
    CMOSInverter,
    CMOSNand,
    CMOSNor,
    CMOSOr,
    CMOSXor,
)
from transistors.types import CircuitParams


class TestCMOSInverter:
    """Test CMOS NOT gate truth table and electrical properties."""

    def test_truth_table(self) -> None:
        """NOT gate: 0->1, 1->0."""
        inv = CMOSInverter()
        assert inv.evaluate_digital(0) == 1
        assert inv.evaluate_digital(1) == 0

    def test_voltage_swing_high_input(self) -> None:
        """Input HIGH -> output near GND."""
        inv = CMOSInverter(CircuitParams(vdd=3.3))
        result = inv.evaluate(3.3)
        assert result.voltage < 0.1

    def test_voltage_swing_low_input(self) -> None:
        """Input LOW -> output near Vdd."""
        inv = CMOSInverter(CircuitParams(vdd=3.3))
        result = inv.evaluate(0.0)
        assert result.voltage > 3.2

    def test_static_power_zero(self) -> None:
        """CMOS should have near-zero static power."""
        inv = CMOSInverter()
        assert inv.static_power < 1e-9

    def test_dynamic_power(self) -> None:
        """Dynamic power should be positive and scale with V^2."""
        inv = CMOSInverter(CircuitParams(vdd=3.3))
        p = inv.dynamic_power(frequency=1e9, c_load=1e-12)
        assert p > 0

    def test_dynamic_power_scales_with_v_squared(self) -> None:
        """Halving Vdd should reduce dynamic power by ~4x."""
        inv_high = CMOSInverter(CircuitParams(vdd=3.3))
        inv_low = CMOSInverter(CircuitParams(vdd=1.65))
        p_high = inv_high.dynamic_power(frequency=1e9, c_load=1e-12)
        p_low = inv_low.dynamic_power(frequency=1e9, c_load=1e-12)
        ratio = p_high / p_low
        assert 3.5 < ratio < 4.5

    def test_vtc_has_sharp_transition(self) -> None:
        """VTC should show output snap from HIGH to LOW."""
        inv = CMOSInverter(CircuitParams(vdd=3.3))
        vtc = inv.voltage_transfer_characteristic(steps=10)
        assert len(vtc) == 11
        # First point: input=0, output should be HIGH
        assert vtc[0][1] > 3.0
        # Last point: input=Vdd, output should be LOW
        assert vtc[-1][1] < 0.5

    def test_rejects_invalid_input(self) -> None:
        """evaluate_digital should reject non-binary inputs."""
        inv = CMOSInverter()
        with pytest.raises((ValueError, TypeError)):
            inv.evaluate_digital(2)
        with pytest.raises((ValueError, TypeError)):
            inv.evaluate_digital(True)

    def test_transistor_count(self) -> None:
        """Inverter uses 2 transistors."""
        inv = CMOSInverter()
        result = inv.evaluate(0.0)
        assert result.transistor_count == 2


class TestCMOSNand:
    """Test CMOS NAND gate truth table."""

    def test_truth_table(self) -> None:
        nand = CMOSNand()
        assert nand.evaluate_digital(0, 0) == 1
        assert nand.evaluate_digital(0, 1) == 1
        assert nand.evaluate_digital(1, 0) == 1
        assert nand.evaluate_digital(1, 1) == 0

    def test_transistor_count(self) -> None:
        nand = CMOSNand()
        assert nand.transistor_count == 4

    def test_voltage_output_high(self) -> None:
        nand = CMOSNand(CircuitParams(vdd=3.3))
        result = nand.evaluate(0.0, 0.0)
        assert result.voltage > 3.0

    def test_voltage_output_low(self) -> None:
        nand = CMOSNand(CircuitParams(vdd=3.3))
        result = nand.evaluate(3.3, 3.3)
        assert result.voltage < 0.5

    def test_rejects_invalid_input(self) -> None:
        nand = CMOSNand()
        with pytest.raises((ValueError, TypeError)):
            nand.evaluate_digital(2, 0)


class TestCMOSNor:
    """Test CMOS NOR gate truth table."""

    def test_truth_table(self) -> None:
        nor = CMOSNor()
        assert nor.evaluate_digital(0, 0) == 1
        assert nor.evaluate_digital(0, 1) == 0
        assert nor.evaluate_digital(1, 0) == 0
        assert nor.evaluate_digital(1, 1) == 0

    def test_rejects_invalid_input(self) -> None:
        nor = CMOSNor()
        with pytest.raises((ValueError, TypeError)):
            nor.evaluate_digital(0, 2)


class TestCMOSAnd:
    """Test CMOS AND gate truth table."""

    def test_truth_table(self) -> None:
        and_gate = CMOSAnd()
        assert and_gate.evaluate_digital(0, 0) == 0
        assert and_gate.evaluate_digital(0, 1) == 0
        assert and_gate.evaluate_digital(1, 0) == 0
        assert and_gate.evaluate_digital(1, 1) == 1

    def test_rejects_invalid_input(self) -> None:
        and_gate = CMOSAnd()
        with pytest.raises((ValueError, TypeError)):
            and_gate.evaluate_digital(True, 0)


class TestCMOSOr:
    """Test CMOS OR gate truth table."""

    def test_truth_table(self) -> None:
        or_gate = CMOSOr()
        assert or_gate.evaluate_digital(0, 0) == 0
        assert or_gate.evaluate_digital(0, 1) == 1
        assert or_gate.evaluate_digital(1, 0) == 1
        assert or_gate.evaluate_digital(1, 1) == 1

    def test_rejects_invalid_input(self) -> None:
        or_gate = CMOSOr()
        with pytest.raises((ValueError, TypeError)):
            or_gate.evaluate_digital(-1, 0)


class TestCMOSXor:
    """Test CMOS XOR gate truth table."""

    def test_truth_table(self) -> None:
        xor_gate = CMOSXor()
        assert xor_gate.evaluate_digital(0, 0) == 0
        assert xor_gate.evaluate_digital(0, 1) == 1
        assert xor_gate.evaluate_digital(1, 0) == 1
        assert xor_gate.evaluate_digital(1, 1) == 0

    def test_evaluate_from_nands(self) -> None:
        """NAND-based XOR should match direct XOR."""
        xor_gate = CMOSXor()
        for a in (0, 1):
            for b in (0, 1):
                assert xor_gate.evaluate_from_nands(a, b) == xor_gate.evaluate_digital(a, b)

    def test_rejects_invalid_input(self) -> None:
        xor_gate = CMOSXor()
        with pytest.raises((ValueError, TypeError)):
            xor_gate.evaluate_digital(0, 2)
