"""End-to-end program tests for the Intel 8051 simulator.

Each test encodes a complete small program in 8051 machine code and verifies
the final CPU/memory state.  These tests exercise the instructions working
together rather than in isolation.

8051 registers used by programs:
  R0–R7  — working registers (bank 0, addresses 0x00–0x07)
  A      — accumulator (SFR 0xE0)
  B      — secondary register (SFR 0xF0)
  DPTR   — 16-bit data pointer (SFR 0x82/0x83)
  SP     — stack pointer (SFR 0x81), reset value 0x07

Label resolution: all programs here use absolute addresses because the
machine code is assembled by hand.  Branch offsets are signed 8-bit values
relative to PC-after-instruction.  For a 2-byte branch instruction (opcode +
rel) at address A, target = (A + 2) + rel.
"""

from __future__ import annotations

from intel8051_simulator import I8051Simulator
from intel8051_simulator.state import SFR_ACC, SFR_PSW, SFR_SP

HALT = bytes([0xA5])


def run(prog: bytes) -> I8051Simulator:
    sim = I8051Simulator()
    sim.execute(prog)
    return sim


# ── Arithmetic programs ───────────────────────────────────────────────────────

class TestSumProgram:
    def test_sum_1_to_10(self):
        """Compute 1+2+…+10 = 55 using DJNZ.

        Register layout:
          R0 = counter (10 down to 1)
          A  = running sum (starts 0)

        Program (each line is one instruction):
          0x00: MOV R0, #10      (0x78 0x0A)
          0x02: ADD A, R0        (0x28)        — A += R0
          0x03: DJNZ R0, -3      (0xD8 0xFD)  — R0--; if R0≠0 goto 0x02
          0x05: HALT             (0xA5)

        PC after DJNZ fetch = 0x05; rel=0xFD=-3; target=0x05-3=0x02 ✓
        """
        prog = bytes([
            0x78, 0x0A,    # 0x00: MOV R0, #10
            0x28,          # 0x02: ADD A, R0
            0xD8, 0xFD,    # 0x03: DJNZ R0, -3 (to 0x02)
            0xA5,          # 0x05: HALT
        ])
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 55

    def test_sum_bytes_in_iram(self):
        """Sum 4 bytes stored at iram[0x30–0x33].

        Uses indirect addressing: R0 as pointer, R1 as counter.

        Layout:
          0x00: MOV R0, #0x30     — pointer
          0x02: MOV R1, #4        — counter
          0x04: ADD A, @R0        — A += [R0]
          0x05: INC R0            — advance pointer
          0x06: DJNZ R1, -4      — R1--; if ≠0 goto 0x04
          0x08: HALT
        """
        prog = bytes([
            0x78, 0x30,    # 0x00: MOV R0, #0x30
            0x79, 0x04,    # 0x02: MOV R1, #4
            0x26,          # 0x04: ADD A, @R0
            0x08,          # 0x05: INC R0
            0xD9, 0xFC,    # 0x06: DJNZ R1, -4 (to 0x04)
            0xA5,          # 0x08: HALT
        ])
        sim = I8051Simulator()
        sim.load(prog)
        # Set iram data AFTER load() (load calls reset which zeroes iram)
        sim._iram[0x30] = 10
        sim._iram[0x31] = 20
        sim._iram[0x32] = 30
        sim._iram[0x33] = 40
        # Step through manually (don't call execute() which would call load/reset again)
        while not sim._halted:
            sim.step()
        assert sim._iram[SFR_ACC] == 100


