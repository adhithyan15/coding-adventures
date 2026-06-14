"""Additional simulator tests to push coverage above 80%.

These tests target paths not yet exercised by test_programs.py or
test_equivalence.py. Specifically:

- DD/FD prefix (IX/IY) instructions
- ED extended instructions (IM, RETI, RETN, LD A,I, LD A,R, LD I,A, LD R,A)
- Block operations: LDI, LDD, LDIR, LDDR, CPI, CPD, CPIR, CPDR
- Conditional branches (JP cc, JR cc, CALL cc, RET cc) — both taken and not-taken
- I/O instructions: IN A,(n), OUT (n),A, IN r,(C), OUT (C),r
- Exchange: EX DE,HL, EX (SP),HL
- LD (BC),A, LD A,(BC), LD (DE),A, LD A,(DE)
- LD (nn),A, LD A,(nn), LD HL,(nn), LD (nn),HL
- LD SP,HL, EI, DI, RST
- ADC A,m, SBC A,m (carry-in variants)
- DDCB/FDCB bit ops on (IX+d)/(IY+d)
- step() on halted raises RuntimeError
- execute() max_steps guard
- set_input_port / get_output_port validation errors
- 16-bit INC/DEC register pairs
"""

import pytest

from z80_gatelevel import Z80GateLevelSimulator


def make_sim() -> Z80GateLevelSimulator:
    """Fresh simulator instance with clean state."""
    return Z80GateLevelSimulator()


def run(program: bytes) -> object:
    sim = make_sim()
    result = sim.execute(program)
    return result.final_state


def run_sim(program: bytes) -> tuple[Z80GateLevelSimulator, object]:
    """Return both the simulator (with state) and the final state."""
    sim = make_sim()
    result = sim.execute(program)
    return sim, result.final_state


# ── DD/FD (IX/IY) prefix instructions ─────────────────────────────────────────

