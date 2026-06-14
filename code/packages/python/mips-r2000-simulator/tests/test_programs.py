"""End-to-end program tests for the MIPS R2000 simulator.

Each test encodes a complete small program in MIPS machine code and verifies
the final CPU/memory state.  These tests exercise instructions working together
rather than in isolation.

MIPS instruction encoding recap:
  R-type: [op:6=0][rs:5][rt:5][rd:5][shamt:5][funct:6]
  I-type: [op:6][rs:5][rt:5][imm16:16]
  J-type: [op:6][target26:26]

Branch target (no delay slots in this simulator):
  target = self._pc + sext(imm16) * 4   (PC is already at PC+4 after fetch)

All programs use big-endian encoding via struct.pack(">I", word).
"""

from __future__ import annotations

import struct

from mips_r2000_simulator import MIPSSimulator
from mips_r2000_simulator.state import REG_RA, REG_SP

# ── Helpers ───────────────────────────────────────────────────────────────────

def w32(v: int) -> bytes:
    """Pack a 32-bit MIPS word as 4 big-endian bytes."""
    return struct.pack(">I", v & 0xFFFF_FFFF)


HALT  = w32(0x0000_000C)   # SYSCALL
NOP   = w32(0x0000_0000)   # SLL $zero,$zero,0


def addiu(rt: int, rs: int, imm: int) -> bytes:
    """ADDIU rt, rs, imm  — op=9."""
    return w32((0x09 << 26) | (rs << 21) | (rt << 16) | (imm & 0xFFFF))


def addu(rd: int, rs: int, rt: int) -> bytes:
    """ADDU rd, rs, rt  — R-type funct=0x21."""
    return w32((rs << 21) | (rt << 16) | (rd << 11) | 0x21)


def subu(rd: int, rs: int, rt: int) -> bytes:
    """SUBU rd, rs, rt  — R-type funct=0x23."""
    return w32((rs << 21) | (rt << 16) | (rd << 11) | 0x23)


def bne(rs: int, rt: int, offset: int) -> bytes:
    """BNE rs, rt, offset  — op=5.  offset in *instructions* (not bytes)."""
    return w32((0x05 << 26) | (rs << 21) | (rt << 16) | (offset & 0xFFFF))


def beq(rs: int, rt: int, offset: int) -> bytes:
    """BEQ rs, rt, offset  — op=4."""
    return w32((0x04 << 26) | (rs << 21) | (rt << 16) | (offset & 0xFFFF))


def lw(rt: int, base: int, offset: int) -> bytes:
    """LW rt, offset(base)  — op=0x23."""
    return w32((0x23 << 26) | (base << 21) | (rt << 16) | (offset & 0xFFFF))


def sw(rt: int, base: int, offset: int) -> bytes:
    """SW rt, offset(base)  — op=0x2B."""
    return w32((0x2B << 26) | (base << 21) | (rt << 16) | (offset & 0xFFFF))


def lb(rt: int, base: int, offset: int) -> bytes:
    """LB rt, offset(base)  — op=0x20 (sign-extend)."""
    return w32((0x20 << 26) | (base << 21) | (rt << 16) | (offset & 0xFFFF))


def sb(rt: int, base: int, offset: int) -> bytes:
    """SB rt, offset(base)  — op=0x28."""
    return w32((0x28 << 26) | (base << 21) | (rt << 16) | (offset & 0xFFFF))


def slt(rd: int, rs: int, rt: int) -> bytes:
    """SLT rd, rs, rt  — R-type funct=0x2A."""
    return w32((rs << 21) | (rt << 16) | (rd << 11) | 0x2A)


def mult(rs: int, rt: int) -> bytes:
    """MULT rs, rt  — R-type funct=0x18."""
    return w32((rs << 21) | (rt << 16) | 0x18)


def mflo(rd: int) -> bytes:
    """MFLO rd  — R-type funct=0x12."""
    return w32((rd << 11) | 0x12)


def mfhi(rd: int) -> bytes:
    """MFHI rd  — R-type funct=0x10."""
    return w32((rd << 11) | 0x10)


def jal_to(pc_of_jal: int, target_addr: int) -> bytes:
    """Build JAL instruction to jump from pc_of_jal to target_addr.

    JAL target:  target_26 = target_addr >> 2  (low 26 bits of word address)
    Actual jump: (PC+4)[31:28] | (target_26 << 2) — for small addresses this
    just equals target_addr.
    """
    target26 = (target_addr >> 2) & 0x03FF_FFFF
    return w32((0x03 << 26) | target26)


