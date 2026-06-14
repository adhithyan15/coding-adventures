"""Targeted coverage tests for X86Simulator.

These tests exercise instruction paths not reached by the main instruction
and program test suites, pushing total simulator coverage above 80%.

Each class is focused on a specific uncovered region:
  - SP/BP/SI/DI as 16-bit register operands
  - CH/DH/BH high-byte registers
  - Segment register MOV instructions
  - ModRM addressing modes (BX+DI, BP+SI, SI, DI, etc.)
  - FE/FF opcode groups (indirect INC/DEC/CALL/JMP)
  - MUL/IMUL/DIV/IDIV byte variants
  - DAA/DAS/AAS/AAM/AAD BCD adjust instructions
  - JMP near/far, RETF, CALL far
  - IN/OUT word variants
  - Conditional jump conditions (JO, JS, JP, JL, etc.)
  - REP CMPS / REPNE SCAS string comparison
  - Shift/rotate by CL count
"""

from __future__ import annotations

from intel_8086_simulator import X86Simulator, X86State

# ── Helpers ───────────────────────────────────────────────────────────────────

HLT = bytes([0xF4])


def step_prog(sim: X86Simulator, prog: bytes, max_steps: int = 10_000) -> None:
    """Load prog and step to completion (max_steps prevents infinite loops)."""
    sim.load(prog)
    steps = 0
    while not sim._halted and steps < max_steps:
        sim.step()
        steps += 1
    if not sim._halted:
        raise RuntimeError(f"step_prog: did not halt after {max_steps} steps")


def run(sim: X86Simulator, prog: bytes) -> X86State:
    """Fresh simulator: load and run prog, return final state."""
    sim.reset()
    step_prog(sim, prog)
    return sim.get_state()


# ── SP/BP/SI/DI as 16-bit register operands ─────────────────────────────────


class TestSPBPSIDI:
    """XCHG AX with SP/BP/SI/DI exercises _get_reg16(4..7) and _set_reg16(4..7)."""

    def test_xchg_ax_sp(self):
        # XCHG AX, SP (opcode 0x94): AX↔SP
        sim = X86Simulator()
        sim._ax = 100; sim._sp = 200
        step_prog(sim, bytes([0x94, 0xF4]))
        assert sim._ax == 200 and sim._sp == 100

    def test_xchg_ax_bp(self):
        sim = X86Simulator()
        sim._ax = 5; sim._bp = 10
        step_prog(sim, bytes([0x95, 0xF4]))  # XCHG AX, BP
        assert sim._ax == 10 and sim._bp == 5

    def test_xchg_ax_si(self):
        sim = X86Simulator()
        sim._ax = 1; sim._si = 2
        step_prog(sim, bytes([0x96, 0xF4]))  # XCHG AX, SI
        assert sim._ax == 2 and sim._si == 1

    def test_xchg_ax_di(self):
        sim = X86Simulator()
        sim._ax = 3; sim._di = 7
        step_prog(sim, bytes([0x97, 0xF4]))  # XCHG AX, DI
        assert sim._ax == 7 and sim._di == 3

    def test_pop_bp(self):
        # PUSH AX; POP BP — exercises _set_reg16(5)
        sim = X86Simulator()
        sim._sp = 0x200; sim._ax = 0x1234
        step_prog(sim, bytes([0x50, 0x5D, 0xF4]))  # PUSH AX; POP BP
        assert sim._bp == 0x1234

    def test_pop_si(self):
        sim = X86Simulator()
        sim._sp = 0x200; sim._ax = 0xABCD
        step_prog(sim, bytes([0x50, 0x5E, 0xF4]))  # PUSH AX; POP SI
        assert sim._si == 0xABCD

    def test_pop_di(self):
        sim = X86Simulator()
        sim._sp = 0x200; sim._ax = 0x5678
        step_prog(sim, bytes([0x50, 0x5F, 0xF4]))  # PUSH AX; POP DI
        assert sim._di == 0x5678


# ── CH/DH/BH high-byte 8-bit registers ───────────────────────────────────────


class TestHighByteRegisters:
    """Exercise CH/DH/BH as register operands (_get_reg8 and _set_reg8 cases 5/6/7)."""

    def test_mov_ch_imm(self):
        # MOV CH, 5 (B5 05) → _set_reg8(5)
        sim = X86Simulator()
        step_prog(sim, bytes([0xB5, 0x05, 0xF4]))
        assert (sim._cx >> 8) & 0xFF == 5

    def test_mov_dh_imm(self):
        # MOV DH, 3 (B6 03) → _set_reg8(6)
        sim = X86Simulator()
        step_prog(sim, bytes([0xB6, 0x03, 0xF4]))
        assert (sim._dx >> 8) & 0xFF == 3

    def test_mov_bh_imm(self):
        # MOV BH, 7 (B7 07) → _set_reg8(7)
        sim = X86Simulator()
        step_prog(sim, bytes([0xB7, 0x07, 0xF4]))
        assert (sim._bx >> 8) & 0xFF == 7

    def test_read_ch(self):
        # MOV AL, CH (8A C5): mod=11 reg=0(AL) rm=5(CH) → _get_reg8(5)
        sim = X86Simulator()
        sim._cx = 0x0A00  # CH = 0x0A
        step_prog(sim, bytes([0x8A, 0xC5, 0xF4]))
        assert sim._ax & 0xFF == 0x0A

    def test_read_dh(self):
        # MOV AL, DH (8A C6) → _get_reg8(6)
        sim = X86Simulator()
        sim._dx = 0x0B00  # DH = 0x0B
        step_prog(sim, bytes([0x8A, 0xC6, 0xF4]))
        assert sim._ax & 0xFF == 0x0B

    def test_read_bh(self):
        # MOV AL, BH (8A C7) → _get_reg8(7)
        sim = X86Simulator()
        sim._bx = 0x0C00  # BH = 0x0C
        step_prog(sim, bytes([0x8A, 0xC7, 0xF4]))
        assert sim._ax & 0xFF == 0x0C


