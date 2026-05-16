"""Full-program tests for the gate-level MIPS R2000 simulator.

Each test encodes a MIPS program as big-endian bytes, runs it on the
gate-level simulator, and checks the final register / memory state.
"""

import struct

from mips_r2000_gatelevel import MIPSR2000GateLevelSimulator


def w(word: int) -> bytes:
    """Pack a 32-bit unsigned value as big-endian bytes."""
    return struct.pack(">I", word)


HALT = w(0x0000_000C)  # SYSCALL — halts the simulator


def r(rs: int, rt: int, rd: int, shamt: int, funct: int) -> bytes:
    """Encode R-type instruction word."""
    return w((0 << 26) | (rs << 21) | (rt << 16) | (rd << 11) | (shamt << 6) | funct)


def i_instr(op: int, rs: int, rt: int, imm: int) -> bytes:
    """Encode I-type instruction word (imm16 is masked to 16 bits)."""
    return w((op << 26) | (rs << 21) | (rt << 16) | (imm & 0xFFFF))


def j_instr(op: int, target: int) -> bytes:
    """Encode J-type instruction word."""
    return w((op << 26) | (target & 0x3FF_FFFF))


def run(prog: bytes, max_steps: int = 100_000):
    """Run a program and return the final MIPSState."""
    sim = MIPSR2000GateLevelSimulator()
    result = sim.execute(prog, max_steps=max_steps)
    return result.final_state


# ── Test 1: Sum 1..10 ──────────────────────────────────────────────────────────


class TestSum1To10:
    """ADDIU-based loop summing 1+2+...+10 = 55 into $v0."""

    def _build_prog(self) -> bytes:
        # Registers: $t0=R8 (counter), $t1=R9 (limit=11), $v0=R2 (sum)
        # Instruction layout (byte offsets):
        #   0:  ADDIU $t1, $zero, 11
        #   4:  ADDIU $t0, $zero, 1
        #   8:  ADDU  $v0, $v0, $t0      ← loop start
        #  12:  ADDIU $t0, $t0, 1
        #  16:  BNE   $t0, $t1, -3       ← -3 words = back to byte 8
        #  20:  SYSCALL
        return (
            i_instr(0x09, 0, 9, 11)         # ADDIU $t1, $zero, 11
            + i_instr(0x09, 0, 8, 1)        # ADDIU $t0, $zero, 1
            + r(2, 8, 2, 0, 0x21)           # ADDU $v0, $v0, $t0
            + i_instr(0x09, 8, 8, 1)        # ADDIU $t0, $t0, 1
            + i_instr(0x05, 8, 9, 0xFFFD)   # BNE $t0, $t1, -3
            + HALT
        )

    def test_sum_result(self):
        state = run(self._build_prog())
        assert state.regs[2] == 55  # $v0 = sum

    def test_counter_ends_at_11(self):
        state = run(self._build_prog())
        assert state.regs[8] == 11  # $t0 = 11 when loop exits

    def test_halted(self):
        state = run(self._build_prog())
        assert state.halted


# ── Test 2: Factorial 5 ───────────────────────────────────────────────────────


class TestFactorial5:
    """Compute 5! = 120 using MULTU + MFLO loop.

    $t0 = counter (5 down to 1), $v0 = result (starts 1).
    MULTU $v0, $t0; MFLO $v0; ADDIU $t0, $t0, -1; BGTZ $t0, loop
    """

    def _build_prog(self) -> bytes:
        # byte offsets:
        #  0:  ADDIU $v0, $zero, 1     (result = 1)
        #  4:  ADDIU $t0, $zero, 5     (counter = 5)
        #  8:  MULTU $v0, $t0          ← loop
        # 12:  MFLO  $v0
        # 16:  ADDIU $t0, $t0, -1
        # 20:  BGTZ  $t0, -4           (back to byte 8 = current_pc+offset*4)
        # 24:  SYSCALL
        # At instruction 20 (BGTZ), PC is already 24.  offset*4 = 8-24 = -16 → offset=-4
        return (
            i_instr(0x09, 0, 2, 1)          # ADDIU $v0, $zero, 1
            + i_instr(0x09, 0, 8, 5)        # ADDIU $t0, $zero, 5
            + r(2, 8, 0, 0, 0x19)           # MULTU $v0, $t0
            + r(0, 0, 2, 0, 0x12)           # MFLO $v0
            + i_instr(0x09, 8, 8, 0xFFFF)   # ADDIU $t0, $t0, -1
            + i_instr(0x07, 8, 0, 0xFFFC)   # BGTZ $t0, -4
            + HALT
        )

    def test_factorial_result(self):
        state = run(self._build_prog())
        assert state.regs[2] == 120  # 5! = 120

    def test_counter_at_zero(self):
        state = run(self._build_prog())
        assert state.regs[8] == 0


