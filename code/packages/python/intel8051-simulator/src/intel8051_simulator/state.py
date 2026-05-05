"""I8051State — frozen snapshot of the Intel 8051 CPU state.

The 8051 has a Harvard architecture: code memory, internal RAM (which includes
Special Function Registers at 0x80–0xFF), and external data memory are all
separate address spaces.  The state captures all three along with the
architectural registers.

Memory layout of internal RAM (iram, 256 bytes):
    0x00–0x1F  — 4 register banks (each 8 bytes, R0–R7)
    0x20–0x2F  — bit-addressable area (128 individually-addressable bits)
    0x30–0x7F  — general scratchpad RAM
    0x80–0xFF  — Special Function Registers (SFRs)

PSW (Program Status Word) bit layout:
    Bit 7 = CY  (carry)
    Bit 6 = AC  (auxiliary carry, bit 3→4)
    Bit 5 = F0  (user flag)
    Bit 4 = RS1 (register bank select, high bit)
    Bit 3 = RS0 (register bank select, low bit)
    Bit 2 = OV  (overflow / division error)
    Bit 1 = —   (reserved, always 0)
    Bit 0 = P   (parity of ACC, even parity)
"""

from __future__ import annotations

from dataclasses import dataclass

# ── Constants ──────────────────────────────────────────────────────────────────

CODE_SIZE  = 65536   # Harvard code-memory space (64 KB)
XDATA_SIZE = 65536   # External data-memory space (64 KB)
IRAM_SIZE  = 256     # Internal RAM + SFR space (256 bytes)

# SFR addresses (absolute, i.e. direct-address indices into iram)
SFR_P0   = 0x80
SFR_SP   = 0x81
SFR_DPL  = 0x82
SFR_DPH  = 0x83
SFR_PCON = 0x87
SFR_TCON = 0x88
SFR_TMOD = 0x89
SFR_TL0  = 0x8A
SFR_TL1  = 0x8B
SFR_TH0  = 0x8C
SFR_TH1  = 0x8D
SFR_P1   = 0x90
SFR_SCON = 0x98
SFR_SBUF = 0x99
SFR_P2   = 0xA0
SFR_IE   = 0xA8
SFR_P3   = 0xB0
SFR_IP   = 0xB8
SFR_PSW  = 0xD0
SFR_ACC  = 0xE0
SFR_B    = 0xF0

# Reset values for port latches (all 0xFF on real hardware)
PORT_RESET = 0xFF

# PSW bit masks
PSW_CY  = 0x80
PSW_AC  = 0x40
PSW_F0  = 0x20
PSW_RS1 = 0x10
PSW_RS0 = 0x08
PSW_OV  = 0x04
PSW_P   = 0x01

# HALT sentinel opcode (0xA5 is undefined/reserved on real 8051)
HALT_OPCODE = 0xA5


# ── State dataclass ────────────────────────────────────────────────────────────

@dataclass(frozen=True)
class I8051State:
    """Immutable snapshot of the full 8051 architectural state.

    All mutable simulator state is copied into Python immutables (int, tuple)
    so that a stored snapshot is never affected by subsequent execution.
    """

    pc:     int               # 16-bit program counter
    iram:   tuple[int, ...]   # 256 bytes: lower RAM + SFRs at 0x80+
    xdata:  tuple[int, ...]   # 65536 bytes of external data memory
    code:   tuple[int, ...]   # 65536 bytes of code memory (Harvard)
    halted: bool

    # Convenience: expose architectural registers as direct properties
    # rather than requiring callers to index iram[SFR_xxx].

    @property
    def acc(self) -> int:
        """Accumulator (SFR 0xE0)."""
        return self.iram[SFR_ACC]

    @property
    def b(self) -> int:
        """B register (SFR 0xF0)."""
        return self.iram[SFR_B]

    @property
    def sp(self) -> int:
        """Stack pointer (SFR 0x81)."""
        return self.iram[SFR_SP]

    @property
    def dptr(self) -> int:
        """16-bit data pointer = DPH:DPL (SFRs 0x83:0x82)."""
        return (self.iram[SFR_DPH] << 8) | self.iram[SFR_DPL]

    @property
    def psw(self) -> int:
        """Program status word (SFR 0xD0)."""
        return self.iram[SFR_PSW]

    @property
    def cy(self) -> bool:
        """Carry flag (PSW bit 7)."""
        return bool(self.iram[SFR_PSW] & PSW_CY)

    @property
    def ac(self) -> bool:
        """Auxiliary carry flag (PSW bit 6)."""
        return bool(self.iram[SFR_PSW] & PSW_AC)

    @property
    def ov(self) -> bool:
        """Overflow / division-error flag (PSW bit 2)."""
        return bool(self.iram[SFR_PSW] & PSW_OV)

    @property
    def parity(self) -> bool:
        """Even-parity flag of ACC (PSW bit 0)."""
        return bool(self.iram[SFR_PSW] & PSW_P)

    @property
    def bank(self) -> int:
        """Active register bank (0–3) from PSW RS1:RS0."""
        return (self.iram[SFR_PSW] >> 3) & 0x3
