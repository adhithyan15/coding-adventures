"""
PowerPC 601 (1992) State Dataclass
====================================

The PowerPC 601 was IBM/Apple/Motorola's first PowerPC chip, launched in 1992
and powering the original Power Macintosh line (1994).  It brought RISC
principles — large register file, load/store architecture, fixed-size
instructions — to the consumer desktop market.

Register set at a glance
------------------------
  GPR0–GPR31   32 × 32-bit general-purpose registers
               (GPR0 is treated as 0 in effective-address calculations,
               but holds a real value for non-address operations)
  LR           Link Register   — saved return address (32-bit)
  CTR          Count Register  — branch countdown / indirect call (32-bit)
  XER          Fixed-Point Exception Register (32-bit):
                 bit 0 = SO (Summary Overflow)
                 bit 1 = OV (Overflow)
                 bit 2 = CA (Carry)
  CR           Condition Register (32-bit), divided into 8 four-bit fields:
                 CR0 (bits 0–3, MSB side) … CR7 (bits 28–31, LSB side).
                 Each field: [LT, GT, EQ, SO]
  CIA          Current Instruction Address (the program counter), 32-bit,
               always a multiple of 4.
"""

from __future__ import annotations

from dataclasses import dataclass

# ── Architecture constants ─────────────────────────────────────────────────────

MASK32: int = 0xFFFF_FFFF       # 32-bit unsigned mask
SIGN32: int = 0x8000_0000       # sign bit of a 32-bit quantity
MEM_SIZE: int = 65536           # 64 KiB flat byte-addressed memory
NUM_GPRS: int = 32              # GPR0 through GPR31

# XER bit positions (bit 0 = MSB in PowerPC convention, bit 31 = LSB)
# Stored as a Python integer: bit 31 of XER maps to Python bit weight 2^31.
XER_SO: int = 1 << 31   # Summary Overflow  (bit 0 in PPC, bit 31 in Python int)
XER_OV: int = 1 << 30   # Overflow          (bit 1 in PPC, bit 30 in Python int)
XER_CA: int = 1 << 29   # Carry             (bit 2 in PPC, bit 29 in Python int)

# CR bit positions within the 32-bit CR integer.
# CR0 occupies the top nibble (bits [31:28] of the Python integer).
# CR0.LT = bit 31, CR0.GT = bit 30, CR0.EQ = bit 29, CR0.SO = bit 28.
# CR bit BI (0-indexed from MSB) corresponds to Python bit weight 2^(31-BI).
CR_LT_SHIFT: int = 31   # CR0.LT in Python bit weight
CR_GT_SHIFT: int = 30   # CR0.GT
CR_EQ_SHIFT: int = 29   # CR0.EQ
CR_SO_SHIFT: int = 28   # CR0.SO (CR0 field)


def sext16(v: int) -> int:
    """Sign-extend a 16-bit value to a Python signed integer."""
    v = v & 0xFFFF
    if v & 0x8000:
        return v - 0x10000
    return v


def sext32(v: int) -> int:
    """Reinterpret a masked 32-bit value as a signed Python integer."""
    v = v & MASK32
    if v & SIGN32:
        return v - 0x1_0000_0000
    return v


# ── Immutable state snapshot ───────────────────────────────────────────────────


