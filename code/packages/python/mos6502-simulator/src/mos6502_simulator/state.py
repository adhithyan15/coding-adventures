"""MOS6502State — frozen snapshot of the 6502 CPU after an instruction.

The 6502 has a very small register file compared to the Intel 8080:
  - A (accumulator): 8-bit, all arithmetic targets this
  - X, Y (index registers): 8-bit, used for address offsets
  - S (stack pointer): 8-bit, effective stack address = 0x0100 + S
  - PC (program counter): 16-bit
  - P (processor status): 7 active flags packed into one byte

Memory layout:
  0x0000–0x00FF  Zero page — 2-byte instructions access these faster
  0x0100–0x01FF  Stack     — hardware-fixed; S is an offset into here
  0x0200–0xFFFF  General   — program, data, I/O-mapped registers

Flag register (P) bit layout:
  Bit 7  N  Negative
  Bit 6  V  Overflow
  Bit 5  -  (always 1, unused)
  Bit 4  B  Break (set only in the copy pushed by BRK/PHP)
  Bit 3  D  Decimal mode
  Bit 2  I  Interrupt disable
  Bit 1  Z  Zero
  Bit 0  C  Carry
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class MOS6502State:
    """Immutable snapshot of the MOS 6502 CPU state.

    All integer fields are 0-based unsigned values within their natural
    width (8-bit or 16-bit). The memory field is a 65536-element tuple
    so that the state is hashable and fully self-contained.

    Usage::

        state = sim.get_state()
        assert state.a == 0x42
        assert state.flag_z is False
        assert state.memory[0x0200] == 0x55
    """

    # ── Registers ──────────────────────────────────────────────────────────
    a: int   # Accumulator                  0–255
    x: int   # Index register X             0–255
    y: int   # Index register Y             0–255
    s: int   # Stack pointer                0–255  (addr = 0x0100 + s)
    pc: int  # Program counter              0–65535

    # ── Processor status flags (P register) ────────────────────────────────
    flag_n: bool   # Negative  — bit 7 of last result
    flag_v: bool   # Overflow  — signed arithmetic overflow
    flag_b: bool   # Break     — set by BRK in the pushed P copy
    flag_d: bool   # Decimal   — BCD mode for ADC/SBC
    flag_i: bool   # Interrupt disable
    flag_z: bool   # Zero      — result was zero
    flag_c: bool   # Carry     — carry out (or not-borrow for SBC)

    # ── Simulator metadata ──────────────────────────────────────────────────
    halted: bool             # True after BRK instruction
    memory: tuple[int, ...]  # Full 64 KiB snapshot

    def p_byte(self) -> int:
        """Pack processor flags into the P status register byte.

        Bit 5 (unused) is always 1. Bit 4 (B) reflects flag_b.

        Truth table::

            7  N  flag_n
            6  V  flag_v
            5  -  1 (always set)
            4  B  flag_b
            3  D  flag_d
            2  I  flag_i
            1  Z  flag_z
            0  C  flag_c
        """
        return (
            (int(self.flag_n) << 7)
            | (int(self.flag_v) << 6)
            | 0x20                    # bit 5 always 1
            | (int(self.flag_b) << 4)
            | (int(self.flag_d) << 3)
            | (int(self.flag_i) << 2)
            | (int(self.flag_z) << 1)
            | int(self.flag_c)
        )