# ── Test 3: GCD (Euclidean) ───────────────────────────────────────────────────


class TestGCD:
    """GCD(48, 18) = 6 using SUBU / BNE loop.

    Classic Euclidean algorithm (subtract version):
        while a != b:
            if a > b: a -= b
            else:     b -= a
        result = a
    """

    def _build_prog(self) -> bytes:
        # $t0=a=48, $t1=b=18
        # loop_start (byte 8):
        #   BEQ $t0, $t1, done     (offset = +4 words from here = skip to done)
        #   SLT $t2, $t0, $t1      (a < b?)
        #   BNE $t2, $zero, do_sub_b (offset = +2 to byte 24)
        #   SUBU $t0, $t0, $t1     (a -= b)
        #   J loop_start
        #   SUBU $t1, $t1, $t0     (b -= a)
        #   J loop_start
        # done (byte 40):
        #   ADDU $v0, $t0, $zero   (return a in $v0)
        #   SYSCALL
        #
        # Byte layout:
        #  0:  ADDIU $t0, $zero, 48
        #  4:  ADDIU $t1, $zero, 18
        #  8:  BEQ $t0, $t1, done (+5 = byte 28? let me recalc)
        # Actually let me keep it simple with a straight unrolled version

        # Simple loop: if a == b, done.
        # else if a > b: a-=b, else b-=a, repeat
        # We'll use SLTU to detect a < b

        # byte layout:
        #  0: ADDIU $t0, $zero, 48
        #  4: ADDIU $t1, $zero, 18
        #  8: BEQ $t0, $t1, +5  → jumps to byte 8+(5+1)*4=32 when equal → done at 32
        #
        # Wait, BEQ offset: branch target = PC_after_fetch + offset*4
        # At byte 8, after fetch, PC=12. offset=5 → 12+20=32. Let's check.
        #
        # Bytes:
        #  0:  ADDIU $t0, $zero, 48
        #  4:  ADDIU $t1, $zero, 18
        # loop (byte 8):
        #  8:  BEQ $t0, $t1, +5 → PC=12, target=12+20=32 → done
        # 12:  SLTU $t2, $t0, $t1  (t2=1 if a<b unsigned)
        # 16:  BNE $t2, $zero, +1  → PC=20, target=20+4=24 (sub_b)
        # 20:  SUBU $t0, $t0, $t1  (a -= b)
        # 24:  J loop (byte 8 = word 2)   target26 = 2
        # 28:  SUBU $t1, $t1, $t0  (b -= a) ← sub_b
        # 32:  J loop (byte 8)
        # Wait, after sub_b we also need to loop.
        # Let me redo: at byte 28 sub_b then fallthrough to byte 32 = J loop
        # Actually byte 24 is J loop (to byte 8), byte 28 is sub_b, byte 32 = J loop, byte 36 done
        # But that makes done at 40 not 32. Let me redo carefully:
        #
        # 0:  ADDIU $t0, 0, 48
        # 4:  ADDIU $t1, 0, 18
        # 8:  BEQ $t0, $t1, +5 → target = PC12+20=32? No: +5 = 32 if word count. Let me count:
        #   after this instruction PC = 12.  target = 12 + 5*4 = 32. ← done at byte 36 (add +1)
        #   Actually target = 12 + 5*4 = 32.  That's byte address 32.
        # 12: SLTU $t2, $t0, $t1
        # 16: BNE  $t2, $zero, +1 → PC=20, target=20+4=24  (sub_b is at byte 28, not 24...)
        #   Hmm. Let me just keep it flat to avoid bugs:
        #
        # REVISED simpler: use BGTZ for a>b check:
        #
        # 0:  ADDIU $t0, 0, 48
        # 4:  ADDIU $t1, 0, 18
        # 8:  BEQ $t0, $t1, +4  → target = PC12+16=28 (done at 28)
        # 12: SLT $t2, $t0, $t1   (a < b signed?)
        # 16: BNE $t2, $zero, +1  → target=PC20+4=24 (do b-=a at 24)
        # 20: SUBU $t0, $t0, $t1  (a -= b)
        # 24: J 2 (byte 8)
        # 28: SUBU $t1, $t1, $t0  (b -= a)
        # 32: J 2 (byte 8)
        # Done at byte 28? But BEQ jumps to 28 which is SUBU $t1 — that's wrong.
        #
        # I'll use a clean version:
        # 0:  ADDIU $t0, 0, 48
        # 4:  ADDIU $t1, 0, 18
        # LOOP (byte 8):
        #  8:  BEQ $t0, $t1, +5 → target = 12+20=32 (but then done is at 32)
        # 12:  SLTU $t2, $t0, $t1
        # 16:  BNE $t2, $zero, +2 → target=20+8=28 (sub_b at 28)
        # 20:  SUBU $t0, $t0, $t1
        # 24:  J 2  (→ byte 8)
        # 28: (sub_b) SUBU $t1, $t1, $t0
        # 32: J 2
        # Done at byte 36: ADDU $v0, $t0, $zero; SYSCALL
        # Wait BEQ at byte 8: PC after fetch = 12. Target = 12 + 5*4 = 32. So done at 32... but J 2 is also at 32!
        # Conflict. Let me use +6:
        # BEQ offset = 6 → target = 12 + 24 = 36 (done at 36)
        # 0:  ADDIU $t0, 0, 48
        # 4:  ADDIU $t1, 0, 18
        # LOOP (byte 8):
        #  8:  BEQ  $t0,$t1, +6 → target=12+24=36
        # 12:  SLTU $t2, $t0, $t1
        # 16:  BNE  $t2, $zero, +2 → target=20+8=28
        # 20:  SUBU $t0, $t0, $t1
        # 24:  J 2
        # 28:  SUBU $t1, $t1, $t0
        # 32:  J 2
        # 36:  ADDU $v0, $t0, $zero
        # 40:  SYSCALL

        return (
            i_instr(0x09, 0, 8, 48)         # ADDIU $t0, $zero, 48
            + i_instr(0x09, 0, 9, 18)        # ADDIU $t1, $zero, 18
            + i_instr(0x04, 8, 9, 6)         # BEQ $t0, $t1, +6 → byte 36
            + r(8, 9, 10, 0, 0x2B)           # SLTU $t2, $t0, $t1
            + i_instr(0x05, 10, 0, 2)        # BNE $t2, $zero, +2 → byte 28
            + r(8, 9, 8, 0, 0x23)            # SUBU $t0, $t0, $t1
            + j_instr(0x02, 2)               # J byte 8 (word 2)
            + r(9, 8, 9, 0, 0x23)            # SUBU $t1, $t1, $t0
            + j_instr(0x02, 2)               # J byte 8 (word 2)
            + r(8, 0, 2, 0, 0x21)            # ADDU $v0, $t0, $zero
            + HALT
        )

    def test_gcd_result(self):
        state = run(self._build_prog())
        assert state.regs[2] == 6  # GCD(48, 18) = 6

    def test_halted(self):
        state = run(self._build_prog())
        assert state.halted