@dataclass(frozen=True)
class PowerPC601State:
    """
    Complete, immutable snapshot of the PowerPC 601 simulator at one moment.

    Fields
    ------
    cia     Current Instruction Address (program counter), 32-bit, multiple of 4.
    gpr     32 × 32-bit general-purpose registers (GPR0–GPR31).
            GPR0 participates in EA = 0 for address calculations when rA=0,
            but otherwise holds a real value.
    lr      Link Register — set by bl/bctrl; used by blr.
    ctr     Count Register — decremented by bdnz-style branches; indirect target.
    xer     XER register: [SO, OV, CA, ...] in bits [31, 30, 29, ...].
    cr      Condition Register: 8 × 4-bit fields, CR0 in the MSB nibble.
    memory  65536 bytes stored as a tuple of Python ints (each 0–255).
    halted  True once the simulator fetches the all-zero HALT word.
    """

    cia: int                    # current instruction address, 32-bit
    gpr: tuple[int, ...]        # 32 × 32-bit GPRs
    lr: int                     # link register
    ctr: int                    # count register
    xer: int                    # XER: SO/OV/CA and byte count
    cr: int                     # condition register (8 × 4-bit fields)
    memory: tuple[int, ...]     # 65 536 bytes
    halted: bool

    # ── Convenience properties for registers ─────────────────────────────────

    @property
    def r0(self) -> int: return self.gpr[0]
    @property
    def r1(self) -> int: return self.gpr[1]
    @property
    def r2(self) -> int: return self.gpr[2]
    @property
    def r3(self) -> int: return self.gpr[3]
    @property
    def r4(self) -> int: return self.gpr[4]
    @property
    def r5(self) -> int: return self.gpr[5]
    @property
    def r6(self) -> int: return self.gpr[6]
    @property
    def r7(self) -> int: return self.gpr[7]
    @property
    def r8(self) -> int: return self.gpr[8]
    @property
    def r9(self) -> int: return self.gpr[9]
    @property
    def r10(self) -> int: return self.gpr[10]
    @property
    def r11(self) -> int: return self.gpr[11]
    @property
    def r12(self) -> int: return self.gpr[12]
    @property
    def r13(self) -> int: return self.gpr[13]
    @property
    def r14(self) -> int: return self.gpr[14]
    @property
    def r15(self) -> int: return self.gpr[15]
    @property
    def r16(self) -> int: return self.gpr[16]
    @property
    def r17(self) -> int: return self.gpr[17]
    @property
    def r18(self) -> int: return self.gpr[18]
    @property
    def r19(self) -> int: return self.gpr[19]
    @property
    def r20(self) -> int: return self.gpr[20]
    @property
    def r21(self) -> int: return self.gpr[21]
    @property
    def r22(self) -> int: return self.gpr[22]
    @property
    def r23(self) -> int: return self.gpr[23]
    @property
    def r24(self) -> int: return self.gpr[24]
    @property
    def r25(self) -> int: return self.gpr[25]
    @property
    def r26(self) -> int: return self.gpr[26]
    @property
    def r27(self) -> int: return self.gpr[27]
    @property
    def r28(self) -> int: return self.gpr[28]
    @property
    def r29(self) -> int: return self.gpr[29]
    @property
    def r30(self) -> int: return self.gpr[30]
    @property
    def r31(self) -> int: return self.gpr[31]

    # ── CR field helpers ──────────────────────────────────────────────────────

    def cr_field(self, n: int) -> int:
        """Return the 4-bit CR field n (0=CR0, 7=CR7)."""
        return (self.cr >> (28 - n * 4)) & 0xF

    @property
    def cr0(self) -> int:
        """CR0 nibble: [LT, GT, EQ, SO] from MSB to LSB."""
        return (self.cr >> 28) & 0xF

    @property
    def cr0_lt(self) -> bool:
        """CR0 Less-Than bit."""
        return bool((self.cr >> 31) & 1)

    @property
    def cr0_gt(self) -> bool:
        """CR0 Greater-Than bit."""
        return bool((self.cr >> 30) & 1)

    @property
    def cr0_eq(self) -> bool:
        """CR0 Equal bit."""
        return bool((self.cr >> 29) & 1)

    @property
    def cr0_so(self) -> bool:
        """CR0 Summary-Overflow bit."""
        return bool((self.cr >> 28) & 1)


def make_initial_state() -> PowerPC601State:
    """Return the power-on / reset state: all registers and memory zeroed, CIA=0."""
    return PowerPC601State(
        cia=0,
        gpr=tuple(0 for _ in range(NUM_GPRS)),
        lr=0,
        ctr=0,
        xer=0,
        cr=0,
        memory=tuple(0 for _ in range(MEM_SIZE)),
        halted=False,
    )
