"""State snapshot for the Intel 8086 (1978).

──────────────────────────────────────────────────────────────────────────────
THE MACHINE IN A NUTSHELL
──────────────────────────────────────────────────────────────────────────────

The Intel 8086 is the direct ancestor of every x86 CPU shipping today.
Its register file is small but richly structured:

  General-purpose (16-bit, each with named high/low bytes)
  ────────────────
  AX (AH:AL)  — Accumulator.  Result of MUL, DIV.  I/O with port 0.
  BX (BH:BL)  — Base.  Memory addressing base register.
  CX (CH:CL)  — Counter.  REP prefix counts, LOOP, shift counts.
  DX (DH:DL)  — Data.  High word of 32-bit MUL/DIV result; I/O port address.

  Index / pointer (16-bit only)
  ─────────────────────────────
  SI  — Source Index.  LODS/MOVS/CMPS source pointer (DS segment).
  DI  — Destination Index.  STOS/MOVS/CMPS destination (ES segment).
  SP  — Stack Pointer.  Points to top of stack in SS segment.
  BP  — Base Pointer.  Stack-frame base; defaults to SS segment.

  Segment registers (16-bit)
  ──────────────────────────
  CS  — Code Segment.  Physical instruction fetch = CS×16 + IP.
  DS  — Data Segment.  Default for most memory references.
  SS  — Stack Segment.  PUSH/POP and BP-relative accesses.
  ES  — Extra Segment.  Destination for string operations.

  Instruction pointer
  ───────────────────
  IP  — 16-bit offset within CS.  Physical address = CS×16 + IP.

  FLAGS register (16-bit)
  ───────────────────────
  Bits 15–12: undefined/reserved
  Bit 11: OF — overflow
  Bit 10: DF — direction (0=inc, 1=dec for string ops)
  Bit  9: IF — interrupt enable
  Bit  8: TF — trap (single-step)
  Bit  7: SF — sign
  Bit  6: ZF — zero
  Bit  5: undefined
  Bit  4: AF — auxiliary carry (BCD)
  Bit  3: undefined
  Bit  2: PF — parity (even number of 1-bits in low byte)
  Bit  1: always 1
  Bit  0: CF — carry

──────────────────────────────────────────────────────────────────────────────
MEMORY MODEL
──────────────────────────────────────────────────────────────────────────────

The 8086 uses a segmented 20-bit address space:

    physical = (segment_register × 16 + offset) & 0xFFFFF

This gives a 1 MB address space (0x00000–0xFFFFF).  Segment registers hold
16-bit values; offsets (IP, SP, SI, DI, etc.) are 16-bit.

The simulator stores all 1,048,576 bytes in a single flat tuple.

──────────────────────────────────────────────────────────────────────────────
"""

from __future__ import annotations

from dataclasses import dataclass

_MEM_SIZE = 1_048_576   # 1 MB = 2^20 bytes
_PORT_SIZE = 256         # 256 I/O ports per direction
_WORD_MASK = 0xFFFF      # 16-bit unsigned mask
_BYTE_MASK = 0xFF        # 8-bit unsigned mask