def jr_ra() -> bytes:
    """JR $ra  — R-type funct=0x08, rs=REG_RA=31."""
    return w32((REG_RA << 21) | 0x08)


# ── Programs ──────────────────────────────────────────────────────────────────

class TestSumProgram:

    def test_sum_1_to_10(self):
        """Compute 1+2+…+10 = 55.

        Register layout:
          $t0 (8) = counter (starts 10, decrements to 0)
          $t1 (9) = running sum (starts 0)

        Program:
          0x00: ADDIU $t0, $zero, 10   — counter = 10
          0x04: ADDU  $t1, $t1, $t0   — sum += counter
          0x08: ADDIU $t0, $t0, -1    — counter--
          0x0C: BNE   $t0, $zero, -3  — if counter != 0, go back to 0x04
          0x10: HALT

        Branch at 0x0C: PC after fetch = 0x10; offset = -3 words → 0x10 + (-3*4) = 0x04 ✓
        """
        prog  = addiu(8, 0, 10)           # 0x00: $t0 = 10
        prog += addu(9, 9, 8)             # 0x04: $t1 += $t0
        prog += addiu(8, 8, 0xFFFF)       # 0x08: $t0-- (ADDIU $t0,$t0,-1)
        prog += bne(8, 0, 0xFFFF - 2)     # 0x0C: BNE $t0,$zero,-3
        prog += HALT                       # 0x10
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[9] == 55


class TestMultiply:

    def test_multiply_via_mult(self):
        """7 × 6 = 42 using MULT instruction."""
        prog  = addiu(8, 0, 7)
        prog += addiu(9, 0, 6)
        prog += mult(8, 9)
        prog += mflo(10)
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[10] == 42

    def test_multiply_by_repeated_addition(self):
        """7 × 6 = 42 using a loop.

          $t0 = multiplier (6)
          $t1 = result (starts 0)
          Loop: $t1 += 7; $t0--; BNE $t0,$zero,loop

          0x00: ADDIU $t0, $zero, 6
          0x04: ADDIU $t2, $zero, 7    — constant factor
          0x08: ADDU  $t1, $t1, $t2   — loop: result += 7
          0x0C: ADDIU $t0, $t0, -1    — counter--
          0x10: BNE   $t0, $zero, -3  — PC=0x14; -3*4= -12; target=0x14-12=0x08 ✓
          0x14: HALT
        """
        prog  = addiu(8, 0, 6)            # $t0 = 6
        prog += addiu(10, 0, 7)           # $t2 = 7
        prog += addu(9, 9, 10)            # loop top (0x08): $t1 += $t2
        prog += addiu(8, 8, 0xFFFF)       # $t0--
        prog += bne(8, 0, 0xFFFF - 2)     # BNE back 3 instrs
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[9] == 42

    def test_factorial_5_via_mult(self):
        """5! = 120 using MULT.

        Strategy: start $t1=1; loop $t0=5 downto 1; each iter MULT $t1,$t0 → LO.

          $t0 (8) = current multiplier (5 downto 1)
          $t1 (9) = running product (start 1)

          0x00: ADDIU $t0, $zero, 5
          0x04: ADDIU $t1, $zero, 1
          0x08: MULT  $t1, $t0          — loop top
          0x0C: MFLO  $t1
          0x10: ADDIU $t0, $t0, -1
          0x14: BNE   $t0, $zero, -4   — PC=0x18; -4*4=-16; target=0x18-16=0x08 ✓
          0x18: HALT
        """
        prog  = addiu(8, 0, 5)            # $t0 = 5
        prog += addiu(9, 0, 1)            # $t1 = 1
        prog += mult(9, 8)                # loop top (0x08): HI:LO = $t1 * $t0
        prog += mflo(9)                   # $t1 = LO
        prog += addiu(8, 8, 0xFFFF)       # $t0--
        prog += bne(8, 0, 0xFFFF - 3)     # BNE back 4 instructions
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[9] == 120


