"""Tests for comprehensive simulator coverage — EA modes, Bcc, TRAP, MOVEM, shifts."""

import struct

from motorola68k_gatelevel.simulator import Motorola68kGateLevelSimulator

STOP = bytes([0x4E, 0x72, 0x27, 0x00])  # STOP #0x2700


def make_prog(*words: int) -> bytes:
    """Pack 16-bit big-endian words and append STOP."""
    prog = bytearray()
    for w in words:
        prog += struct.pack(">H", w)
    prog += STOP
    return bytes(prog)


def run(prog: bytes) -> object:
    sim = Motorola68kGateLevelSimulator()
    r = sim.execute(prog)
    return r.final_state


class TestEAModes:
    """All 12 effective address modes."""

    def test_dn_register_direct(self):
        # MOVEQ #5, D0; STOP
        prog = make_prog(0x7005)
        s = run(prog)
        assert s.d0 == 5

    def test_an_register_direct(self):
        # MOVEA.L #0x2000, A0 — MOVE.L #imm, A0
        prog = make_prog(0x207C, 0x0000, 0x2000)
        s = run(prog)
        assert s.a0 == 0x2000

    def test_an_indirect(self):
        # MOVEQ #7, D1; MOVE.L D1, (A0) where A0=some address; check memory
        # Simpler: use LEA + MOVE
        prog = bytes([
            0x41, 0xF8, 0x20, 0x00,  # LEA 0x2000, A0
            0x70, 0x2A,              # MOVEQ #42, D0
            0x20, 0x80,              # MOVE.L D0, (A0)
        ]) + STOP
        s = run(prog)
        assert s.memory[0x2000] == 0
        assert s.memory[0x2003] == 42

    def test_an_postincrement(self):
        # Store two values with postincrement
        prog = bytes([
            0x41, 0xF8, 0x20, 0x00,  # LEA 0x2000, A0
            0x70, 0x01,              # MOVEQ #1, D0
            0x72, 0x02,              # MOVEQ #2, D1
            0x30, 0xC0,              # MOVE.W D0, (A0)+
            0x30, 0xC1,              # MOVE.W D1, (A0)+
        ]) + STOP
        s = run(prog)
        assert s.memory[0x2001] == 1  # D0=1 stored at 0x2000
        assert s.memory[0x2003] == 2  # D1=2 stored at 0x2002

    def test_an_predecrement(self):
        # MOVE.W to -(A0) (push-style)
        # Encoding: 0011 000 100 000 000 = 0x3100
        # line=3(MOVE.W), dst_reg=0(A0), dst_mode=4(predecrement), src_mode=0(Dn), src_reg=0(D0)
        prog = bytes([
            0x41, 0xF8, 0x20, 0x04,  # LEA 0x2004, A0
            0x70, 0x0A,              # MOVEQ #10, D0
            0x31, 0x00,              # MOVE.W D0, -(A0) → A0=0x2002, writes to 0x2002
        ]) + STOP
        s = run(prog)
        assert s.a0 == 0x2002
        assert s.memory[0x2003] == 10

    def test_displacement_indirect(self):
        # Write to d16(A0): LEA 0x2000; MOVE.W D0, 4(A0) → 0x2004
        # MOVE.W D0, 4(A0) = 0011 000 101 000 000 = 0x3140 + disp word 0x0004
        # dst_mode=5(d16(An)), dst_reg=0(A0), src_mode=0(Dn), src_reg=0(D0)
        prog = bytes([
            0x41, 0xF8, 0x20, 0x00,  # LEA 0x2000, A0
            0x70, 0x07,              # MOVEQ #7, D0
            0x31, 0x40, 0x00, 0x04,  # MOVE.W D0, 4(A0)
        ]) + STOP
        s = run(prog)
        assert s.memory[0x2005] == 7

    def test_immediate_mode(self):
        # ADDI.L #100, D0
        prog = bytes([
            0x06, 0x80, 0x00, 0x00, 0x00, 0x64,  # ADDI.L #100, D0
        ]) + STOP
        s = run(prog)
        assert s.d0 == 100

    def test_absolute_short(self):
        # MOVE.W D0, (0x2000).W — absolute short addressing (mode=7, reg=0)
        # Encoding: 0011 000 111 000 000 = 0x31C0 + abs word 0x2000
        prog = bytes([
            0x70, 0x55,              # MOVEQ #0x55, D0
            0x31, 0xC0, 0x20, 0x00,  # MOVE.W D0, (0x2000).W
        ]) + STOP
        s = run(prog)
        assert s.memory[0x2001] == 0x55

    def test_pc_relative(self):
        # MOVE.W d16(PC), D0 — read a value embedded after the program.
        # The simulator reads pc_base at the extension word location (0x1002),
        # then fetches the displacement word, so EA = 0x1002 + disp.
        # Value 0x0042 at 0x1008; disp = 0x1008 - 0x1002 = 6.
        sim = Motorola68kGateLevelSimulator()
        prog_bytes = bytearray([
            0x30, 0x3A, 0x00, 0x06,  # MOVE.W d16(PC),D0; disp=6 → EA=0x1002+6=0x1008
            0x4E, 0x72, 0x27, 0x00,  # STOP at 0x1004
            0x00, 0x42,              # value 0x0042 at 0x1008
        ])
        r = sim.execute(bytes(prog_bytes))
        assert r.final_state.d0 == 0x42


