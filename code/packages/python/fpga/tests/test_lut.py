"""Tests for the LUT (Look-Up Table).

Coverage targets:
- Creation with default and custom truth tables
- Configuration / reprogramming
- Evaluation for all common boolean functions
- Edge cases (k=2, k=6)
- Validation (bad k, bad truth table, bad inputs)
"""

from __future__ import annotations

import pytest

from fpga.lut import LUT

# ─── Helper: common truth tables ─────────────────────────────────────

def and2_table_k4() -> list[int]:
    """2-input AND using I0, I1 in a 4-input LUT."""
    tt = [0] * 16
    tt[3] = 1  # I0=1, I1=1 → index 3
    return tt


def or2_table_k4() -> list[int]:
    """2-input OR using I0, I1."""
    tt = [0] * 16
    tt[1] = 1  # I0=1, I1=0
    tt[2] = 1  # I0=0, I1=1
    tt[3] = 1  # I0=1, I1=1
    return tt


def xor2_table_k4() -> list[int]:
    """2-input XOR using I0, I1."""
    tt = [0] * 16
    tt[1] = 1  # I0=1, I1=0
    tt[2] = 1  # I0=0, I1=1
    return tt


# ─── Creation ─────────────────────────────────────────────────────────

class TestLUTCreation:
    def test_default_k4(self) -> None:
        lut = LUT()
        assert lut.k == 4
        assert lut.truth_table == [0] * 16

    def test_custom_k(self) -> None:
        lut = LUT(k=2)
        assert lut.k == 2
        assert len(lut.truth_table) == 4

    def test_k6(self) -> None:
        lut = LUT(k=6)
        assert lut.k == 6
        assert len(lut.truth_table) == 64

    def test_with_initial_truth_table(self) -> None:
        tt = and2_table_k4()
        lut = LUT(k=4, truth_table=tt)
        assert lut.truth_table == tt

    def test_rejects_k_1(self) -> None:
        with pytest.raises(ValueError, match="between 2 and 6"):
            LUT(k=1)

    def test_rejects_k_7(self) -> None:
        with pytest.raises(ValueError, match="between 2 and 6"):
            LUT(k=7)

    def test_rejects_k_0(self) -> None:
        with pytest.raises(ValueError, match="between 2 and 6"):
            LUT(k=0)

    def test_rejects_bool_k(self) -> None:
        with pytest.raises(TypeError, match="must be an int"):
            LUT(k=True)  # type: ignore[arg-type]


# ─── Configuration ────────────────────────────────────────────────────

class TestLUTConfigure:
    def test_configure_changes_truth_table(self) -> None:
        lut = LUT(k=4)
        assert lut.truth_table == [0] * 16
        lut.configure(and2_table_k4())
        assert lut.truth_table == and2_table_k4()

    def test_reconfigure(self) -> None:
        """Can reprogram a LUT by calling configure again."""
        lut = LUT(k=4, truth_table=and2_table_k4())
        lut.configure(xor2_table_k4())
        assert lut.truth_table == xor2_table_k4()

    def test_rejects_wrong_length(self) -> None:
        lut = LUT(k=4)
        with pytest.raises(ValueError, match="does not match"):
            lut.configure([0] * 8)

    def test_rejects_non_list(self) -> None:
        lut = LUT(k=4)
        with pytest.raises(TypeError, match="must be a list"):
            lut.configure((0,) * 16)  # type: ignore[arg-type]

    def test_rejects_invalid_bit(self) -> None:
        lut = LUT(k=4)
        tt = [0] * 16
        tt[5] = 2
        with pytest.raises(ValueError, match="must be 0 or 1"):
            lut.configure(tt)


# ─── Evaluation ───────────────────────────────────────────────────────

class TestLUTEvaluate:
    def test_and_gate(self) -> None:
        lut = LUT(k=4, truth_table=and2_table_k4())
        assert lut.evaluate([0, 0, 0, 0]) == 0
        assert lut.evaluate([1, 0, 0, 0]) == 0
        assert lut.evaluate([0, 1, 0, 0]) == 0
        assert lut.evaluate([1, 1, 0, 0]) == 1

    def test_or_gate(self) -> None:
        lut = LUT(k=4, truth_table=or2_table_k4())
        assert lut.evaluate([0, 0, 0, 0]) == 0
        assert lut.evaluate([1, 0, 0, 0]) == 1
        assert lut.evaluate([0, 1, 0, 0]) == 1
        assert lut.evaluate([1, 1, 0, 0]) == 1

    def test_xor_gate(self) -> None:
        lut = LUT(k=4, truth_table=xor2_table_k4())
        assert lut.evaluate([0, 0, 0, 0]) == 0
        assert lut.evaluate([1, 0, 0, 0]) == 1
        assert lut.evaluate([0, 1, 0, 0]) == 1
        assert lut.evaluate([1, 1, 0, 0]) == 0

    def test_all_ones_truth_table(self) -> None:
        """A LUT with all 1s always outputs 1."""
        lut = LUT(k=2, truth_table=[1, 1, 1, 1])
        assert lut.evaluate([0, 0]) == 1
        assert lut.evaluate([1, 0]) == 1
        assert lut.evaluate([0, 1]) == 1
        assert lut.evaluate([1, 1]) == 1

    def test_k2_exhaustive(self) -> None:
        """Test all 4 input combinations for a k=2 AND gate."""
        lut = LUT(k=2, truth_table=[0, 0, 0, 1])
        assert lut.evaluate([0, 0]) == 0
        assert lut.evaluate([1, 0]) == 0
        assert lut.evaluate([0, 1]) == 0
        assert lut.evaluate([1, 1]) == 1

    def test_uses_all_4_inputs(self) -> None:
        """Set only the last entry (all inputs = 1) to verify all bits matter."""
        tt = [0] * 16
        tt[15] = 1  # I0=1, I1=1, I2=1, I3=1
        lut = LUT(k=4, truth_table=tt)
        assert lut.evaluate([1, 1, 1, 1]) == 1
        assert lut.evaluate([0, 1, 1, 1]) == 0
        assert lut.evaluate([1, 0, 1, 1]) == 0
        assert lut.evaluate([1, 1, 0, 1]) == 0
        assert lut.evaluate([1, 1, 1, 0]) == 0

    def test_reprogrammed_evaluation(self) -> None:
        """After reprogramming, evaluation uses the new truth table."""
        lut = LUT(k=4, truth_table=and2_table_k4())
        assert lut.evaluate([1, 1, 0, 0]) == 1
        lut.configure(xor2_table_k4())
        assert lut.evaluate([1, 1, 0, 0]) == 0  # XOR(1,1) = 0

    def test_rejects_wrong_input_length(self) -> None:
        lut = LUT(k=4)
        with pytest.raises(ValueError, match="does not match"):
            lut.evaluate([0, 0])

    def test_rejects_non_list_inputs(self) -> None:
        lut = LUT(k=4)
        with pytest.raises(TypeError, match="must be a list"):
            lut.evaluate((0, 0, 0, 0))  # type: ignore[arg-type]

    def test_rejects_invalid_input_bit(self) -> None:
        lut = LUT(k=4)
        with pytest.raises(ValueError, match="must be 0 or 1"):
            lut.evaluate([0, 2, 0, 0])


# ─── Truth table property ────────────────────────────────────────────

class TestLUTTruthTable:
    def test_returns_copy(self) -> None:
        lut = LUT(k=4, truth_table=and2_table_k4())
        tt = lut.truth_table
        tt[0] = 1  # Mutate the copy
        assert lut.truth_table[0] == 0  # Original unchanged
