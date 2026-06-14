"""State snapshot for the DEC PDP-11 (1970).

──────────────────────────────────────────────────────────────────────────────
THE MACHINE IN A NUTSHELL
──────────────────────────────────────────────────────────────────────────────

The DEC PDP-11 is the computer on which Unix was born.  Designed at Digital
Equipment Corporation in 1970, it was a 16-bit minicomputer that introduced
the concept of the *orthogonal ISA*: any addressing mode can be applied to
any register in any instruction.  Eight 16-bit registers, eight addressing
modes, and a beautifully uniform encoding that influenced every clean-ISA
design that followed — including the Motorola 68000 and, through it, the MIPS
and ARM architectures.

Register file:

  R0–R5   General-purpose 16-bit registers.  Fully symmetric: any ALU
          operation can target any register with any addressing mode.

  R6      SP — Stack Pointer.  Pre-decremented on push (``-(R6)``),
          post-incremented on pop (``(R6)+``).  The PDP-11 hardware stack
          grows downward, just like every major architecture since.

  R7      PC — Program Counter.  The PC is in the general register file.
          This means PC-relative addressing (``addr``, ``@addr``) and
          immediate addressing (``#n``, ``@#addr``) are just the ordinary
          autoincrement and index modes applied to R7.  There is no special
          branch encoding — branches use a separate 8-bit signed offset.

Processor Status Word (PSW), bits 3–0:

  Bit 3: N — Negative; MSB of result
  Bit 2: Z — Zero; result is zero
  Bit 1: V — oVerflow; signed result out of range
  Bit 0: C — Carry; unsigned overflow/borrow

Memory:

  64 KB flat byte-addressed little-endian address space.
  Load address: 0x1000.  Initial SP: 0xF000.

──────────────────────────────────────────────────────────────────────────────
"""

from __future__ import annotations

from dataclasses import dataclass

# ── Size / mask constants ─────────────────────────────────────────────────────
MEM_SIZE  = 65_536       # 64 KB — full 16-bit address space
ADDR_MASK = 0xFFFF       # 16-bit address mask
WORD_MASK = 0xFFFF       # 16-bit unsigned mask
BYTE_MASK = 0xFF         # 8-bit unsigned mask
WORD_MSB  = 0x8000       # MSB of 16-bit value
BYTE_MSB  = 0x80         # MSB of 8-bit value

LOAD_ADDR = 0x1000       # programs loaded at 0x1000
INIT_SP   = 0xF000       # initial stack pointer

# Register indices
SP = 6   # R6 = stack pointer
PC = 7   # R7 = program counter


@dataclass(frozen=True)
class PDP11State:
    """Immutable snapshot of the DEC PDP-11 CPU state.

    All register fields are unsigned 16-bit integers (0–65535).
    The ``psw`` field stores the full Processor Status Word; bits 3–0 are
    the condition codes N, Z, V, C.
    ``memory`` is a 65536-byte tuple of the entire address space.

    Attributes
    ----------
    r :
        Eight 16-bit general-purpose registers as a tuple.
        r[6] = SP (stack pointer), r[7] = PC (program counter).
    psw :
        16-bit Processor Status Word.  Bits 3–0 = N, Z, V, C.
    halted :
        True after a HALT instruction executes.
    memory :
        Full 64 KB address space as an immutable tuple of bytes.

    Examples
    --------
    >>> import dataclasses
    >>> mem = tuple([0] * 65536)
    >>> s = PDP11State(r=(0,)*8, psw=0, halted=False, memory=mem)
    >>> s.r[7]   # PC starts at 0
    0
    >>> s.n
    False
    """

    r: tuple[int, ...]      # R0–R7 (length 8), each 0–65535
    psw: int                # Processor Status Word (bits 3-0 = N,Z,V,C)
    halted: bool
    memory: tuple[int, ...]  # 65536 bytes

    # ------------------------------------------------------------------
    # Condition code properties (PSW bits 3–0)
    # ------------------------------------------------------------------
    #
    #  Bit 3: N — Negative
    #  Bit 2: Z — Zero
    #  Bit 1: V — oVerflow
    #  Bit 0: C — Carry
    #

    @property
    def n(self) -> bool:
        """Negative flag (PSW bit 3).

        >>> PDP11State(r=(0,)*8, psw=0b1000, halted=False, memory=tuple([0]*65536)).n
        True
        """
        return bool(self.psw & 0b1000)

    @property
    def z(self) -> bool:
        """Zero flag (PSW bit 2).

        >>> PDP11State(r=(0,)*8, psw=0b0100, halted=False, memory=tuple([0]*65536)).z
        True
        """
        return bool(self.psw & 0b0100)

    @property
    def v(self) -> bool:
        """Overflow flag (PSW bit 1).

        >>> PDP11State(r=(0,)*8, psw=0b0010, halted=False, memory=tuple([0]*65536)).v
        True
        """
        return bool(self.psw & 0b0010)

    @property
    def c(self) -> bool:
        """Carry flag (PSW bit 0).

        >>> PDP11State(r=(0,)*8, psw=0b0001, halted=False, memory=tuple([0]*65536)).c
        True
        """
        return bool(self.psw & 0b0001)