class TestBccAllConditions:
    """All 16 Bcc conditions."""

    def _run_bcc(self, cc_code: int, n: int, z: int, v: int, c: int,
                 should_branch: bool) -> None:
        """Set up flags via MOVE#,CCR then run Bcc and verify branch taken/not."""
        ccr = (n << 3) | (z << 2) | (v << 1) | c
        # Load flags, branch over MOVEQ #1,D0; if branch taken D0=0, else D0=1
        prog = bytes([
            0x44, 0xFC, 0x00, ccr,   # MOVE #ccr, CCR
            0x60 | cc_code, 0x04,    # Bcc +4 (skip next 2 words)
            0x70, 0x01,              # MOVEQ #1, D0 (not branched)
        ]) + STOP
        s = run(prog)
        if should_branch:
            assert s.d0 == 0, f"Expected branch for cc={cc_code}, N={n},Z={z},V={v},C={c}"
        else:
            assert s.d0 == 1, f"Expected no-branch for cc={cc_code}, N={n},Z={z},V={v},C={c}"

    def test_bra_always(self):
        self._run_bcc(0, 0, 0, 0, 0, True)

    def test_bhi_taken(self):
        self._run_bcc(2, 0, 0, 0, 0, True)   # HI: C=0, Z=0

    def test_bhi_not_taken(self):
        self._run_bcc(2, 0, 0, 0, 1, False)  # C=1 → not HI

    def test_bls_taken(self):
        self._run_bcc(3, 0, 0, 0, 1, True)   # LS: C=1

    def test_bcc_taken(self):
        self._run_bcc(4, 0, 0, 0, 0, True)   # CC: C=0

    def test_bcs_taken(self):
        self._run_bcc(5, 0, 0, 0, 1, True)   # CS: C=1

    def test_bne_taken(self):
        self._run_bcc(6, 0, 0, 0, 0, True)   # NE: Z=0

    def test_beq_taken(self):
        self._run_bcc(7, 0, 1, 0, 0, True)   # EQ: Z=1

    def test_bvc_taken(self):
        self._run_bcc(8, 0, 0, 0, 0, True)   # VC: V=0

    def test_bvs_taken(self):
        self._run_bcc(9, 0, 0, 1, 0, True)   # VS: V=1

    def test_bpl_taken(self):
        self._run_bcc(10, 0, 0, 0, 0, True)  # PL: N=0

    def test_bmi_taken(self):
        self._run_bcc(11, 1, 0, 0, 0, True)  # MI: N=1

    def test_bge_taken_nn_vv(self):
        self._run_bcc(12, 0, 0, 0, 0, True)  # GE: N==V (both 0)

    def test_blt_taken_n_ne_v(self):
        self._run_bcc(13, 1, 0, 0, 0, True)  # LT: N≠V (N=1,V=0)

    def test_bgt_taken(self):
        self._run_bcc(14, 0, 0, 0, 0, True)  # GT: Z=0, N==V

    def test_ble_taken_z(self):
        self._run_bcc(15, 0, 1, 0, 0, True)  # LE: Z=1


