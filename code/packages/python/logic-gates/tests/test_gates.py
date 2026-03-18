"""Tests for logic gates — exhaustive truth table verification."""

import pytest

from logic_gates import (
    AND,
    NAND,
    NOR,
    NOT,
    OR,
    XNOR,
    XOR,
    AND_N,
    OR_N,
    nand_and,
    nand_not,
    nand_or,
    nand_xor,
)


# === Fundamental gates: truth table tests ===


class TestNOT:
    def test_not_0(self) -> None:
        assert NOT(0) == 1

    def test_not_1(self) -> None:
        assert NOT(1) == 0


class TestAND:
    def test_0_0(self) -> None:
        assert AND(0, 0) == 0

    def test_0_1(self) -> None:
        assert AND(0, 1) == 0

    def test_1_0(self) -> None:
        assert AND(1, 0) == 0

    def test_1_1(self) -> None:
        assert AND(1, 1) == 1


class TestOR:
    def test_0_0(self) -> None:
        assert OR(0, 0) == 0

    def test_0_1(self) -> None:
        assert OR(0, 1) == 1

    def test_1_0(self) -> None:
        assert OR(1, 0) == 1

    def test_1_1(self) -> None:
        assert OR(1, 1) == 1


class TestXOR:
    def test_0_0(self) -> None:
        assert XOR(0, 0) == 0

    def test_0_1(self) -> None:
        assert XOR(0, 1) == 1

    def test_1_0(self) -> None:
        assert XOR(1, 0) == 1

    def test_1_1(self) -> None:
        assert XOR(1, 1) == 0


class TestNAND:
    def test_0_0(self) -> None:
        assert NAND(0, 0) == 1

    def test_0_1(self) -> None:
        assert NAND(0, 1) == 1

    def test_1_0(self) -> None:
        assert NAND(1, 0) == 1

    def test_1_1(self) -> None:
        assert NAND(1, 1) == 0


class TestNOR:
    def test_0_0(self) -> None:
        assert NOR(0, 0) == 1

    def test_0_1(self) -> None:
        assert NOR(0, 1) == 0

    def test_1_0(self) -> None:
        assert NOR(1, 0) == 0

    def test_1_1(self) -> None:
        assert NOR(1, 1) == 0


class TestXNOR:
    def test_0_0(self) -> None:
        assert XNOR(0, 0) == 1

    def test_0_1(self) -> None:
        assert XNOR(0, 1) == 0

    def test_1_0(self) -> None:
        assert XNOR(1, 0) == 0

    def test_1_1(self) -> None:
        assert XNOR(1, 1) == 1


# === NAND-derived gates: verify they match direct implementations ===


class TestNandDerived:
    """Every NAND-derived gate must produce identical output to its direct version."""

    @pytest.mark.parametrize("a", [0, 1])
    def test_nand_not_matches_not(self, a: int) -> None:
        assert nand_not(a) == NOT(a)

    @pytest.mark.parametrize("a,b", [(0, 0), (0, 1), (1, 0), (1, 1)])
    def test_nand_and_matches_and(self, a: int, b: int) -> None:
        assert nand_and(a, b) == AND(a, b)

    @pytest.mark.parametrize("a,b", [(0, 0), (0, 1), (1, 0), (1, 1)])
    def test_nand_or_matches_or(self, a: int, b: int) -> None:
        assert nand_or(a, b) == OR(a, b)

    @pytest.mark.parametrize("a,b", [(0, 0), (0, 1), (1, 0), (1, 1)])
    def test_nand_xor_matches_xor(self, a: int, b: int) -> None:
        assert nand_xor(a, b) == XOR(a, b)


# === Multi-input variants ===


class TestAND_N:
    def test_all_ones(self) -> None:
        assert AND_N(1, 1, 1, 1) == 1

    def test_one_zero(self) -> None:
        assert AND_N(1, 1, 0, 1) == 0

    def test_all_zeros(self) -> None:
        assert AND_N(0, 0, 0) == 0

    def test_two_inputs(self) -> None:
        assert AND_N(1, 1) == 1
        assert AND_N(1, 0) == 0

    def test_too_few_inputs(self) -> None:
        with pytest.raises(ValueError, match="at least 2"):
            AND_N(1)


class TestOR_N:
    def test_all_zeros(self) -> None:
        assert OR_N(0, 0, 0, 0) == 0

    def test_one_one(self) -> None:
        assert OR_N(0, 0, 1, 0) == 1

    def test_all_ones(self) -> None:
        assert OR_N(1, 1, 1) == 1

    def test_two_inputs(self) -> None:
        assert OR_N(0, 0) == 0
        assert OR_N(0, 1) == 1

    def test_too_few_inputs(self) -> None:
        with pytest.raises(ValueError, match="at least 2"):
            OR_N(0)


# === Input validation ===


class TestValidation:
    def test_invalid_int_value(self) -> None:
        with pytest.raises(ValueError, match="must be 0 or 1"):
            AND(2, 1)

    def test_negative_value(self) -> None:
        with pytest.raises(ValueError, match="must be 0 or 1"):
            OR(-1, 0)

    def test_string_input(self) -> None:
        with pytest.raises(TypeError, match="must be an int"):
            NOT("a")  # type: ignore[arg-type]

    def test_bool_input(self) -> None:
        with pytest.raises(TypeError, match="must be an int"):
            AND(True, False)  # type: ignore[arg-type]

    def test_float_input(self) -> None:
        with pytest.raises(TypeError, match="must be an int"):
            XOR(1.0, 0)  # type: ignore[arg-type]