@dataclass(frozen=True)
class X86State:
    """Immutable snapshot of the Intel 8086 CPU state.

    All integer register fields are unsigned:
      - 16-bit registers: 0–65535
      - 8-bit halves: derived via properties (al, ah, etc.)
      - memory: tuple of 1,048,576 unsigned bytes (0–255)
      - ports: tuples of 256 unsigned bytes each

    Use ``ax_signed`` / ``al_signed`` for signed arithmetic views.
    Use ``flags`` for the packed 16-bit FLAGS register.

    Attributes
    ----------
    ax, bx, cx, dx :
        16-bit general-purpose registers (unsigned).

    si, di, sp, bp :
        16-bit index / pointer registers (unsigned).

    cs, ds, ss, es :
        16-bit segment registers (unsigned).

    ip :
        16-bit instruction pointer (offset within CS).

    cf, pf, af, zf, sf, tf, if\\_, df, of :
        Individual flag booleans.  ``if_`` uses a trailing underscore because
        ``if`` is a Python keyword.

    halted :
        True after ``HLT`` executes.

    input_ports :
        256-byte tuple of I/O input port values.

    output_ports :
        256-byte tuple of I/O output port values.

    memory :
        1,048,576-byte flat tuple of the entire 1 MB address space.

    Examples
    --------
    >>> s = X86State(
    ...     ax=0x1234, bx=0, cx=0, dx=0,
    ...     si=0, di=0, sp=0, bp=0,
    ...     cs=0, ds=0, ss=0, es=0, ip=0,
    ...     cf=False, pf=False, af=False, zf=True,
    ...     sf=False, tf=False, if_=False, df=False, of=False,
    ...     halted=False,
    ...     input_ports=tuple([0]*256),
    ...     output_ports=tuple([0]*256),
    ...     memory=tuple([0]*1_048_576),
    ... )
    >>> s.ah
    18
    >>> s.al
    52
    >>> s.zf
    True
    """

    # ── General-purpose registers ────────────────────────────────────────────
    ax: int
    bx: int
    cx: int
    dx: int

    # ── Index / pointer registers ────────────────────────────────────────────
    si: int
    di: int
    sp: int
    bp: int

    # ── Segment registers ────────────────────────────────────────────────────
    cs: int
    ds: int
    ss: int
    es: int

    # ── Instruction pointer ──────────────────────────────────────────────────
    ip: int

    # ── Flags ────────────────────────────────────────────────────────────────
    cf: bool   # carry
    pf: bool   # parity
    af: bool   # auxiliary carry
    zf: bool   # zero
    sf: bool   # sign
    tf: bool   # trap
    if_: bool  # interrupt enable (trailing _ avoids Python keyword)
    df: bool   # direction
    of: bool   # overflow

    halted: bool

    # ── I/O ports ────────────────────────────────────────────────────────────
    input_ports: tuple[int, ...]   # 256 bytes
    output_ports: tuple[int, ...]  # 256 bytes

    # ── Memory ───────────────────────────────────────────────────────────────
    memory: tuple[int, ...]        # 1,048,576 bytes

    # ------------------------------------------------------------------
    # 8-bit half-register accessors
    # ------------------------------------------------------------------

    @property
    def al(self) -> int:
        """Low byte of AX (bits 7–0)."""
        return self.ax & _BYTE_MASK

    @property
    def ah(self) -> int:
        """High byte of AX (bits 15–8)."""
        return (self.ax >> 8) & _BYTE_MASK

    @property
    def bl(self) -> int:
        """Low byte of BX."""
        return self.bx & _BYTE_MASK

    @property
    def bh(self) -> int:
        """High byte of BX."""
        return (self.bx >> 8) & _BYTE_MASK

    @property
    def cl(self) -> int:
        """Low byte of CX."""
        return self.cx & _BYTE_MASK

    @property
    def ch(self) -> int:
        """High byte of CX."""
        return (self.cx >> 8) & _BYTE_MASK

    @property
    def dl(self) -> int:
        """Low byte of DX."""
        return self.dx & _BYTE_MASK

    @property
    def dh(self) -> int:
        """High byte of DX."""
        return (self.dx >> 8) & _BYTE_MASK

    # ------------------------------------------------------------------
    # Signed views
    # ------------------------------------------------------------------

    @property
    def ax_signed(self) -> int:
        """AX as a signed 16-bit integer (−32768…32767).

        Examples
        --------
        >>> # Use a minimal X86State for illustration
        >>> # ax=0x8000 is -32768 in two's complement
        >>> 0x8000 - 0x10000
        -32768
        """
        return self.ax if self.ax < 0x8000 else self.ax - 0x10000

    @property
    def al_signed(self) -> int:
        """AL as a signed 8-bit integer (−128…127)."""
        v = self.al
        return v if v < 0x80 else v - 0x100

    # ------------------------------------------------------------------
    # FLAGS register
    # ------------------------------------------------------------------

    @property
    def flags(self) -> int:
        """Pack all flags into the 16-bit x86 FLAGS register value.

        Layout (only named bits):
            bit  0: CF  bit  2: PF  bit  4: AF  bit  6: ZF
            bit  7: SF  bit  8: TF  bit  9: IF  bit 10: DF
            bit 11: OF  bit  1: always 1

        Examples
        --------
        >>> # ZF=1 only → bit 6 set + bit 1 always set = 0x0042
        >>> 0x0042
        66
        """
        return (
            (int(self.cf) << 0)
            | (1 << 1)               # bit 1 always 1
            | (int(self.pf) << 2)
            | (int(self.af) << 4)
            | (int(self.zf) << 6)
            | (int(self.sf) << 7)
            | (int(self.tf) << 8)
            | (int(self.if_) << 9)
            | (int(self.df) << 10)
            | (int(self.of) << 11)
        )
