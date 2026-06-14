"""Shared test helpers for the ARMv7-A simulator test suite."""

import struct

from armv7a_simulator import ARMv7ASimulator, ARMv7AState


def run(prog: list[int] | bytes) -> ARMv7AState:
    """
    Execute a program (list of bytes or bytes object) and return the final state.

    The program is terminated by the halt sentinel: two zero bytes (0x00, 0x00),
    which form the 16-bit halt halfword 0x0000.
    """
    if isinstance(prog, list):
        b = bytes(prog) + b"\x00\x00"
    else:
        b = prog + b"\x00\x00"
    sim = ARMv7ASimulator()
    return sim.execute(b)


def hw(halfword: int) -> list[int]:
    """Pack a 16-bit halfword into a little-endian byte list."""
    return list(struct.pack("<H", halfword))


def w32(word: int) -> list[int]:
    """Pack a 32-bit word into a little-endian byte list."""
    return list(struct.pack("<I", word))


def thumb2_bl(offset: int) -> list[int]:
    """
    Encode a Thumb-2 BL instruction with the given signed byte offset.

    The offset is relative to (PC + 4), i.e., the address 4 bytes past the
    start of the BL instruction.  It must be even (Thumb) and fit in 25 bits
    signed.

    Returns 4 bytes (two 16-bit halfwords, little-endian each).
    """
    # offset is in bytes; Thumb BL encodes in half-words (offset >> 1 gives imm24)
    assert offset % 2 == 0, "BL offset must be even"
    # S:I1:I2:imm10:imm11:0
    raw = offset & 0x1FFFFFF
    s = (raw >> 24) & 1
    imm10 = (raw >> 13) & 0x3FF
    imm11 = (raw >> 1) & 0x7FF
    i1 = (raw >> 23) & 1
    i2 = (raw >> 22) & 1
    j1 = (~(i1 ^ s)) & 1
    j2 = (~(i2 ^ s)) & 1
    hw1 = 0xF000 | (s << 10) | imm10
    hw2 = 0xD000 | (j1 << 13) | (j2 << 11) | imm11
    return list(struct.pack("<HH", hw1, hw2))