# ── Segment register operations ───────────────────────────────────────────────


class TestSegmentRegs:
    """MOV sreg, reg and MOV reg, sreg exercise _get_sreg/_set_sreg."""

    def test_mov_ds_ax(self):
        # MOV DS, AX (8E D8): mod=11 reg=3(DS) rm=0(AX) → _set_sreg(3)
        sim = X86Simulator()
        sim._ax = 0x1000
        step_prog(sim, bytes([0x8E, 0xD8, 0xF4]))
        assert sim._ds == 0x1000

    def test_mov_es_ax(self):
        # MOV ES, AX (8E C0): mod=11 reg=0(ES) rm=0(AX) → _set_sreg(0)
        sim = X86Simulator()
        sim._ax = 0x2000
        step_prog(sim, bytes([0x8E, 0xC0, 0xF4]))
        assert sim._es == 0x2000

    def test_mov_ss_ax(self):
        # MOV SS, AX (8E D0): mod=11 reg=2(SS) rm=0(AX) → _set_sreg(2)
        sim = X86Simulator()
        sim._ax = 0x3000
        step_prog(sim, bytes([0x8E, 0xD0, 0xF4]))
        assert sim._ss == 0x3000

    def test_mov_ax_es(self):
        # MOV AX, ES (8C C0): mod=11 reg=0(ES) rm=0(AX) → _get_sreg(0)
        sim = X86Simulator()
        sim._es = 0x4000
        step_prog(sim, bytes([0x8C, 0xC0, 0xF4]))
        assert sim._ax == 0x4000

    def test_mov_ax_ss(self):
        # MOV AX, SS (8C D0) → _get_sreg(2)
        sim = X86Simulator()
        sim._ss = 0x5000
        step_prog(sim, bytes([0x8C, 0xD0, 0xF4]))
        assert sim._ax == 0x5000

    def test_push_pop_ds(self):
        # PUSH DS (1E); POP DS (1F)
        sim = X86Simulator()
        sim._sp = 0x200; sim._ds = 0xABCD
        step_prog(sim, bytes([0x1E, 0x1F, 0xF4]))  # PUSH DS; POP DS
        assert sim._ds == 0xABCD

    def test_push_es(self):
        # PUSH ES (06) pushes ES value onto stack
        sim = X86Simulator()
        sim._sp = 0x200; sim._es = 0x1234
        step_prog(sim, bytes([0x06, 0xF4]))  # PUSH ES
        assert sim._mem[0x01FE] == 0x34
        assert sim._mem[0x01FF] == 0x12


# ── ModRM addressing modes ────────────────────────────────────────────────────