# ── Test 4: Bitwise patterns ───────────────────────────────────────────────────


class TestBitwisePatterns:
    """AND/OR/XOR/NOR with known bit patterns."""

    def test_and(self):
        prog = (
            i_instr(0x0F, 0, 8, 0xFF00)     # LUI $t0, 0xFF00
            + i_instr(0x0F, 0, 9, 0x0FF0)   # LUI $t1, 0x0FF0
            + r(8, 9, 2, 0, 0x24)           # AND $v0, $t0, $t1
            + HALT
        )
        state = run(prog)
        assert state.regs[2] == 0x0F000000  # 0xFF000000 & 0x0FF00000

    def test_or(self):
        prog = (
            i_instr(0x0F, 0, 8, 0x0F0F)
            + i_instr(0x0F, 0, 9, 0xF0F0)
            + r(8, 9, 2, 0, 0x25)  # OR $v0, $t0, $t1
            + HALT
        )
        state = run(prog)
        assert state.regs[2] == 0xFFFF0000

    def test_xor_self_is_zero(self):
        prog = (
            i_instr(0x0F, 0, 8, 0x1234)
            + r(8, 8, 2, 0, 0x26)  # XOR $v0, $t0, $t0
            + HALT
        )
        state = run(prog)
        assert state.regs[2] == 0

    def test_nor_with_zero(self):
        # NOR $v0, $t0, $zero = NOT($t0)
        prog = (
            i_instr(0x09, 0, 8, 0x00FF)     # ADDIU $t0, $zero, 0xFF
            + r(8, 0, 2, 0, 0x27)           # NOR $v0, $t0, $zero
            + HALT
        )
        state = run(prog)
        assert state.regs[2] == 0xFFFF_FF00  # NOT(0xFF) = 0xFFFFFF00


