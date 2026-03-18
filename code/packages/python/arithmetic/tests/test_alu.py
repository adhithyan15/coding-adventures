"""Tests for the ALU."""

import pytest

from arithmetic import ALU, ALUOp


def _int_to_bits(n: int, width: int) -> list[int]:
    """Convert integer to LSB-first bit list."""
    return [(n >> i) & 1 for i in range(width)]


def _bits_to_int(bits: list[int]) -> int:
    """Convert LSB-first bit list to integer."""
    return sum(bit << i for i, bit in enumerate(bits))


@pytest.fixture
def alu() -> ALU:
    return ALU(bit_width=8)


class TestALUAdd:
    def test_1_plus_2(self, alu: ALU) -> None:
        """The target program: x = 1 + 2 = 3."""
        result = alu.execute(ALUOp.ADD, _int_to_bits(1, 8), _int_to_bits(2, 8))
        assert _bits_to_int(result.value) == 3
        assert result.zero is False
        assert result.carry is False

    def test_0_plus_0(self, alu: ALU) -> None:
        result = alu.execute(ALUOp.ADD, _int_to_bits(0, 8), _int_to_bits(0, 8))
        assert _bits_to_int(result.value) == 0
        assert result.zero is True

    def test_overflow(self, alu: ALU) -> None:
        result = alu.execute(ALUOp.ADD, _int_to_bits(255, 8), _int_to_bits(1, 8))
        assert _bits_to_int(result.value) == 0
        assert result.carry is True
        assert result.zero is True


class TestALUSub:
    def test_5_minus_3(self, alu: ALU) -> None:
        result = alu.execute(ALUOp.SUB, _int_to_bits(5, 8), _int_to_bits(3, 8))
        assert _bits_to_int(result.value) == 2
        assert result.zero is False

    def test_3_minus_3(self, alu: ALU) -> None:
        result = alu.execute(ALUOp.SUB, _int_to_bits(3, 8), _int_to_bits(3, 8))
        assert _bits_to_int(result.value) == 0
        assert result.zero is True


class TestALUBitwise:
    def test_and(self, alu: ALU) -> None:
        # 0b11001100 AND 0b10101010 = 0b10001000
        result = alu.execute(ALUOp.AND, _int_to_bits(0xCC, 8), _int_to_bits(0xAA, 8))
        assert _bits_to_int(result.value) == 0x88

    def test_or(self, alu: ALU) -> None:
        # 0b11001100 OR 0b10101010 = 0b11101110
        result = alu.execute(ALUOp.OR, _int_to_bits(0xCC, 8), _int_to_bits(0xAA, 8))
        assert _bits_to_int(result.value) == 0xEE

    def test_xor(self, alu: ALU) -> None:
        # 0b11001100 XOR 0b10101010 = 0b01100110
        result = alu.execute(ALUOp.XOR, _int_to_bits(0xCC, 8), _int_to_bits(0xAA, 8))
        assert _bits_to_int(result.value) == 0x66

    def test_not(self, alu: ALU) -> None:
        # NOT 0b00000000 = 0b11111111
        result = alu.execute(ALUOp.NOT, _int_to_bits(0, 8), [])
        assert _bits_to_int(result.value) == 255


class TestALUFlags:
    def test_zero_flag(self, alu: ALU) -> None:
        result = alu.execute(ALUOp.AND, _int_to_bits(0xF0, 8), _int_to_bits(0x0F, 8))
        assert result.zero is True

    def test_negative_flag(self, alu: ALU) -> None:
        # MSB set = negative in two's complement
        result = alu.execute(ALUOp.ADD, _int_to_bits(128, 8), _int_to_bits(0, 8))
        assert result.negative is True

    def test_signed_overflow(self, alu: ALU) -> None:
        # 127 + 1 = 128, but in signed 8-bit, 127 + 1 = -128 (overflow)
        result = alu.execute(ALUOp.ADD, _int_to_bits(127, 8), _int_to_bits(1, 8))
        assert result.overflow is True


class TestALUValidation:
    def test_wrong_bit_width(self, alu: ALU) -> None:
        with pytest.raises(ValueError, match="8 bits"):
            alu.execute(ALUOp.ADD, [0, 1], [0, 1])

    def test_invalid_bit_width(self) -> None:
        with pytest.raises(ValueError, match="at least 1"):
            ALU(bit_width=0)