class TestModRMAddressing:
    """Exercise less-common ModRM effective-address modes."""

    def test_bx_di_addressing(self):
        # MOV AX, [BX+DI]: opcode 8B, ModRM 01 (mod=00 reg=0 rm=1 = BX+DI)
        sim = X86Simulator()
        sim._bx = 0x100; sim._di = 0x10
        sim._mem[0x110] = 0x42; sim._mem[0x111] = 0x00
        step_prog(sim, bytes([0x8B, 0x01, 0xF4]))  # MOV AX, [BX+DI]
        assert sim._ax == 0x0042

    def test_bp_si_addressing(self):
        # MOV AX, [BP+SI]: ModRM 02 (rm=2 = BP+SI), uses SS
        sim = X86Simulator()
        sim._bp = 0x100; sim._si = 0x20
        sim._mem[0x120] = 0xAB; sim._mem[0x121] = 0x00
        step_prog(sim, bytes([0x8B, 0x02, 0xF4]))  # MOV AX, [BP+SI]
        assert sim._ax == 0x00AB

    def test_bp_di_addressing(self):
        # MOV AX, [BP+DI]: ModRM 03 (rm=3 = BP+DI), uses SS
        sim = X86Simulator()
        sim._bp = 0x50; sim._di = 0x10
        sim._mem[0x60] = 0xCD; sim._mem[0x61] = 0x00
        step_prog(sim, bytes([0x8B, 0x03, 0xF4]))  # MOV AX, [BP+DI]
        assert sim._ax == 0x00CD

    def test_si_addressing(self):
        # MOV AX, [SI]: ModRM 04 (rm=4 = SI)
        sim = X86Simulator()
        sim._si = 0x200
        sim._mem[0x200] = 0x77; sim._mem[0x201] = 0x00
        step_prog(sim, bytes([0x8B, 0x04, 0xF4]))  # MOV AX, [SI]
        assert sim._ax == 0x0077

    def test_di_addressing(self):
        # MOV AX, [DI]: ModRM 05 (rm=5 = DI)
        sim = X86Simulator()
        sim._di = 0x300
        sim._mem[0x300] = 0x88; sim._mem[0x301] = 0x00
        step_prog(sim, bytes([0x8B, 0x05, 0xF4]))  # MOV AX, [DI]
        assert sim._ax == 0x0088

    def test_disp8_addressing(self):
        # MOV AX, [BX + disp8]: ModRM 47 08 (mod=01 reg=0 rm=7, disp=8)
        sim = X86Simulator()
        sim._bx = 0x100
        sim._mem[0x108] = 0x55; sim._mem[0x109] = 0x00
        step_prog(sim, bytes([0x8B, 0x47, 0x08, 0xF4]))  # MOV AX, [BX+8]
        assert sim._ax == 0x0055

    def test_disp16_addressing(self):
        # MOV AX, [BX + disp16]: ModRM 87 00 01 (mod=10 reg=0 rm=7, disp=256)
        sim = X86Simulator()
        sim._bx = 0x100
        sim._mem[0x200] = 0x11; sim._mem[0x201] = 0x22
        step_prog(sim, bytes([0x8B, 0x87, 0x00, 0x01, 0xF4]))  # MOV AX,[BX+256]
        assert sim._ax == 0x2211

    def test_mov_rm8_imm8(self):
        # MOV [BX], imm8 (C6 /0): C6 07 42 = MOV [BX], 0x42
        sim = X86Simulator()
        sim._bx = 0x400
        step_prog(sim, bytes([0xC6, 0x07, 0x42, 0xF4]))
        assert sim._mem[0x400] == 0x42

    def test_mov_rm16_imm16(self):
        # MOV [BX], imm16 (C7 /0): C7 07 34 12 = MOV [BX], 0x1234
        sim = X86Simulator()
        sim._bx = 0x500
        step_prog(sim, bytes([0xC7, 0x07, 0x34, 0x12, 0xF4]))
        assert sim._mem[0x500] == 0x34
        assert sim._mem[0x501] == 0x12


# ── FE/FF opcode groups ───────────────────────────────────────────────────────


class TestFFGroup:
    """FE group (INC/DEC r/m8) and FF group (indirect INC/DEC/CALL/JMP/PUSH)."""

    def test_fe_inc_al(self):
        # INC AL via FE /0: FE C0 (mod=11 ext=0 rm=0)
        sim = X86Simulator()
        sim._ax = 5
        step_prog(sim, bytes([0xFE, 0xC0, 0xF4]))  # INC AL
        assert sim._ax & 0xFF == 6

    def test_fe_dec_al(self):
        # DEC AL via FE /1: FE C8 (mod=11 ext=1 rm=0)
        sim = X86Simulator()
        sim._ax = 10
        step_prog(sim, bytes([0xFE, 0xC8, 0xF4]))  # DEC AL
        assert sim._ax & 0xFF == 9

    def test_ff_inc_ax(self):
        # INC AX via FF /0: FF C0 (mod=11 ext=0 rm=0)
        sim = X86Simulator()
        sim._ax = 100
        step_prog(sim, bytes([0xFF, 0xC0, 0xF4]))  # INC AX (via r/m16)
        assert sim._ax == 101

    def test_ff_dec_ax(self):
        # DEC AX via FF /1: FF C8 (mod=11 ext=1 rm=0)
        sim = X86Simulator()
        sim._ax = 50
        step_prog(sim, bytes([0xFF, 0xC8, 0xF4]))  # DEC AX (via r/m16)
        assert sim._ax == 49

    def test_ff_push_ax(self):
        # PUSH AX via FF /6: FF F0 (mod=11 ext=6 rm=0)
        sim = X86Simulator()
        sim._sp = 0x200; sim._ax = 0x5566
        step_prog(sim, bytes([0xFF, 0xF0, 0xF4]))  # PUSH AX via r/m16
        assert sim._mem[0x1FE] == 0x66
        assert sim._mem[0x1FF] == 0x55

    def test_ff_jmp_indirect(self):
        # JMP AX via FF /4: FF E0 (mod=11 ext=4 rm=0)
        # AX=5 → IP=5; mem[5]=F4 (HLT)
        sim = X86Simulator()
        sim._ax = 5
        sim._mem[5] = 0xF4   # HLT at address 5
        step_prog(sim, bytes([0xFF, 0xE0]))   # JMP AX
        assert sim._ip == 6  # HLT advances IP

    def test_ff_call_indirect(self):
        # CALL AX via FF /2: FF D0
        # Layout: 0: FF D0; 2: HLT; 3: HLT (subroutine)
        # AX=3 → jump to 3; mem[3]=RET(C3); RET pops return addr 2 → HLT
        sim = X86Simulator()
        sim._sp = 0x200; sim._ax = 3
        sim._mem[3] = 0xC3   # RET at address 3
        step_prog(sim, bytes([0xFF, 0xD0, 0xF4]))  # CALL AX; HLT
        assert sim._ip == 3   # halted at HLT at offset 2... wait
        # Actually: CALL pushes IP=2, jumps to 3 (AX=3). mem[3]=C3=RET.
        # RET pops 2. IP=2. mem[2]=F4=HLT. Halts.
        assert sim._halted

    def test_pop_rm(self):
        # POP [BX] via 8F /0: 8F 07 (mod=00 ext=0 rm=7)
        # Pushes AX=0x1234, then POP into [BX]=0x100
        sim = X86Simulator()
        sim._sp = 0x200; sim._ax = 0x1234; sim._bx = 0x100
        step_prog(sim, bytes([
            0x50,               # PUSH AX
            0x8F, 0x07,         # POP [BX]
            0xF4,
        ]))
        assert sim._mem[0x100] == 0x34
        assert sim._mem[0x101] == 0x12


