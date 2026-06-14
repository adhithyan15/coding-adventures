"""State snapshot for the SPARC V8 simulator — Layer 07r.

SPARCState is a frozen dataclass capturing the complete CPU state at a point
in time.  Freezing it makes snapshots safe to pass around without copying.

SPARC V8 register organisation
────────────────────────────────
Unlike most architectures, SPARC does not have a flat register file.  Instead
registers are organised as *overlapping windows*:

  Global registers (r0–r7 / %g0–%g7):
    Always accessible in every window.  %g0 is hardwired to zero.

  Windowed registers (r8–r31 in the *current* window):
    Each window has three groups of 8:
      %o0–%o7  (r8–r15)  — "out" registers (arguments to callees)
      %l0–%l7  (r16–r23) — "local" registers (private to this window)
      %i0–%i7  (r24–r31) — "in" registers (arguments from callers)

    The key insight: the *ins* of window W are the *outs* of window W+1.
    SAVE rotates the window (CWP decrements); RESTORE restores it.

Physical register layout (this simulator, NWINDOWS=3):
    Physical index 0–7:         globals %g0–%g7
    Physical index 8–23:        window-0 outs+locals
    Physical index 24–39:       window-1 outs+locals
    Physical index 40–55:       window-2 outs+locals

    The "ins" of window W = outs of window (W+1) % NWINDOWS:
      Window 0 ins → physical 24–31 (window-1 outs)
      Window 1 ins → physical 40–47 (window-2 outs)
      Window 2 ins → physical  8–15 (window-0 outs)

Total physical registers: 8 + 3*16 = 56.

Special registers
─────────────────
  PC   — 32-bit program counter
  nPC  — 32-bit next-PC (for delay-slot model; advances with PC each step)
  PSR  — Processor Status Register:
           bit 23: N (negative)
           bit 22: Z (zero)
           bit 21: V (overflow)
           bit 20: C (carry)
           bits 4:0: CWP (current window pointer)
  Y    — 32-bit multiply/divide auxiliary register

Memory
──────
  65536 bytes flat, big-endian.  Programs load at address 0x0000.
"""

from __future__ import annotations

from dataclasses import dataclass

# ── Constants ──────────────────────────────────────────────────────────────────

MEM_SIZE   = 0x10000   # 64 KB flat address space
NUM_PHYS   = 56        # total physical registers (8 globals + 3*16 windowed)
NWINDOWS   = 3         # number of register windows

# Physical register base addresses for each window's outs+locals
WINDOW_BASE = [8 + w * 16 for w in range(NWINDOWS)]  # [8, 24, 40]

# HALT convention: ta 0  (Ticc, condition=always=8, trap#=0)
# Encoding: op=2, rd=8, op3=0x3A, rs1=0, i=1, simm13=0
# = (2<<30) | (8<<25) | (0x3A<<19) | (1<<13) = 0x91D02000
HALT_WORD: int = 0x91D0_2000

# Register aliases in the *virtual* r0–r31 numbering (within current window)
REG_G0  = 0   # hardwired zero
REG_O0  = 8   # first "out" register (argument 0 to callee)
REG_O6  = 14  # stack pointer (%sp)
REG_O7  = 15  # link register for CALL instruction
REG_L0  = 16  # first "local" register
REG_I0  = 24  # first "in" register (argument 0 from caller)
REG_I6  = 30  # frame pointer (%fp)
REG_I7  = 31  # return address − 8


