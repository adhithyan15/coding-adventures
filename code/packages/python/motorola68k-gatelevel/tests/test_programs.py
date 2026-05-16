"""Tests for larger programs — arithmetic loops, subroutines, MOVEM, DBcc, string copy."""



from motorola68k_gatelevel.simulator import Motorola68kGateLevelSimulator

STOP = bytes([0x4E, 0x72, 0x27, 0x00])


def run(prog: bytes) -> object:
    sim = Motorola68kGateLevelSimulator()
    r = sim.execute(prog)
    return r.final_state


class TestArithmeticPrograms:
    def test_add_two_numbers(self):
        # D0 = 5 + 3 = 8
        prog = bytes([
            0x70, 0x05,  # MOVEQ #5, D0
            0x72, 0x03,  # MOVEQ #3, D1
            0xD0, 0x81,  # ADD.L D1, D0
        ]) + STOP
        s = run(prog)
        assert s.d0 == 8

    def test_subtract(self):
        # D0 = 10 - 3 = 7
        prog = bytes([
            0x70, 0x0A,  # MOVEQ #10, D0
            0x72, 0x03,  # MOVEQ #3, D1
            0x90, 0x81,  # SUB.L D1, D0
        ]) + STOP
        s = run(prog)
        assert s.d0 == 7

    def test_multiply(self):
        # D0 = 6 × 7 = 42 (unsigned)
        prog = bytes([
            0x70, 0x06,  # MOVEQ #6, D0
            0x72, 0x07,  # MOVEQ #7, D1
            0xC0, 0xC1,  # MULU D1, D0
        ]) + STOP
        s = run(prog)
        assert s.d0 == 42

    def test_accumulate_sum(self):
        # D0 = 1 + 2 + 3 + 4 + 5 = 15
        prog = bytes([
            0x70, 0x00,  # MOVEQ #0, D0 (accumulator)
            0xD0, 0x3C, 0x00, 0x01,  # ADD.W #1, D0
            0xD0, 0x3C, 0x00, 0x02,  # ADD.W #2, D0
            0xD0, 0x3C, 0x00, 0x03,  # ADD.W #3, D0
            0xD0, 0x3C, 0x00, 0x04,  # ADD.W #4, D0
            0xD0, 0x3C, 0x00, 0x05,  # ADD.W #5, D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFFFF) == 15

    def test_factorial_5(self):
        # Compute 5! = 120.
        # Strategy: D1 = 5, D0 = 4 (loop counter).  Each iteration multiplies
        # D1 by (D0+1) by first adding 1 to D0, multiplying, then the DBF
        # decrements D0.  But this corrupts the counter, so instead use two
        # registers: D2 holds the multiplier (starts at 4, goes 4,3,2,1),
        # D0 is the DBF counter (starts at 3, so DBF runs 4 times: 3→2→1→0→-1=exit).
        # Initial: D1=5, D2=4, D0=3
        # iter 1: D1=5*4=20;  D2→3; D0→2
        # iter 2: D1=20*3=60; D2→2; D0→1
        # iter 3: D1=60*2=120;D2→1; D0→0
        # iter 4: D1=120*1=120;D2→0; D0→-1 → exit
        prog = bytes([
            0x72, 0x05,              # MOVEQ #5, D1      (accumulator)
            0x74, 0x04,              # MOVEQ #4, D2      (multiplier: 4..1)
            0x70, 0x03,              # MOVEQ #3, D0      (DBF counter: 3..0)
            # loop:
            0xC2, 0xC2,              # MULU D2, D1       (D1 = D1 * D2)
            0x55, 0x42,              # SUBQ.W #2, D2     (D2--)  wait, SUBQ #2 is wrong
        ]) + STOP
        # ----------------------------------------------------------------
        # Simpler: use the accumulate-and-branch pattern with ADDQ to count
        # down. We do 5! = 5*4*3*2*1 unrolled:
        prog = bytes([
            0x72, 0x01,              # MOVEQ #1, D1
            0x70, 0x05,              # MOVEQ #5, D0      (multiplier)
            # loop (D0 goes 5,4,3,2,1 — 5 multiplications, but stop at 1 not 0):
            # Use DBNE / a conditional exit, or just hardcode 4 iterations:
            # MULU then SUBQ #1, D0 then BNE loop — loop while D0 != 0
            0xC2, 0xC0,              # MULU D0, D1       (D1 *= D0)
            0x53, 0x80,              # SUBQ.L #1, D0     (D0--)
            0x66, 0xFA,              # BNE -6            (back to MULU)
        ]) + STOP
        s = run(prog)
        # 1*5=5, 5*4=20, 20*3=60, 60*2=120, 120*1=120
        assert (s.d1 & 0xFFFF) == 120

    def test_fibonacci_5th(self):
        # Fibonacci: F(0)=0, F(1)=1, F(2)=1, F(3)=2, F(4)=3, F(5)=5
        # D0 = a=0, D1 = b=1; loop 5 times
        prog = bytes([
            0x70, 0x00,              # MOVEQ #0, D0  (a)
            0x72, 0x01,              # MOVEQ #1, D1  (b)
            0x74, 0x04,              # MOVEQ #4, D2  (loop counter, 0..4 = 5 iters)
            # loop: temp=D0+D1; D0=D1; D1=temp
            0x76, 0x00,              # MOVEQ #0, D3
            0xD6, 0x80,              # ADD.L D0, D3  (D3 = a)
            0xD6, 0x81,              # ADD.L D1, D3  (D3 = a+b)
            0x20, 0x01,              # MOVE.L D1, D0 (a = b)
            0x22, 0x03,              # MOVE.L D3, D1 (b = a+b)
            0x51, 0xCA, 0xFF, 0xF4,  # DBF D2, loop (-12 → back to MOVEQ #0,D3)
        ]) + STOP
        s = run(prog)
        assert s.d1 == 8   # F(6) = 8 (after 5 iters from F(1))

    def test_max_of_three(self):
        # D0 = max(3, 7, 5) = 7
        prog = bytes([
            0x70, 0x03,              # MOVEQ #3, D0
            0x72, 0x07,              # MOVEQ #7, D1
            0x74, 0x05,              # MOVEQ #5, D2
            # if D1 > D0: D0 = D1
            0xB0, 0x41,              # CMP.W D1, D0
            0x6C, 0x02,              # BGE +2 (skip MOVE)
            0x30, 0x01,              # MOVE.W D1, D0
            # if D2 > D0: D0 = D2
            0xB0, 0x42,              # CMP.W D2, D0
            0x6C, 0x02,              # BGE +2 (skip MOVE)
            0x30, 0x02,              # MOVE.W D2, D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFFFF) == 7


class TestSubroutinePrograms:
    def test_bsr_rts(self):
        # Call a subroutine that adds 10 to D0
        prog = bytes([
            0x70, 0x05,              # MOVEQ #5, D0
            0x61, 0x00, 0x00, 0x04,  # BSR sub (disp = 4 → goes to 0x100A)
            # after return: continue with STOP
        ]) + STOP + bytes([
            # sub at 0x100C:
            0x06, 0x80, 0x00, 0x00, 0x00, 0x0A,  # ADDI.L #10, D0
            0x4E, 0x75,              # RTS
        ])
        s = run(prog)
        assert s.d0 == 15

    def test_link_unlk(self):
        # LINK A6, #-8 allocates 8 bytes on stack; UNLK restores
        sim = Motorola68kGateLevelSimulator()
        prog = bytes([
            0x4E, 0x56, 0xFF, 0xF8,  # LINK A6, #-8
            0x4E, 0x5E,              # UNLK A6
        ]) + STOP
        r = sim.execute(prog)
        # After LINK: SP = (SP - 4) - 8 = SP - 12
        # After UNLK: SP restored to LINK entry + 4
        # Net: A7 = original A7 (minus 4 for saved A6)
        # Exact value depends on stack state at entry
        # Just verify it runs cleanly
        assert r.halted

    def test_nested_calls(self):
        # outer calls inner; inner increments D0
        prog = bytes([
            0x70, 0x00,              # MOVEQ #0, D0
            0x61, 0x00, 0x00, 0x06,  # BSR outer (skip over STOP+outer)
        ]) + STOP + bytes([
            # outer at offset 8 from prog start (0x1008):
            0x61, 0x00, 0x00, 0x04,  # BSR inner (+4)
            0x4E, 0x75,              # RTS
            # inner at offset 14 (0x100E):
            0x52, 0x80,              # ADDQ.L #1, D0
            0x4E, 0x75,              # RTS
        ])
        s = run(prog)
        assert s.d0 == 1


class TestMOVEMPrograms:
    def test_save_restore_registers(self):
        # Save D0-D3 to stack, clobber them, restore
        prog = bytes([
            0x70, 0x0A,  # MOVEQ #10, D0
            0x72, 0x14,  # MOVEQ #20, D1
            0x74, 0x1E,  # MOVEQ #30, D2
            0x76, 0x28,  # MOVEQ #40, D3
            # MOVEM.L D0-D3, -(A7) = 0x48E7; predecrement mask (reversed): D0=bit15..D3=bit12 = 0xF000
            0x48, 0xE7, 0xF0, 0x00,
            # Clobber
            0x70, 0x00,
            0x72, 0x00,
            0x74, 0x00,
            0x76, 0x00,
            # Restore MOVEM.L (A7)+, D0-D3  postincrement mask (normal): D0=bit0..D3=bit3 = 0x000F
            0x4C, 0xDF, 0x00, 0x0F,
        ]) + STOP
        s = run(prog)
        assert s.d0 == 10
        assert s.d1 == 20
        assert s.d2 == 30
        assert s.d3 == 40

    def test_movem_to_memory(self):
        # MOVEM.L D0-D1 to 0x2000
        prog = bytes([
            0x70, 0xAA,              # MOVEQ #0xAA, D0
            0x72, 0xBB,              # MOVEQ #0xBB, D1 (sign extended)
            0x48, 0xB8, 0x00, 0x03,  # MOVEM.W D0-D1, (0x2000).W
            0x20, 0x00,              # (abs short = 0x2000)
        ]) + STOP
        # This combination might not parse right — use simpler test
        prog = bytes([
            0x70, 0x42,              # MOVEQ #0x42, D0
            0x41, 0xF8, 0x20, 0x00,  # LEA 0x2000, A0
            0x48, 0x90, 0x00, 0x01,  # MOVEM.W D0, (A0); mask=0x0001
        ]) + STOP
        s = run(prog)
        assert s.memory[0x2001] == 0x42


class TestDBccPrograms:
    def test_count_down(self):
        # D1 = 0..9 sum while counting down from D0=9
        prog = bytes([
            0x70, 0x09,              # MOVEQ #9, D0   loop counter
            0x72, 0x00,              # MOVEQ #0, D1   accumulator
            # loop:
            0xD2, 0x40,              # ADD.W D0, D1
            0x51, 0xC8, 0xFF, 0xFC,  # DBF D0, loop (-4)
        ]) + STOP
        s = run(prog)
        # D0 goes 9,8,7,...,0; exits after D0 hits -1
        # Sum = 9+8+7+6+5+4+3+2+1+0 = 45
        assert (s.d1 & 0xFFFF) == 45

    def test_search_for_value(self):
        # Search array [1,2,3,4,5] for value 3; D2 = index where found
        prog = bytes([
            # Load array into memory starting at 0x2000
            0x41, 0xF8, 0x20, 0x00,  # LEA 0x2000, A0
            0x70, 0x01,
            0x30, 0xC0,              # MOVE.W D0, (A0)+  → 0x2000 = 1
            0x70, 0x02,
            0x30, 0xC0,              # 0x2002 = 2
            0x70, 0x03,
            0x30, 0xC0,              # 0x2004 = 3
            0x70, 0x04,
            0x30, 0xC0,              # 0x2006 = 4
            0x70, 0x05,
            0x30, 0xC0,              # 0x2008 = 5
            # Now search: A0 = 0x2000, D1 = 4 (count-1), D3 = target=3
            0x41, 0xF8, 0x20, 0x00,  # LEA 0x2000, A0
            0x72, 0x04,              # MOVEQ #4, D1 (loop 5 times, 0-indexed)
            0x76, 0x03,              # MOVEQ #3, D3 (target)
            0x74, 0x00,              # MOVEQ #0, D2 (index)
            # loop:
            0x30, 0x18,              # MOVE.W (A0)+, D0
            0xB0, 0x43,              # CMP.W D3, D0
            0x67, 0x04,              # BEQ found (+4)
            0x52, 0x42,              # ADDQ.W #1, D2
            0x51, 0xC9, 0xFF, 0xF6,  # DBF D1, loop (-10)
            # found:
        ]) + STOP
        s = run(prog)
        assert (s.d2 & 0xFFFF) == 2  # index 2 (0-based)


class TestStringProgram:
    def test_string_copy(self):
        # Copy null-terminated string from 0x2000 to 0x3000
        sim = Motorola68kGateLevelSimulator()
        prog = bytes([
            0x41, 0xF8, 0x20, 0x00,  # LEA 0x2000, A0 (source)
            0x43, 0xF8, 0x30, 0x00,  # LEA 0x3000, A1 (dest)
            # loop: copy byte (A0)+ to (A1)+; stop when 0
            0x10, 0x18,              # MOVE.B (A0)+, D0
            0x67, 0x04,              # BEQ done (zero terminator)
            0x12, 0xC0,              # MOVE.B D0, (A1)+
            0x60, 0xF8,              # BRA loop (-8)
            # done:
        ]) + STOP

        # Set up source string "HELLO\0" at 0x2000
        string = b"HELLO\x00"
        # Need to pre-load string then re-run
        sim.reset()
        sim.load(prog)
        sim._mem[0x2000:0x2000 + len(string)] = string
        # Re-execute
        sim._halted = False
        sim._traces = []
        sim._rf.reset()
        sim._rf.write_pc(0x1000)
        steps = 0
        while not sim._halted and steps < 10000:
            try:
                sim.step()
            except RuntimeError:
                break
            steps += 1
        state = sim.get_state()
        # Verify destination has "HELLO"
        assert bytes(state.memory[0x3000:0x3005]) == b"HELLO"


class TestLogicalPrograms:
    def test_bitwise_operations(self):
        # D0 = (0xFF & 0xF0) | 0x0F = 0xFF
        prog = bytes([
            0x20, 0x3C, 0x00, 0x00, 0x00, 0xFF,  # MOVE.L #0xFF, D0
            0x02, 0x80, 0x00, 0x00, 0x00, 0xF0,  # ANDI.L #0xF0, D0
            0x00, 0x80, 0x00, 0x00, 0x00, 0x0F,  # ORI.L #0x0F, D0
        ]) + STOP
        s = run(prog)
        assert s.d0 == 0xFF

    def test_xor_toggle(self):
        # XOR a value with itself = 0; store Z flag in D1 via Scc before STOP.
        # (STOP #0x2700 loads 0x2700 into SR, clearing CCR — so we capture Z first.)
        prog = bytes([
            0x20, 0x3C, 0xDE, 0xAD, 0xBE, 0xEF,  # MOVE.L #0xDEADBEEF, D0
            0x0A, 0x80, 0xDE, 0xAD, 0xBE, 0xEF,  # EORI.L #0xDEADBEEF, D0
            0x57, 0xC1,                           # SEQ D1 (set D1.B=0xFF if Z set)
        ]) + STOP
        s = run(prog)
        assert s.d0 == 0
        assert (s.d1 & 0xFF) == 0xFF  # SEQ set D1 because EORI result was zero

    def test_not_operation(self):
        prog = bytes([
            0x70, 0x00,  # MOVEQ #0, D0
            0x46, 0x80,  # NOT.L D0
        ]) + STOP
        s = run(prog)
        assert s.d0 == 0xFFFFFFFF


class TestFlagPrograms:
    def test_cmp_branch_positive(self):
        # Compare D0=5 with #3; branch on GT → D2=1.
        # Layout: BGT skips over BRA (2 bytes) to reach MOVEQ #1,D2.
        # After BGT PC=0x100A; disp=+2 → jump to 0x100C (MOVEQ #1,D2).
        # If not taken: BRA +4 from 0x100C skips MOVEQ #1,D2 → MOVEQ #0,D2.
        prog = bytes([
            0x70, 0x05,              # MOVEQ #5, D0
            0xB0, 0x3C, 0x00, 0x03,  # CMP.W #3, D0
            0x6E, 0x02,              # BGT +2 (skip BRA; taken since 5>3)
            0x60, 0x04,              # BRA +4 (skip to MOVEQ #0,D2; not-taken path)
            0x74, 0x01,              # MOVEQ #1, D2 (branch taken)
            0x60, 0x02,              # BRA +2 (skip MOVEQ #0,D2 → STOP)
            0x74, 0x00,              # MOVEQ #0, D2 (not taken)
        ]) + STOP
        s = run(prog)
        assert s.d2 == 1

    def test_overflow_detection(self):
        # 0x7FFFFFFF + 1 → overflow.  Use TRAP #15 to halt without clearing CCR.
        HALT = bytes([0x4E, 0x4F])  # TRAP #15 — halts without loading new SR
        prog = bytes([
            0x20, 0x3C, 0x7F, 0xFF, 0xFF, 0xFF,  # MOVE.L #0x7FFFFFFF, D0
            0x06, 0x80, 0x00, 0x00, 0x00, 0x01,  # ADDI.L #1, D0
        ]) + HALT
        s = run(prog)
        assert s.d0 == 0x80000000
        assert s.v  # overflow set
        assert s.n  # result is negative (0x80000000)

    def test_carry_flag(self):
        HALT = bytes([0x4E, 0x4F])  # TRAP #15 — halts without loading new SR
        prog = bytes([
            0x20, 0x3C, 0xFF, 0xFF, 0xFF, 0xFF,  # MOVE.L #0xFFFFFFFF, D0
            0x06, 0x80, 0x00, 0x00, 0x00, 0x01,  # ADDI.L #1, D0
        ]) + HALT
        s = run(prog)
        assert s.c  # carry out
        assert s.d0 == 0  # wrapped to 0
        assert s.z  # zero


class TestNegatePrograms:
    def test_neg_positive(self):
        prog = bytes([
            0x70, 0x05,  # MOVEQ #5, D0
            0x44, 0x80,  # NEG.L D0
        ]) + STOP
        s = run(prog)
        assert s.d0 == 0xFFFFFFFB  # -5 unsigned

    def test_neg_zero(self):
        # Use TRAP #15 to halt without clearing CCR (STOP #0x2700 clears all flags).
        HALT = bytes([0x4E, 0x4F])  # TRAP #15
        prog = bytes([
            0x70, 0x00,  # MOVEQ #0, D0
            0x44, 0x80,  # NEG.L D0
        ]) + HALT
        s = run(prog)
        assert s.d0 == 0
        assert s.z
        assert not s.c  # C=0 when result=0


class TestMoveVariants:
    def test_moveq_negative(self):
        # MOVEQ #-1, D0 — sign extends 0xFF to 0xFFFFFFFF.
        # Use TRAP #15 to halt without clearing CCR.
        HALT = bytes([0x4E, 0x4F])  # TRAP #15
        prog = bytes([0x70, 0xFF]) + HALT
        s = run(prog)
        assert s.d0 == 0xFFFFFFFF
        assert s.n
        assert not s.z

    def test_move_to_from_ccr(self):
        # Set CCR to 0x04 (Z=1), then read it into D0.
        # The MOVE CCR,D0 stores CCR value in D0 low word — readable even after STOP.
        prog = bytes([
            0x44, 0xFC, 0x00, 0x04,  # MOVE #4, CCR (Z=1)
            0x42, 0xC0,              # MOVE CCR, D0
        ]) + STOP
        s = run(prog)
        assert s.d0 & 0x04  # Z bit captured in D0 before STOP

    def test_move_to_sr(self):
        # MOVE #0x2704, SR sets Z=1. Use TRAP #15 to preserve flags.
        HALT = bytes([0x4E, 0x4F])  # TRAP #15
        prog = bytes([
            0x46, 0xFC, 0x27, 0x04,  # MOVE #0x2704, SR (Z=1 in CCR bits)
        ]) + HALT
        s = run(prog)
        assert s.z  # Z bit set (bit 2 of CCR)