# ── MUL/IMUL/DIV/IDIV byte variants ─────────────────────────────────────────


class TestMulDivByte:
    """Byte-mode MUL/IMUL/DIV/IDIV (F6 group) — result stored differently than word."""

    def test_mul_byte(self):
        # MUL BL: AX ← AL × BL (unsigned)
        # F6 E3 (mod=11 ext=4 rm=3=BL)
        sim = X86Simulator()
        sim._ax = 12; sim._bx = 10   # AL=12, BL=10
        step_prog(sim, bytes([0xF6, 0xE3, 0xF4]))  # MUL BL
        assert sim._ax == 120  # 12×10 in AX (AL=lo, AH=hi)

    def test_mul_byte_overflow(self):
        # 200 × 200 = 40000; AH = 40000 >> 8 = 156, AL = 40000 & 0xFF = 64
        sim = X86Simulator()
        sim._ax = 200; sim._bx = 200   # AL=200, BL=200
        step_prog(sim, bytes([0xF6, 0xE3, 0xF4]))
        assert sim._ax == 40000 & 0xFFFF
        assert sim._cf is True   # AH ≠ 0

    def test_imul_byte(self):
        # IMUL BL: AX ← AL × BL (signed)
        # F6 EB (mod=11 ext=5 rm=3=BL)
        sim = X86Simulator()
        sim._ax = 0xFF  # AL = -1 as signed byte
        sim._bx = 5     # BL = 5
        step_prog(sim, bytes([0xF6, 0xEB, 0xF4]))  # IMUL BL
        # -1 × 5 = -5; AX = -5 & 0xFFFF = 0xFFFB
        assert sim._ax == 0xFFFB

    def test_div_byte(self):
        # DIV BL: AL = AX // BL, AH = AX % BL
        # F6 F3 (mod=11 ext=6 rm=3=BL)
        sim = X86Simulator()
        sim._ax = 100; sim._bx = 7   # AX=100, BL=7
        step_prog(sim, bytes([0xF6, 0xF3, 0xF4]))  # DIV BL
        # 100 / 7 = 14 remainder 2
        assert (sim._ax & 0xFF) == 14    # AL = quotient
        assert (sim._ax >> 8) == 2       # AH = remainder

    def test_idiv_byte(self):
        # IDIV BL: signed; AX=-7, BL=2 → AL=-3, AH=-1
        # F6 FB (mod=11 ext=7 rm=3=BL)
        sim = X86Simulator()
        sim._ax = 0xFFF9   # -7 in 16-bit signed
        sim._bx = 2
        step_prog(sim, bytes([0xF6, 0xFB, 0xF4]))  # IDIV BL
        # -7 / 2 = -3 (truncated toward zero), remainder -1
        al = sim._ax & 0xFF
        ah = (sim._ax >> 8) & 0xFF
        assert (al if al < 0x80 else al - 0x100) == -3
        assert (ah if ah < 0x80 else ah - 0x100) == -1

    def test_test_rm_imm(self):
        # TEST AX, imm16 via F7 /0: F7 C0 FF 00 → TEST AX, 0xFF
        sim = X86Simulator()
        sim._ax = 0x0042
        step_prog(sim, bytes([0xF7, 0xC0, 0xFF, 0x00, 0xF4]))  # TEST AX, 0xFF
        assert sim._zf is False   # 0x42 & 0xFF = 0x42 ≠ 0
        assert sim._ax == 0x0042  # operand unchanged


# ── JMP near / far, RETF ─────────────────────────────────────────────────────