def virt_to_phys(virt: int, cwp: int) -> int:
    """Map virtual register number (0–31) to physical register index.

    Virtual registers 0–7 are globals → physical 0–7 (no mapping needed).
    Virtual registers 8–23 are outs+locals of current window.
    Virtual registers 24–31 are ins of current window = outs of next window.

    Args:
        virt: virtual register number 0–31
        cwp:  current window pointer 0–NWINDOWS−1

    Returns:
        Index into the physical register array (0–55).

    Example (CWP=0, NWINDOWS=3):
        virt 0–7   → phys 0–7   (globals)
        virt 8–23  → phys 8–23  (window 0 outs+locals)
        virt 24–31 → phys 24–31 (window 1 outs = window 0 ins)
    """
    if virt < 8:
        return virt                               # global
    if virt < 24:
        return WINDOW_BASE[cwp] + (virt - 8)     # own outs + locals
    # ins = outs of the *next* window (caller's window)
    next_w = (cwp + 1) % NWINDOWS
    return WINDOW_BASE[next_w] + (virt - 24)


# ── State dataclass ────────────────────────────────────────────────────────────

@dataclass(frozen=True)
class SPARCState:
    """Immutable snapshot of the SPARC V8 CPU state.

    All integer register fields store unsigned 32-bit values (Python ints in
    [0, 2**32)).  Signed interpretation is done by the simulator helpers.

    Attributes:
        pc      — 32-bit program counter
        npc     — 32-bit next-PC (for delay-slot model; nPC = PC+4 in most steps)
        regs    — 56 physical registers, each unsigned 32-bit
        cwp     — current window pointer (0 to NWINDOWS−1)
        psr_n   — PSR Negative flag
        psr_z   — PSR Zero flag
        psr_v   — PSR oVerflow flag
        psr_c   — PSR Carry flag
        y       — Y register (multiply/divide auxiliary)
        memory  — 65536 bytes of flat big-endian memory
        halted  — True once HALT (ta 0) is executed
    """

    pc:     int
    npc:    int
    regs:   tuple[int, ...]   # 56 physical registers
    cwp:    int
    psr_n:  bool
    psr_z:  bool
    psr_v:  bool
    psr_c:  bool
    y:      int
    memory: tuple[int, ...]   # 65536 bytes
    halted: bool

    # ── Convenience register views (virtual, relative to current CWP) ─────────

    def _r(self, virt: int) -> int:
        """Read virtual register virt in the current window."""
        return self.regs[virt_to_phys(virt, self.cwp)]

    @property
    def g0(self) -> int: return 0
    @property
    def g1(self) -> int: return self._r(1)
    @property
    def g2(self) -> int: return self._r(2)
    @property
    def g3(self) -> int: return self._r(3)
    @property
    def g4(self) -> int: return self._r(4)
    @property
    def g5(self) -> int: return self._r(5)
    @property
    def g6(self) -> int: return self._r(6)
    @property
    def g7(self) -> int: return self._r(7)

    @property
    def o0(self) -> int: return self._r(8)
    @property
    def o1(self) -> int: return self._r(9)
    @property
    def o2(self) -> int: return self._r(10)
    @property
    def o3(self) -> int: return self._r(11)
    @property
    def o4(self) -> int: return self._r(12)
    @property
    def o5(self) -> int: return self._r(13)
    @property
    def sp(self) -> int: return self._r(14)    # %o6 = %sp
    @property
    def o7(self) -> int: return self._r(15)    # link register

    @property
    def l0(self) -> int: return self._r(16)
    @property
    def l1(self) -> int: return self._r(17)
    @property
    def l2(self) -> int: return self._r(18)
    @property
    def l3(self) -> int: return self._r(19)
    @property
    def l4(self) -> int: return self._r(20)
    @property
    def l5(self) -> int: return self._r(21)
    @property
    def l6(self) -> int: return self._r(22)
    @property
    def l7(self) -> int: return self._r(23)

    @property
    def i0(self) -> int: return self._r(24)
    @property
    def i1(self) -> int: return self._r(25)
    @property
    def i2(self) -> int: return self._r(26)
    @property
    def i3(self) -> int: return self._r(27)
    @property
    def i4(self) -> int: return self._r(28)
    @property
    def i5(self) -> int: return self._r(29)
    @property
    def fp(self) -> int: return self._r(30)    # %i6 = %fp
    @property
    def i7(self) -> int: return self._r(31)    # return address − 8