class TestMultiply:
    def test_multiply_by_repeated_addition(self):
        """Multiply 7 × 6 = 42 using repeated addition.

        R0 = multiplier (6)
        A  = result, start 0
        Loop: A += 7; DJNZ R0 back

          0x00: MOV R0, #6
          0x02: ADD A, #7
          0x04: DJNZ R0, -4  (target: 0x04+2-4=0x02)
          0x06: HALT
        """
        prog = bytes([
            0x78, 0x06,    # MOV R0, #6
            0x24, 0x07,    # ADD A, #7
            0xD8, 0xFC,    # DJNZ R0, -4
            0xA5,
        ])
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 42

    def test_mul_ab_factorial_5(self):
        """Compute 5! = 120 using MUL AB.

        Strategy: accumulate in B, then move to A each step.
          A=1*2=2; A=2*3=6; A=6*4=24; A=24*5=120
        But MUL AB does B:A = A*B, and our result fits in A (120 < 256).

          0x00: MOV A, #1
          0x02: MOV B, #2; MUL AB  → A=2, B=0
          0x06: MOV B, #3; MUL AB  → A=6
          0x0A: MOV B, #4; MUL AB  → A=24
          0x0E: MOV B, #5; MUL AB  → A=120
          0x12: HALT
        """
        from intel8051_simulator.state import SFR_B as B_SFR
        prog = bytes([
            0x74, 0x01,              # MOV A, #1
            0x75, B_SFR, 0x02, 0xA4, # MOV B,#2; MUL AB
            0x75, B_SFR, 0x03, 0xA4, # MOV B,#3; MUL AB
            0x75, B_SFR, 0x04, 0xA4, # MOV B,#4; MUL AB
            0x75, B_SFR, 0x05, 0xA4, # MOV B,#5; MUL AB
            0xA5,
        ])
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 120


class TestStringCopy:
    def test_copy_bytes_via_xdata(self):
        """Copy 4 bytes from xdata[0x100] to xdata[0x200].

        Uses MOVX with @Ri (R0=src, R1=dst, R2=count).
        R0 and R1 are 8-bit, so src/dst must be in 0x00–0xFF.
        Use xdata[0x00–0x03] → xdata[0x10–0x13].

          0x00: MOV R0, #0x00       — src pointer
          0x02: MOV R1, #0x10       — dst pointer
          0x04: MOV R2, #4          — count
          0x06: MOVX A, @R0         — A = xdata[R0]
          0x07: MOVX @R1, A         — xdata[R1] = A
          0x08: INC R0
          0x09: INC R1
          0x0A: DJNZ R2, -6  (to 0x06)
          0x0C: HALT
        """
        prog = bytes([
            0x78, 0x00,    # MOV R0, #0
            0x79, 0x10,    # MOV R1, #0x10
            0x7A, 0x04,    # MOV R2, #4
            0xE2,          # MOVX A, @R0
            0xF3,          # MOVX @R1, A
            0x08,          # INC R0
            0x09,          # INC R1
            0xDA, 0xFA,    # DJNZ R2, -6 (PC=0x0C; 0x0C-6=0x06)
            0xA5,
        ])
        sim = I8051Simulator()
        sim.load(prog)
        for i in range(4):
            sim._xdata[i] = 0x10 + i
        sim.execute(prog)
        for i in range(4):
            assert sim._xdata[0x10 + i] == 0x10 + i


class TestSubroutinePrograms:
    def test_simple_subroutine(self):
        """Call a subroutine that doubles A.

        Main at 0x00; subroutine at 0x10.
          0x00: MOV A, #7
          0x02: LCALL 0x10
          0x05: HALT
          (skips 0x05–0x0F)
          0x10: ADD A, A  — but ADD A,A not in ISA; use ADD A,Rn
                Actually: MOV R0,A; ADD A,R0  → double
          0x10: MOV R0, A    (0xF8)
          0x11: ADD A, R0   (0x28)
          0x12: RET
        """
        prog = bytearray(0x20)
        prog[0x00] = 0x74; prog[0x01] = 0x07   # MOV A, #7
        prog[0x02] = 0x12; prog[0x03] = 0x00; prog[0x04] = 0x10  # LCALL 0x10
        prog[0x05] = 0xA5   # HALT
        prog[0x10] = 0xF8   # MOV R0, A
        prog[0x11] = 0x28   # ADD A, R0
        prog[0x12] = 0x22   # RET
        sim = I8051Simulator()
        result = sim.execute(bytes(prog))
        assert result.ok
        assert sim._iram[SFR_ACC] == 14

    def test_stack_balance(self):
        """Nested call/return must leave SP at initial value (0x07)."""
        prog = bytearray(0x30)
        # Outer call
        prog[0x00] = 0x12; prog[0x01] = 0x00; prog[0x02] = 0x10
        prog[0x03] = 0xA5   # HALT
        # Subroutine at 0x10: calls inner at 0x20, returns
        prog[0x10] = 0x12; prog[0x11] = 0x00; prog[0x12] = 0x20
        prog[0x13] = 0x22   # RET
        # Inner at 0x20
        prog[0x20] = 0x04   # INC A
        prog[0x21] = 0x22   # RET
        sim = I8051Simulator()
        result = sim.execute(bytes(prog))
        assert result.ok
        assert sim._iram[SFR_SP] == 0x07


