"""End-to-end program tests for AlphaSimulator.

Each test runs a small but complete Alpha AXP program and checks the result.
Programs are hand-assembled from Alpha instruction encodings.

Alpha branch target formula (no delay slots!):
  target = (PC_of_branch + 4) + sext(disp21) * 4

Alpha conventions used here:
  r1–r5   — scratch / arguments
  r26     — return address (ra)
  r27     — procedure value (pv)
  HALT    = 0x00000000 = call_pal 0
"""

from __future__ import annotations

import struct

from alpha_axp_simulator import AlphaSimulator

# ── Encoding helpers ──────────────────────────────────────────────────────────

def w32(v: int) -> bytes:
    return struct.pack("<I", v & 0xFFFF_FFFF)


HALT = w32(0x0000_0000)


def mov_i(rd: int, imm8: int) -> bytes:
    """BIS r31, imm8, rd — load 8-bit zero-extended immediate."""
    return w32((0x11 << 26) | (31 << 21) | ((imm8 & 0xFF) << 13) | (1 << 12) | (0x20 << 5) | rd)


def addq_rr(rc: int, ra: int, rb: int) -> bytes:
    """ADDQ ra, rb, rc."""
    return w32((0x10 << 26) | (ra << 21) | (rb << 16) | (0x20 << 5) | rc)


def addq_ri(rc: int, ra: int, lit: int) -> bytes:
    """ADDQ ra, #lit, rc."""
    return w32((0x10 << 26) | (ra << 21) | ((lit & 0xFF) << 13) | (1 << 12) | (0x20 << 5) | rc)


def subq_rr(rc: int, ra: int, rb: int) -> bytes:
    """SUBQ ra, rb, rc."""
    return w32((0x10 << 26) | (ra << 21) | (rb << 16) | (0x29 << 5) | rc)


def subq_ri(rc: int, ra: int, lit: int) -> bytes:
    """SUBQ ra, #lit, rc."""
    return w32((0x10 << 26) | (ra << 21) | ((lit & 0xFF) << 13) | (1 << 12) | (0x29 << 5) | rc)


def mulq_rr(rc: int, ra: int, rb: int) -> bytes:
    """MULQ ra, rb, rc (lower 64 bits)."""
    return w32((0x13 << 26) | (ra << 21) | (rb << 16) | (0x20 << 5) | rc)


def cmpeq_rr(rc: int, ra: int, rb: int) -> bytes:
    return w32((0x10 << 26) | (ra << 21) | (rb << 16) | (0x2D << 5) | rc)


def cmplt_rr(rc: int, ra: int, rb: int) -> bytes:
    """CMPLT ra, rb, rc — signed: rc = 1 if ra < rb."""
    return w32((0x10 << 26) | (ra << 21) | (rb << 16) | (0x4D << 5) | rc)


def cmple_rr(rc: int, ra: int, rb: int) -> bytes:
    return w32((0x10 << 26) | (ra << 21) | (rb << 16) | (0x6D << 5) | rc)


def stq(ra: int, rb: int, disp: int) -> bytes:
    """STQ ra, disp(rb)."""
    return w32((0x2D << 26) | (ra << 21) | (rb << 16) | (disp & 0xFFFF))


def ldq(ra: int, rb: int, disp: int) -> bytes:
    """LDQ ra, disp(rb)."""
    return w32((0x29 << 26) | (ra << 21) | (rb << 16) | (disp & 0xFFFF))


def stl(ra: int, rb: int, disp: int) -> bytes:
    return w32((0x2C << 26) | (ra << 21) | (rb << 16) | (disp & 0xFFFF))


def ldl(ra: int, rb: int, disp: int) -> bytes:
    return w32((0x28 << 26) | (ra << 21) | (rb << 16) | (disp & 0xFFFF))


def ldbu(ra: int, rb: int, disp: int) -> bytes:
    return w32((0x0A << 26) | (ra << 21) | (rb << 16) | (disp & 0xFFFF))


def stb(ra: int, rb: int, disp: int) -> bytes:
    return w32((0x0E << 26) | (ra << 21) | (rb << 16) | (disp & 0xFFFF))


def br(ra: int, disp21: int) -> bytes:
    return w32((0x30 << 26) | (ra << 21) | (disp21 & 0x1F_FFFF))


def bsr(ra: int, disp21: int) -> bytes:
    return w32((0x34 << 26) | (ra << 21) | (disp21 & 0x1F_FFFF))


def beq(ra: int, disp21: int) -> bytes:
    return w32((0x39 << 26) | (ra << 21) | (disp21 & 0x1F_FFFF))