# ── Test 5: SLT comparisons ────────────────────────────────────────────────────


class TestSLT:
    def test_slt_positive(self):
        prog = (
            i_instr(0x09, 0, 8, 1)    # $t0 = 1
            + i_instr(0x09, 0, 9, 2)  # $t1 = 2
            + r(8, 9, 2, 0, 0x2A)     # SLT $v0, $t0, $t1  → 1
            + HALT
        )
        state = run(prog)
        assert state.regs[2] == 1

    def test_slt_equal(self):
        prog = (
            i_instr(0x09, 0, 8, 5)
            + r(8, 8, 2, 0, 0x2A)  # SLT $v0, $t0, $t0  → 0
            + HALT
        )
        state = run(prog)
        assert state.regs[2] == 0

    def test_sltu_unsigned(self):
        # -1 (0xFFFFFFFF) > 1 unsigned
        prog = (
            i_instr(0x09, 0, 8, 1)           # $t0 = 1
            + i_instr(0x0F, 0, 9, 0xFFFF)    # $t1 = 0xFFFF0000
            + i_instr(0x0D, 9, 9, 0xFFFF)    # $t1 |= 0xFFFF → 0xFFFFFFFF
            + r(8, 9, 2, 0, 0x2B)            # SLTU $v0, $t0, $t1 → 1
            + HALT
        )
        state = run(prog)
        assert state.regs[2] == 1


# ── Test 6: Memory load/store ──────────────────────────────────────────────────


class TestMemory:
    def test_sw_lw_roundtrip(self):
        """Store 0xDEADBEEF to address 0x0100, load it back."""
        prog = (
            i_instr(0x0F, 0, 8, 0xDEAD)         # LUI $t0, 0xDEAD
            + i_instr(0x0D, 8, 8, 0xBEEF)        # ORI $t0, $t0, 0xBEEF
            + i_instr(0x2B, 0, 8, 0x0100)        # SW $t0, 0x100($zero)
            + i_instr(0x23, 0, 9, 0x0100)        # LW $t1, 0x100($zero)
            + HALT
        )
        state = run(prog)
        assert state.regs[8] == 0xDEAD_BEEF
        assert state.regs[9] == 0xDEAD_BEEF

    def test_sb_lb(self):
        """Store byte, load byte with sign extension."""
        # Store 0xFF to address 0x200
        prog = (
            i_instr(0x09, 0, 8, 0x00FF)      # ADDIU $t0, $zero, 0xFF
            + i_instr(0x28, 0, 8, 0x0200)    # SB $t0, 0x200($zero)
            + i_instr(0x20, 0, 9, 0x0200)    # LB $t1, 0x200($zero) (sign-extend: -1)
            + i_instr(0x24, 0, 10, 0x0200)   # LBU $t2, 0x200($zero) (zero-extend: 255)
            + HALT
        )
        state = run(prog)
        assert state.regs[9] == 0xFFFF_FFFF   # -1 sign-extended
        assert state.regs[10] == 0xFF          # zero-extended

    def test_sh_lh(self):
        """Store halfword, load with sign extension."""
        prog = (
            i_instr(0x0F, 0, 8, 0x8000)      # LUI $t0, 0x8000 → 0x80000000
            + i_instr(0x0D, 8, 8, 0x0000)    # no-op ORI
            # Extract low halfword: SRL by 16 to get 0 — instead just store 0x8000:
            # Use ADDIU $t0, $zero, 0x8000? No, 0x8000 sign-extends to -32768
            # Use ORI $t0, $zero, 0x8000
            + i_instr(0x09, 0, 8, 0)         # ADDIU $t0, $zero, 0 (reset)
            + i_instr(0x0D, 0, 8, 0x8000)    # ORI $t0, $zero, 0x8000
            + i_instr(0x29, 0, 8, 0x0200)    # SH $t0, 0x200($zero)
            + i_instr(0x21, 0, 9, 0x0200)    # LH $t1, 0x200($zero) (sign-ext: -32768)
            + i_instr(0x25, 0, 10, 0x0200)   # LHU $t2, 0x200($zero) (zero-ext: 0x8000)
            + HALT
        )
        state = run(prog)
        assert state.regs[9] == 0xFFFF_8000   # -32768 sign-extended
        assert state.regs[10] == 0x8000


