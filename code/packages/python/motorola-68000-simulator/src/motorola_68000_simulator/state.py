"""State snapshot for the Motorola 68000 (1979).

──────────────────────────────────────────────────────────────────────────────
THE MACHINE IN A NUTSHELL
──────────────────────────────────────────────────────────────────────────────

The Motorola 68000 is the CPU that clean-ISA advocates point to as "what the
8086 should have been."  Released in 1979 — one year after Intel's 8086 — it
powered the Apple Macintosh, Commodore Amiga, Atari ST, Sun workstations,
and Sega Genesis.

Register file overview:

  Data registers (32-bit, fully orthogonal)
  ─────────────────────────────────────────
  D0 – D7  General purpose.  Any ALU op can target any Dn.
           Byte ops affect bits 7–0 only (upper bits unchanged).
           Word ops affect bits 15–0 only (upper 16 bits unchanged).
           Longword ops affect all 32 bits.

  Address registers (32-bit, no byte access)
  ─────────────────────────────────────────
  A0 – A6  General purpose address/pointer registers.
  A7       Supervisor stack pointer (SSP).  This simulator is always
           in supervisor mode, so A7 == SSP throughout.

  Program counter
  ───────────────
  PC       32-bit register; only bits 23–0 are significant (24-bit
           address bus).  Instructions must be word-aligned (even).

  Status register (16-bit)
  ────────────────────────
  Bit 15: T1  — trace mode (1 = trace all instructions)
  Bit 14: T0  — trace mode (1 = trace branches only; 68020+ feature)
  Bit 13: S   — supervisor mode (always 1 in this simulator)
  Bit 12: M   — master state (68020+ only; 0 here)
  Bit 11: 0   — reserved
  Bits 10–8: I2 I1 I0 — interrupt priority mask (0=allow all, 7=block all)
  Bits 7–5: 0  — reserved
  Bit  4: X   — extend (set same as C for ADD/SUB; used by ADDX/SUBX)
  Bit  3: N   — negative (MSB of result)
  Bit  2: Z   — zero (result == 0)
  Bit  1: V   — overflow (signed overflow)
  Bit  0: C   — carry (unsigned overflow/borrow)

  The lower 5 bits (bits 4–0) form the Condition Code Register (CCR).
  MOVE #imm, CCR only modifies the CCR; MOVE #imm, SR modifies the whole SR.

──────────────────────────────────────────────────────────────────────────────
MEMORY MODEL
──────────────────────────────────────────────────────────────────────────────

Linear flat 24-bit address space: 0x000000 – 0xFFFFFF (16,777,216 bytes).

  • Big-endian: most-significant byte at lowest address.
  • Word and longword accesses must be to even (word-aligned) addresses.
  • No segmentation — just a flat byte array.

Memory layout used by this simulator:

  0x000000 – 0x0003FF   Exception vector table (256 vectors × 4 bytes each).
                        Vector 0 (0x000000) = initial SSP.
                        Vector 1 (0x000004) = initial PC.
  0x001000              Program load address.  reset() sets PC = 0x001000.
  0x00F000              Stack base.  reset() sets A7 (SSP) = 0x00F000.
  0xFFFFFF              Top of address space.

  Addresses wrap with & 0xFFFFFF, so address 0x1000000 == 0x000000.

──────────────────────────────────────────────────────────────────────────────
"""

from __future__ import annotations

from dataclasses import dataclass

_MEM_SIZE   = 16 * 1024 * 1024  # 16 MB = 16,777,216 bytes
_ADDR_MASK  = 0xFF_FFFF          # 24-bit address mask
_LONG_MASK  = 0xFFFF_FFFF        # 32-bit unsigned mask
_WORD_MASK  = 0xFFFF             # 16-bit unsigned mask
_BYTE_MASK  = 0xFF               # 8-bit unsigned mask
_LONG_MSB   = 0x8000_0000        # MSB of 32-bit value
_WORD_MSB   = 0x8000             # MSB of 16-bit value
_BYTE_MSB   = 0x80               # MSB of 8-bit value


