"""Cross-validation: gate-level simulator vs behavioral simulator.

Every program below is run on both the behavioral MIPSSimulator and the
gate-level MIPSR2000GateLevelSimulator.  Final register state, HI/LO,
and PC must match exactly.
"""

import struct

from mips_r2000_simulator import MIPSSimulator as BehavioralSim

from mips_r2000_gatelevel import MIPSR2000GateLevelSimulator as GateSim


def w(word: int) -> bytes:
    """Encode a 32-bit word as big-endian bytes."""
    return struct.pack(">I", word)


HALT = w(0x0000_000C)  # SYSCALL


def r(rs: int, rt: int, rd: int, shamt: int, funct: int) -> bytes:
    """Encode R-type instruction."""
    return w((0 << 26) | (rs << 21) | (rt << 16) | (rd << 11) | (shamt << 6) | funct)


def i_instr(op: int, rs: int, rt: int, imm: int) -> bytes:
    """Encode I-type instruction."""
    return w((op << 26) | (rs << 21) | (rt << 16) | (imm & 0xFFFF))


def j_instr(op: int, target: int) -> bytes:
    """Encode J-type instruction."""
    return w((op << 26) | (target & 0x3FF_FFFF))


def cross_validate(prog: bytes, max_steps: int = 50_000) -> None:
    """Run program on both simulators and assert identical final state."""
    b_sim = BehavioralSim()
    b_result = b_sim.execute(prog, max_steps=max_steps)
    bs = b_result.final_state

    g_sim = GateSim()
    g_result = g_sim.execute(prog, max_steps=max_steps)
    gs = g_result.final_state

    assert bs.regs == gs.regs, (
        f"Registers differ:\n  behavioral: {bs.regs}\n  gate-level: {gs.regs}"
    )
    assert bs.hi == gs.hi, f"HI differs: behavioral={bs.hi}, gate={gs.hi}"
    assert bs.lo == gs.lo, f"LO differs: behavioral={bs.lo}, gate={gs.lo}"
    assert bs.halted == gs.halted, (
        f"halted differs: behavioral={bs.halted}, gate={gs.halted}"
    )


# ── Test programs ──────────────────────────────────────────────────────────────