# ── Test 7: Branch and jump instructions ──────────────────────────────────────


class TestBranchJump:
    def test_beq_taken(self):
        # BEQ $t0, $t0, +1 → skip next instruction
        prog = (
            i_instr(0x09, 0, 8, 5)        # ADDIU $t0, $zero, 5
            + i_instr(0x04, 8, 8, 1)      # BEQ $t0, $t0, +1 (skip ADDIU $v0, 99)
            + i_instr(0x09, 0, 2, 99)     # ADDIU $v0, $zero, 99 (should be skipped)
            + i_instr(0x09, 0, 2, 42)     # ADDIU $v0, $zero, 42
            + HALT
        )
        state = run(prog)
        assert state.regs[2] == 42

    def test_bne_not_taken(self):
        prog = (
            i_instr(0x09, 0, 8, 5)
            + i_instr(0x09, 0, 9, 5)
            + i_instr(0x05, 8, 9, 5)      # BNE $t0, $t1, +5 (should NOT branch)
            + i_instr(0x09, 0, 2, 7)      # ADDIU $v0, $zero, 7 (executed)
            + HALT
        )
        state = run(prog)
        assert state.regs[2] == 7

    def test_bltz(self):
        prog = (
            i_instr(0x0F, 0, 8, 0xFFFF)   # LUI $t0, 0xFFFF → negative
            + i_instr(0x01, 8, 0, 1)      # BLTZ $t0, +1 (skip next)
            + i_instr(0x09, 0, 2, 99)     # skipped
            + i_instr(0x09, 0, 2, 1)      # $v0 = 1
            + HALT
        )
        state = run(prog)
        assert state.regs[2] == 1

    def test_bgez(self):
        prog = (
            i_instr(0x09, 0, 8, 5)        # $t0 = 5 (positive)
            + i_instr(0x01, 8, 1, 1)      # BGEZ $t0, +1 (skip next)
            + i_instr(0x09, 0, 2, 99)     # skipped
            + i_instr(0x09, 0, 2, 2)      # $v0 = 2
            + HALT
        )
        state = run(prog)
        assert state.regs[2] == 2

    def test_jalr_jr(self):
        """JALR to subroutine, JR to return."""
        # 0: ADDIU $t1, $zero, 16  ($t1 = address of func = byte 16)
        # 4: JALR $ra, $t1
        # 8: ADDIU $v0, $v0, 1  (executed on return, $v0 = 1)
        # 12: SYSCALL
        # 16: ADDIU $v0, $zero, 100   (func: $v0 = 100)
        # 20: JR $ra
        prog = (
            i_instr(0x09, 0, 9, 16)        # ADDIU $t1, $zero, 16
            + r(9, 0, 31, 0, 0x09)         # JALR $ra, $t1 (rd=31, rs=9)
            + i_instr(0x09, 2, 2, 1)       # ADDIU $v0, $v0, 1
            + HALT
            + i_instr(0x09, 0, 2, 100)     # func: ADDIU $v0, $zero, 100
            + r(31, 0, 0, 0, 0x08)         # JR $ra
        )
        state = run(prog)
        assert state.regs[2] == 101  # 100 + 1


# ── Test 8: Overflow detection ────────────────────────────────────────────────


class TestOverflow:
    def test_add_overflow_raises(self):
        prog = (
            i_instr(0x0F, 0, 8, 0x7FFF)     # LUI $t0, 0x7FFF → 0x7FFF0000
            + i_instr(0x0D, 8, 8, 0xFFFF)   # ORI → 0x7FFFFFFF (MAX_INT)
            + r(8, 8, 2, 0, 0x20)           # ADD $v0, $t0, $t0 → overflow
            + HALT
        )
        sim = MIPSR2000GateLevelSimulator()
        result = sim.execute(prog)
        assert result.error is not None
        assert "overflow" in result.error.lower()

    def test_addu_no_overflow(self):
        prog = (
            i_instr(0x0F, 0, 8, 0x7FFF)
            + i_instr(0x0D, 8, 8, 0xFFFF)
            + r(8, 8, 2, 0, 0x21)   # ADDU — wraps silently
            + HALT
        )
        state = run(prog)
        assert state.regs[2] == 0xFFFF_FFFE  # 0x7FFFFFFF + 0x7FFFFFFF