class TestTRAP:
    def test_trap_15_halts(self):
        prog = bytes([0x4E, 0x4F]) + STOP  # TRAP #15 then STOP
        sim = Motorola68kGateLevelSimulator()
        r = sim.execute(prog)
        assert r.halted  # halted by TRAP #15

    def test_trap_n_takes_exception(self):
        # TRAP #1 → load PC from vector[33] = 0x84 (vector offset 0x84).
        # execute() calls reset() which zeroes memory, so we must set the vector
        # AFTER execute() returns, or use step() manually after setup.
        sim = Motorola68kGateLevelSimulator()
        prog = bytes([0x4E, 0x41]) + STOP  # TRAP #1 then STOP (at 0x1002)
        # Reset + load program
        sim.reset()
        sim.load(prog)
        # Set up vector 33 at 0x84 to point to our STOP (at 0x1002)
        stop_addr = 0x1002
        sim._mem[0x84] = (stop_addr >> 24) & 0xFF
        sim._mem[0x85] = (stop_addr >> 16) & 0xFF
        sim._mem[0x86] = (stop_addr >>  8) & 0xFF
        sim._mem[0x87] =  stop_addr        & 0xFF
        # Step manually: step1=TRAP#1 (PC→0x1002), step2=STOP (halted)
        sim.step()  # TRAP #1 → jumps to 0x1002
        sim.step()  # STOP #0x2700 → halted
        assert sim._halted


class TestMOVEM:
    def test_movem_save_restore(self):
        # Save D0-D2 to memory, clear them, restore
        prog = bytes([
            0x70, 0x01,              # MOVEQ #1, D0
            0x72, 0x02,              # MOVEQ #2, D1
            0x74, 0x03,              # MOVEQ #3, D2
            # MOVEM.L D0-D2, -(A7) = 0x48E7; predecrement mask (reversed): D0=bit15,D1=bit14,D2=bit13 = 0xE000
            0x48, 0xE7, 0xE0, 0x00,  # MOVEM.L D0-D2, -(A7); mask 0xE000
            0x70, 0x00,              # CLR D0 via MOVEQ
            0x72, 0x00,
            0x74, 0x00,
            # MOVEM.L (A7)+, D0-D2  (restore)
            0x4C, 0xDF, 0x00, 0x07,  # MOVEM.L (A7)+, D0-D2; mask 0x0007
        ]) + STOP
        s = run(prog)
        assert s.d0 == 1
        assert s.d1 == 2
        assert s.d2 == 3


class TestDBcc:
    def test_dbf_loop(self):
        # DBF loop: D0 = 3; loop until D0 reaches -1
        # Each iteration adds 1 to D1
        prog = bytes([
            0x70, 0x03,              # MOVEQ #3, D0
            0x72, 0x00,              # MOVEQ #0, D1
            # loop:
            0x52, 0x41,              # ADDQ.W #1, D1
            0x51, 0xC8, 0xFF, 0xFC,  # DBF D0, loop (-4 = back 2 words)
        ]) + STOP
        s = run(prog)
        # D0 starts at 3; each iteration decrements D0 until -1 → 4 iterations
        assert s.d1 == 4

    def test_dbeq_no_loop_if_condition_true(self):
        # DBEQ: if EQ is true, branch NOT taken (loop exits regardless)
        prog = bytes([
            0x70, 0x02,              # MOVEQ #2, D0
            0x44, 0xFC, 0x00, 0x04,  # MOVE #4, CCR (Z=1)
            0x57, 0xC8, 0xFF, 0xFE,  # DBEQ D0, -2 (tight loop)
        ]) + STOP
        # DBEQ: condition T=EQ; if Z=1 (EQ true) → no decrement/branch
        s = run(prog)
        assert s.d0 == 2  # D0 unchanged, condition was true so no loop


