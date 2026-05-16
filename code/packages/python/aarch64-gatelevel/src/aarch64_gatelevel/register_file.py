"""register_file.py — Gate-level 64-bit register file for the AArch64 simulator.

The register file stores all programmer-visible integer state as bit lists:
  - 32 GPRs (X0–X30, XZR), each 64 bits
  - SP (Stack Pointer), 64 bits
  - PC (Program Counter), 64 bits — managed externally but storable here

Gate-level storage
──────────────────
Each register is stored as a list[int] of 64 bits (a "register flip-flop bank").
On a real chip, each bit would be stored in a D flip-flop; reads are
combinational (the flip-flop output drives the bus) and writes are clocked
(the D input is loaded on the rising edge).

Here we model this as list[int] containing 0 or 1 values.  The external API
converts to/from Python integers for the instruction-level interface.

XZR special case
────────────────
Register index 31 is XZR (the zero register):
  - Reads always return 0 (all 64 bits are 0)
  - Writes are silently discarded (the flip-flop bank is never updated)

In a real AArch64 implementation, XZR is literally hardwired to 0V on every
bit line.  There is no storage behind it; any write is simply not connected
to anything.

W-register (32-bit) semantics
──────────────────────────────
When a W-register (32-bit view of an X register) is written, the 32-bit
result is zero-extended to fill the full 64-bit register.  This is the
AArch64 mandatory behavior: writing W0 clears the upper 32 bits of X0.

SP vs XZR at index 31
─────────────────────
In the AArch64 encoding space, register index 31 has two interpretations
depending on context:
  - Most arithmetic instructions: XZR (hardwired zero, writes discarded)
  - Load/store addressing (Rn field): SP (stack pointer, a separate register)

The register file exposes both:
  - read_bits(31) / write(31, ...) → XZR semantics (zero / discard)
  - read_sp_bits() / write_sp(...)  → SP semantics

The simulator dispatcher handles which interpretation to use per instruction.
"""

from __future__ import annotations

from .bits import bits_to_int, int_to_bits

# Register file constants
_NUM_GPRS: int = 32     # X0–X30, XZR (index 31)
_REG_BITS_64: int = 64
_REG_BITS_32: int = 32
_XZR: int = 31          # XZR register index

_ZERO_64: list[int] = [0] * 64   # hardwired zero (not mutable — always copied)


