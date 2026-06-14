"""Register file for the Intel 8080 gate-level simulator.

=== The 8080 Register Architecture ===

The Intel 8080 has seven 8-bit working registers: A, B, C, D, E, H, L.
It also has a 16-bit stack pointer (SP) and a 16-bit program counter (PC).
These are stored in D flip-flop arrays — one flip-flop per bit.

Compared to the 8008:
- Same 7 working registers (A, B, C, D, E, H, L)
- PC expanded to 16 bits (was 14 bits in 8008)
- SP is now an explicit 16-bit register (8008 had an internal stack)
- 256 I/O ports (8008 had limited I/O)

=== Gate cost per register ===

Each 8-bit register: 8 D flip-flops = ~16 NOR gates = ~64 transistors.
Each 16-bit register: 16 D flip-flops = ~32 NOR gates = ~128 transistors.

7 × 8-bit working registers + flag register (5 bits) ≈ 448 + 40 transistors.
2 × 16-bit registers (PC + SP) ≈ 256 transistors.

=== Register encoding ===

3-bit codes for register operands (from opcode bits):
    000 = B    001 = C    010 = D    011 = E
    100 = H    101 = L    110 = M    111 = A

Code 6 (M) is a pseudo-register — it means "memory[H:L]". The register file
raises ValueError if you try to read or write index 6 directly.

Register pair codes (2-bit):
    00 = BC    01 = DE    10 = HL    11 = SP

=== Implementation model ===

The `register()` function from `logic_gates.sequential` models the D flip-flop
array behaviorally: given data bits and a clock signal, it captures the bits.
We wrap this in a stateful class that stores the flip-flop state between steps.
Each read() or write() triggers a clock cycle (low then high).
"""

from __future__ import annotations

from logic_gates import register

from intel8080_gatelevel.bits import (
    add_16bit,
    bits_to_int,
    int_to_bits,
)

# Register index constants (match 3-bit hardware encoding)
REG_B = 0
REG_C = 1
REG_D = 2
REG_E = 3
REG_H = 4
REG_L = 5
REG_M = 6   # pseudo-register — raises ValueError if accessed directly
REG_A = 7

# Register pair codes
PAIR_BC = 0
PAIR_DE = 1
PAIR_HL = 2
PAIR_SP = 3


class Register8:
    """8-bit register modeled as an array of 8 D flip-flops.

    All 8 flip-flops share the same clock signal. On a write(), the data
    is clocked in. On a read(), the stored bits are returned.

    In the real 8080, registers are latched on the rising edge of the
    internal clock. We simulate this by running two `register()` calls:
    first with clock=0 (master absorbs data) then clock=1 (slave outputs).

    Usage:
        >>> r = Register8()
        >>> r.write(0xAB)
        >>> r.read()
        171
    """

    def __init__(self) -> None:
        """Initialize to zero (power-on state: all flip-flops reset)."""
        # State for each of 8 flip-flops: list of internal state dicts
        self._state: list[dict[str, int]] | None = None
        self._value: int = 0   # cached integer value (optimization)

    def write(self, value: int) -> None:
        """Clock a new 8-bit value into the register.

        Simulates a rising clock edge: clock=0 (master latch absorbs data)
        then clock=1 (slave latch outputs the captured data).

        Args:
            value: 8-bit integer (0–255). Values are masked to 8 bits.
        """
        value = value & 0xFF
        bits = int_to_bits(value, 8)

        # Rising edge simulation: clock low then clock high
        _out_low, state_low = register(bits, 0, self._state, width=8)
        out_bits, self._state = register(bits, 1, state_low, width=8)

        self._value = bits_to_int(out_bits)

    def read(self) -> int:
        """Read the stored 8-bit value from the flip-flops.

        Returns:
            8-bit integer (0–255).
        """
        return self._value

    def read_bits(self) -> list[int]:
        """Read the stored value as a list of bits (LSB first).

        Returns:
            List of 8 bits, index 0 = LSB.
        """
        return int_to_bits(self._value, 8)


class Register16:
    """16-bit register modeled as an array of 16 D flip-flops.

    Used for the Stack Pointer (SP) and Program Counter (PC).
    Supports increment and decrement via the 16-bit ripple-carry adder.

    The 8080's SP is decremented before pushing (pre-decrement) and
    incremented after popping (post-increment). The PC is incremented
    by 1, 2, or 3 depending on instruction length.

    Usage:
        >>> r = Register16()
        >>> r.write(0x1234)
        >>> r.read()
        4660
        >>> r.inc()
        >>> r.read()
        4661
        >>> r.dec()
        >>> r.read()
        4660
    """

    def __init__(self) -> None:
        """Initialize to zero."""
        self._state: list[dict[str, int]] | None = None
        self._value: int = 0

    def write(self, value: int) -> None:
        """Clock a new 16-bit value into the register.

        Args:
            value: 16-bit integer (0–65535). Masked to 16 bits.
        """
        value = value & 0xFFFF
        bits = int_to_bits(value, 16)

        _out_low, state_low = register(bits, 0, self._state, width=16)
        out_bits, self._state = register(bits, 1, state_low, width=16)

        self._value = bits_to_int(out_bits)

    def read(self) -> int:
        """Read the stored 16-bit value.

        Returns:
            16-bit integer (0–65535).
        """
        return self._value

    def inc(self, amount: int = 1) -> None:
        """Increment the register by `amount` via the 16-bit adder.

        Used for PC advancement (amount=1, 2, or 3) and SP increment (2).
        Routes through the ripple_carry_adder gate chain: 16 full-adder
        stages for a 16-bit increment.

        Args:
            amount: Amount to add (default 1). Must be 0–65535.
        """
        new_val, _cout = add_16bit(self._value, amount & 0xFFFF, 0)
        self.write(new_val & 0xFFFF)

    def dec(self, amount: int = 1) -> None:
        """Decrement the register by `amount` via two's complement subtraction.

        Implements decrement as: value + NOT(amount) + 1
        Routes through the same ripple-carry adder as addition.

        For SP: dec(2) means SP = SP - 2 (pre-push).
        For SP: inc(2) means SP = SP + 2 (post-pop).

        Args:
            amount: Amount to subtract (default 1). Must be 0–65535.
        """
        # Two's complement: subtract by adding the two's complement
        twos = (~amount + 1) & 0xFFFF
        new_val, _cout = add_16bit(self._value, twos, 0)
        self.write(new_val & 0xFFFF)