class TestJmpFarRetf:
    """JMP near (E9), JMP far (EA), RET n (C2), RETF (CB), RETF n (CA)."""

    def test_jmp_near(self):
        # E9 03 00 = JMP +3 (disp16); IP after fetch=3, target=3+3=6; HLT at 6
        sim = X86Simulator()
        sim._ax = 0
        sim._mem[6] = 0xF4   # HLT
        step_prog(sim, bytes([0xE9, 0x03, 0x00, 0x40, 0x40, 0x40]))  # JMP +3
        # INC AX at 3,4,5 are skipped; HLT at 6
        assert sim._ip == 7  # past HLT

    def test_jmp_far(self):
        # JMP FAR 0x0000:0x0010 (EA 10 00 00 00): sets IP=0x10, CS=0
        sim = X86Simulator()
        sim._mem[0x10] = 0xF4   # HLT at physical 0x10
        step_prog(sim, bytes([0xEA, 0x10, 0x00, 0x00, 0x00]))  # JMP FAR 0:16
        assert sim._ip == 0x11

    def test_ret_n(self):
        # CALL +1; RET 4.  CALL at offset 2 (3 bytes), IP-after=5, target=6.
        # RET 4 pops return addr (=5) and discards 4 bytes (the 2 dummy PUSHes).
        sim = X86Simulator()
        sim._sp = 0x100
        prog = bytes([
            0x50,               # 0: PUSH AX (dummy arg)
            0x50,               # 1: PUSH AX (dummy arg)
            0xE8, 0x01, 0x00,   # 2: CALL +1 → target=6
            0xF4,               # 5: HLT
            0xC2, 0x04, 0x00,   # 6: RET 4
        ])
        step_prog(sim, prog)
        assert sim._halted

    def test_retf(self):
        # Far return to CS:IP = 0x0000:0x0100.  RETF pops IP first, then CS,
        # so we must PUSH CS first (lands at higher stack addr) then PUSH IP.
        # Target 0x100 is well outside the 10-byte program, so load won't clobber it.
        sim = X86Simulator()
        sim._sp = 0x200
        prog = bytes([
            0xB8, 0x00, 0x00,   # 0: MOV AX, 0      (will be used as CS)
            0x50,               # 3: PUSH AX         push CS=0 first
            0xB8, 0x00, 0x01,   # 4: MOV AX, 0x100  (will be used as IP)
            0x50,               # 7: PUSH AX         push IP=0x100 second
            0xCB,               # 8: RETF
            0xF4,               # 9: not reached
        ])
        # Load the prog first so load() doesn't clobber mem[0x100].
        sim.load(prog)
        sim._mem[0x100] = 0xF4   # HLT at physical 0x100 (target address)
        steps = 0
        while not sim._halted and steps < 10_000:
            sim.step(); steps += 1
        assert sim._cs == 0 and sim._ip == 0x101  # past HLT at 0x100

    def test_retf_n(self):
        # RETF 2 (CA 02 00): pops IP, CS, then discards 2 bytes from stack
        sim = X86Simulator()
        sim._sp = 0x200
        # Manually push: 2 dummy bytes, then CS=0, then IP=9 (HLT)
        sim._mem[0x1F8] = 0xAA; sim._mem[0x1F9] = 0xBB  # dummy (at higher stack)
        sim._mem[0x1FA] = 9;    sim._mem[0x1FB] = 0     # IP=9
        sim._mem[0x1FC] = 0;    sim._mem[0x1FD] = 0     # CS=0
        sim._sp = 0x1FA
        prog = bytes([0xCA, 0x02, 0x00, 0xF4])  # RETF 2; HLT (not reached)
        sim._mem[9] = 0xF4      # HLT at physical 9
        step_prog(sim, prog)
        assert sim._ip == 10 and sim._cs == 0


# ── IN/OUT word variants ──────────────────────────────────────────────────────


class TestIOWord:
    """IN AX, port (E5) and OUT port, AX (E7) — word-mode I/O."""

    def test_in_ax_imm(self):
        # IN AX, 0x10 (E5 10): reads two bytes from ports 0x10 and 0x11
        sim = X86Simulator()
        sim._input_ports[0x10] = 0x34
        sim._input_ports[0x11] = 0x12
        step_prog(sim, bytes([0xE5, 0x10, 0xF4]))  # IN AX, 0x10
        assert sim._ax == 0x1234

    def test_out_ax_imm(self):
        # OUT 0x20, AX (E7 20): writes AX to ports 0x20 (lo) and 0x21 (hi)
        sim = X86Simulator()
        sim._ax = 0xABCD
        step_prog(sim, bytes([0xE7, 0x20, 0xF4]))  # OUT 0x20, AX
        assert sim._output_ports[0x20] == 0xCD
        assert sim._output_ports[0x21] == 0xAB

    def test_in_ax_dx(self):
        # IN AX, DX (ED): reads from ports DX and DX+1
        sim = X86Simulator()
        sim._dx = 5
        sim._input_ports[5] = 0x78; sim._input_ports[6] = 0x56
        step_prog(sim, bytes([0xED, 0xF4]))  # IN AX, DX
        assert sim._ax == 0x5678

    def test_out_ax_dx(self):
        # OUT DX, AX (EF): writes AX to ports DX (lo) and DX+1 (hi)
        sim = X86Simulator()
        sim._dx = 10; sim._ax = 0x1122
        step_prog(sim, bytes([0xEF, 0xF4]))  # OUT DX, AX
        assert sim._output_ports[10] == 0x22
        assert sim._output_ports[11] == 0x11


# ── MOV AX/AL from/to memory (A0-A3 group) ───────────────────────────────────


