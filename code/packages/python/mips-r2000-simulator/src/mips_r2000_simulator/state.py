"""State snapshot for the MIPS R2000 simulator — Layer 07q.

MIPSState is a frozen dataclass capturing the complete CPU state at a point
in time.  Freezing it makes snapshots safe to pass around without copying.

MIPS R2000 register file
─────────────────────────
  R0  ($zero)  — hardwired 0; writes are silently discarded
  R1  ($at)    — assembler temporary
  R2  ($v0)    — return value (also used for syscall number on Linux)
  R3  ($v1)    — return value (second word)
  R4  ($a0)    — argument 0
  R5  ($a1)    — argument 1
  R6  ($a2)    — argument 2
  R7  ($a3)    — argument 3
  R8–R15  ($t0–$t7) — caller-saved temporaries
  R16–R23 ($s0–$s7) — callee-saved registers
  R24–R25 ($t8–$t9) — more temporaries
  R26–R27 ($k0–$k1) — kernel reserved
  R28 ($gp) — global pointer
  R29 ($sp) — stack pointer
  R30 ($fp) — frame pointer (also called $s8)
  R31 ($ra) — return address (set by JAL / JALR)

Special registers
─────────────────
  HI — high 32 bits of MULT/MULTU; remainder of DIV/DIVU
  LO — low  32 bits of MULT/MULTU; quotient  of DIV/DIVU
  PC — 32-bit program counter (wraps modulo MEM_SIZE in simulator)

Memory
──────
  65536 bytes flat, big-endian.  Programs load at address 0x0000.
"""

from __future__ import annotations

from dataclasses import dataclass

# ── Constants ──────────────────────────────────────────────────────────────────

MEM_SIZE = 0x10000       # 64 KB flat address space for the simulator
NUM_REGS = 32            # 32 general-purpose registers

# HALT convention: SYSCALL (op=0, funct=0x0C) halts the simulator.
# Encoding: all-zero high 20 bits + funct = 0b001100 = 0x0C
# Big-endian bytes: [0x00, 0x00, 0x00, 0x0C]
HALT_OPCODE_WORD: int = 0x0000_000C   # SYSCALL instruction word

# Register aliases (by ABI convention)
REG_ZERO = 0   # always zero
REG_AT   = 1   # assembler temporary
REG_V0   = 2   # return value / syscall number
REG_V1   = 3   # second return value
REG_A0   = 4   # argument 0
REG_A1   = 5
REG_A2   = 6
REG_A3   = 7
REG_T0   = 8   # temporaries
REG_T1   = 9
REG_T2   = 10
REG_T3   = 11
REG_T4   = 12
REG_T5   = 13
REG_T6   = 14
REG_T7   = 15
REG_S0   = 16  # saved registers
REG_S1   = 17
REG_S2   = 18
REG_S3   = 19
REG_S4   = 20
REG_S5   = 21
REG_S6   = 22
REG_S7   = 23
REG_T8   = 24
REG_T9   = 25
REG_K0   = 26
REG_K1   = 27
REG_GP   = 28  # global pointer
REG_SP   = 29  # stack pointer
REG_FP   = 30  # frame pointer
REG_RA   = 31  # return address


# ── State dataclass ────────────────────────────────────────────────────────────

@dataclass(frozen=True)
class MIPSState:
    """Immutable snapshot of the MIPS R2000 CPU state.

    All integer fields are unsigned Python ints in the range [0, 2**32).
    Signed interpretation is done by the simulator and helper functions, not
    stored here.

    Attributes:
        pc     — 32-bit program counter (addresses modulo MEM_SIZE in sim)
        regs   — 32 general-purpose registers, each unsigned 32-bit
                 regs[0] is always 0 (R0/$zero)
        hi     — HI special register (unsigned 32-bit)
        lo     — LO special register (unsigned 32-bit)
        memory — 65536 bytes of flat big-endian memory
        halted — True once SYSCALL (our HALT sentinel) is executed
    """

    pc:     int
    regs:   tuple[int, ...]
    hi:     int
    lo:     int
    memory: tuple[int, ...]
    halted: bool

    # ── Convenience register properties ───────────────────────────────────────

    @property
    def zero(self) -> int:
        """$zero — always 0."""
        return 0

    @property
    def v0(self) -> int:
        """$v0 — return value / syscall number (R2)."""
        return self.regs[REG_V0]

    @property
    def v1(self) -> int:
        """$v1 — second return value (R3)."""
        return self.regs[REG_V1]

    @property
    def a0(self) -> int:
        """$a0 — argument 0 (R4)."""
        return self.regs[REG_A0]

    @property
    def a1(self) -> int:
        """$a1 — argument 1 (R5)."""
        return self.regs[REG_A1]

    @property
    def t0(self) -> int:
        """$t0 — temporary 0 (R8)."""
        return self.regs[REG_T0]

    @property
    def t1(self) -> int:
        """$t1 — temporary 1 (R9)."""
        return self.regs[REG_T1]

    @property
    def s0(self) -> int:
        """$s0 — saved register 0 (R16)."""
        return self.regs[REG_S0]

    @property
    def s1(self) -> int:
        """$s1 — saved register 1 (R17)."""
        return self.regs[REG_S1]

    @property
    def sp(self) -> int:
        """$sp — stack pointer (R29)."""
        return self.regs[REG_SP]

    @property
    def fp(self) -> int:
        """$fp — frame pointer (R30)."""
        return self.regs[REG_FP]

    @property
    def ra(self) -> int:
        """$ra — return address (R31)."""
        return self.regs[REG_RA]