class TestFibonacci:
    def test_fibonacci_8_terms(self):
        """Store first 8 Fibonacci numbers in iram[0x30–0x37].

        F = 1,1,2,3,5,8,13,21

        Strategy: use R0/R1 as current/next; R2 as loop counter.
          Store results to increasing addresses.

          0x00: MOV 0x30, #1    — F[0]=1
          0x03: MOV 0x31, #1    — F[1]=1
          0x06: MOV R0, #0x30   — pointer to prev-prev
          0x08: MOV R1, #0x31   — pointer to prev
          0x0A: MOV R2, #6      — 6 more terms
          Loop at 0x0C:
          0x0C: MOV A, @R0      — A = F[i-2]
          0x0D: ADD A, @R1      — A += F[i-1] = F[i]
          0x0E: INC R0          — advance prev-prev pointer
          0x0F: INC R1          — advance prev pointer
          0x10: MOV @R1, A      — store F[i] (R1 now points at F[i])
                Wait: R1 was at F[i-1] and we incremented it, so it now points
                at F[i] (the next slot) — correct!
          0x11: DJNZ R2, -7  (to 0x0C)
          0x13: HALT
        """
        prog = bytes([
            0x75, 0x30, 0x01,   # 0x00: MOV 0x30, #1
            0x75, 0x31, 0x01,   # 0x03: MOV 0x31, #1
            0x78, 0x30,         # 0x06: MOV R0, #0x30
            0x79, 0x31,         # 0x08: MOV R1, #0x31
            0x7A, 0x06,         # 0x0A: MOV R2, #6
            # Loop top at 0x0C:
            0xE4,               # 0x0C: CLR A        — reset accumulator each iteration
            0x26,               # 0x0D: ADD A, @R0   — A = F[i-2]
            0x27,               # 0x0E: ADD A, @R1   — A = F[i-2] + F[i-1] = F[i]
            0x08,               # 0x0F: INC R0       — prev-prev pointer advances
            0x09,               # 0x10: INC R1       — prev pointer advances (now points at F[i] slot)
            0xF7,               # 0x11: MOV @R1, A   — store F[i]
            0xDA, 0xF8,         # 0x12: DJNZ R2, -8 (PC=0x14; 0x14-8=0x0C) ✓
            0xA5,               # 0x14: HALT
        ])
        sim = I8051Simulator()
        result = sim.execute(prog)
        assert result.ok
        expected = [1, 1, 2, 3, 5, 8, 13, 21]
        for i, v in enumerate(expected):
            assert sim._iram[0x30 + i] == v, f"F[{i}]={sim._iram[0x30+i]}, expected {v}"


class TestBCDAddition:
    def test_bcd_add_two_digits(self):
        """Add BCD 28 + 47 = 75 using DA A."""
        prog = bytes([0x74, 0x28, 0x24, 0x47, 0xD4]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x75   # BCD 75

    def test_bcd_add_with_carry_propagation(self):
        """Add BCD 56 + 78 = 134.  Result in A=0x34, CY=1."""
        prog = bytes([0x74, 0x56, 0x24, 0x78, 0xD4]) + HALT
        sim = run(prog)
        assert sim._iram[SFR_ACC] == 0x34
        assert sim._iram[SFR_PSW] & 0x80   # CY=1 → hundreds digit is 1


class TestLookupTable:
    def test_movc_lookup_table(self):
        """Read a 4-entry lookup table from code memory using MOVC A,@A+DPTR.

        Table at code[0x10]: [10, 20, 30, 40]
        Test: A=2; DPTR=0x10; MOVC A,@A+DPTR → A = code[0x12] = 30
        """
        prog = bytearray(0x20)
        prog[0x00] = 0x74; prog[0x01] = 0x02   # MOV A, #2
        prog[0x02] = 0x90; prog[0x03] = 0x00; prog[0x04] = 0x10  # MOV DPTR, #0x10
        prog[0x05] = 0x93   # MOVC A, @A+DPTR
        prog[0x06] = 0xA5   # HALT
        prog[0x10] = 10; prog[0x11] = 20; prog[0x12] = 30; prog[0x13] = 40
        sim = I8051Simulator()
        result = sim.execute(bytes(prog))
        assert result.ok
        assert sim._iram[SFR_ACC] == 30