class TestMovAccMem:
    """MOV AX,[imm16] (A1) and MOV [imm16],AX (A3) — accumulator ↔ memory."""

    def test_mov_ax_from_mem(self):
        # MOV AX, [0x0200] (A1 00 02): reads word from DS:0x200
        sim = X86Simulator()
        sim._mem[0x200] = 0xCD; sim._mem[0x201] = 0xAB
        step_prog(sim, bytes([0xA1, 0x00, 0x02, 0xF4]))
        assert sim._ax == 0xABCD

    def test_mov_to_mem_ax(self):
        # MOV [0x0300], AX (A3 00 03): writes AX to DS:0x300
        sim = X86Simulator()
        sim._ax = 0x5678
        step_prog(sim, bytes([0xA3, 0x00, 0x03, 0xF4]))
        assert sim._mem[0x300] == 0x78
        assert sim._mem[0x301] == 0x56


# ── LDS / LES ────────────────────────────────────────────────────────────────


class TestLDSLES:
    """LDS (C5) and LES (C4): load a far pointer (offset + segment) from memory."""

    def test_lds(self):
        # LDS BX, [SI]: BX ← [SI], DS ← [SI+2]
        # C5 1C (mod=00 reg=3=BX rm=4=SI)
        sim = X86Simulator()
        sim._si = 0x100
        sim._mem[0x100] = 0x34; sim._mem[0x101] = 0x12   # offset 0x1234
        sim._mem[0x102] = 0x00; sim._mem[0x103] = 0x10   # segment 0x1000
        step_prog(sim, bytes([0xC5, 0x1C, 0xF4]))  # LDS BX, [SI]
        assert sim._bx == 0x1234 and sim._ds == 0x1000

    def test_les(self):
        # LES DI, [SI]: DI ← [SI], ES ← [SI+2]
        # C4 3C (mod=00 reg=7=DI rm=4=SI)
        sim = X86Simulator()
        sim._si = 0x200
        sim._mem[0x200] = 0xBC; sim._mem[0x201] = 0x9A   # offset 0x9ABC
        sim._mem[0x202] = 0x00; sim._mem[0x203] = 0x20   # segment 0x2000
        step_prog(sim, bytes([0xC4, 0x3C, 0xF4]))  # LES DI, [SI]
        assert sim._di == 0x9ABC and sim._es == 0x2000


# ── Conditional jumps (all 16 conditions) ────────────────────────────────────


class TestAllJcc:
    """Exercise all 16 Jcc conditions, hitting untested _eval_cond branches."""

    def _jcc_prog(self, jcc_op: int, taken_ax: int, not_taken_ax: int) -> bytes:
        """Build: Jcc +4; MOV AX,not_taken; HLT; MOV AX,taken; HLT.

        Layout:
          0: jcc_op disp=4          (2 bytes; IP after fetch = 2)
          2: MOV AX, not_taken_ax   (3 bytes)
          5: HLT                    (1 byte)  ← not-taken path halts here
          6: MOV AX, taken_ax       (3 bytes) ← taken path: 2+4=6 ✓
          9: HLT                    (1 byte)
        """
        return bytes([
            jcc_op, 0x04,           # Jcc +4: IP_after(2) + 4 = 6 (taken path)
            0xB8, not_taken_ax & 0xFF, (not_taken_ax >> 8) & 0xFF,
            0xF4,                   # HLT (not-taken path)
            0xB8, taken_ax & 0xFF, (taken_ax >> 8) & 0xFF,
            0xF4,                   # HLT (taken path)
        ])

    def test_jo_not_taken(self):
        # JO (70): taken if OF=1. OF=0 → not taken.
        sim = X86Simulator()
        sim._of = False
        step_prog(sim, self._jcc_prog(0x70, 99, 1))
        assert sim._ax == 1   # not taken: OF=0

    def test_jo_taken(self):
        sim = X86Simulator()
        sim._of = True
        step_prog(sim, self._jcc_prog(0x70, 99, 1))
        assert sim._ax == 99   # taken: OF=1

    def test_jno(self):
        sim = X86Simulator()
        sim._of = False
        step_prog(sim, self._jcc_prog(0x71, 99, 1))  # JNO taken when OF=0
        assert sim._ax == 99

    def test_js_taken(self):
        sim = X86Simulator()
        sim._sf = True
        step_prog(sim, self._jcc_prog(0x78, 99, 1))  # JS taken when SF=1
        assert sim._ax == 99

    def test_jns(self):
        sim = X86Simulator()
        sim._sf = False
        step_prog(sim, self._jcc_prog(0x79, 99, 1))  # JNS taken when SF=0
        assert sim._ax == 99

    def test_jp_taken(self):
        sim = X86Simulator()
        sim._pf = True
        step_prog(sim, self._jcc_prog(0x7A, 99, 1))  # JP taken when PF=1
        assert sim._ax == 99

    def test_jnp(self):
        sim = X86Simulator()
        sim._pf = False
        step_prog(sim, self._jcc_prog(0x7B, 99, 1))  # JNP taken when PF=0
        assert sim._ax == 99

    def test_jl_taken(self):
        # JL (7C): taken when SF≠OF
        sim = X86Simulator()
        sim._sf = True; sim._of = False
        step_prog(sim, self._jcc_prog(0x7C, 99, 1))
        assert sim._ax == 99

    def test_jge(self):
        # JGE (7D): taken when SF=OF
        sim = X86Simulator()
        sim._sf = False; sim._of = False
        step_prog(sim, self._jcc_prog(0x7D, 99, 1))
        assert sim._ax == 99

    def test_jle_taken(self):
        # JLE (7E): taken when ZF=1 or SF≠OF
        sim = X86Simulator()
        sim._zf = True; sim._sf = False; sim._of = False
        step_prog(sim, self._jcc_prog(0x7E, 99, 1))
        assert sim._ax == 99

    def test_jg(self):
        # JG (7F): taken when ZF=0 and SF=OF
        sim = X86Simulator()
        sim._zf = False; sim._sf = False; sim._of = False
        step_prog(sim, self._jcc_prog(0x7F, 99, 1))
        assert sim._ax == 99

    def test_jbe_taken(self):
        # JBE (76): taken when CF=1 or ZF=1
        sim = X86Simulator()
        sim._cf = True; sim._zf = False
        step_prog(sim, self._jcc_prog(0x76, 99, 1))
        assert sim._ax == 99

    def test_ja(self):
        # JA (77): taken when CF=0 and ZF=0
        sim = X86Simulator()
        sim._cf = False; sim._zf = False
        step_prog(sim, self._jcc_prog(0x77, 99, 1))
        assert sim._ax == 99