def bne(ra: int, disp21: int) -> bytes:
    return w32((0x3D << 26) | (ra << 21) | (disp21 & 0x1F_FFFF))


def blt(ra: int, disp21: int) -> bytes:
    return w32((0x3A << 26) | (ra << 21) | (disp21 & 0x1F_FFFF))


def ble(ra: int, disp21: int) -> bytes:
    return w32((0x3B << 26) | (ra << 21) | (disp21 & 0x1F_FFFF))


def bgt(ra: int, disp21: int) -> bytes:
    return w32((0x3F << 26) | (ra << 21) | (disp21 & 0x1F_FFFF))


def ret(rb: int = 26) -> bytes:
    """RET r31, (rb) — return via register."""
    return w32((0x1A << 26) | (31 << 21) | (rb << 16) | (0x02 << 14))


def jsr(ra: int, rb: int) -> bytes:
    """JSR ra, (rb) — call indirect."""
    return w32((0x1A << 26) | (ra << 21) | (rb << 16) | (0x01 << 14))


def bis_rr(rc: int, ra: int, rb: int) -> bytes:
    """BIS ra, rb, rc — OR (MOV register-to-register)."""
    return w32((0x11 << 26) | (ra << 21) | (rb << 16) | (0x20 << 5) | rc)


def umulh_rr(rc: int, ra: int, rb: int) -> bytes:
    return w32((0x13 << 26) | (ra << 21) | (rb << 16) | (0x30 << 5) | rc)


# ── Helper: disp for branch at src_addr targeting dst_addr ───────────────────

def disp(src_addr: int, dst_addr: int) -> int:
    """Compute 21-bit branch displacement for Alpha branch instructions.

    Alpha branch target = (PC_of_branch + 4) + disp * 4
    So: disp = (dst_addr - (src_addr + 4)) // 4
    """
    d = (dst_addr - (src_addr + 4)) // 4
    return d & 0x1F_FFFF


# ── Sum 1..N ──────────────────────────────────────────────────────────────────

class TestSum1ToN:
    """Compute sum = 1 + 2 + ... + 10 = 55."""

    def test_sum_1_to_10(self):
        # Registers:
        #   r1 = accumulator (sum)
        #   r2 = counter (1 to 10)
        #   r3 = limit (10)
        #
        # addr  0: mov r1, 0       # sum = 0
        # addr  4: mov r2, 1       # counter = 1
        # addr  8: mov r3, 10      # limit = 10
        # addr  C: addq r1, r2, r1 # sum += counter   ← loop top
        # addr 10: addq r2, 1, r2  # counter++
        # addr 14: cmple r2, r3, r4 # r4 = (counter <= limit)
        # addr 18: bne  r4, -3     # if r4 != 0, branch back to addr C
        #          (disp = (0xC - (0x18 + 4)) / 4 = (0xC - 0x1C) / 4 = -4 → 0x1FFFFC)
        # addr 1C: HALT

        prog  = mov_i(1, 0)                    # 0x00
        prog += mov_i(2, 1)                    # 0x04
        prog += mov_i(3, 10)                   # 0x08
        prog += addq_rr(1, 1, 2)              # 0x0C  loop top
        prog += addq_ri(2, 2, 1)              # 0x10
        prog += cmple_rr(4, 2, 3)            # 0x14  r4 = (counter <= 10)
        prog += bne(4, disp(0x18, 0x0C))     # 0x18
        prog += HALT                           # 0x1C

        result = AlphaSimulator().execute(prog)
        assert result.ok
        assert result.final_state.regs[1] == 55


# ── Factorial ─────────────────────────────────────────────────────────────────

class TestFactorial:
    """Compute n! iteratively. Test 5! = 120."""

    def test_factorial_5(self):
        # r1 = n = 5
        # r2 = result = 1
        # r3 = scratch (compare result)
        #
        # addr  0: mov r1, 5
        # addr  4: mov r2, 1
        # addr  8: cmpeq r1, r31, r3   # r3 = (r1 == 0)?
        # addr  C: bne r3, end          # if r1==0 jump to HALT
        # addr 10: mulq r2, r1, r2      # result *= n
        # addr 14: subq r1, 1, r1       # n--
        # addr 18: br r31, -4           # back to cmpeq at addr 8
        #          (disp = (8 - (0x18+4)) / 4 = (8 - 28) / 4 = -5 → 0x1FFFFB)
        # addr 1C: HALT  ← end

        prog  = mov_i(1, 5)                    # 0x00
        prog += mov_i(2, 1)                    # 0x04
        prog += cmpeq_rr(3, 1, 31)            # 0x08  r3 = (r1==0)
        prog += bne(3, disp(0x0C, 0x1C))      # 0x0C  if zero → done
        prog += mulq_rr(2, 2, 1)              # 0x10
        prog += subq_ri(1, 1, 1)              # 0x14  n--
        prog += br(31, disp(0x18, 0x08))      # 0x18  → cmpeq
        prog += HALT                           # 0x1C

        result = AlphaSimulator().execute(prog)
        assert result.ok
        assert result.final_state.regs[2] == 120