class TestMemoryPrograms:

    def test_word_copy(self):
        """Copy 4 words from memory[0x0100] to memory[0x0200].

        Register layout:
          $t0 (8) = source pointer
          $t1 (9) = dest pointer
          $t2 (10) = counter (4)
          $t3 (11) = temp value

          0x00: ADDIU $t0, $zero, 0x100
          0x04: ADDIU $t1, $zero, 0x200
          0x08: ADDIU $t2, $zero, 4
          0x0C: LW    $t3, 0($t0)        — loop top
          0x10: SW    $t3, 0($t1)
          0x14: ADDIU $t0, $t0, 4
          0x18: ADDIU $t1, $t1, 4
          0x1C: ADDIU $t2, $t2, -1
          0x20: BNE   $t2, $zero, -6    — PC=0x24; -6*4=-24; target=0x24-24=0x0C ✓
          0x24: HALT
        """
        prog  = addiu(8, 0, 0x100)        # 0x00
        prog += addiu(9, 0, 0x200)        # 0x04
        prog += addiu(10, 0, 4)           # 0x08
        prog += lw(11, 8, 0)             # 0x0C: loop top
        prog += sw(11, 9, 0)             # 0x10
        prog += addiu(8, 8, 4)           # 0x14
        prog += addiu(9, 9, 4)           # 0x18
        prog += addiu(10, 10, 0xFFFF)    # 0x1C: $t2--
        prog += bne(10, 0, 0xFFFF - 5)   # 0x20: BNE back 6 instrs
        prog += HALT                       # 0x24

        sim = MIPSSimulator()
        sim.load(prog)
        # Manually write source data after load (load calls reset)
        src = [0x11223344, 0xAABBCCDD, 0xDEADBEEF, 0x0BADF00D]
        for i, val in enumerate(src):
            sim._store_word(0x100 + i * 4, val)
        while not sim._halted:
            sim.step()
        for i, val in enumerate(src):
            assert sim._load_word(0x200 + i * 4) == val

    def test_byte_copy(self):
        """Copy 8 bytes from memory[0x0400] to memory[0x0500] using LB/SB.

          $t0 = src (0x400), $t1 = dst (0x500), $t2 = count (8)

          Loop:
            LB  $t3, 0($t0)
            SB  $t3, 0($t1)
            ADDIU $t0, $t0, 1
            ADDIU $t1, $t1, 1
            ADDIU $t2, $t2, -1
            BNE $t2, $zero, back_to_LB
        """
        prog  = addiu(8, 0, 0x400)
        prog += addiu(9, 0, 0x500)
        prog += addiu(10, 0, 8)
        prog += lb(11, 8, 0)             # loop top (0x0C)
        prog += sb(11, 9, 0)
        prog += addiu(8, 8, 1)
        prog += addiu(9, 9, 1)
        prog += addiu(10, 10, 0xFFFF)
        prog += bne(10, 0, 0xFFFF - 5)   # BNE back 6 instrs
        prog += HALT

        sim = MIPSSimulator()
        sim.load(prog)
        source = bytes(range(8))
        for i, b in enumerate(source):
            sim._mem[0x400 + i] = b
        while not sim._halted:
            sim.step()
        for i, b in enumerate(source):
            assert sim._mem[0x500 + i] == b