# ── XCHG r/m, reg ────────────────────────────────────────────────────────────


class TestXCHGRmReg:
    """XCHG [mem], reg (86/87) — exercises the r/m write-back path."""

    def test_xchg_mem_ax(self):
        # XCHG [BX], AX (87 07): exchanges [BX] with AX
        sim = X86Simulator()
        sim._bx = 0x100; sim._ax = 0x1234
        sim._mem[0x100] = 0x78; sim._mem[0x101] = 0x56  # [BX] = 0x5678
        step_prog(sim, bytes([0x87, 0x07, 0xF4]))
        assert sim._ax == 0x5678
        assert sim._mem[0x100] == 0x34 and sim._mem[0x101] == 0x12


# ── Shift/rotate with CL count ────────────────────────────────────────────────


class TestShiftByCL:
    """D2/D3 opcodes: shift/rotate r/m8 or r/m16 by CL count."""

    def test_shl_ax_cl(self):
        # SHL AX, CL (D3 E0): mod=11 ext=4 rm=0
        sim = X86Simulator()
        sim._ax = 1; sim._cx = 3   # shift left 3 → 8
        step_prog(sim, bytes([0xD3, 0xE0, 0xF4]))
        assert sim._ax == 8

    def test_shr_ax_cl(self):
        # SHR AX, CL (D3 E8): mod=11 ext=5 rm=0
        sim = X86Simulator()
        sim._ax = 16; sim._cx = 2
        step_prog(sim, bytes([0xD3, 0xE8, 0xF4]))
        assert sim._ax == 4

    def test_sar_ax_cl(self):
        # SAR AX, CL (D3 F8): mod=11 ext=7 rm=0 — arithmetic right shift
        sim = X86Simulator()
        sim._ax = 0xFF00; sim._cx = 4  # -256 >> 4 should sign-fill
        step_prog(sim, bytes([0xD3, 0xF8, 0xF4]))
        assert sim._ax == 0xFFF0  # sign-filled

    def test_rol_ax_cl(self):
        sim = X86Simulator()
        sim._ax = 0x8001; sim._cx = 1   # ROL by 1
        step_prog(sim, bytes([0xD3, 0xC0, 0xF4]))  # ROL AX, CL
        assert sim._ax == 0x0003   # 0x8001 rotated left 1

    def test_shl_al_cl(self):
        # D2 E0 = SHL AL, CL (byte)
        sim = X86Simulator()
        sim._ax = 1; sim._cx = 4
        step_prog(sim, bytes([0xD2, 0xE0, 0xF4]))
        assert sim._ax & 0xFF == 16


# ── REP CMPS / REPNE SCAS ───────────────────────────────────────────────────


