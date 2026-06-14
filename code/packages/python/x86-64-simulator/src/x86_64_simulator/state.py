"""x86-64 CPU state — immutable snapshot produced by each simulation step.

The x86-64 (AMD64) register file in 64-bit long mode:

    GPR indices (16 registers, 64-bit each)
    ────────────────────────────────────────
    0  RAX   1  RCX   2  RDX   3  RBX
    4  RSP   5  RBP   6  RSI   7  RDI
    8  R8    9  R9   10  R10  11  R11
   12  R12  13  R13  14  R14  15  R15

    RIP (instruction pointer) and RFLAGS are separate fields.

    RFLAGS bit positions (only these five are tracked):
    ────────────────────────────────────────────────────
    Bit  0   CF  Carry flag
    Bit  2   PF  Parity flag  (low byte of result has even number of 1-bits)
    Bit  6   ZF  Zero flag
    Bit  7   SF  Sign flag
    Bit 11   OF  Overflow flag

Memory: 64 KiB byte-addressed flat array (indices 0x0000–0xFFFF).
Addresses wrap modulo MEM_SIZE (65 536).

HALT sentinel: opcode 0xF4 (HLT) halts the simulator.
"""

from __future__ import annotations

from dataclasses import dataclass

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

MEM_SIZE: int = 65_536          # 64 KiB of flat memory
MASK64: int = 0xFFFF_FFFF_FFFF_FFFF  # 64-bit unsigned mask
MASK32: int = 0xFFFF_FFFF
MASK16: int = 0xFFFF
MASK8:  int = 0xFF

# RFLAGS bit positions
CF_BIT = 0
PF_BIT = 2
ZF_BIT = 6
SF_BIT = 7
OF_BIT = 11

# Register indices
RAX = 0; RCX = 1; RDX = 2; RBX = 3
RSP = 4; RBP = 5; RSI = 6; RDI = 7
R8  = 8; R9  = 9; R10 = 10; R11 = 11
R12 = 12; R13 = 13; R14 = 14; R15 = 15


# ---------------------------------------------------------------------------
# Immutable state snapshot
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class X86_64State:
    """Immutable snapshot of x86-64 CPU state after each instruction.

    Fields
    ------
    pc : int
        Current value of RIP (instruction pointer).  Points to the *next*
        instruction to execute (already advanced by the just-executed instr).
    gpr : tuple[int, ...]
        16-element tuple of 64-bit unsigned register values.
        Index order: RAX(0) RCX(1) RDX(2) RBX(3) RSP(4) RBP(5) RSI(6)
                     RDI(7) R8(8) R9(9) R10(10) R11(11) R12(12) R13(13)
                     R14(14) R15(15).
    rflags : int
        Condition flags register (only CF/PF/ZF/SF/OF bits are meaningful).
    memory : tuple[int, ...]
        MEM_SIZE bytes (65 536).  Little-endian storage.
    halted : bool
        True after HLT (0xF4) has been executed or max_steps exceeded.
    """

    pc:     int
    gpr:    tuple[int, ...]  # 16 elements
    rflags: int
    memory: tuple[int, ...]  # MEM_SIZE elements
    halted: bool

    # ------------------------------------------------------------------
    # Register convenience properties
    # ------------------------------------------------------------------

    @property
    def rax(self) -> int: return self.gpr[RAX]

    @property
    def rcx(self) -> int: return self.gpr[RCX]

    @property
    def rdx(self) -> int: return self.gpr[RDX]

    @property
    def rbx(self) -> int: return self.gpr[RBX]

    @property
    def rsp(self) -> int: return self.gpr[RSP]

    @property
    def rbp(self) -> int: return self.gpr[RBP]

    @property
    def rsi(self) -> int: return self.gpr[RSI]

    @property
    def rdi(self) -> int: return self.gpr[RDI]

    @property
    def r8(self)  -> int: return self.gpr[R8]

    @property
    def r9(self)  -> int: return self.gpr[R9]

    @property
    def r10(self) -> int: return self.gpr[R10]

    @property
    def r11(self) -> int: return self.gpr[R11]

    @property
    def r12(self) -> int: return self.gpr[R12]

    @property
    def r13(self) -> int: return self.gpr[R13]

    @property
    def r14(self) -> int: return self.gpr[R14]

    @property
    def r15(self) -> int: return self.gpr[R15]

    # ------------------------------------------------------------------
    # Flag convenience properties
    # ------------------------------------------------------------------

    @property
    def cf(self) -> bool: return bool((self.rflags >> CF_BIT) & 1)

    @property
    def pf(self) -> bool: return bool((self.rflags >> PF_BIT) & 1)

    @property
    def zf(self) -> bool: return bool((self.rflags >> ZF_BIT) & 1)

    @property
    def sf(self) -> bool: return bool((self.rflags >> SF_BIT) & 1)

    @property
    def of(self) -> bool: return bool((self.rflags >> OF_BIT) & 1)

    # ------------------------------------------------------------------
    # Memory read helpers (little-endian)
    # ------------------------------------------------------------------

    def read8(self, addr: int) -> int:
        """Read one byte from memory at *addr* (wraps modulo MEM_SIZE)."""
        return self.memory[addr & (MEM_SIZE - 1)]

    def read16(self, addr: int) -> int:
        """Read 16-bit little-endian word from *addr*."""
        a = addr & (MEM_SIZE - 1)
        return self.memory[a] | (self.memory[(a + 1) & (MEM_SIZE - 1)] << 8)

    def read32(self, addr: int) -> int:
        """Read 32-bit little-endian dword from *addr*."""
        a = addr & (MEM_SIZE - 1)
        v = 0
        for i in range(4):
            v |= self.memory[(a + i) & (MEM_SIZE - 1)] << (8 * i)
        return v

    def read64(self, addr: int) -> int:
        """Read 64-bit little-endian qword from *addr*."""
        a = addr & (MEM_SIZE - 1)
        v = 0
        for i in range(8):
            v |= self.memory[(a + i) & (MEM_SIZE - 1)] << (8 * i)
        return v