@dataclass(frozen=True)
class M68KState:
    """Immutable snapshot of the Motorola 68000 CPU state.

    All integer register fields are unsigned 32-bit values (0–4,294,967,295).
    The status register (``sr``) is a 16-bit unsigned value.
    Memory is a tuple of 16,777,216 unsigned bytes (0–255).

    CCR bit properties (`.x`, `.n`, `.z`, `.v`, `.c`) extract condition
    code flags from ``sr``.

    Attributes
    ----------
    d0 … d7 :
        32-bit data registers (unsigned).

    a0 … a7 :
        32-bit address registers (unsigned).  ``a7`` is always the
        supervisor stack pointer in this simulator.

    pc :
        32-bit program counter.  Only bits 23–0 are significant.

    sr :
        16-bit status register.  Bits 15–8 = system byte (T/S/M/I).
        Bits 4–0 = CCR (X/N/Z/V/C).

    halted :
        ``True`` after STOP or TRAP #15 executes.

    memory :
        16,777,216-byte flat tuple of the entire 16 MB address space.

    Examples
    --------
    >>> import dataclasses
    >>> s = M68KState(
    ...     d0=42, d1=0, d2=0, d3=0, d4=0, d5=0, d6=0, d7=0,
    ...     a0=0, a1=0, a2=0, a3=0, a4=0, a5=0, a6=0, a7=0x00F000,
    ...     pc=0x001000, sr=0x2700, halted=False,
    ...     memory=tuple([0] * 16_777_216),
    ... )
    >>> s.d0
    42
    >>> s.z
    False
    >>> s.sr & 0x2000  # S bit: supervisor mode
    8192
    """

    # ── Data registers ───────────────────────────────────────────────────────
    d0: int
    d1: int
    d2: int
    d3: int
    d4: int
    d5: int
    d6: int
    d7: int

    # ── Address registers (A7 = supervisor stack pointer) ────────────────────
    a0: int
    a1: int
    a2: int
    a3: int
    a4: int
    a5: int
    a6: int
    a7: int

    # ── Program counter ──────────────────────────────────────────────────────
    pc: int

    # ── Status register ──────────────────────────────────────────────────────
    sr: int

    # ── Halt flag ────────────────────────────────────────────────────────────
    halted: bool

    # ── Memory ───────────────────────────────────────────────────────────────
    memory: tuple[int, ...]   # 16,777,216 bytes

    # ------------------------------------------------------------------
    # Register tuple accessors
    # ------------------------------------------------------------------

    @property
    def d(self) -> tuple[int, ...]:
        """All 8 data registers as a tuple (D0 first).

        >>> s = M68KState(
        ...     d0=1, d1=2, d2=3, d3=4, d4=5, d5=6, d6=7, d7=8,
        ...     a0=0, a1=0, a2=0, a3=0, a4=0, a5=0, a6=0, a7=0,
        ...     pc=0, sr=0x2700, halted=False,
        ...     memory=tuple([0]*16_777_216),
        ... )
        >>> s.d
        (1, 2, 3, 4, 5, 6, 7, 8)
        """
        return (self.d0, self.d1, self.d2, self.d3,
                self.d4, self.d5, self.d6, self.d7)

    @property
    def a(self) -> tuple[int, ...]:
        """All 8 address registers as a tuple (A0 first, A7 last).

        >>> s = M68KState(
        ...     d0=0, d1=0, d2=0, d3=0, d4=0, d5=0, d6=0, d7=0,
        ...     a0=10, a1=20, a2=0, a3=0, a4=0, a5=0, a6=0, a7=0xF000,
        ...     pc=0, sr=0x2700, halted=False,
        ...     memory=tuple([0]*16_777_216),
        ... )
        >>> s.a[0]
        10
        >>> s.a[7]
        61440
        """
        return (self.a0, self.a1, self.a2, self.a3,
                self.a4, self.a5, self.a6, self.a7)

    # ------------------------------------------------------------------
    # CCR flag properties (extracted from SR bits 4–0)
    # ------------------------------------------------------------------
    #
    # SR bit layout for CCR:
    #   bit 4 = X (extend)
    #   bit 3 = N (negative)
    #   bit 2 = Z (zero)
    #   bit 1 = V (overflow)
    #   bit 0 = C (carry)
    #

    @property
    def x(self) -> bool:
        """Extend flag (bit 4 of SR).

        Set the same as C by ADD/SUB; used by extended ops (ADDX, SUBX,
        NEGX).  Unlike C, X is *not* modified by logic operations or TST.

        >>> # SR with X set = 0b000...010000 = 0x10
        >>> bool(0x10 & (1 << 4))
        True
        """
        return bool(self.sr & (1 << 4))

    @property
    def n(self) -> bool:
        """Negative flag (bit 3 of SR) — copy of MSB of result.

        >>> bool(0x08 & (1 << 3))
        True
        """
        return bool(self.sr & (1 << 3))

    @property
    def z(self) -> bool:
        """Zero flag (bit 2 of SR) — set when result is exactly zero.

        >>> bool(0x04 & (1 << 2))
        True
        """
        return bool(self.sr & (1 << 2))

    @property
    def v(self) -> bool:
        """Overflow flag (bit 1 of SR) — signed overflow occurred.

        >>> bool(0x02 & (1 << 1))
        True
        """
        return bool(self.sr & (1 << 1))

    @property
    def c(self) -> bool:
        """Carry flag (bit 0 of SR) — unsigned carry/borrow out of MSB.

        >>> bool(0x01 & (1 << 0))
        True
        """
        return bool(self.sr & (1 << 0))

    # ------------------------------------------------------------------
    # Signed views of data registers
    # ------------------------------------------------------------------

    def d_signed(self, n: int) -> int:
        """Return data register n as a signed 32-bit integer.

        Parameters
        ----------
        n : int
            Register number 0–7.

        Examples
        --------
        >>> # 0x80000000 is -2147483648 in two's complement
        >>> 0x80000000 - 0x100000000
        -2147483648
        """
        v = self.d[n]
        return v if v < _LONG_MSB else v - (1 << 32)

    def d_word_signed(self, n: int) -> int:
        """Return the low 16 bits of data register n as a signed integer."""
        v = self.d[n] & _WORD_MASK
        return v if v < _WORD_MSB else v - 0x10000

    def d_byte_signed(self, n: int) -> int:
        """Return the low 8 bits of data register n as a signed integer."""
        v = self.d[n] & _BYTE_MASK
        return v if v < _BYTE_MSB else v - 0x100