class RegisterFile:
    """7 × 8-bit working registers for the Intel 8080 gate-level simulator.

    Contains: A (accumulator), B, C, D, E, H, L.
    Code 6 (M) is a pseudo-register — it is NOT stored here.
    The control unit handles M by substituting a memory read/write.

    Access by 3-bit register code (0=B, 1=C, 2=D, 3=E, 4=H, 5=L, 7=A).

    Register pair access (read_pair / write_pair):
        0 = BC: (B << 8) | C
        1 = DE: (D << 8) | E
        2 = HL: (H << 8) | L
        3 = SP: delegated to the SP Register16

    Usage:
        >>> rf = RegisterFile()
        >>> rf.write(REG_A, 42)
        >>> rf.read(REG_A)
        42
        >>> rf.write(REG_B, 0x12)
        >>> rf.write(REG_C, 0x34)
        >>> rf.read_pair(PAIR_BC)
        0x1234
    """

    def __init__(self) -> None:
        """Initialize all registers to zero."""
        # 8 Register8 objects (index 6 is unused — M is a pseudo-register)
        self._regs: list[Register8] = [Register8() for _ in range(8)]

    def read(self, code: int) -> int:
        """Read an 8-bit integer value from register at 3-bit code.

        Args:
            code: Register code 0–7 (NOT 6 — M is a pseudo-register).

        Returns:
            8-bit integer (0–255).

        Raises:
            ValueError: If code == 6 (M pseudo-register).
        """
        if code == REG_M:
            msg = (
                "Register M (code 6) is a pseudo-register — "
                "the control unit must substitute a memory access"
            )
            raise ValueError(msg)
        return self._regs[code].read()

    def write(self, code: int, value: int) -> None:
        """Write an 8-bit value to register at 3-bit code.

        Args:
            code:  Register code 0–7 (NOT 6 — M is a pseudo-register).
            value: 8-bit integer (0–255). Masked to 8 bits.

        Raises:
            ValueError: If code == 6 (M pseudo-register).
        """
        if code == REG_M:
            msg = (
                "Register M (code 6) is a pseudo-register — "
                "the control unit must substitute a memory write"
            )
            raise ValueError(msg)
        self._regs[code].write(value & 0xFF)

    def read_pair(self, pair_code: int, sp: Register16 | None = None) -> int:
        """Read a 16-bit register pair.

        Args:
            pair_code: 0=BC, 1=DE, 2=HL, 3=SP.
            sp:        The Stack Pointer Register16 (required if pair_code=3).

        Returns:
            16-bit value (0–65535).
        """
        match pair_code:
            case 0:
                return (self._regs[REG_B].read() << 8) | self._regs[REG_C].read()
            case 1:
                return (self._regs[REG_D].read() << 8) | self._regs[REG_E].read()
            case 2:
                return (self._regs[REG_H].read() << 8) | self._regs[REG_L].read()
            case 3:
                if sp is None:
                    msg = "SP Register16 required for pair_code=3"
                    raise ValueError(msg)
                return sp.read()
            case _:
                msg = f"Invalid pair code: {pair_code}"
                raise ValueError(msg)

    def write_pair(
        self,
        pair_code: int,
        value: int,
        sp: Register16 | None = None,
    ) -> None:
        """Write a 16-bit value to a register pair.

        The high byte goes into the first register of the pair,
        the low byte into the second.

        Args:
            pair_code: 0=BC, 1=DE, 2=HL, 3=SP.
            value:     16-bit integer (0–65535).
            sp:        SP Register16 (required if pair_code=3).
        """
        value = value & 0xFFFF
        hi = (value >> 8) & 0xFF
        lo = value & 0xFF

        match pair_code:
            case 0:
                self._regs[REG_B].write(hi)
                self._regs[REG_C].write(lo)
            case 1:
                self._regs[REG_D].write(hi)
                self._regs[REG_E].write(lo)
            case 2:
                self._regs[REG_H].write(hi)
                self._regs[REG_L].write(lo)
            case 3:
                if sp is None:
                    msg = "SP Register16 required for pair_code=3"
                    raise ValueError(msg)
                sp.write(value)
            case _:
                msg = f"Invalid pair code: {pair_code}"
                raise ValueError(msg)

    def read_bits(self, code: int) -> list[int]:
        """Read a register as a list of 8 bits (LSB first).

        Used by the ALU to get bit-list operands without a separate conversion.

        Args:
            code: Register code 0–7 (NOT 6).

        Returns:
            List of 8 bits, index 0 = LSB.
        """
        return self._regs[code].read_bits()