class TestIXInstructions:
    def test_ld_ix_nn(self):
        """DD 21 nn nn — LD IX, nn."""
        program = bytes([
            0xDD, 0x21, 0x34, 0x12,  # LD IX, 0x1234
            0x76,
        ])
        state = run(program)
        assert state.ix == 0x1234

    def test_ld_iy_nn(self):
        """FD 21 nn nn — LD IY, nn."""
        program = bytes([
            0xFD, 0x21, 0xCD, 0xAB,  # LD IY, 0xABCD
            0x76,
        ])
        state = run(program)
        assert state.iy == 0xABCD

    def test_ld_indexed_mem_store_load(self):
        """Store a value at (IX+d), then load it back."""
        # LD IX, 0x8000
        # LD A, 0x55
        # LD (IX+3), A
        # LD B, (IX+3)
        # HALT
        program = bytes([
            0xDD, 0x21, 0x00, 0x80,  # LD IX, 0x8000
            0x3E, 0x55,              # LD A, 0x55
            0xDD, 0x77, 0x03,        # LD (IX+3), A   (offset +3)
            0xDD, 0x46, 0x03,        # LD B, (IX+3)
            0x76,
        ])
        state = run(program)
        assert state.b == 0x55

    def test_add_ix_bc(self):
        """DD 09 — ADD IX, BC."""
        program = bytes([
            0xDD, 0x21, 0x00, 0x10,  # LD IX, 0x1000
            0x01, 0x00, 0x01,        # LD BC, 0x0100
            0xDD, 0x09,              # ADD IX, BC
            0x76,
        ])
        state = run(program)
        assert state.ix == 0x1100

    def test_inc_ix(self):
        """DD 23 — INC IX."""
        program = bytes([
            0xDD, 0x21, 0xFF, 0xFF,  # LD IX, 0xFFFF
            0xDD, 0x23,              # INC IX
            0x76,
        ])
        state = run(program)
        assert state.ix == 0x0000

    def test_dec_ix(self):
        """DD 2B — DEC IX."""
        program = bytes([
            0xDD, 0x21, 0x00, 0x10,  # LD IX, 0x1000
            0xDD, 0x2B,              # DEC IX
            0x76,
        ])
        state = run(program)
        assert state.ix == 0x0FFF

    def test_push_pop_ix(self):
        """DD E5 = PUSH IX; DD E1 = POP IX."""
        program = bytes([
            0x31, 0x00, 0x80,        # LD SP, 0x8000
            0xDD, 0x21, 0x78, 0x56,  # LD IX, 0x5678
            0xDD, 0xE5,              # PUSH IX
            0xDD, 0x21, 0x00, 0x00,  # LD IX, 0
            0xDD, 0xE1,              # POP IX
            0x76,
        ])
        state = run(program)
        assert state.ix == 0x5678

    def test_ld_sp_ix(self):
        """DD F9 — LD SP, IX."""
        program = bytes([
            0xDD, 0x21, 0x00, 0x80,  # LD IX, 0x8000
            0xDD, 0xF9,              # LD SP, IX
            0x76,
        ])
        state = run(program)
        assert state.sp == 0x8000

    def test_jp_ix(self):
        """DD E9 — JP (IX): jump to address in IX."""
        # IX = 0x0008; code at 0x0008 loads A=0x77 then HALT
        program = bytes([
            0xDD, 0x21, 0x08, 0x00,  # LD IX, 0x0008  (0x0000)
            0xDD, 0xE9,              # JP (IX)         (0x0004)
            0x3E, 0xFF,              # LD A, 0xFF      (0x0006) — skipped
            0x3E, 0x77,              # LD A, 0x77      (0x0008)
            0x76,                    # HALT            (0x000A)
        ])
        state = run(program)
        assert state.a == 0x77

    def test_ld_ix_nn_from_mem(self):
        """DD 2A — LD IX, (nn): load IX from memory."""
        sim = make_sim()
        # Plant 0x1234 at address 0x8000/0x8001
        sim._memory[0x8000] = 0x34  # lo
        sim._memory[0x8001] = 0x12  # hi
        program = bytes([
            0xDD, 0x2A, 0x00, 0x80,  # LD IX, (0x8000)
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._rf.read_ix() == 0x1234

    def test_ld_nn_ix(self):
        """DD 22 — LD (nn), IX: store IX to memory."""
        program = bytes([
            0xDD, 0x21, 0xAB, 0xCD,  # LD IX, 0xCDAB
            0xDD, 0x22, 0x00, 0x80,  # LD (0x8000), IX
            0x76,
        ])
        sim = make_sim()
        sim.execute(program)
        # lo byte at 0x8000, hi at 0x8001
        assert sim._memory[0x8000] == 0xAB
        assert sim._memory[0x8001] == 0xCD

    def test_alu_with_indexed(self):
        """ADD A, (IX+d) — ALU op using indexed addressing."""
        sim = make_sim()
        sim._memory[0x8005] = 0x07  # value at (IX+5) = 7
        program = bytes([
            0xDD, 0x21, 0x00, 0x80,  # LD IX, 0x8000
            0x3E, 0x03,              # LD A, 3
            0xDD, 0x86, 0x05,        # ADD A, (IX+5)
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._rf.read8(7) == 10  # A = 7+3 = 10

    def test_inc_dec_indexed_mem(self):
        """DD 34 = INC (IX+d); DD 35 = DEC (IX+d)."""
        sim = make_sim()
        sim._memory[0x8002] = 0x0A  # initial value at (IX+2) = 10
        program = bytes([
            0xDD, 0x21, 0x00, 0x80,  # LD IX, 0x8000
            0xDD, 0x34, 0x02,        # INC (IX+2)   → 11
            0xDD, 0x34, 0x02,        # INC (IX+2)   → 12
            0xDD, 0x35, 0x02,        # DEC (IX+2)   → 11
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._memory[0x8002] == 0x0B  # 10 + 2 - 1 = 11

    def test_ld_indexed_store(self):
        """DD 36 — LD (IX+d), n: store immediate byte to (IX+d)."""
        sim = make_sim()
        program = bytes([
            0xDD, 0x21, 0x00, 0x80,  # LD IX, 0x8000
            0xDD, 0x36, 0x01, 0xBB,  # LD (IX+1), 0xBB
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._memory[0x8001] == 0xBB

    def test_ex_sp_ix(self):
        """DD E3 — EX (SP), IX."""
        sim = make_sim()
        # Set up: IX = 0x1234; stack at 0x8000 contains 0x5678
        program = bytes([
            0x31, 0x00, 0x80,        # LD SP, 0x8000
            0xDD, 0x21, 0x34, 0x12,  # LD IX, 0x1234
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._memory[0x7FFE] = 0x78  # lo of 0x5678
        sim._memory[0x7FFF] = 0x56  # hi of 0x5678
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        # Now set SP to 0x7FFE and do EX (SP), IX
        sim2 = make_sim()
        program2 = bytes([
            0xDD, 0x21, 0x34, 0x12,  # LD IX, 0x1234
            0x31, 0xFE, 0x7F,        # LD SP, 0x7FFE
            0xDD, 0xE3,              # EX (SP), IX
            0x76,
        ])
        for i, b in enumerate(program2):
            sim2._memory[i] = b
        sim2._memory[0x7FFE] = 0x78
        sim2._memory[0x7FFF] = 0x56
        sim2._pc.write(0)
        while not sim2._halted:
            sim2.step()
        assert sim2._rf.read_ix() == 0x5678
        assert sim2._memory[0x7FFE] == 0x34
        assert sim2._memory[0x7FFF] == 0x12


# ── IY variant ─────────────────────────────────────────────────────────────────

class TestIYInstructions:
    def test_ld_iy_store_load(self):
        """IY indexed store and load."""
        sim = make_sim()
        program = bytes([
            0xFD, 0x21, 0x00, 0x90,  # LD IY, 0x9000
            0x3E, 0xCC,              # LD A, 0xCC
            0xFD, 0x77, 0x00,        # LD (IY+0), A
            0xFD, 0x7E, 0x00,        # LD A, (IY+0)  → A should be 0xCC
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._rf.read8(7) == 0xCC  # REG_A = 7


# ── DDCB/FDCB bit ops ──────────────────────────────────────────────────────────

class TestDDCBInstructions:
    def test_bit_ix_d(self):
        """DDCB — BIT n,(IX+d)."""
        sim = make_sim()
        sim._memory[0x8000] = 0b00001000  # bit 3 set
        program = bytes([
            0xDD, 0x21, 0x00, 0x80,  # LD IX, 0x8000
            0xDD, 0xCB, 0x00, 0x5E,  # BIT 3, (IX+0)  → Z=0 (bit 3 set)
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        flags = sim._rf.read_flags()
        assert flags['z'] == 0  # bit 3 is set → Z flag is 0

    def test_set_ix_d(self):
        """DDCB — SET n,(IX+d): set bit n in (IX+d)."""
        sim = make_sim()
        sim._memory[0x8000] = 0x00
        program = bytes([
            0xDD, 0x21, 0x00, 0x80,  # LD IX, 0x8000
            0xDD, 0xCB, 0x00, 0xC6,  # SET 0, (IX+0)  → bit 0 set
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._memory[0x8000] == 0x01

    def test_res_ix_d(self):
        """DDCB — RES n,(IX+d): clear bit n in (IX+d)."""
        sim = make_sim()
        sim._memory[0x8000] = 0xFF
        program = bytes([
            0xDD, 0x21, 0x00, 0x80,  # LD IX, 0x8000
            0xDD, 0xCB, 0x00, 0x86,  # RES 0, (IX+0)  → bit 0 cleared
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._memory[0x8000] == 0xFE

    def test_rlc_ix_d(self):
        """DDCB — RLC (IX+d)."""
        sim = make_sim()
        sim._memory[0x8000] = 0x80  # 10000000 → RLC → 00000001, C=1
        program = bytes([
            0xDD, 0x21, 0x00, 0x80,  # LD IX, 0x8000
            0xDD, 0xCB, 0x00, 0x06,  # RLC (IX+0)
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._memory[0x8000] == 0x01

    def test_rl_ix_d(self):
        """DDCB — RL (IX+d) with carry clear."""
        sim = make_sim()
        sim._memory[0x8000] = 0x80  # 10000000, C=0 → RL → 00000000, C=1
        program = bytes([
            0xAF,                    # XOR A (clear carry)
            0xDD, 0x21, 0x00, 0x80,  # LD IX, 0x8000
            0xDD, 0xCB, 0x00, 0x16,  # RL (IX+0)
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._memory[0x8000] == 0x00
        flags = sim._rf.read_flags()
        assert flags['c'] == 1


# ── ED extended instructions ────────────────────────────────────────────────────

class TestEDInstructions:
    def test_ld_i_a(self):
        """ED 47 — LD I, A."""
        program = bytes([
            0x3E, 0x42,  # LD A, 0x42
            0xED, 0x47,  # LD I, A
            0x76,
        ])
        sim = make_sim()
        sim.execute(program)
        assert sim._i == 0x42

    def test_ld_r_a(self):
        """ED 4F — LD R, A: R auto-increments each instruction fetch.

        After LD R,A with value 0x10, the R register is immediately
        incremented by the subsequent fetch of HALT (0x76). So we check
        that R was set to something near 0x10 rather than an exact value.
        The key test is that LD R,A writes A into R (LD R,A executes, then
        R increments for HALT fetch). Net R = (0x10 + 1) & 0x7F = 0x11.
        """
        program = bytes([
            0x3E, 0x10,  # LD A, 0x10
            0xED, 0x4F,  # LD R, A   (R ← A, then R++ for HALT fetch)
            0x76,
        ])
        sim = make_sim()
        sim.execute(program)
        # After setting R=0x10, one more fetch (HALT) increments R to 0x11
        assert sim._r == 0x11

    def test_ld_a_i(self):
        """ED 57 — LD A, I."""
        sim = make_sim()
        sim._i = 0x55
        program = bytes([
            0xED, 0x57,  # LD A, I
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._rf.read8(7) == 0x55  # A = I

    def test_ld_a_r(self):
        """ED 5F — LD A, R: read R into A.

        R auto-increments on each instruction fetch. Setting sim._r = 0x22
        and then executing LD A,R: the fetch of ED increments R to 0x23,
        the fetch of 5F increments R to 0x24. Then LD A,R reads R=0x24.
        (The HALT fetch bumps R to 0x25 after the read.)
        """
        sim = make_sim()
        sim._r = 0x22
        program = bytes([
            0xED, 0x5F,  # LD A, R
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        # R was 0x22; incremented by: fetch(ED)=0x23, fetch(5F)=0x24
        # LD A, R reads 0x24 at that moment
        assert sim._rf.read8(7) == 0x24  # A = R value at time of read

    def test_im_modes(self):
        """ED 46 = IM 0; ED 56 = IM 1; ED 5E = IM 2."""
        program = bytes([
            0xED, 0x56,  # IM 1
            0x76,
        ])
        sim = make_sim()
        sim.execute(program)
        assert sim._im == 1

        program2 = bytes([0xED, 0x5E, 0x76])  # IM 2
        sim2 = make_sim()
        sim2.execute(program2)
        assert sim2._im == 2

        program3 = bytes([0xED, 0x46, 0x76])  # IM 0
        sim3 = make_sim()
        sim3.execute(program3)
        assert sim3._im == 0

    def test_reti(self):
        """ED 4D — RETI: return from interrupt (RET equivalent here)."""
        program = bytes([
            0x31, 0x00, 0x80,        # LD SP, 0x8000
            0xCD, 0x08, 0x00,        # CALL 0x0008
            0x76,                    # HALT (at 0x0006)
            0x00,                    # NOP  (at 0x0007) padding
            0x3E, 0x42,              # LD A, 0x42  (at 0x0008)
            0xED, 0x4D,              # RETI        (at 0x000A)
        ])
        state = run(program)
        assert state.a == 0x42

    def test_retn(self):
        """ED 45 — RETN: return from NMI (RET equivalent here)."""
        program = bytes([
            0x31, 0x00, 0x80,        # LD SP, 0x8000
            0xCD, 0x08, 0x00,        # CALL 0x0008
            0x76,                    # HALT (at 0x0006)
            0x00,                    # NOP  (at 0x0007) padding
            0x3E, 0x33,              # LD A, 0x33  (at 0x0008)
            0xED, 0x45,              # RETN        (at 0x000A)
        ])
        state = run(program)
        assert state.a == 0x33

    def test_ld_rp_nn_from_mem(self):
        """ED 4B — LD BC, (nn): load register pair from memory."""
        sim = make_sim()
        sim._memory[0x9000] = 0x34
        sim._memory[0x9001] = 0x12
        program = bytes([
            0xED, 0x4B, 0x00, 0x90,  # LD BC, (0x9000)
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._rf.read8(0) == 0x12  # B = hi
        assert sim._rf.read8(1) == 0x34  # C = lo

    def test_ld_nn_rp(self):
        """ED 43 — LD (nn), BC: store register pair to memory."""
        program = bytes([
            0x01, 0xCD, 0xAB,        # LD BC, 0xABCD
            0xED, 0x43, 0x00, 0x90,  # LD (0x9000), BC
            0x76,
        ])
        sim = make_sim()
        sim.execute(program)
        assert sim._memory[0x9000] == 0xCD  # lo
        assert sim._memory[0x9001] == 0xAB  # hi

    def test_in_r_c(self):
        """ED xx — IN r,(C): read from port C into register r."""
        sim = make_sim()
        sim.set_input_port(0x20, 0xBB)  # port 0x20 returns 0xBB
        program = bytes([
            0x0E, 0x20,              # LD C, 0x20
            0xED, 0x40,              # IN B,(C)
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._rf.read8(0) == 0xBB  # B

    def test_out_c_r(self):
        """ED xx — OUT (C),r: write register r to port C."""
        program = bytes([
            0x0E, 0x30,              # LD C, 0x30
            0x06, 0x77,              # LD B, 0x77
            0xED, 0x41,              # OUT (C),B
            0x76,
        ])
        sim = make_sim()
        sim.execute(program)
        assert sim.get_output_port(0x30) == 0x77


# ── Block operations ─────────────────────────────────────────────────────────

class TestBlockOps:
    def _setup_ldir_sim(self) -> Z80GateLevelSimulator:
        """Set up a sim with source data for block ops."""
        sim = make_sim()
        sim._memory[0x8000] = 0x11
        sim._memory[0x8001] = 0x22
        sim._memory[0x8002] = 0x33
        return sim

    def test_ldi(self):
        """ED A0 — LDI: copy one byte (HL)→(DE), inc both, dec BC."""
        sim = self._setup_ldir_sim()
        program = bytes([
            0x21, 0x00, 0x80,  # LD HL, 0x8000 (src)
            0x11, 0x00, 0xA0,  # LD DE, 0xA000 (dst)
            0x01, 0x01, 0x00,  # LD BC, 1
            0xED, 0xA0,        # LDI
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._memory[0xA000] == 0x11

    def test_ldd(self):
        """ED A8 — LDD: copy one byte (HL)→(DE), dec both, dec BC."""
        sim = self._setup_ldir_sim()
        program = bytes([
            0x21, 0x02, 0x80,  # LD HL, 0x8002 (src end)
            0x11, 0x02, 0xA0,  # LD DE, 0xA002 (dst end)
            0x01, 0x01, 0x00,  # LD BC, 1
            0xED, 0xA8,        # LDD
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._memory[0xA002] == 0x33

    def test_ldir(self):
        """ED B0 — LDIR: block copy HL→DE, BC bytes."""
        sim = make_sim()
        sim._memory[0x8000] = 0xAA
        sim._memory[0x8001] = 0xBB
        sim._memory[0x8002] = 0xCC
        program = bytes([
            0x21, 0x00, 0x80,  # LD HL, 0x8000
            0x11, 0x00, 0x90,  # LD DE, 0x9000
            0x01, 0x03, 0x00,  # LD BC, 3
            0xED, 0xB0,        # LDIR
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._memory[0x9000] == 0xAA
        assert sim._memory[0x9001] == 0xBB
        assert sim._memory[0x9002] == 0xCC

    def test_lddr(self):
        """ED B8 — LDDR: block copy downwards."""
        sim = make_sim()
        sim._memory[0x8002] = 0x11
        sim._memory[0x8001] = 0x22
        sim._memory[0x8000] = 0x33
        program = bytes([
            0x21, 0x02, 0x80,  # LD HL, 0x8002 (src end)
            0x11, 0x02, 0x90,  # LD DE, 0x9002 (dst end)
            0x01, 0x03, 0x00,  # LD BC, 3
            0xED, 0xB8,        # LDDR
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._memory[0x9002] == 0x11
        assert sim._memory[0x9001] == 0x22
        assert sim._memory[0x9000] == 0x33

    def test_cpi(self):
        """ED A1 — CPI: compare A with (HL); HL++; BC--."""
        program = bytes([
            0x3E, 0x42,        # LD A, 0x42
            0x21, 0x00, 0x80,  # LD HL, 0x8000
            0x01, 0x01, 0x00,  # LD BC, 1
            0xED, 0xA1,        # CPI
            0x76,
        ])
        sim = make_sim()
        sim._memory[0x8000] = 0x42  # match!
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        flags = sim._rf.read_flags()
        assert flags['z'] == 1  # A == (HL)

    def test_cpd(self):
        """ED A9 — CPD: compare A with (HL); HL--; BC--."""
        program = bytes([
            0x3E, 0x99,        # LD A, 0x99
            0x21, 0x02, 0x80,  # LD HL, 0x8002
            0x01, 0x01, 0x00,  # LD BC, 1
            0xED, 0xA9,        # CPD
            0x76,
        ])
        sim = make_sim()
        sim._memory[0x8002] = 0x99  # match
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        flags = sim._rf.read_flags()
        assert flags['z'] == 1

    def test_cpir(self):
        """ED B1 — CPIR: search forward until match or BC=0."""
        program = bytes([
            0x3E, 0xBB,        # LD A, 0xBB
            0x21, 0x00, 0x80,  # LD HL, 0x8000
            0x01, 0x03, 0x00,  # LD BC, 3
            0xED, 0xB1,        # CPIR
            0x76,
        ])
        sim = make_sim()
        sim._memory[0x8000] = 0x11
        sim._memory[0x8001] = 0xBB  # match at offset 1
        sim._memory[0x8002] = 0x33
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        flags = sim._rf.read_flags()
        assert flags['z'] == 1  # found the match

    def test_cpdr(self):
        """ED B9 — CPDR: search backward until match or BC=0."""
        program = bytes([
            0x3E, 0x44,        # LD A, 0x44
            0x21, 0x02, 0x80,  # LD HL, 0x8002
            0x01, 0x03, 0x00,  # LD BC, 3
            0xED, 0xB9,        # CPDR
            0x76,
        ])
        sim = make_sim()
        sim._memory[0x8002] = 0x55
        sim._memory[0x8001] = 0x44  # match at offset 1 backwards
        sim._memory[0x8000] = 0x33
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        flags = sim._rf.read_flags()
        assert flags['z'] == 1  # found the match


# ── Conditional branches ─────────────────────────────────────────────────────

class TestConditionalBranches:
    def test_jp_z_taken(self):
        """JP Z, nn — jump taken when Z=1."""
        program = bytes([
            0xAF,              # XOR A (sets Z=1)
            0xCA, 0x06, 0x00,  # JP Z, 0x0006
            0x3E, 0xFF,        # LD A, 0xFF  (skipped)
            0x76,              # HALT at 0x0006
        ])
        state = run(program)
        assert state.a == 0x00  # XOR A result preserved

    def test_jp_z_not_taken(self):
        """JP Z, nn — jump NOT taken when Z=0.

        Z80 resets with F=0xFF (Z=1). To get Z=0 we must execute an
        ALU operation that produces a non-zero result. OR A with 1 = 1,
        Z=0.
        """
        program = bytes([
            0x3E, 0x01,        # LD A, 1
            0xB7,              # OR A   (A | A = 1, Z=0)
            0xCA, 0x09, 0x00,  # JP Z, 0x0009 (NOT taken — Z=0)
            0x3E, 0x42,        # LD A, 0x42   (executed)
            0x76,
        ])
        state = run(program)
        assert state.a == 0x42  # executed the LD A after JP

    def test_jp_nc_taken(self):
        """JP NC, nn — jump taken when C=0."""
        program = bytes([
            0xAF,              # XOR A (C=0)
            0xD2, 0x05, 0x00,  # JP NC, 0x0005
            0x76,              # HALT — skipped
            0x3E, 0x77,        # LD A, 0x77 at 0x0005
            0x76,
        ])
        state = run(program)
        assert state.a == 0x77

    def test_jp_c_taken(self):
        """JP C, nn — jump taken when C=1."""
        program = bytes([
            0x37,              # SCF (C=1)
            0xDA, 0x05, 0x00,  # JP C, 0x0005
            0x76,              # HALT — skipped
            0x3E, 0x55,        # LD A, 0x55 at 0x0005
            0x76,
        ])
        state = run(program)
        assert state.a == 0x55

    def test_jp_po_pe(self):
        """JP PO/PE — parity odd / parity even."""
        # XOR A gives A=0, P/V=1 (even parity → PE)
        program = bytes([
            0xAF,              # XOR A  (P/V=1 = even parity)
            0xEA, 0x05, 0x00,  # JP PE, 0x0005
            0x76,              # skipped
            0x3E, 0x66,        # LD A, 0x66 at 0x0005
            0x76,
        ])
        state = run(program)
        assert state.a == 0x66

    def test_jp_p_m(self):
        """JP P/M — positive/minus (sign flag).

        SUB 1 from A=0 gives A=0xFF, S=1 (bit 7 set = negative).
        JP M jumps when S=1.
        """
        program = bytes([
            0x3E, 0x00,        # LD A, 0           (0x0000)
            0xD6, 0x01,        # SUB 1 → A=0xFF, S=1  (0x0002)
            0xFA, 0x08, 0x00,  # JP M, 0x0008       (0x0004) — taken (S=1)
            0x76,              # HALT (skipped)     (0x0007)
            0x3E, 0x11,        # LD A, 0x11         (0x0008)
            0x76,              # HALT               (0x000A)
        ])
        state = run(program)
        assert state.a == 0x11

    def test_jr_z_taken(self):
        """JR Z, e — taken when Z=1.

        After fetching the displacement byte, PC = 0x0003.
        JR Z, +2 → new PC = 0x0003 + 2 = 0x0005 (the HALT).
        The LD A, 0xFF at 0x0003 is therefore skipped.
        """
        # 0x0000: AF      XOR A  (Z=1)
        # 0x0001: 28 02   JR Z, +2  → jump to 0x0005 (after PC=0x0003)
        # 0x0003: 3E FF   LD A, 0xFF  (skipped)
        # 0x0005: 76      HALT
        program = bytes([
            0xAF,        # XOR A   (Z=1)
            0x28, 0x02,  # JR Z, +2
            0x3E, 0xFF,  # LD A, 0xFF  (skipped)
            0x76,        # HALT at 0x0005
        ])
        state = run(program)
        assert state.a == 0x00  # XOR A result; 0xFF was skipped

    def test_jr_nc_jrc(self):
        """JR NC and JR C."""
        # JR NC: C=0 after XOR A → branch taken
        program_nc = bytes([
            0xAF,        # XOR A (C=0)
            0x30, 0x02,  # JR NC, +2  (to 0x0005)
            0x3E, 0xFF,  # LD A, 0xFF (skipped)
            0x76,        # HALT at 0x0005
        ])
        state = run(program_nc)
        assert state.a == 0x00

        # JR C: SCF sets C=1 → branch taken
        program_c = bytes([
            0x37,        # SCF (C=1)
            0x38, 0x02,  # JR C, +2  (to 0x0005)
            0x3E, 0xFF,  # LD A, 0xFF (skipped)
            0x76,        # HALT at 0x0005
        ])
        state2 = run(program_c)
        # A was unset before, check Z flag was carried from SCF
        assert state2.flag_c is True

    def test_call_cc_taken_and_not_taken(self):
        """CALL cc, nn — conditional call.

        Z80 resets with Z=1. Use OR A on a non-zero value to get Z=0,
        enabling CALL NZ to be taken.
        """
        # CALL NZ — NZ taken (Z=0 after OR A with non-zero A)
        program_taken = bytes([
            0x31, 0x00, 0x80,        # LD SP, 0x8000   (0x0000)
            0x3E, 0x01,              # LD A, 1         (0x0003)
            0xB7,                    # OR A → Z=0      (0x0005)
            0xC4, 0x0D, 0x00,        # CALL NZ, 0x000D (0x0006, taken — Z=0)
            0x76,                    # HALT             (0x0009 — reached after RET)
            0x00, 0x00, 0x00,        # padding
            0x3E, 0x44,              # LD A, 0x44      at 0x000D
            0xC9,                    # RET
        ])
        state = run(program_taken)
        assert state.a == 0x44

        # CALL Z — not taken because Z=0
        program_not_taken = bytes([
            0x31, 0x00, 0x80,        # LD SP, 0x8000
            0x3E, 0x01,              # LD A, 1
            0xB7,                    # OR A → Z=0
            0xCC, 0x10, 0x00,        # CALL Z, 0x0010 (NOT taken — Z=0)
            0x3E, 0x55,              # LD A, 0x55
            0x76,                    # HALT
        ])
        state2 = run(program_not_taken)
        assert state2.a == 0x55

    def test_ret_cc(self):
        """RET cc — conditional return."""
        # RET NZ: call subroutine, Z=1 means RET Z is taken
        program = bytes([
            0x31, 0x00, 0x80,        # LD SP, 0x8000
            0xCD, 0x09, 0x00,        # CALL 0x0009   (at 0x0003)
            0x3E, 0x44,              # LD A, 0x44    (at 0x0006, after return)
            0x76,                    # HALT           (at 0x0008)
            0xAF,                    # XOR A (Z=1)   (at 0x0009)
            0xC8,                    # RET Z          (at 0x000A, Z=1 so return)
            0x3E, 0xFF,              # LD A, 0xFF    (0x000B, skipped)
            0xC9,                    # RET            (0x000D)
        ])
        state = run(program)
        assert state.a == 0x44  # returned from subroutine at RET Z, got LD A,0x44


# ── Memory-indirect load/store instructions ───────────────────────────────────

class TestMemoryIndirect:
    def test_ld_a_bc(self):
        """0A — LD A, (BC)."""
        sim = make_sim()
        sim._memory[0x9000] = 0xBB
        program = bytes([
            0x01, 0x00, 0x90,  # LD BC, 0x9000
            0x0A,              # LD A, (BC)
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._rf.read8(7) == 0xBB

    def test_ld_bc_a(self):
        """02 — LD (BC), A."""
        program = bytes([
            0x3E, 0xCC,        # LD A, 0xCC
            0x01, 0x00, 0x90,  # LD BC, 0x9000
            0x02,              # LD (BC), A
            0x76,
        ])
        sim = make_sim()
        sim.execute(program)
        assert sim._memory[0x9000] == 0xCC

    def test_ld_a_de(self):
        """1A — LD A, (DE)."""
        sim = make_sim()
        sim._memory[0x9001] = 0xDD
        program = bytes([
            0x11, 0x01, 0x90,  # LD DE, 0x9001
            0x1A,              # LD A, (DE)
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._rf.read8(7) == 0xDD

    def test_ld_de_a(self):
        """12 — LD (DE), A."""
        program = bytes([
            0x3E, 0xEE,        # LD A, 0xEE
            0x11, 0x01, 0x90,  # LD DE, 0x9001
            0x12,              # LD (DE), A
            0x76,
        ])
        sim = make_sim()
        sim.execute(program)
        assert sim._memory[0x9001] == 0xEE

    def test_ld_a_nn(self):
        """3A — LD A, (nn): load A from absolute address."""
        sim = make_sim()
        sim._memory[0xB000] = 0xAB
        program = bytes([
            0x3A, 0x00, 0xB0,  # LD A, (0xB000)
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._rf.read8(7) == 0xAB

    def test_ld_nn_a(self):
        """32 — LD (nn), A: store A to absolute address."""
        program = bytes([
            0x3E, 0x99,        # LD A, 0x99
            0x32, 0x00, 0xC0,  # LD (0xC000), A
            0x76,
        ])
        sim = make_sim()
        sim.execute(program)
        assert sim._memory[0xC000] == 0x99

    def test_ld_hl_nn(self):
        """2A — LD HL, (nn): load HL from absolute address."""
        sim = make_sim()
        sim._memory[0xD000] = 0x78  # lo
        sim._memory[0xD001] = 0x56  # hi
        program = bytes([
            0x2A, 0x00, 0xD0,  # LD HL, (0xD000)
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._rf.read8(4) == 0x56  # H
        assert sim._rf.read8(5) == 0x78  # L

    def test_ld_nn_hl(self):
        """22 — LD (nn), HL: store HL to absolute address."""
        program = bytes([
            0x21, 0x34, 0x12,  # LD HL, 0x1234  (L=0x34, H=0x12)
            0x22, 0x00, 0xE0,  # LD (0xE000), HL
            0x76,
        ])
        sim = make_sim()
        sim.execute(program)
        assert sim._memory[0xE000] == 0x34  # lo (L)
        assert sim._memory[0xE001] == 0x12  # hi (H)


# ── Exchange instructions ────────────────────────────────────────────────────

class TestExchangeInstructions:
    def test_ex_de_hl(self):
        """EB — EX DE, HL: exchange DE and HL."""
        program = bytes([
            0x11, 0x78, 0x56,  # LD DE, 0x5678
            0x21, 0x34, 0x12,  # LD HL, 0x1234
            0xEB,              # EX DE, HL
            0x76,
        ])
        state = run(program)
        assert state.d == 0x12
        assert state.e == 0x34
        assert state.h == 0x56
        assert state.l == 0x78

    def test_ex_sp_hl(self):
        """E3 — EX (SP), HL: exchange top-of-stack with HL."""
        sim = make_sim()
        # Stack at 0x8000, contains 0xBEEF
        sim._memory[0x7FFE] = 0xEF  # lo
        sim._memory[0x7FFF] = 0xBE  # hi
        program = bytes([
            0x31, 0xFE, 0x7F,  # LD SP, 0x7FFE
            0x21, 0x34, 0x12,  # LD HL, 0x1234
            0xE3,              # EX (SP), HL  → HL=0xBEEF, (SP)=0x1234
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._rf.read8(4) == 0xBE  # H
        assert sim._rf.read8(5) == 0xEF  # L
        assert sim._memory[0x7FFE] == 0x34
        assert sim._memory[0x7FFF] == 0x12


# ── I/O instructions ─────────────────────────────────────────────────────────

class TestIOInstructions:
    def test_out_n_a(self):
        """D3 — OUT (n), A: write A to port n."""
        program = bytes([
            0x3E, 0x42,  # LD A, 0x42
            0xD3, 0x10,  # OUT (0x10), A
            0x76,
        ])
        sim = make_sim()
        sim.execute(program)
        assert sim.get_output_port(0x10) == 0x42

    def test_in_a_n(self):
        """DB — IN A, (n): read from port n into A."""
        sim = make_sim()
        sim.set_input_port(0x20, 0x99)
        program = bytes([
            0xDB, 0x20,  # IN A, (0x20)
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._rf.read8(7) == 0x99

    def test_ei_di(self):
        """EI (0xFB) and DI (0xF3)."""
        program = bytes([
            0xF3,  # DI
            0xFB,  # EI
            0x76,
        ])
        sim = make_sim()
        sim.execute(program)
        assert sim._iff1 is True

    def test_io_port_error(self):
        """set_input_port and get_output_port raise on bad port."""
        sim = make_sim()
        with pytest.raises(ValueError):
            sim.set_input_port(256, 0)
        with pytest.raises(ValueError):
            sim.set_input_port(0, 256)
        with pytest.raises(ValueError):
            sim.get_output_port(-1)


# ── ADC / SBC register variants ──────────────────────────────────────────────

class TestADCSBCVariants:
    def test_adc_a_r(self):
        """ADC A, r — add with carry."""
        program = bytes([
            0xAF,        # XOR A (C=0)
            0x3E, 0x05,  # LD A, 5
            0x06, 0x03,  # LD B, 3
            0x88,        # ADC A, B  (5 + 3 + 0 = 8)
            0x76,
        ])
        state = run(program)
        assert state.a == 8

    def test_adc_a_r_with_carry(self):
        """ADC A, r with C=1."""
        program = bytes([
            0x37,        # SCF (C=1)
            0x3E, 0x05,  # LD A, 5
            0x06, 0x03,  # LD B, 3
            0x88,        # ADC A, B  (5 + 3 + 1 = 9)
            0x76,
        ])
        state = run(program)
        assert state.a == 9

    def test_sbc_a_r(self):
        """SBC A, r — subtract with borrow."""
        program = bytes([
            0xAF,        # XOR A (C=0)
            0x3E, 0x0A,  # LD A, 10
            0x06, 0x03,  # LD B, 3
            0x98,        # SBC A, B  (10 - 3 - 0 = 7)
            0x76,
        ])
        state = run(program)
        assert state.a == 7

    def test_sbc_a_r_with_borrow(self):
        """SBC A, r with C=1 (borrow)."""
        program = bytes([
            0x37,        # SCF (C=1)
            0x3E, 0x0A,  # LD A, 10
            0x06, 0x03,  # LD B, 3
            0x98,        # SBC A, B  (10 - 3 - 1 = 6)
            0x76,
        ])
        state = run(program)
        assert state.a == 6

    def test_adc_a_immediate(self):
        """CE — ADC A, n."""
        program = bytes([
            0x37,        # SCF
            0x3E, 0x10,  # LD A, 16
            0xCE, 0x04,  # ADC A, 4  (16 + 4 + 1 = 21)
            0x76,
        ])
        state = run(program)
        assert state.a == 21

    def test_sbc_a_immediate(self):
        """DE — SBC A, n."""
        program = bytes([
            0xAF,        # XOR A (C=0)
            0x3E, 0x10,  # LD A, 16
            0xDE, 0x04,  # SBC A, 4  (16 - 4 - 0 = 12)
            0x76,
        ])
        state = run(program)
        assert state.a == 12


# ── LD SP,HL, RST, DAA ───────────────────────────────────────────────────────

class TestMiscInstructions:
    def test_ld_sp_hl(self):
        """F9 — LD SP, HL."""
        program = bytes([
            0x21, 0x00, 0x80,  # LD HL, 0x8000
            0xF9,              # LD SP, HL
            0x76,
        ])
        state = run(program)
        assert state.sp == 0x8000

    def test_rst(self):
        """RST p: push PC and jump to p*8."""
        # RST 0x08 (opcode 0xCF): push PC, jump to 0x0008
        # Code at 0x0008 loads A=0x77, returns
        program = bytes([
            0x31, 0x00, 0x80,  # LD SP, 0x8000
            0xCF,              # RST 0x08     (at 0x0003)
            0x76,              # HALT         (at 0x0004 - reached after RET)
            0x00, 0x00, 0x00,  # padding
            0x3E, 0x77,        # LD A, 0x77   at 0x0008
            0xC9,              # RET
        ])
        state = run(program)
        assert state.a == 0x77

    def test_daa_after_add(self):
        """DAA: BCD adjust after ADD."""
        # 0x05 + 0x05 = 0x0A (binary) → DAA → 0x10 (BCD for 5+5=10)
        program = bytes([
            0xAF,        # XOR A (clear flags, C=0)
            0x3E, 0x05,  # LD A, 0x05
            0xC6, 0x05,  # ADD A, 0x05  → A=0x0A
            0x27,        # DAA          → A=0x10
            0x76,
        ])
        state = run(program)
        assert state.a == 0x10

    def test_jr_unconditional(self):
        """18 — JR e: unconditional relative jump."""
        program = bytes([
            0x18, 0x02,  # JR +2  (jump over next instruction)
            0x3E, 0xFF,  # LD A, 0xFF (skipped)
            0x76,        # HALT
        ])
        state = run(program)
        assert state.a != 0xFF


# ── step() error and max_steps guard ────────────────────────────────────────

class TestSimulatorErrors:
    def test_step_when_halted_raises(self):
        """step() on a halted CPU raises RuntimeError."""
        sim = make_sim()
        sim.execute(bytes([0x76]))  # HALT immediately
        with pytest.raises(RuntimeError, match="halted"):
            sim.step()

    def test_max_steps_guard(self):
        """execute() stops after max_steps without halting."""
        # Infinite loop: JR -2 (self-referential loop)
        program = bytes([
            0x18, 0xFE,  # JR -2 (loops forever)
        ])
        sim = make_sim()
        result = sim.execute(program, max_steps=50)
        assert result.halted is False
        assert result.steps == 50

    def test_get_state_returns_z80state(self):
        """get_state() returns a Z80State with expected fields."""
        program = bytes([
            0x3E, 0x42,  # LD A, 0x42
            0x76,
        ])
        sim, state = run_sim(program)
        assert state.a == 0x42
        assert hasattr(state, 'flag_z')
        assert hasattr(state, 'flag_c')
        assert hasattr(state, 'pc')


# ── 16-bit INC/DEC register pairs ────────────────────────────────────────────

class TestRPIncDec:
    def test_inc_bc(self):
        """03 — INC BC."""
        program = bytes([
            0x01, 0xFF, 0x00,  # LD BC, 0x00FF
            0x03,              # INC BC  → 0x0100
            0x76,
        ])
        state = run(program)
        assert state.b == 0x01
        assert state.c == 0x00

    def test_inc_de(self):
        """13 — INC DE."""
        program = bytes([
            0x11, 0xFF, 0xFF,  # LD DE, 0xFFFF
            0x13,              # INC DE  → 0x0000
            0x76,
        ])
        state = run(program)
        assert state.d == 0x00
        assert state.e == 0x00

    def test_dec_hl(self):
        """2B — DEC HL."""
        program = bytes([
            0x21, 0x00, 0x10,  # LD HL, 0x1000
            0x2B,              # DEC HL  → 0x0FFF
            0x76,
        ])
        state = run(program)
        assert state.h == 0x0F
        assert state.l == 0xFF

    def test_inc_sp(self):
        """33 — INC SP."""
        program = bytes([
            0x31, 0xFF, 0xFF,  # LD SP, 0xFFFF
            0x33,              # INC SP  → 0x0000
            0x76,
        ])
        state = run(program)
        assert state.sp == 0x0000

    def test_dec_sp(self):
        """3B — DEC SP."""
        program = bytes([
            0x31, 0x00, 0x10,  # LD SP, 0x1000
            0x3B,              # DEC SP  → 0x0FFF
            0x76,
        ])
        state = run(program)
        assert state.sp == 0x0FFF


# ── CB rotate/shift on memory (HL) ──────────────────────────────────────────

class TestCBMemoryOps:
    def test_rlc_hl(self):
        """CB 06 — RLC (HL)."""
        sim = make_sim()
        sim._memory[0x8000] = 0x80
        program = bytes([
            0x21, 0x00, 0x80,  # LD HL, 0x8000
            0xCB, 0x06,        # RLC (HL)
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._memory[0x8000] == 0x01

    def test_bit_hl(self):
        """CB 5E — BIT 3, (HL)."""
        sim = make_sim()
        sim._memory[0x8000] = 0x08  # bit 3 set
        program = bytes([
            0x21, 0x00, 0x80,  # LD HL, 0x8000
            0xCB, 0x5E,        # BIT 3, (HL)
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        flags = sim._rf.read_flags()
        assert flags['z'] == 0  # bit 3 is set → Z=0

    def test_res_hl(self):
        """CB 86 — RES 0, (HL)."""
        sim = make_sim()
        sim._memory[0x8000] = 0xFF
        program = bytes([
            0x21, 0x00, 0x80,  # LD HL, 0x8000
            0xCB, 0x86,        # RES 0, (HL)
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._memory[0x8000] == 0xFE

    def test_set_hl(self):
        """CB C6 — SET 0, (HL)."""
        sim = make_sim()
        sim._memory[0x8000] = 0x00
        program = bytes([
            0x21, 0x00, 0x80,  # LD HL, 0x8000
            0xCB, 0xC6,        # SET 0, (HL)
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._memory[0x8000] == 0x01

    def test_sra_reg(self):
        """CB 2x — SRA r: arithmetic right shift (bit 7 preserved)."""
        program = bytes([
            0x3E, 0x80,  # LD A, 0x80 (10000000)
            0xCB, 0x2F,  # SRA A → 11000000 = 0xC0
            0x76,
        ])
        state = run(program)
        assert state.a == 0xC0  # bit 7 duplicated

    def test_srl_reg(self):
        """CB 3x — SRL r: logical right shift (bit 7 = 0)."""
        program = bytes([
            0x3E, 0x80,  # LD A, 0x80 (10000000)
            0xCB, 0x3F,  # SRL A → 01000000 = 0x40
            0x76,
        ])
        state = run(program)
        assert state.a == 0x40  # bit 7 becomes 0

    def test_rl_rr_through_carry(self):
        """CB 1x RL and CB 1x RR rotate through carry."""
        # RL A with C=1: shift left, bit0 gets old C=1
        program_rl = bytes([
            0x37,        # SCF (C=1)
            0x3E, 0x40,  # LD A, 0x40 (01000000)
            0xCB, 0x17,  # RL A → (0b10000001) = 0x81, C=0
            0x76,
        ])
        state = run(program_rl)
        assert state.a == 0x81

        # RR A with C=1: shift right, bit7 gets old C=1
        program_rr = bytes([
            0x37,        # SCF (C=1)
            0x3E, 0x02,  # LD A, 0x02 (00000010)
            0xCB, 0x1F,  # RR A → (0b10000001) = 0x81, C=0
            0x76,
        ])
        state2 = run(program_rr)
        assert state2.a == 0x81


# ── INC/DEC on (HL) ─────────────────────────────────────────────────────────

class TestIncDecMemory:
    def test_inc_hl_mem(self):
        """34 — INC (HL): increment byte at (HL)."""
        sim = make_sim()
        sim._memory[0x8000] = 0x0F
        program = bytes([
            0x21, 0x00, 0x80,  # LD HL, 0x8000
            0x34,              # INC (HL)  → 0x10
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._memory[0x8000] == 0x10

    def test_dec_hl_mem(self):
        """35 — DEC (HL): decrement byte at (HL)."""
        sim = make_sim()
        sim._memory[0x8000] = 0x10
        program = bytes([
            0x21, 0x00, 0x80,  # LD HL, 0x8000
            0x35,              # DEC (HL)  → 0x0F
            0x76,
        ])
        for i, b in enumerate(program):
            sim._memory[i] = b
        sim._pc.write(0)
        while not sim._halted:
            sim.step()
        assert sim._memory[0x8000] == 0x0F


# ── LD (HL), n ───────────────────────────────────────────────────────────────

class TestLDHLn:
    def test_ld_hl_n(self):
        """36 — LD (HL), n: store immediate to (HL)."""
        program = bytes([
            0x21, 0x00, 0x80,  # LD HL, 0x8000
            0x36, 0xAB,        # LD (HL), 0xAB
            0x76,
        ])
        sim = make_sim()
        sim.execute(program)
        assert sim._memory[0x8000] == 0xAB


# ── JP (HL) ──────────────────────────────────────────────────────────────────

class TestJPHL:
    def test_jp_hl(self):
        """E9 — JP (HL): jump to address in HL."""
        program = bytes([
            0x21, 0x07, 0x00,  # LD HL, 0x0007
            0xE9,              # JP (HL)
            0x3E, 0xFF,        # LD A, 0xFF (skipped)
            0x00,              # NOP padding
            0x3E, 0x42,        # LD A, 0x42 at 0x0007
            0x76,
        ])
        state = run(program)
        assert state.a == 0x42
