"""Tests for Z80 branch, call, and return instructions.

Covers: JP nn
JP cc,nn
JP (HL)
JR e
JR cc,e
        DJNZ
        CALL nn
        CALL cc,nn
        RET
        RET cc
        RST p.
"""

from z80_simulator import Z80Simulator

# ── JP nn ─────────────────────────────────────────────────────────────────────

class TestJP:
    def test_jp_absolute(self):
        # JP 0x0005; NOP; HALT; HALT (at 0x0005)
        sim = Z80Simulator()
        prog = bytes([
            0xC3, 0x06, 0x00,   # JP 0x0006
            0x3E, 0x01,         # LD A, 1  (skipped)
            0x76,               # HALT at 0x0005 (also skipped)
            0x76,               # HALT at 0x0006
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0   # LD A,1 was skipped

    def test_jp_hl(self):
        # JP (HL): jump to address in HL
        sim = Z80Simulator()
        prog = bytes([
            0x21, 0x05, 0x00,   # LD HL, 0x0005
            0xE9,               # JP (HL)
            0x3E, 0x01,         # LD A, 1 (skipped)
            0x76,               # HALT at 0x0005
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0


# ── JP cc, nn ─────────────────────────────────────────────────────────────────

class TestJPCC:
    def test_jp_nz_taken(self):
        # A=1, CP 0 → Z=0; JP NZ → taken
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x01,         # LD A, 1
            0xFE, 0x00,         # CP 0 → Z=0
            0xC2, 0x08, 0x00,   # JP NZ, 0x0008
            0x3E, 0xFF,         # LD A, 0xFF (skipped)
            0x76,               # HALT at 0x0008
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0x01

    def test_jp_nz_not_taken(self):
        # A=0, XOR A → Z=1; JP NZ → not taken
        sim = Z80Simulator()
        prog = bytes([
            0xAF,               # XOR A → Z=1
            0xC2, 0x06, 0x00,   # JP NZ, 0x0006 (not taken)
            0x3E, 0x42,         # LD A, 0x42 (executed)
            0x76,               # HALT at 0x0005
            0x3E, 0xFF,         # (at 0x0006, never reached)
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0x42

    def test_jp_z_taken(self):
        # XOR A → Z=1; JP Z → taken
        sim = Z80Simulator()
        prog = bytes([
            0xAF,               # XOR A → Z=1
            0xCA, 0x05, 0x00,   # JP Z, 0x0005
            0x76,               # HALT at 0x0004 (skipped)
            0x76,               # HALT at 0x0005
        ])
        r = sim.execute(prog)
        assert r.final_state.pc == 0x0006

    def test_jp_c_taken(self):
        # SCF; JP C → taken
        sim = Z80Simulator()
        prog = bytes([
            0x37,               # SCF
            0xDA, 0x05, 0x00,   # JP C, 0x0005
            0x76,               # HALT at 0x0004 (skipped)
            0x76,               # HALT at 0x0005
        ])
        r = sim.execute(prog)
        assert r.final_state.pc == 0x0006


# ── JR e (relative jump) ──────────────────────────────────────────────────────

class TestJR:
    def test_jr_forward(self):
        # JR +2: skip over two bytes
        sim = Z80Simulator()
        prog = bytes([
            0x18, 0x02,         # JR +2 (skip 2 bytes after this instruction)
            0x3E, 0x01,         # LD A, 1 (skipped)
            0x76,               # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0

    def test_jr_backward_loop(self):
        # Simple 3-iteration counter using JR:
        # LD B, 3
        # LOOP: DEC B; JR NZ, -3 (back to DEC B)
        # HALT
        sim = Z80Simulator()
        prog = bytes([
            0x06, 0x03,         # LD B, 3
            0x05,               # DEC B        <- offset 2
            0x20, 0xFD,         # JR NZ, -3   (target = offset 2)
            0x76,               # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.b == 0

    def test_jr_nz_not_taken(self):
        # XOR A → Z=1; JR NZ not taken
        sim = Z80Simulator()
        prog = bytes([
            0xAF,               # XOR A → Z=1
            0x20, 0x02,         # JR NZ, +2 (not taken)
            0x3E, 0x42,         # LD A, 0x42 (executed)
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0x42

    def test_jr_z_taken(self):
        sim = Z80Simulator()
        prog = bytes([
            0xAF,               # XOR A → Z=1
            0x28, 0x02,         # JR Z, +2 (taken: skip LD A,0x01)
            0x3E, 0x01,         # LD A, 1 (skipped)
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0

    def test_jr_c_taken(self):
        sim = Z80Simulator()
        prog = bytes([
            0x37,               # SCF
            0x38, 0x02,         # JR C, +2 (taken)
            0x3E, 0x01,         # LD A, 1 (skipped)
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0


# ── DJNZ ─────────────────────────────────────────────────────────────────────

class TestDJNZ:
    def test_djnz_loops_until_b_zero(self):
        # LD B, 5; LOOP: DEC A wait — DJNZ does DEC B for us
        # LD A, 0; LD B, 5; LOOP: INC A; DJNZ -3
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x00,         # LD A, 0
            0x06, 0x05,         # LD B, 5
            0x3C,               # INC A           <- offset 4
            0x10, 0xFD,         # DJNZ -3  (→ offset 4)
            0x76,               # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 5
        assert r.final_state.b == 0

    def test_djnz_does_not_jump_when_b_is_one(self):
        sim = Z80Simulator()
        prog = bytes([
            0x06, 0x01,         # LD B, 1 (will hit zero immediately)
            0x10, 0xFE,         # DJNZ -2 (should not loop)
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.b == 0


# ── CALL / RET ────────────────────────────────────────────────────────────────

class TestCallRet:
    def test_call_and_ret(self):
        # CALL subroutine; HALT; subroutine: LD A, 0x42; RET
        sim = Z80Simulator()
        prog = bytes([
            0x31, 0x00, 0x80,   # LD SP, 0x8000
            0xCD, 0x07, 0x00,   # CALL 0x0007
            0x76,               # HALT at 0x0006
            0x3E, 0x42,         # LD A, 0x42  (subroutine at 0x0007)
            0xC9,               # RET
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0x42
        assert r.final_state.pc == 0x0007  # returned, then HALT executed

    def test_call_saves_return_address(self):
        sim = Z80Simulator()
        prog = bytes([
            0x31, 0x00, 0x80,   # LD SP, 0x8000
            0xCD, 0x07, 0x00,   # CALL 0x0007
            0x76,               # HALT at 0x0006
            0xC9,               # RET immediately (subroutine at 0x0007)
        ])
        r = sim.execute(prog)
        # Return address pushed = 0x0006
        assert r.final_state.memory[0x7FFE] == 0x06
        assert r.final_state.memory[0x7FFF] == 0x00


# ── CALL cc / RET cc ──────────────────────────────────────────────────────────

class TestCallRetCC:
    def test_call_nz_taken(self):
        # Memory layout:
        #   0-2:  LD SP, 0x8000
        #   3-4:  LD A, 1          → A=1, Z not set
        #   5-6:  CP 0             → Z=0 (1 ≠ 0)
        #   7-9:  CALL NZ, 0x000B  → push return addr 0x000A, jump to 0x000B
        #   10:   HALT (0x000A)    ← subroutine returns here
        #   11-12: LD A, 0x42      (subroutine at 0x000B)
        #   13:   RET
        sim = Z80Simulator()
        prog = bytes([
            0x31, 0x00, 0x80,   # LD SP, 0x8000 (0-2)
            0x3E, 0x01,         # LD A, 1       (3-4)
            0xFE, 0x00,         # CP 0          (5-6)
            0xC4, 0x0B, 0x00,   # CALL NZ, 0x000B (7-9)
            0x76,               # HALT at 0x000A
            0x3E, 0x42,         # LD A, 0x42 (subroutine at 0x000B)
            0xC9,               # RET
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0x42

    def test_ret_z_not_taken(self):
        # RET Z is only taken when Z=1.  We set Z=0 via CP before RET Z.
        # Memory layout:
        #   0-2:  LD SP; 3-5: CALL 0x0009; 6: HALT (return pt)
        #   7-8 padding; 9-10: LD A,0x55; 11-12: CP 0x66 → Z=0; 13: RET Z (not taken)
        #   14-15: LD A,0x66; 16: RET
        sim = Z80Simulator()
        prog = bytes([
            0x31, 0x00, 0x80,   # LD SP, 0x8000    (0-2)
            0xCD, 0x09, 0x00,   # CALL 0x0009      (3-5; return addr=6)
            0x76,               # HALT at 0x0006   (6)
            0x00, 0x00,         # padding          (7-8)
            0x3E, 0x55,         # LD A, 0x55       (9-10; subroutine at 0x0009)
            0xFE, 0x66,         # CP 0x66 → Z=0    (11-12)
            0xC8,               # RET Z (not taken — Z=0)  (13)
            0x3E, 0x66,         # LD A, 0x66       (14-15; executed)
            0xC9,               # RET              (16)
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0x66


# ── RST ───────────────────────────────────────────────────────────────────────

class TestRST:
    def test_rst_08(self):
        # RST 0x08 (0xCF): push PC, jump to 0x0008.
        # Layout: LD SP(0-2), RST(3), LD A 1(4-5), HALT(6), NOP(7), HALT(8)
        # RST jumps to 0x0008 = offset 8 = second HALT.
        sim = Z80Simulator()
        prog = bytes([
            0x31, 0x00, 0x80,   # LD SP, 0x8000  (0-2)
            0xCF,               # RST 0x08       (3)
            0x3E, 0x01,         # LD A, 1        (4-5; skipped)
            0x76,               # HALT at 0x0006 (6; skipped)
            0x00,               # NOP padding    (7)
            0x76,               # HALT at 0x0008 (8)
        ])
        r = sim.execute(prog)
        assert r.final_state.pc == 0x0009  # PC past HALT at 0x0008
        assert r.final_state.a == 0

    def test_rst_38(self):
        # RST 0x38 (0xFF): jump to 0x0038
        sim = Z80Simulator()
        prog = bytearray(0x40)  # 64 bytes of NOP
        prog[0] = 0x31
        prog[1] = 0x00
        prog[2] = 0x80   # LD SP, 0x8000
        prog[3] = 0xFF                                     # RST 0x38
        prog[4] = 0x76                                     # HALT (skipped)
        prog[0x38] = 0x76                                  # HALT at 0x0038
        r = sim.execute(bytes(prog))
        assert r.final_state.pc == 0x0039