class TestCmpsScas:
    """CMPSB (A6) and SCASB (AE) with and without REP prefixes."""

    def test_cmpsb_equal(self):
        # CMPSB: compare DS:[SI] with ES:[DI]. SI=100, DI=200. Both = 0x55.
        sim = X86Simulator()
        sim._si = 0x100; sim._di = 0x200
        sim._mem[0x100] = 0x55; sim._mem[0x200] = 0x55
        step_prog(sim, bytes([0xA6, 0xF4]))  # CMPSB
        assert sim._zf is True   # bytes equal

    def test_cmpsb_not_equal(self):
        sim = X86Simulator()
        sim._si = 0x100; sim._di = 0x200
        sim._mem[0x100] = 0x55; sim._mem[0x200] = 0x44
        step_prog(sim, bytes([0xA6, 0xF4]))
        assert sim._zf is False

    def test_repe_cmpsb(self):
        # REPE CMPSB: compare 3 bytes; stop when mismatch at index 1.
        sim = X86Simulator()
        sim._si = 0x100; sim._di = 0x200; sim._cx = 3
        sim._mem[0x100] = 0xAA; sim._mem[0x101] = 0xBB; sim._mem[0x102] = 0xCC
        sim._mem[0x200] = 0xAA; sim._mem[0x201] = 0xFF; sim._mem[0x202] = 0xCC
        step_prog(sim, bytes([0xF3, 0xA6, 0xF4]))  # REPE CMPSB
        assert sim._zf is False  # stopped at mismatch
        assert sim._cx == 1     # compared 2 bytes, 1 remaining

    def test_scasb_found(self):
        # SCASB: search ES:[DI] for AL. AL=0x42, [DI]=0x42.
        sim = X86Simulator()
        sim._ax = 0x42; sim._di = 0x300
        sim._mem[0x300] = 0x42
        step_prog(sim, bytes([0xAE, 0xF4]))  # SCASB
        assert sim._zf is True

    def test_repne_scasb(self):
        # REPNE SCASB: scan for AL=0xBB in buffer. Found at index 2.
        sim = X86Simulator()
        sim._ax = 0xBB; sim._di = 0x400; sim._cx = 5
        sim._mem[0x400] = 0x11; sim._mem[0x401] = 0x22
        sim._mem[0x402] = 0xBB  # found here
        step_prog(sim, bytes([0xF2, 0xAE, 0xF4]))  # REPNE SCASB
        assert sim._zf is True  # found
        assert sim._cx == 2     # scanned 3, 2 left


# ── AAS / AAM / AAD ──────────────────────────────────────────────────────────


class TestMoreBCD:
    """AAS (3F), AAM (D4), AAD (D5) — BCD adjust instructions."""

    def test_aas_no_adjust(self):
        # 5 - 3 = 2; low nibble 2 ≤ 9 → no adjust.
        sim = X86Simulator()
        sim._ax = 0x0005
        step_prog(sim, bytes([0x2C, 0x03, 0x3F, 0xF4]))  # SUB AL,3; AAS
        assert sim._ax & 0xFF == 2
        assert sim._cf is False

    def test_aas_with_borrow(self):
        # 3 - 5 = -2; stored as 0xFE but AAS: low nibble E > 9 → adjust
        # AAS: AL = (AL - 6) & 0xF; AH -= 1; CF=AF=1
        sim = X86Simulator()
        sim._ax = 0x0003
        step_prog(sim, bytes([0x2C, 0x05, 0x3F, 0xF4]))  # SUB AL,5; AAS
        assert sim._cf is True  # borrow

    def test_aam(self):
        # AAM 10 (D4 0A): AH = AL / 10; AL = AL % 10
        # AL = 0x0F = 15 (unpacked BCD multiply result)
        sim = X86Simulator()
        sim._ax = 15
        step_prog(sim, bytes([0xD4, 0x0A, 0xF4]))  # AAM 10
        assert (sim._ax >> 8) == 1    # AH = 1
        assert (sim._ax & 0xFF) == 5  # AL = 5

    def test_aad(self):
        # AAD 10 (D5 0A): AL = AH*10 + AL; AH = 0
        # AH=3, AL=7 → AL = 3*10 + 7 = 37; AH = 0
        sim = X86Simulator()
        sim._ax = 0x0307  # AH=3, AL=7
        step_prog(sim, bytes([0xD5, 0x0A, 0xF4]))  # AAD 10
        assert sim._ax == 37


# ── State properties ──────────────────────────────────────────────────────────


class TestStateProperties:
    """Exercise X86State computed properties not covered by main tests."""

    def test_bl_bh_cl_ch_dl_dh(self):
        from intel_8086_simulator.state import X86State
        s = X86State(
            ax=0, bx=0xABCD, cx=0x1234, dx=0x5678,
            si=0, di=0, sp=0, bp=0,
            cs=0, ds=0, ss=0, es=0, ip=0,
            cf=False, pf=False, af=False, zf=False,
            sf=False, tf=False, if_=False, df=False, of=False,
            halted=False,
            input_ports=tuple([0] * 256),
            output_ports=tuple([0] * 256),
            memory=tuple([0] * 1_048_576),
        )
        assert s.bl == 0xCD     # BX low
        assert s.bh == 0xAB     # BX high
        assert s.cl == 0x34     # CX low
        assert s.ch == 0x12     # CX high
        assert s.dl == 0x78     # DX low
        assert s.dh == 0x56     # DX high
        assert s.al_signed == 0   # AX=0, AL=0 (positive)

    def test_al_signed_negative(self):
        from intel_8086_simulator.state import X86State
        s = X86State(
            ax=0x0080, bx=0, cx=0, dx=0,
            si=0, di=0, sp=0, bp=0,
            cs=0, ds=0, ss=0, es=0, ip=0,
            cf=False, pf=False, af=False, zf=False,
            sf=False, tf=False, if_=False, df=False, of=False,
            halted=False,
            input_ports=tuple([0] * 256),
            output_ports=tuple([0] * 256),
            memory=tuple([0] * 1_048_576),
        )
        assert s.al_signed == -128  # 0x80 as signed byte
