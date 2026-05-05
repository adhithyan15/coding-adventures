"""End-to-end program tests for the SPARC V8 simulator.

Each test encodes a complete small program in SPARC machine code and verifies
the final CPU/memory state.

SPARC instruction encoding recap:
  Format 1: [op:2=01][disp30:30]                         — CALL
  Format 2: [op:2=00][rd:5][op2:3][imm22:22]             — SETHI, Bicc
  Format 3: [op:2][rd:5][op3:6][rs1:5][i:1][rest:13/18]  — ALU/Memory

Branch target = PC_of_branch + sign_extend(disp22) * 4
  (No delay slots in this simulator — branch takes effect immediately)
"""

from __future__ import annotations

import struct

from sparc_v8_simulator import SPARCSimulator

# ── Helpers ───────────────────────────────────────────────────────────────────

def w32(v: int) -> bytes:
    return struct.pack(">I", v & 0xFFFF_FFFF)


HALT = w32(0x91D0_2000)   # ta 0
NOP  = w32(0x0100_0000)   # sethi 0, %g0


def sethi(rd: int, imm22: int) -> bytes:
    return w32((rd << 25) | (0x4 << 22) | (imm22 & 0x3FFFFF))


def add_i(rd: int, rs1: int, simm13: int) -> bytes:
    """ADD rd, rs1, simm13."""
    return w32((0x2 << 30) | (rd << 25) | (0x00 << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


def add_r(rd: int, rs1: int, rs2: int) -> bytes:
    """ADD rd, rs1, rs2."""
    return w32((0x2 << 30) | (rd << 25) | (0x00 << 19) | (rs1 << 14) | rs2)


def addcc_i(rd: int, rs1: int, simm13: int) -> bytes:
    """ADDcc rd, rs1, simm13."""
    return w32((0x2 << 30) | (rd << 25) | (0x10 << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


def sub_i(rd: int, rs1: int, simm13: int) -> bytes:
    """SUB rd, rs1, simm13."""
    return w32((0x2 << 30) | (rd << 25) | (0x04 << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


def subcc_r(rd: int, rs1: int, rs2: int) -> bytes:
    """SUBcc rd, rs1, rs2 — sets condition codes."""
    return w32((0x2 << 30) | (rd << 25) | (0x14 << 19) | (rs1 << 14) | rs2)


def or_i(rd: int, rs1: int, simm13: int) -> bytes:
    """OR rd, rs1, simm13 (also used as MOV imm)."""
    return w32((0x2 << 30) | (rd << 25) | (0x02 << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


def smul_r(rd: int, rs1: int, rs2: int) -> bytes:
    """SMUL rd, rs1, rs2."""
    return w32((0x2 << 30) | (rd << 25) | (0x0B << 19) | (rs1 << 14) | rs2)


def ld(rd: int, rs1: int, simm13: int) -> bytes:
    """LD rd, [rs1+simm13]."""
    return w32((0x3 << 30) | (rd << 25) | (0x00 << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


def st(rd: int, rs1: int, simm13: int) -> bytes:
    """ST rd, [rs1+simm13]."""
    return w32((0x3 << 30) | (rd << 25) | (0x04 << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


def ldsb(rd: int, rs1: int, simm13: int) -> bytes:
    """LDSB rd, [rs1+simm13]."""
    return w32((0x3 << 30) | (rd << 25) | (0x09 << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


def stb(rd: int, rs1: int, simm13: int) -> bytes:
    """STB rd, [rs1+simm13]."""
    return w32((0x3 << 30) | (rd << 25) | (0x05 << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


def slt(rd: int, rs1: int, rs2: int) -> bytes:
    """SUBcc + ADDX idiom: rd = (rs1 < rs2) via condition codes.
    Actually just use SUBcc + conditional branch in programs.
    """
    return subcc_r(rd, rs1, rs2)


def bne(disp22: int) -> bytes:
    """BNE disp22 — branch if not equal (Z=0), op=0, op2=2, cond=9."""
    return w32((0x0 << 30) | (0x9 << 25) | (0x2 << 22) | (disp22 & 0x3FFFFF))


def be(disp22: int) -> bytes:
    """BE disp22 — branch if equal (Z=1), cond=1."""
    return w32((0x0 << 30) | (0x1 << 25) | (0x2 << 22) | (disp22 & 0x3FFFFF))


def bl(disp22: int) -> bytes:
    """BL disp22 — branch if less (signed, N!=V), cond=3."""
    return w32((0x0 << 30) | (0x3 << 25) | (0x2 << 22) | (disp22 & 0x3FFFFF))


def ble(disp22: int) -> bytes:
    """BLE disp22 — branch if ≤ (signed, Z=1 or N!=V), cond=2."""
    return w32((0x0 << 30) | (0x2 << 25) | (0x2 << 22) | (disp22 & 0x3FFFFF))


def bpos(disp22: int) -> bytes:
    """BPOS disp22 — branch if positive (N=0), cond=14."""
    return w32((0x0 << 30) | (0xE << 25) | (0x2 << 22) | (disp22 & 0x3FFFFF))


def ba(disp22: int) -> bytes:
    """BA disp22 — branch always, cond=8."""
    return w32((0x0 << 30) | (0x8 << 25) | (0x2 << 22) | (disp22 & 0x3FFFFF))


def save_i(rd: int, rs1: int, simm13: int) -> bytes:
    """SAVE rd, rs1, simm13."""
    return w32((0x2 << 30) | (rd << 25) | (0x3C << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


def restore_i(rd: int, rs1: int, simm13: int) -> bytes:
    """RESTORE rd, rs1, simm13."""
    return w32((0x2 << 30) | (rd << 25) | (0x3D << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


def jmpl_r(rd: int, rs1: int, simm13: int) -> bytes:
    """JMPL rd, rs1+simm13."""
    return w32((0x2 << 30) | (rd << 25) | (0x38 << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


# ── Sum 1 to 10 ───────────────────────────────────────────────────────────────

class TestSumProgram:

    def test_sum_1_to_10(self):
        """Compute 1+2+…+10 = 55 using ADDcc + BNE loop.

        Register layout:
          %g1 (r1) = counter (starts 10, decrements to 0)
          %g2 (r2) = running sum (starts 0)

        Program:
          0x00: ADD  %g1, %g0, 10    — counter = 10
          0x04: ADD  %g2, %g2, %g1  — loop top: sum += counter
          0x08: ADDcc %g1, %g1, -1  — counter--; sets Z flag
          0x0C: BNE  -2              — if Z=0 (counter != 0), go back 2 instrs
          0x10: HALT

        Branch at 0x0C: PC of BNE = 0x0C; target = 0x0C + (-2)*4 = 0x04 ✓
        """
        prog  = add_i(1, 0, 10)             # 0x00: %g1 = 10
        prog += add_r(2, 2, 1)              # 0x04: %g2 += %g1  (loop top)
        prog += addcc_i(1, 1, 0x1FFF)       # 0x08: %g1-- (simm13=-1 = 0x1FFF)
        prog += bne(0x3FFFFE)               # 0x0C: BNE -2 (disp22 = -2 = 0x3FFFFE)
        prog += HALT                         # 0x10
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(2) == 55


# ── Multiply ──────────────────────────────────────────────────────────────────

class TestMultiply:

    def test_multiply_via_smul(self):
        """7 × 6 = 42 using SMUL."""
        prog  = add_i(1, 0, 7)
        prog += add_i(2, 0, 6)
        prog += smul_r(3, 1, 2)
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(3) == 42

    def test_multiply_by_repeated_addition(self):
        """7 × 6 = 42 using a loop.

          %g1 = counter (6)
          %g2 = accumulator (0)
          %g3 = addend (7)
          Loop: %g2 += %g3; ADDcc %g1, %g1, -1; BNE back
        """
        prog  = add_i(1, 0, 6)              # 0x00: counter = 6
        prog += add_i(3, 0, 7)              # 0x04: addend = 7
        prog += add_r(2, 2, 3)              # 0x08: loop top: acc += 7
        prog += addcc_i(1, 1, 0x1FFF)       # 0x0C: counter--
        prog += bne(0x3FFFFE)               # 0x10: BNE -2 → 0x08
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(2) == 42

    def test_factorial_5(self):
        """5! = 120 using SMUL loop.

          %g1 = counter (5 downto 1)
          %g2 = product (starts 1)
          Loop: SMUL %g2, %g2, %g1; ADDcc %g1, %g1, -1; BNE back
        """
        prog  = add_i(1, 0, 5)              # 0x00: counter = 5
        prog += add_i(2, 0, 1)              # 0x04: product = 1
        prog += smul_r(2, 2, 1)             # 0x08: loop top: product *= counter
        prog += addcc_i(1, 1, 0x1FFF)       # 0x0C: counter--
        prog += bne(0x3FFFFE)               # 0x10: BNE -2 → 0x08
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(2) == 120


# ── Memory programs ───────────────────────────────────────────────────────────

class TestMemoryPrograms:

    def test_word_copy(self):
        """Copy 4 words from memory[0x0100] to memory[0x0200].

          %g1 = source pointer (0x100)
          %g2 = dest pointer   (0x200)
          %g3 = counter (4)
          %g4 = temp

          Loop:
            LD  %g4, [%g1]
            ST  %g4, [%g2]
            ADD %g1, %g1, 4
            ADD %g2, %g2, 4
            ADDcc %g3, %g3, -1
            BNE back
        """
        prog  = or_i(1, 0, 0x100)           # 0x00: %g1 = 0x100
        prog += or_i(2, 0, 0x200)           # 0x04: %g2 = 0x200
        prog += add_i(3, 0, 4)              # 0x08: counter = 4
        prog += ld(4, 1, 0)                 # 0x0C: loop top — load
        prog += st(4, 2, 0)                 # 0x10: store
        prog += add_i(1, 1, 4)              # 0x14: src += 4
        prog += add_i(2, 2, 4)              # 0x18: dst += 4
        prog += addcc_i(3, 3, 0x1FFF)       # 0x1C: counter--
        prog += bne(0x3FFFFB)               # 0x20: BNE -5 → 0x0C  (0x20 + (-5)*4 = 0x0C)
        prog += HALT

        sim = SPARCSimulator()
        sim.load(prog)
        src = [0x11223344, 0xAABBCCDD, 0xDEADBEEF, 0x0BADF00D]
        for i, v in enumerate(src):
            sim._store_word(0x100 + i * 4, v)
        while not sim._halted:
            sim.step()
        for i, v in enumerate(src):
            assert sim._load_word(0x200 + i * 4) == v

    def test_byte_copy(self):
        """Copy 8 bytes from memory[0x0400] to memory[0x0500] using LDSB/STB.

          %g1 = src (0x400), %g2 = dst (0x500), %g3 = count (8)
        """
        prog  = or_i(1, 0, 0x400)
        prog += or_i(2, 0, 0x500)
        prog += add_i(3, 0, 8)
        prog += ldsb(4, 1, 0)               # loop top (0x0C): load byte
        prog += stb(4, 2, 0)                # store byte
        prog += add_i(1, 1, 1)
        prog += add_i(2, 2, 1)
        prog += addcc_i(3, 3, 0x1FFF)
        prog += bne(0x3FFFFB)               # BNE -5 → loop top (0x20 + (-5)*4 = 0x0C)
        prog += HALT

        sim = SPARCSimulator()
        sim.load(prog)
        for i in range(8):
            sim._mem[0x400 + i] = i
        while not sim._halted:
            sim.step()
        for i in range(8):
            assert sim._mem[0x500 + i] == i


# ── Fibonacci ─────────────────────────────────────────────────────────────────

class TestFibonacci:

    def test_fibonacci_8_terms(self):
        """Store first 8 Fibonacci numbers at memory[0x0100..0x011F].

        F = [1, 1, 2, 3, 5, 8, 13, 21]

        Strategy: write F[0]=1, F[1]=1, then iterate:
          F[i] = F[i-2] + F[i-1], stored via pointer advance.

          %g1 = pointer to F[0]  (0x100)
          %g2 = counter (6 iterations)
          Each iteration: load F[i-2], F[i-1], sum, store F[i], advance ptr.
        """
        prog  = or_i(1, 0, 0x100)           # 0x00: %g1 = 0x100
        prog += add_i(5, 0, 1)              # 0x04: %g5 = 1 (constant)
        prog += st(5, 1, 0)                 # 0x08: mem[0x100] = 1 (F[0])
        prog += st(5, 1, 4)                 # 0x0C: mem[0x104] = 1 (F[1])
        prog += add_i(2, 0, 6)              # 0x10: counter = 6
        prog += ld(3, 1, 0)                 # 0x14: loop top: F[i-2]
        prog += ld(4, 1, 4)                 # 0x18: F[i-1]
        prog += add_r(5, 3, 4)              # 0x1C: F[i] = F[i-2] + F[i-1]
        prog += st(5, 1, 8)                 # 0x20: mem[ptr+8] = F[i]
        prog += add_i(1, 1, 4)              # 0x24: advance pointer
        prog += addcc_i(2, 2, 0x1FFF)       # 0x28: counter--
        prog += bne(0x3FFFFA)               # 0x2C: BNE -6 → 0x14
        prog += HALT

        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert result.ok
        expected = [1, 1, 2, 3, 5, 8, 13, 21]
        for i, v in enumerate(expected):
            got = sim._load_word(0x100 + i * 4)
            assert got == v, f"F[{i}]={got}, expected {v}"


# ── Bubble sort ───────────────────────────────────────────────────────────────

class TestSort:

    def test_bubble_sort_4_words(self):
        """Bubble sort [40, 10, 30, 20] → [10, 20, 30, 40].

        3 passes × 3 compare-swaps (hard-coded flat loop).

        Each compare-swap of mem[ptr+off] and mem[ptr+off+4]:
          LD  %g5, [ptr+off]
          LD  %g6, [ptr+off+4]
          SUBcc %g0, %g5, %g6   — %g5 > %g6? (if positive: swap)
          BLE skip               — branch if %g5 ≤ %g6 (no swap needed)
          ST  %g6, [ptr+off]
          ST  %g5, [ptr+off+4]
          skip:
        """
        BASE = 0x0100

        def compare_swap(p: bytes, offset: int, ptr_reg: int) -> bytes:
            p += ld(5, ptr_reg, offset)
            p += ld(6, ptr_reg, offset + 4)
            p += subcc_r(0, 5, 6)          # sets flags for %g5 - %g6
            p += ble(3)                    # BLE skip (disp22=3 → jumps past both STs)
            p += st(6, ptr_reg, offset)
            p += st(5, ptr_reg, offset + 4)
            return p

        prog = or_i(1, 0, BASE)            # %g1 = base address
        for _ in range(3):
            for offset in [0, 4, 8]:
                prog = compare_swap(prog, offset, 1)
        prog += HALT

        sim = SPARCSimulator()
        sim.load(prog)
        for i, v in enumerate([40, 10, 30, 20]):
            sim._store_word(BASE + i * 4, v)
        while not sim._halted:
            sim.step()
        result = [sim._load_word(BASE + i * 4) for i in range(4)]
        assert result == [10, 20, 30, 40]


# ── Subroutine with SAVE / RESTORE ────────────────────────────────────────────

class TestSubroutine:

    def test_simple_subroutine_doubles_value(self):
        """Call a subroutine that doubles %o0; verify result and return.

        Main:
          0x00: OR  %o0, %g0, 7        — argument = 7
          0x04: CALL subroutine (0x10) — %o7 = 0x04, jump to 0x10
          0x08: HALT

        Subroutine at 0x10:
          0x10: SAVE %sp, %sp, -64     — create stack frame
          0x14: ADD  %l0, %i0, %i0    — %l0 = arg * 2 = 14
          0x18: OR   %i0, %l0, 0       — return value in %i0
          0x1C: RESTORE %g0, %g0, 0   — restore window
          0x20: JMPL %g0, %o7+8       — return: jump to %o7+8

        Wait — SPARC ABI: return via 'retl' = JMPL %g0, %o7+8
        But after RESTORE, the window has been restored, so %o7 is back in
        the caller's window.  We use JMPL %i7+8 before RESTORE.

        Simplified for this test: no SAVE/RESTORE, just direct call.
        """
        prog = bytearray(0x28)
        # Main
        struct.pack_into(">I", prog, 0x00,
            (0x2 << 30) | (8 << 25) | (0x02 << 19) | (0 << 14) | (1 << 13) | 7)  # OR %o0, %g0, 7
        # CALL to 0x10: disp30 = (0x10 - 0x04) / 4 = 3
        struct.pack_into(">I", prog, 0x04, (0x1 << 30) | 3)    # CALL +3 → 0x10
        struct.pack_into(">I", prog, 0x08, 0x91D0_2000)          # HALT
        # Padding
        struct.pack_into(">I", prog, 0x0C, 0x0100_0000)          # NOP
        # Subroutine at 0x10 (no window change for simplicity):
        #   ADD %o0, %o0, %o0   — double the argument
        struct.pack_into(">I", prog, 0x10,
            (0x2 << 30) | (8 << 25) | (0x00 << 19) | (8 << 14) | 8)  # ADD %o0, %o0, %o0
        #   JMPL %g0, %o7+4   — return to caller+4 (no delay slot in this simulator)
        #   SPARC ABI uses %o7+8 (assuming delay slot); we use %o7+4 since we
        #   have no delay slots, so CALL at 0x04 stores %o7=0x04, return is 0x08.
        struct.pack_into(">I", prog, 0x14,
            (0x2 << 30) | (0 << 25) | (0x38 << 19) | (15 << 14) | (1 << 13) | 4)  # JMPL %g0, %o7+4
        struct.pack_into(">I", prog, 0x18, 0x0100_0000)          # NOP

        sim = SPARCSimulator()
        result = sim.execute(bytes(prog))
        assert result.ok
        assert sim._get_reg(8) == 14    # %o0 = 7 * 2
        assert sim._get_reg(15) == 0x04  # %o7 = address of CALL

    def test_save_restore_preserves_globals(self):
        """SAVE then RESTORE preserves global registers %g1–%g7."""
        prog  = add_i(1, 0, 99)                # %g1 = 99
        prog += add_i(2, 0, 42)                # %g2 = 42
        prog += save_i(0, 0, 0)                # SAVE (window changes)
        prog += restore_i(0, 0, 0)             # RESTORE (back to window 0)
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(1) == 99    # %g1 unchanged across windows
        assert sim._get_reg(2) == 42    # %g2 unchanged across windows