class RegisterFile:
    """Gate-level 64-bit register file for AArch64.

    Stores 32 GPRs plus SP as lists of 64 bits (LSB-first).
    All read/write operations convert between integers and bit lists.

    XZR convention:
      - read(31, ...) always returns the zero vector
      - write(31, ...) is a no-op (writes are silently discarded)
      - SP is separate; accessed via read_sp_bits() / write_sp()

    Example
    ───────
    >>> rf = RegisterFile()
    >>> rf.write_int(3, 42, sf=1)
    >>> rf.read(3, sf=1)
    42
    >>> rf.write_int(31, 0xDEAD, sf=1)  # XZR: write discarded
    >>> rf.read(31, sf=1)
    0
    """

    def __init__(self) -> None:
        # 32 GPRs × 64 bits each, all initialized to 0 (LSB-first bit lists)
        self._gprs: list[list[int]] = [
            [0] * _REG_BITS_64 for _ in range(_NUM_GPRS)
        ]
        # Stack pointer: separate 64-bit register
        self._sp: list[int] = [0] * _REG_BITS_64

    # ── GPR read operations ──────────────────────────────────────────────────

    def read_bits(self, idx: int, sf: int) -> list[int]:
        """Read a register as a bit list.

        idx=31 always returns a 64-element zero list (XZR).
        sf=1 → return 64-bit list; sf=0 → return the low 32 bits.

        Parameters
        ──────────
        idx : register number 0–31 (31 = XZR)
        sf  : 1→64-bit, 0→32-bit (W-register view)
        """
        if idx == _XZR:
            return [0] * (64 if sf else 32)
        bits = self._gprs[idx]
        if sf:
            return bits[:]   # 64-bit full copy
        return bits[:32]     # W-register: low 32 bits

    def read(self, idx: int, sf: int) -> int:
        """Read a register as a Python integer.

        idx=31 returns 0 (XZR).
        sf=1 → 64-bit unsigned int; sf=0 → 32-bit unsigned int.

        Example
        ───────
        >>> rf = RegisterFile()
        >>> rf.write_int(5, 0xDEADBEEF, sf=1)
        >>> rf.read(5, sf=1)
        3735928559
        """
        if idx == _XZR:
            return 0
        bits = self._gprs[idx]
        if sf:
            return bits_to_int(bits)
        return bits_to_int(bits[:32])

    # ── GPR write operations ─────────────────────────────────────────────────

    def write(self, idx: int, value_bits: list[int], sf: int) -> None:
        """Write a register from a bit list.

        idx=31 (XZR): write is silently discarded.
        sf=0 (W-register): value_bits is 32 bits; the 64-bit register is
          updated with the 32-bit value zero-extended to 64 bits.
        sf=1 (X-register): value_bits is 64 bits; stored directly.

        Parameters
        ──────────
        idx        : register number 0–31 (31 = XZR, write discarded)
        value_bits : LSB-first bit list (32 for sf=0, 64 for sf=1)
        sf         : 1→64-bit write, 0→32-bit write (zero-extends to 64)
        """
        if idx == _XZR:
            return   # XZR writes are discarded
        if sf:
            self._gprs[idx] = value_bits[:]
        else:
            # W-register write: zero-extend to 64 bits
            self._gprs[idx] = value_bits[:32] + [0] * 32

    def write_int(self, idx: int, value: int, sf: int) -> None:
        """Write an integer value to a register.

        Converts the integer to a bit list, then calls write().

        Parameters
        ──────────
        idx   : register number 0–31
        value : integer to store (masked to 32 or 64 bits)
        sf    : 1→64-bit, 0→32-bit

        Example
        ───────
        >>> rf = RegisterFile()
        >>> rf.write_int(0, 255, sf=0)   # W0 = 255 (zero-extends to X0)
        >>> rf.read(0, sf=1)
        255
        """
        if idx == _XZR:
            return
        if sf:
            bits = int_to_bits(value & 0xFFFF_FFFF_FFFF_FFFF, 64)
            self._gprs[idx] = bits
        else:
            lo_bits = int_to_bits(value & 0xFFFF_FFFF, 32)
            self._gprs[idx] = lo_bits + [0] * 32

    # ── SP access ────────────────────────────────────────────────────────────

    def read_sp_bits(self) -> list[int]:
        """Read the Stack Pointer as a 64-bit bit list."""
        return self._sp[:]

    def read_sp(self) -> int:
        """Read the Stack Pointer as a Python integer."""
        return bits_to_int(self._sp)

    def write_sp(self, value_bits: list[int]) -> None:
        """Write the Stack Pointer from a 64-bit bit list."""
        self._sp = value_bits[:]

    def write_sp_int(self, value: int) -> None:
        """Write the Stack Pointer from an integer."""
        self._sp = int_to_bits(value & 0xFFFF_FFFF_FFFF_FFFF, 64)

    # ── Snapshot interface ───────────────────────────────────────────────────

    def get_gprs_tuple(self) -> tuple[int, ...]:
        """Return all 32 GPR values as a tuple (for state snapshot).

        GPR[31] is always 0 (XZR enforcement).
        """
        result = []
        for i in range(_NUM_GPRS):
            if i == _XZR:
                result.append(0)
            else:
                result.append(bits_to_int(self._gprs[i]))
        return tuple(result)

    def reset(self) -> None:
        """Reset all registers and SP to zero."""
        self._gprs = [[0] * _REG_BITS_64 for _ in range(_NUM_GPRS)]
        self._sp = [0] * _REG_BITS_64