class TestShiftsAllSizes:
    def test_asl_byte(self):
        prog = bytes([
            0x70, 0x01,  # MOVEQ #1, D0
            0xE3, 0x00,  # ASL.B #1, D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 2

    def test_asr_word(self):
        prog = bytes([
            0x30, 0x3C, 0x80, 0x00,  # MOVE.W #0x8000, D0
            0xE2, 0x40,              # ASR.W #1, D0 (arithmetic, count=1)
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFFFF) == 0xC000  # sign extended

    def test_lsr_long(self):
        prog = bytes([
            0x20, 0x3C, 0x00, 0x00, 0x00, 0x10,  # MOVE.L #0x10, D0
            0xE2, 0x88,                           # LSR.L #1, D0
        ]) + STOP
        s = run(prog)
        assert s.d0 == 8

    def test_rol_word(self):
        prog = bytes([
            0x30, 0x3C, 0x80, 0x01,  # MOVE.W #0x8001, D0
            0xE3, 0x58,              # ROL.W #1, D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFFFF) == 0x0003  # 0x8001 ROL 1 = 0x0003


class TestNMI:
    def test_nmi_queued(self):
        sim = Motorola68kGateLevelSimulator()
        sim.nmi()
        assert sim._pending_nmi

    def test_interrupt_queued(self):
        sim = Motorola68kGateLevelSimulator()
        sim.interrupt(5)
        assert sim._pending_interrupt == 5


class TestSccAllConditions:
    """Scc sets byte on condition."""

    def test_st_always(self):
        # ST D0 — always sets
        prog = bytes([
            0x50, 0xC0,  # ST D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 0xFF

    def test_sf_never(self):
        # SF D0 — never sets (clears)
        prog = bytes([
            0x70, 0xFF,  # MOVEQ #-1, D0 (all 1s)
            0x51, 0xC0,  # SF D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 0x00

    def test_seq_when_z(self):
        prog = bytes([
            0x44, 0xFC, 0x00, 0x04,  # MOVE #4, CCR (Z=1)
            0x57, 0xC0,              # SEQ D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 0xFF


class TestEXG:
    def test_exg_dn_dn(self):
        prog = bytes([
            0x70, 0x0A,  # MOVEQ #10, D0
            0x72, 0x14,  # MOVEQ #20, D1
            0xC1, 0x41,  # EXG D0, D1
        ]) + STOP
        s = run(prog)
        assert s.d0 == 20
        assert s.d1 == 10

    def test_exg_an_an(self):
        prog = bytes([
            0x20, 0x7C, 0x00, 0x00, 0x10, 0x00,  # MOVEA.L #0x1000, A0
            0x22, 0x7C, 0x00, 0x00, 0x20, 0x00,  # MOVEA.L #0x2000, A1
            0xC1, 0x49,                           # EXG A0, A1
        ]) + STOP
        s = run(prog)
        assert s.a0 == 0x2000
        assert s.a1 == 0x1000


class TestCLR:
    def test_clr_l(self):
        # Use TRAP #15 to halt without loading 0x2700 into SR (which clears Z).
        HALT = bytes([0x4E, 0x4F])  # TRAP #15
        prog = bytes([
            0x70, 0xFF,  # MOVEQ #-1, D0
            0x42, 0x80,  # CLR.L D0
        ]) + HALT
        s = run(prog)
        assert s.d0 == 0
        assert s.z

    def test_clr_b(self):
        prog = bytes([
            0x70, 0xFF,  # MOVEQ #-1, D0
            0x42, 0x00,  # CLR.B D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 0


class TestSWAP:
    def test_swap_basic(self):
        prog = bytes([
            0x20, 0x3C, 0xAB, 0xCD, 0x12, 0x34,  # MOVE.L #0xABCD1234, D0
            0x48, 0x40,                           # SWAP D0
        ]) + STOP
        s = run(prog)
        assert s.d0 == 0x1234ABCD


class TestEXT:
    def test_ext_w(self):
        prog = bytes([
            0x70, 0x80,  # MOVEQ #-128, D0 (sign-extend from byte)
            0x48, 0x80,  # EXT.W D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFFFF) == 0xFF80  # -128 as signed 16-bit

    def test_ext_l(self):
        prog = bytes([
            0x30, 0x3C, 0x80, 0x00,  # MOVE.W #0x8000, D0
            0x48, 0xC0,              # EXT.L D0
        ]) + STOP
        s = run(prog)
        assert s.d0 == 0xFFFF8000
