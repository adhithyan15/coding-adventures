"""End-to-end Z80 program tests.

Each test loads and runs a complete Z80 program that exercises
multiple instructions working together.  Programs are short enough
that the code bytes do not overlap with data addresses.

Data lives at addresses ≥ 0x1000 to avoid overlap with code.
"""

from z80_simulator import Z80Simulator

# ── Sum 1..N using DJNZ ───────────────────────────────────────────────────────

class TestSumProgram:
    def test_sum_1_to_5(self):
        """Sum = 1+2+3+4+5 = 15.

        Algorithm::
            LD B, 5        ; loop counter (count down)
            LD HL, 0       ; accumulator in HL
            LOOP:
              LD A, B
              ADD A, L
              LD L, A
              LD A, H
              ADC A, 0
              LD H, A
              DJNZ LOOP
            LD A, L        ; result in A
            HALT
        """
        sim = Z80Simulator()
        prog = bytes([
            0x06, 0x05,             # LD B, 5
            0x21, 0x00, 0x00,       # LD HL, 0
            # LOOP at offset 5:
            0x78,                   # LD A, B       (offset 5)
            0x85,                   # ADD A, L
            0x6F,                   # LD L, A
            0x7C,                   # LD A, H
            0xCE, 0x00,             # ADC A, 0
            0x67,                   # LD H, A
            0x10, 0xF7,             # DJNZ -9  → offset 5 (5 - 14 = -9 = 0xF7)
            0x7D,                   # LD A, L
            0x76,                   # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 15

    def test_sum_1_to_10(self):
        """Sum = 1+2+...+10 = 55."""
        sim = Z80Simulator()
        prog = bytes([
            0x06, 0x0A,             # LD B, 10
            0x21, 0x00, 0x00,       # LD HL, 0
            0x78,                   # LD A, B      (offset 5)
            0x85,                   # ADD A, L
            0x6F,                   # LD L, A
            0x7C,                   # LD A, H
            0xCE, 0x00,             # ADC A, 0
            0x67,                   # LD H, A
            0x10, 0xF7,             # DJNZ → offset 5
            0x7D,                   # LD A, L
            0x76,                   # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 55


# ── Factorial using multiplication loop ───────────────────────────────────────

class TestFactorialProgram:
    def test_factorial_5(self):
        """5! = 120.

        Algorithm (uses 16-bit HL for product):
            LD B, 5         ; count
            LD HL, 1        ; product
            LOOP:
              LD DE, HL
              LD A, B
              LD C, 0
              MUL: ADD HL, DE; DEC A; JR NZ, MUL  ; multiply HL by B
              actually simpler: repeated addition
        Instead, use a tight loop: multiply HL by B via repeated addition.

        Simpler: compute 5! with a loop using LD A / MULTIPLY via BC.
        We'll use the straightforward: result = 1*2*3*4*5 step by step.

        Actually for simplicity let's just do it iteratively:
            LD A, 1         ; product in A (fits in 8 bits since 5!=120)
            LD B, 2         ; multiplier starts at 2
            OUTER:
              LD C, A        ; C = product
              LD A, 0        ; A = 0 (accumulate inner)
              INNER:
                ADD A, C
                DEC B        ; Oops, this doesn't work cleanly.

        Let's use the simplest possible: repeated multiply by 2,3,4,5.
        """
        sim = Z80Simulator()
        # A = 1*2 = 2
        # A = A*3: add A to itself 2 more times
        # etc.  Too complex for inline bytecode.
        # Instead: use a pre-computed 5! = 120 by running
        # LD A, 1; ADD A, A (×1); ... manual multiplication
        # Simplest: compute 1*2 = 2, 2*3 = 6, 6*4 = 24, 24*5 = 120
        # Each multiplication by N = add A to itself (N-1) times.
        # For small N this is practical.
        prog = bytes([
            # LD A,1; multiply by 2: ADD A,A
            0x3E, 0x01,             # LD A, 1
            # × 2: add A to itself 1 time
            0x87,                   # ADD A, A  → A=2
            # × 3: save A in C; add C twice (A = A+C+C = 2*3 = 6)
            0x4F,                   # LD C, A   C=2
            0x81,                   # ADD A, C  A=4
            0x81,                   # ADD A, C  A=6
            # × 4: save A in C; add C 3 times (A = A+3C = 6 + 18 = 24)
            0x4F,                   # LD C, A   C=6
            0x81, 0x81, 0x81,       # ADD A, C × 3  A = 6+18 = 24
            # × 5: save A in C; add C 4 times
            0x4F,                   # LD C, A   C=24
            0x81, 0x81, 0x81, 0x81, # ADD A, C × 4  A = 24+96 = 120
            0x76,                   # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 120

    def test_factorial_4(self):
        """4! = 24."""
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x01,         # LD A, 1
            0x87,               # × 2  → A=2
            0x4F,               # LD C, A (2)
            0x81, 0x81,         # × 3  → A=6
            0x4F,               # LD C, A (6)
            0x81, 0x81, 0x81,   # × 4  → A=24
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 24


# ── Fibonacci using LDIR to shift window ─────────────────────────────────────

class TestFibonacciProgram:
    def test_first_8_fibonacci_in_memory(self):
        """Generate F(1)..F(8) = 1,1,2,3,5,8,13,21.

        Algorithm:
            Store running pair (prev, curr) at 0x1000 and 0x1001.
            Accumulate results at 0x2000.
        """
        sim = Z80Simulator()
        # Compute Fibonacci and store into 0x2000..0x2007
        # F(0)=0 (unused), F(1)=1, F(2)=1, F(3)=2, ...
        # We use two 8-bit registers: D=prev, E=curr
        prog = bytes([
            # Init
            0x16, 0x00,             # LD D, 0 (prev)
            0x1E, 0x01,             # LD E, 1 (curr)
            0x21, 0x00, 0x20,       # LD HL, 0x2000 (output ptr)
            0x06, 0x08,             # LD B, 8 (count)
            # LOOP (at offset 9):
            0x73,                   # LD (HL), E   store curr
            0x23,                   # INC HL
            0x7B,                   # LD A, E      A = curr
            0x82,                   # ADD A, D     A = prev+curr (next)
            0x53,                   # LD D, E      prev = curr
            0x5F,                   # LD E, A      curr = next
            0x10, 0xF8,             # DJNZ -8  → offset 9
            0x76,                   # HALT
        ])
        r = sim.execute(prog)
        m = r.final_state.memory
        expected = [1, 1, 2, 3, 5, 8, 13, 21]
        for i, val in enumerate(expected):
            assert m[0x2000 + i] == val, f"F({i+1}) = {val}, got {m[0x2000+i]}"


# ── Block copy using LDIR ─────────────────────────────────────────────────────

class TestBlockCopyProgram:
    def test_ldir_copies_string(self):
        """Copy 8 bytes from 0x1000 to 0x2000 using LDIR."""
        data = [0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x57, 0x6F, 0x72]  # "HelloWor"
        sim = Z80Simulator()

        # Build program that sets up source data then LDIR
        prog = []
        # Store 8 bytes at 0x1000 via direct LD operations
        for i, b in enumerate(data):
            # LD A, b; LD (0x1000+i), A
            prog += [0x3E, b, 0x32, (0x1000 + i) & 0xFF, (0x1000 + i) >> 8]
        prog += [
            0x21, 0x00, 0x10,   # LD HL, 0x1000
            0x11, 0x00, 0x20,   # LD DE, 0x2000
            0x01, 0x08, 0x00,   # LD BC, 8
            0xED, 0xB0,         # LDIR
            0x76,
        ]
        r = sim.execute(bytes(prog))
        m = r.final_state.memory
        for i, b in enumerate(data):
            assert m[0x2000 + i] == b


# ── Byte search using CPIR ────────────────────────────────────────────────────

class TestSearchProgram:
    def test_find_byte_in_array(self):
        """Find 0x42 in an 8-byte array; verify HL points past the found byte."""
        data = [0x11, 0x22, 0x33, 0x42, 0x55, 0x66, 0x77, 0x88]
        sim = Z80Simulator()

        prog = []
        for i, b in enumerate(data):
            prog += [0x3E, b, 0x32, (0x1000 + i) & 0xFF, (0x1000 + i) >> 8]
        prog += [
            0x3E, 0x42,         # LD A, 0x42 (search target)
            0x21, 0x00, 0x10,   # LD HL, 0x1000
            0x01, 0x08, 0x00,   # LD BC, 8
            0xED, 0xB1,         # CPIR
            0x76,
        ]
        r = sim.execute(bytes(prog))
        assert r.final_state.flag_z is True   # found
        # 0x42 was at index 3 (0x1003); CPIR increments HL after each check
        assert r.final_state.hl == 0x1004


# ── Counter with JR loop ──────────────────────────────────────────────────────

class TestCounterProgram:
    def test_count_down_to_zero(self):
        """Count A from 10 down to 0 using JR NZ."""
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x0A,     # LD A, 10        (offset 0)
            # LOOP (offset 2):
            0x3D,           # DEC A
            0x20, 0xFD,     # JR NZ, -3  → offset 2
            0x76,           # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0
        assert r.final_state.flag_z is True


# ── Bitwise operations program ────────────────────────────────────────────────

class TestBitwiseProgram:
    def test_extract_high_nibble(self):
        """Extract the high nibble of 0xAB → 0x0A using AND + RRCA × 4."""
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0xAB,     # LD A, 0xAB
            0xE6, 0xF0,     # AND 0xF0  → A=0xA0
            # Shift right 4 times (RRCA doesn't affect other flags)
            0x0F, 0x0F, 0x0F, 0x0F,  # RRCA × 4
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0x0A

    def test_extract_low_nibble(self):
        """Extract the low nibble of 0xAB → 0x0B using AND."""
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0xAB,     # LD A, 0xAB
            0xE6, 0x0F,     # AND 0x0F → A=0x0B
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0x0B