# ── Fibonacci ─────────────────────────────────────────────────────────────────

class TestFibonacci:
    """Compute Fibonacci(10) = 55 iteratively."""

    def test_fib_10(self):
        # r1 = a = 0, r2 = b = 1, r3 = n = 10, r4 = temp
        # Loop: temp = b; b = a + b; a = temp; n--; if n > 0 loop
        #
        # addr  0: mov r1, 0
        # addr  4: mov r2, 1
        # addr  8: mov r3, 10
        # addr  C: bis r4, r31, r2    # temp = b  (BIS r2, r31, r4)
        # addr 10: addq r1, r2, r2   # b = a + b (new b)
        # addr 14: bis r1, r31, r4   # a = temp  (BIS r4, r31, r1)
        # addr 18: subq r3, 1, r3    # n--
        # addr 1C: bgt r3, -5        # if n > 0 loop back to addr C
        #          (disp = (0xC - (0x1C+4)) / 4 = (0xC - 0x20) / 4 = -5 → 0x1FFFFB)
        # addr 20: HALT   → r1 = fib(10) = 55

        prog  = mov_i(1, 0)                    # 0x00
        prog += mov_i(2, 1)                    # 0x04
        prog += mov_i(3, 10)                   # 0x08
        prog += bis_rr(4, 2, 31)              # 0x0C  temp = b
        prog += addq_rr(2, 1, 2)              # 0x10  b = a + b
        prog += bis_rr(1, 4, 31)              # 0x14  a = temp
        prog += subq_ri(3, 3, 1)              # 0x18  n--
        prog += bgt(3, disp(0x1C, 0x0C))      # 0x1C
        prog += HALT                           # 0x20

        result = AlphaSimulator().execute(prog)
        assert result.ok
        assert result.final_state.regs[1] == 55


# ── Byte copy ─────────────────────────────────────────────────────────────────

class TestByteCopy:
    """Copy N bytes from one memory region to another."""

    def test_copy_8_bytes(self):
        # Source at 0x200, destination at 0x300, count = 8
        # r1 = src ptr, r2 = dst ptr, r3 = count
        # Loop:
        #   LDBU r4, 0(r1)
        #   STB  r4, 0(r2)
        #   ADDQ r1, 1, r1
        #   ADDQ r2, 1, r2
        #   SUBQ r3, 1, r3
        #   BNE  r3, loop
        #
        # addr  0: mov r3, 8     # count = 8
        # addr  4: ldbu r4, 0(r1)    ← loop top
        # addr  8: stb  r4, 0(r2)
        # addr  C: addq r1, 1, r1
        # addr 10: addq r2, 1, r2
        # addr 14: subq r3, 1, r3
        # addr 18: bne r3, -5    (disp = (4 - (0x18+4)) / 4 = (4 - 28)/4 = -6 → 0x1FFFFA)
        #          Wait: disp = (0x4 - 0x1C) / 4 = -6 → 0x1FFFFA
        # addr 1C: HALT

        src_base = 0x200
        dst_base = 0x300
        src_data = bytes(range(8))

        prog  = mov_i(3, 8)                    # 0x00 count = 8
        prog += ldbu(4, 1, 0)                  # 0x04 loop top
        prog += stb(4, 2, 0)                   # 0x08
        prog += addq_ri(1, 1, 1)              # 0x0C
        prog += addq_ri(2, 2, 1)              # 0x10
        prog += subq_ri(3, 3, 1)              # 0x14
        prog += bne(3, disp(0x18, 0x04))      # 0x18
        prog += HALT                           # 0x1C

        sim = AlphaSimulator()
        sim.load(prog)
        sim._regs[1] = src_base
        sim._regs[2] = dst_base
        for i, b in enumerate(src_data):
            sim._mem[src_base + i] = b

        while not sim._halted:
            sim.step()

        for i, b in enumerate(src_data):
            assert sim._mem[dst_base + i] == b


# ── Subroutine call / return ──────────────────────────────────────────────────

