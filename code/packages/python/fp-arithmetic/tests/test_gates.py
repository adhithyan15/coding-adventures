"""Tests for _gates.py — standalone logic gate primitives and ripple carry adder."""

from fp_arithmetic._gates import AND, NOT, OR, XOR, full_adder, ripple_carry_adder


class TestAND:
    """Tests for the AND gate."""

    def test_0_0(self) -> None:
        assert AND(0, 0) == 0

    def test_0_1(self) -> None:
        assert AND(0, 1) == 0

    def test_1_0(self) -> None:
        assert AND(1, 0) == 0

    def test_1_1(self) -> None:
        assert AND(1, 1) == 1


class TestOR:
    """Tests for the OR gate."""

    def test_0_0(self) -> None:
        assert OR(0, 0) == 0

    def test_0_1(self) -> None:
        assert OR(0, 1) == 1

    def test_1_0(self) -> None:
        assert OR(1, 0) == 1

    def test_1_1(self) -> None:
        assert OR(1, 1) == 1


class TestNOT:
    """Tests for the NOT gate."""

    def test_0(self) -> None:
        assert NOT(0) == 1

    def test_1(self) -> None:
        assert NOT(1) == 0


class TestXOR:
    """Tests for the XOR gate."""

    def test_0_0(self) -> None:
        assert XOR(0, 0) == 0

    def test_0_1(self) -> None:
        assert XOR(0, 1) == 1

    def test_1_0(self) -> None:
        assert XOR(1, 0) == 1

    def test_1_1(self) -> None:
        assert XOR(1, 1) == 0


class TestFullAdder:
    """Tests for the single-bit full adder."""

    def test_000(self) -> None:
        assert full_adder(0, 0, 0) == (0, 0)

    def test_001(self) -> None:
        assert full_adder(0, 0, 1) == (1, 0)

    def test_010(self) -> None:
        assert full_adder(0, 1, 0) == (1, 0)

    def test_011(self) -> None:
        assert full_adder(0, 1, 1) == (0, 1)

    def test_100(self) -> None:
        assert full_adder(1, 0, 0) == (1, 0)

    def test_101(self) -> None:
        assert full_adder(1, 0, 1) == (0, 1)

    def test_110(self) -> None:
        assert full_adder(1, 1, 0) == (0, 1)

    def test_111(self) -> None:
        assert full_adder(1, 1, 1) == (1, 1)


class TestRippleCarryAdder:
    """Tests for the ripple carry adder."""

    def test_zero_plus_zero(self) -> None:
        result, carry = ripple_carry_adder([0, 0, 0, 0], [0, 0, 0, 0])
        assert result == [0, 0, 0, 0]
        assert carry == 0

    def test_one_plus_one(self) -> None:
        # 1 + 1 = 2, LSB first: [1,0,0,0] + [1,0,0,0] = [0,1,0,0]
        result, carry = ripple_carry_adder([1, 0, 0, 0], [1, 0, 0, 0])
        assert result == [0, 1, 0, 0]
        assert carry == 0

    def test_max_plus_one(self) -> None:
        # 15 + 1 = 16, which overflows 4 bits
        result, carry = ripple_carry_adder([1, 1, 1, 1], [1, 0, 0, 0])
        assert result == [0, 0, 0, 0]
        assert carry == 1

    def test_with_carry_in(self) -> None:
        # 0 + 0 + carry_in=1 = 1
        result, carry = ripple_carry_adder([0, 0, 0, 0], [0, 0, 0, 0], carry_in=1)
        assert result == [1, 0, 0, 0]
        assert carry == 0

    def test_five_plus_three(self) -> None:
        # 5 (0101 LSB: 1010) + 3 (0011 LSB: 1100) = 8 (1000 LSB: 0001)
        # Actually: 5 = [1,0,1,0], 3 = [1,1,0,0], 8 = [0,0,0,1]
        result, carry = ripple_carry_adder([1, 0, 1, 0], [1, 1, 0, 0])
        assert result == [0, 0, 0, 1]
        assert carry == 0