class TestSubroutinePrograms:

    def test_simple_subroutine(self):
        """Call a subroutine that doubles $t0; verify $ra and result.

        Main at 0x00; subroutine at 0x10.

          0x00: ADDIU $t0, $zero, 7     — argument
          0x04: JAL   0x10              — call double($t0)
          0x08: HALT
          (gap: 0x0C is skipped)
          0x10: ADDU $t0, $t0, $t0     — $t0 = $t0 + $t0
          0x14: JR   $ra               — return
        """
        prog = bytearray(0x18)
        struct.pack_into(">I", prog, 0x00, (0x09 << 26) | (0 << 21) | (8 << 16) | 7)  # ADDIU $t0,7
        # JAL to 0x10: target26 = 0x10 >> 2 = 4; (PC+4)=0x08; (0x08 & 0xF000)|(4<<2)=0x10 ✓
        struct.pack_into(">I", prog, 0x04, (0x03 << 26) | 4)   # JAL 0x10
        struct.pack_into(">I", prog, 0x08, 0x0000_000C)          # HALT
        struct.pack_into(">I", prog, 0x0C, 0x0000_0000)          # NOP (padding)
        # Subroutine at 0x10
        struct.pack_into(">I", prog, 0x10, (8 << 21) | (8 << 16) | (8 << 11) | 0x21)  # ADDU $t0,$t0,$t0
        struct.pack_into(">I", prog, 0x14, (REG_RA << 21) | 0x08)  # JR $ra
        sim = MIPSSimulator()
        result = sim.execute(bytes(prog))
        assert result.ok
        assert sim._regs[8] == 14      # 7 * 2
        assert sim._regs[REG_RA] == 0x08   # return address

    def test_stack_balance_after_nested_calls(self):
        """Nested call leaves stack pointer unchanged.

        We use $sp to track a "manual" stack.  On a bare simulator reset,
        $sp = 0.  We set $sp to 0x1000, call an outer subroutine that calls
        an inner one.  After both return, $sp should still be 0x1000.

        Here we use the standard convention:
          ADDIU $sp, $sp, -4  ; push
          SW $ra, 0($sp)      ; save $ra
          ...
          LW $ra, 0($sp)      ; restore $ra
          ADDIU $sp, $sp, 4   ; pop
          JR $ra              ; return

        Main:
          0x00: ADDIU $sp, $zero, 0x1000
          0x04: JAL outer (0x20)
          0x08: HALT

        Outer at 0x20:
          0x20: ADDIU $sp, $sp, -4
          0x24: SW    $ra, 0($sp)
          0x28: JAL inner (0x40)
          0x2C: LW    $ra, 0($sp)
          0x30: ADDIU $sp, $sp, 4
          0x34: JR    $ra

        Inner at 0x40:
          0x40: NOP
          0x44: JR    $ra
        """
        prog = bytearray(0x48)
        # Main
        struct.pack_into(">I", prog, 0x00, (0x09 << 26) | (0 << 21) | (REG_SP << 16) | 0x1000)  # ADDIU $sp,0x1000
        struct.pack_into(">I", prog, 0x04, (0x03 << 26) | 8)    # JAL to 0x20 (8<<2=0x20)
        struct.pack_into(">I", prog, 0x08, 0x0000_000C)           # HALT
        # Pad gap 0x0C–0x1C
        for off in range(0x0C, 0x20, 4):
            struct.pack_into(">I", prog, off, 0)
        # Outer at 0x20
        struct.pack_into(">I", prog, 0x20, (0x09 << 26) | (REG_SP << 21) | (REG_SP << 16) | 0xFFFC)  # ADDIU $sp,$sp,-4
        struct.pack_into(">I", prog, 0x24, (0x2B << 26) | (REG_SP << 21) | (REG_RA << 16) | 0)        # SW $ra, 0($sp)
        struct.pack_into(">I", prog, 0x28, (0x03 << 26) | 16)   # JAL to 0x40 (16<<2=0x40)
        struct.pack_into(">I", prog, 0x2C, (0x23 << 26) | (REG_SP << 21) | (REG_RA << 16) | 0)        # LW $ra, 0($sp)
        struct.pack_into(">I", prog, 0x30, (0x09 << 26) | (REG_SP << 21) | (REG_SP << 16) | 4)        # ADDIU $sp,$sp,4
        struct.pack_into(">I", prog, 0x34, (REG_RA << 21) | 0x08)   # JR $ra
        # Pad 0x38–0x3C
        for off in range(0x38, 0x40, 4):
            struct.pack_into(">I", prog, off, 0)
        # Inner at 0x40
        struct.pack_into(">I", prog, 0x40, 0)                        # NOP
        struct.pack_into(">I", prog, 0x44, (REG_RA << 21) | 0x08)   # JR $ra
        sim = MIPSSimulator()
        result = sim.execute(bytes(prog))
        assert result.ok
        assert sim._regs[REG_SP] == 0x1000