class TestSubroutine:
    """Call a subroutine that doubles its argument via BSR/RET."""

    def test_double_via_subroutine(self):
        # Caller:
        #   addr  0: mov r1, 21           # arg = 21
        #   addr  4: BSR r26, +1          # call double at addr C; r26=8
        #   addr  8: HALT
        # double (addr C):
        #   addr  C: addq r1, r1, r1      # r1 = r1 * 2
        #   addr 10: ret (r26)            # return
        #
        # BSR at addr 4: target = (4+4) + disp*4 = 8 + disp*4 = 0xC
        # → disp = (0xC - 8) / 4 = 1

        prog  = mov_i(1, 21)               # 0x00
        prog += bsr(26, disp(0x04, 0x0C)) # 0x04  BSR r26, → addr C; r26 = 8
        prog += HALT                       # 0x08
        prog += addq_rr(1, 1, 1)          # 0x0C  r1 = r1 * 2
        prog += ret(26)                    # 0x10  RET r31, (r26)

        result = AlphaSimulator().execute(prog)
        assert result.ok
        assert result.final_state.regs[1] == 42
        assert result.final_state.regs[26] == 8   # saved return addr


# ── Quadword array sum ────────────────────────────────────────────────────────

class TestArraySum:
    """Sum an array of 4 quadwords stored in memory."""

    def test_sum_quad_array(self):
        # Array at 0x200: [10, 20, 30, 40] (quadwords, little-endian)
        # r1 = base ptr, r2 = count=4, r3 = accumulator
        #
        # addr  0: mov r2, 4
        # addr  4: ldq r4, 0(r1)       ← loop
        # addr  8: addq r3, r4, r3
        # addr  C: addq r1, 8, r1
        # addr 10: subq r2, 1, r2
        # addr 14: bne r2, -4          disp = (4 - (0x14+4)) / 4 = (4 - 0x18) / 4 = -5 → 0x1FFFFB
        # addr 18: HALT

        array_base = 0x200
        values = [10, 20, 30, 40]

        prog  = mov_i(2, 4)                    # 0x00 count = 4
        prog += ldq(4, 1, 0)                   # 0x04 loop top
        prog += addq_rr(3, 3, 4)              # 0x08
        prog += addq_ri(1, 1, 8)              # 0x0C  ptr += 8
        prog += subq_ri(2, 2, 1)              # 0x10
        prog += bne(2, disp(0x14, 0x04))      # 0x14
        prog += HALT                           # 0x18

        sim = AlphaSimulator()
        sim.load(prog)
        sim._regs[1] = array_base
        sim._regs[3] = 0
        for i, v in enumerate(values):
            addr = array_base + i * 8
            for j in range(8):
                sim._mem[addr + j] = (v >> (j * 8)) & 0xFF

        while not sim._halted:
            sim.step()

        assert sim._get_reg(3) == 100   # 10+20+30+40


# ── UMULH for 128-bit product ─────────────────────────────────────────────────

class TestUMULH:
    """UMULH computes the upper 64 bits of an unsigned 128-bit product."""

    def test_umulh_large_product(self):
        # 0xFFFFFFFFFFFFFFFF * 0xFFFFFFFFFFFFFFFF
        # = (2^64 - 1)^2 = 2^128 - 2*2^64 + 1
        # Upper 64 bits = 2^64 - 2 = 0xFFFFFFFFFFFFFFFE
        sim = AlphaSimulator()
        prog = umulh_rr(3, 1, 2) + HALT
        sim.load(prog)
        sim._regs[1] = 0xFFFF_FFFF_FFFF_FFFF
        sim._regs[2] = 0xFFFF_FFFF_FFFF_FFFF
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0xFFFF_FFFF_FFFF_FFFE

    def test_umulh_simple_case(self):
        """2^32 * 2^32 = 2^64 → upper 64 bits = 1."""
        sim = AlphaSimulator()
        prog = umulh_rr(3, 1, 2) + HALT
        sim.load(prog)
        sim._regs[1] = 1 << 32
        sim._regs[2] = 1 << 32
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 1


# ── Bubble sort ───────────────────────────────────────────────────────────────