class TestEquivalence:
    def test_arithmetic_sum_loop(self):
        """Sum 1..10 using ADDIU + BNE + SYSCALL.

        After execution: $v0 = 55 (sum of 1..10).

        Register usage:
          $t0 (R8)  = counter (1..10)
          $v0 (R2)  = accumulator
          $t1 (R9)  = limit (11)

        Assembly:
          ADDIU $t1, $zero, 11  # limit = 11
          ADDIU $t0, $zero, 1   # counter = 1
        loop:
          ADDU  $v0, $v0, $t0   # sum += counter
          ADDIU $t0, $t0, 1     # counter++
          BNE   $t0, $t1, loop  # if counter != 11, loop (offset = -3)
          SYSCALL
        """
        prog = (
            i_instr(0x09, 0, 9, 11)    # ADDIU $t1, $zero, 11
            + i_instr(0x09, 0, 8, 1)   # ADDIU $t0, $zero, 1
            + r(0, 8, 2, 0, 0x21)      # ADDU $v0, $zero, $t0 ... wait, ADDU $v0, $v0, $t0
            # Actually: ADDU rd=2, rs=2, rt=8, funct=0x21
            + r(2, 8, 2, 0, 0x21)      # ADDU $v0, $v0, $t0
            + i_instr(0x09, 8, 8, 1)   # ADDIU $t0, $t0, 1
            # BNE $t0, $t1, -3 words = offset=-3
            + i_instr(0x05, 8, 9, 0xFFFD)  # BNE $t0, $t1, -3
            + HALT
        )
        cross_validate(prog)

    def test_bitwise_ops(self):
        """AND/OR/XOR/NOR on known values.

        $t0 = 0xFF00FF00, $t1 = 0x00FF00FF
        and → $s0, or → $s1, xor → $s2, nor → $s3
        """
        # Load 0xFF00FF00 into $t0
        lui_hi = i_instr(0x0F, 0, 8, 0xFF00)   # LUI $t0, 0xFF00
        ori_lo = i_instr(0x0D, 8, 8, 0xFF00)   # ORI $t0, $t0, 0xFF00

        # Load 0x00FF00FF into $t1
        lui_hi2 = i_instr(0x0F, 0, 9, 0x00FF)  # LUI $t1, 0x00FF
        ori_lo2 = i_instr(0x0D, 9, 9, 0x00FF)  # ORI $t1, $t1, 0x00FF

        prog = (
            lui_hi + ori_lo + lui_hi2 + ori_lo2
            + r(8, 9, 16, 0, 0x24)   # AND $s0, $t0, $t1
            + r(8, 9, 17, 0, 0x25)   # OR  $s1, $t0, $t1
            + r(8, 9, 18, 0, 0x26)   # XOR $s2, $t0, $t1
            + r(8, 9, 19, 0, 0x27)   # NOR $s3, $t0, $t1
            + HALT
        )
        cross_validate(prog)

    def test_shift_ops(self):
        """SLL/SRL/SRA by various amounts."""
        prog = (
            i_instr(0x09, 0, 8, 1)          # ADDIU $t0, $zero, 1
            + r(0, 8, 9, 4, 0x00)            # SLL $t1, $t0, 4   → 16
            + r(0, 9, 10, 2, 0x02)           # SRL $t2, $t1, 2   → 4
            + r(0, 10, 11, 1, 0x03)          # SRA $t3, $t2, 1   → 2
            # Load negative value for SRA sign-fill test
            + i_instr(0x0F, 0, 12, 0x8000)   # LUI $t4, 0x8000
            + r(0, 12, 13, 4, 0x03)          # SRA $t5, $t4, 4
            + HALT
        )
        cross_validate(prog)

    def test_mult_div(self):
        """MULT and DIV on small values, compare HI:LO."""
        prog = (
            i_instr(0x09, 0, 8, 6)     # ADDIU $t0, $zero, 6
            + i_instr(0x09, 0, 9, 7)   # ADDIU $t1, $zero, 7
            + r(8, 9, 0, 0, 0x18)      # MULT $t0, $t1  → HI=0, LO=42
            + r(0, 0, 2, 0, 0x12)      # MFLO $v0
            + i_instr(0x09, 0, 8, 42)  # ADDIU $t0, $zero, 42
            + i_instr(0x09, 0, 9, 7)   # ADDIU $t1, $zero, 7
            + r(8, 9, 0, 0, 0x1B)      # DIVU $t0, $t1 → LO=6, HI=0
            + r(0, 0, 3, 0, 0x12)      # MFLO $v1
            + HALT
        )
        cross_validate(prog)

    def test_slt_sltu(self):
        """SLT/SLTU comparisons."""
        prog = (
            i_instr(0x09, 0, 8, 3)              # ADDIU $t0, $zero, 3
            + i_instr(0x09, 0, 9, 7)            # ADDIU $t1, $zero, 7
            + r(8, 9, 16, 0, 0x2A)              # SLT $s0, $t0, $t1  → 1 (3 < 7)
            + r(9, 8, 17, 0, 0x2A)              # SLT $s1, $t1, $t0  → 0 (7 > 3)
            # unsigned: $t2 = 0xFFFFFFFF
            + i_instr(0x0F, 0, 10, 0xFFFF)      # LUI $t2, 0xFFFF
            + i_instr(0x0D, 10, 10, 0xFFFF)     # ORI $t2, $t2, 0xFFFF
            + r(8, 10, 18, 0, 0x2B)             # SLTU $s2, $t0, $t2  → 1 (3 < 0xFFFFFFFF)
            + r(10, 8, 19, 0, 0x2B)             # SLTU $s3, $t2, $t0  → 0
            + HALT
        )
        cross_validate(prog)

    def test_function_call(self):
        """JAL + JR $ra — return address preserved.

        Layout (addresses in words):
          0: JAL func (target = word 3, i.e. byte 12)
          1: ADDIU $v0, $v0, 1  ← return point
          2: SYSCALL
          3: ADDIU $v0, $zero, 99  ← func start
          4: JR $ra
        """
        # Byte addresses: 0=JAL, 4=ADDIU, 8=SYSCALL, 12=func, 16=JR
        # JAL target = byte_addr >> 2 = 12 >> 2 = 3
        jal_word = j_instr(0x03, 3)  # JAL to word 3
        prog = (
            jal_word                          # 0: JAL func
            + i_instr(0x09, 2, 2, 1)          # 4: ADDIU $v0, $v0, 1 (100)
            + HALT                            # 8: SYSCALL
            + i_instr(0x09, 0, 2, 99)         # 12: ADDIU $v0, $zero, 99
            + r(31, 0, 0, 0, 0x08)            # 16: JR $ra
        )
        cross_validate(prog)

    def test_nop(self):
        """NOP instructions are handled identically."""
        prog = (
            w(0x0000_0000)  # NOP
            + w(0x0000_0000)  # NOP
            + HALT
        )
        cross_validate(prog)

    def test_memory_lw_sw(self):
        """LW/SW round-trip: store then reload."""
        # Store 0xDEAD into memory at address 0x100, then load it back
        prog = (
            i_instr(0x0F, 0, 8, 0xDEAD)    # LUI $t0, 0xDEAD
            # Store $t0 at mem[0x100]
            + i_instr(0x2B, 0, 8, 0x100)    # SW $t0, 0x100($zero)
            # Load it back into $t1
            + i_instr(0x23, 0, 9, 0x100)    # LW $t1, 0x100($zero)
            + HALT
        )
        cross_validate(prog)