class TestFibonacci:

    def test_fibonacci_8_terms_in_memory(self):
        """Store first 8 Fibonacci numbers in memory[0x0100–0x011F].

        F = 1, 1, 2, 3, 5, 8, 13, 21 (word-sized)

        Strategy: Iterate 6 times computing F[i] = F[i-2] + F[i-1].

          $t0 = pointer to F[0]  (starts at 0x100)
          $t1 = counter (6 iterations)
          At each step: load F[i-2] from 0($t0), F[i-1] from 4($t0),
                        sum into F[i] at 8($t0), then advance $t0 by 4.

          0x00: ADDIU $t0, $zero, 0x100
          0x04: SW    $zero, 0($t0)       — clear all 8 entries (via 8 SW nops)
              Actually: store F[0]=1 and F[1]=1 first.
          0x04: ADDIU $t2, $zero, 1
          0x08: SW    $t2, 0($t0)         — F[0]=1
          0x0C: SW    $t2, 4($t0)         — F[1]=1
          0x10: ADDIU $t1, $zero, 6       — 6 more terms
          Loop at 0x14:
          0x14: LW    $t3, 0($t0)         — F[i-2]
          0x18: LW    $t4, 4($t0)         — F[i-1]
          0x1C: ADDU  $t5, $t3, $t4       — F[i]
          0x20: SW    $t5, 8($t0)         — store F[i]
          0x24: ADDIU $t0, $t0, 4         — advance pointer
          0x28: ADDIU $t1, $t1, -1
          0x2C: BNE   $t1, $zero, -6     — PC=0x30; -6*4=-24; target=0x30-24=0x18
              Wait: loop top is at 0x14, so we need -7 to go back to 0x14
              PC after BNE fetch = 0x30; offset = -7 → 0x30 - 28 = 0x14 ✓
          0x30: HALT
        """
        prog  = addiu(8, 0, 0x100)        # 0x00: $t0 = 0x100
        prog += addiu(10, 0, 1)           # 0x04: $t2 = 1
        prog += sw(10, 8, 0)             # 0x08: mem[0x100] = 1 (F[0])
        prog += sw(10, 8, 4)             # 0x0C: mem[0x104] = 1 (F[1])
        prog += addiu(9, 0, 6)           # 0x10: $t1 = 6
        prog += lw(11, 8, 0)             # 0x14: loop top — F[i-2]
        prog += lw(12, 8, 4)             # 0x18: F[i-1]
        prog += addu(13, 11, 12)          # 0x1C: F[i] = F[i-2]+F[i-1]
        prog += sw(13, 8, 8)             # 0x20: mem[$t0+8] = F[i]
        prog += addiu(8, 8, 4)           # 0x24: $t0 += 4
        prog += addiu(9, 9, 0xFFFF)      # 0x28: $t1--
        prog += bne(9, 0, 0xFFFF - 6)    # 0x2C: BNE back 7 instrs (to 0x14)
        prog += HALT                       # 0x30

        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert result.ok
        expected = [1, 1, 2, 3, 5, 8, 13, 21]
        for i, v in enumerate(expected):
            got = sim._load_word(0x100 + i * 4)
            assert got == v, f"F[{i}]={got}, expected {v}"


class TestSortProgram:

    def test_bubble_sort_4_words(self):
        """Bubble sort 4 words in memory using SLT + conditional swap.

        Values at 0x0100: [40, 10, 30, 20] → sorted: [10, 20, 30, 40]

        Simple selection sort for clarity:
          Outer loop i: 0..3
          Inner loop j: i+1..3
          If mem[i] > mem[j]: swap

        For simplicity we implement insertion sort in assembly using
        a loop with SLT and BEQ (branch if already in order).

        Actual program: a simple 3-pass bubble sort.
        Pass 1: compare [0,1] [1,2] [2,3]
        Pass 2: compare [0,1] [1,2] [2,3]
        Pass 3: compare [0,1] [1,2] [2,3]

        Hard-coded 3 passes × 3 compares = 9 compare-swap steps.

        Each compare-swap at addr A and B (adjacent):
          LW $s0, 0(A)
          LW $s1, 4(A)
          SLT $t0, $s1, $s0    — is mem[A+4] < mem[A]?
          BEQ $t0, $zero, skip — if not, skip swap
          SW $s1, 0(A)
          SW $s0, 4(A)
          skip:

        We'll build this as a flat sequence for 3 passes.
        """
        BASE = 0x0100

        def compare_swap(prog: bytearray, offset: int, a_ptr_reg: int) -> bytearray:
            """Append compare-swap of mem[a_ptr_reg+offset] and mem[a_ptr_reg+offset+4]."""
            # LW $s0, offset($ptr)
            prog += lw(16, a_ptr_reg, offset)
            # LW $s1, offset+4($ptr)
            prog += lw(17, a_ptr_reg, offset + 4)
            # SLT $t0, $s1, $s0  — is mem[offset+4] < mem[offset]?
            prog += slt(8, 17, 16)
            # BEQ $t0, $zero, +2  — skip the two SWs if already in order
            prog += beq(8, 0, 2)
            # SW $s1, offset($ptr)
            prog += sw(17, a_ptr_reg, offset)
            # SW $s0, offset+4($ptr)
            prog += sw(16, a_ptr_reg, offset + 4)
            # (fall through — no extra NOP needed, BEQ target is here+2*4 = after both SWs)
            return prog

        # Build program as bytearray
        prog = b""
        # Load BASE address into $t2 (10)
        prog += addiu(10, 0, BASE)
        # 3 passes × 3 adjacent compares
        for _ in range(3):
            for offset in [0, 4, 8]:
                prog = compare_swap(prog, offset, 10)
        prog += HALT

        sim = MIPSSimulator()
        sim.load(prog)
        # Write unsorted data after load (reset zeroes memory)
        unsorted = [40, 10, 30, 20]
        for i, v in enumerate(unsorted):
            sim._store_word(BASE + i * 4, v)
        while not sim._halted:
            sim.step()

        result = [sim._load_word(BASE + i * 4) for i in range(4)]
        assert result == [10, 20, 30, 40]