class TestBubbleSort:
    """Bubble-sort 4 quadwords in memory."""

    def test_bubble_sort_4_quads(self):
        # Array at 0x200: [40, 10, 30, 20] → sorted [10, 20, 30, 40]
        #
        # Simple selection-sort variant (easier to hand-encode):
        #   for i in range(4):
        #     min_idx = i
        #     for j in range(i+1, 4):
        #       if arr[j] < arr[min_idx]: min_idx = j
        #     swap arr[i], arr[min_idx]
        #
        # For simplicity, do a fixed 4-element bubble sort with known pass count.
        # We'll encode 3 passes of compare-and-swap on adjacent pairs.
        #
        # A simpler approach: encode the comparisons explicitly as a straight-line
        # sequence of compare/swap pairs for a 4-element network sort.
        #
        # Sorting network for 4 elements: (0,1), (2,3), (0,2), (1,3), (1,2)
        # 5 compare-and-swap pairs.

        array_base = 0x200

        def cmp_swap(i: int, j: int, addr: int) -> tuple[bytes, int]:
            """Emit compare-and-swap of array[i] and array[j].

            Registers: r10=base, r11=a[i], r12=a[j], r13=scratch
            Returns (bytes, next_addr).
            """
            instr  = ldq(11, 10, i * 8)       # r11 = a[i]
            instr += ldq(12, 10, j * 8)       # r12 = a[j]
            instr += cmplt_rr(13, 12, 11)     # r13 = (a[j] < a[i])
            # beq r13, skip (disp=2, skip the two stores)
            instr += beq(13, 2)               # if not (a[j]<a[i]), skip swap
            instr += stq(12, 10, i * 8)       # a[i] = a[j]
            instr += stq(11, 10, j * 8)       # a[j] = a[i]
            return instr, addr + len(instr)

        prog = b""
        pairs = [(0, 1), (2, 3), (0, 2), (1, 3), (1, 2)]
        for i, j in pairs:
            chunk, _ = cmp_swap(i, j, 0)
            prog += chunk
        prog += HALT

        sim = AlphaSimulator()
        sim.load(prog)
        sim._regs[10] = array_base
        values = [40, 10, 30, 20]
        for idx, v in enumerate(values):
            addr = array_base + idx * 8
            for k in range(8):
                sim._mem[addr + k] = (v >> (k * 8)) & 0xFF

        while not sim._halted:
            sim.step()

        result = []
        for idx in range(4):
            addr = array_base + idx * 8
            val = 0
            for k in range(8):
                val |= sim._mem[addr + k] << (k * 8)
            result.append(val)

        assert result == [10, 20, 30, 40]


# ── Multi-instruction sequence: multiply-accumulate ───────────────────────────

class TestMultiplyAccumulate:
    """Dot product: sum(a[i] * b[i]) for i in 0..3."""

    def test_dot_product(self):
        # a = [1, 2, 3, 4], b = [4, 3, 2, 1] → dot = 4+6+6+4 = 20
        a = [1, 2, 3, 4]
        b = [4, 3, 2, 1]
        a_base = 0x400
        b_base = 0x480

        # r1 = a_ptr, r2 = b_ptr, r3 = count=4, r4 = accum
        # Loop:
        #   ldq r5, 0(r1); ldq r6, 0(r2)
        #   mulq r5, r6, r5; addq r4, r5, r4
        #   addq r1, 8, r1; addq r2, 8, r2; subq r3, 1, r3
        #   bne r3, loop
        # addr  0: mov r3, 4
        # addr  4: ldq r5, 0(r1)    ← loop top
        # addr  8: ldq r6, 0(r2)
        # addr  C: mulq r5, r6, r5
        # addr 10: addq r4, r5, r4
        # addr 14: addq r1, 8, r1
        # addr 18: addq r2, 8, r2
        # addr 1C: subq r3, 1, r3
        # addr 20: bne r3, -8   disp=(4 - (0x20+4))/4 = (4-36)/4 = -8 → 0x1FFFF8
        # addr 24: HALT

        prog  = mov_i(3, 4)                    # 0x00
        prog += ldq(5, 1, 0)                   # 0x04
        prog += ldq(6, 2, 0)                   # 0x08
        prog += mulq_rr(5, 5, 6)              # 0x0C
        prog += addq_rr(4, 4, 5)              # 0x10
        prog += addq_ri(1, 1, 8)              # 0x14
        prog += addq_ri(2, 2, 8)              # 0x18
        prog += subq_ri(3, 3, 1)              # 0x1C
        prog += bne(3, disp(0x20, 0x04))      # 0x20
        prog += HALT                           # 0x24

        sim = AlphaSimulator()
        sim.load(prog)
        sim._regs[1] = a_base
        sim._regs[2] = b_base
        sim._regs[4] = 0

        for idx, v in enumerate(a):
            addr = a_base + idx * 8
            for k in range(8):
                sim._mem[addr + k] = (v >> (k * 8)) & 0xFF
        for idx, v in enumerate(b):
            addr = b_base + idx * 8
            for k in range(8):
                sim._mem[addr + k] = (v >> (k * 8)) & 0xFF

        while not sim._halted:
            sim.step()

        assert sim._get_reg(4) == 20
