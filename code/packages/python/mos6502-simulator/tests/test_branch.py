"""Tests for all 8 branch instructions and JMP/JSR/RTS/RTI."""

from __future__ import annotations

from mos6502_simulator import MOS6502Simulator


def run(prog: list[int]):
    return MOS6502Simulator().execute(bytes(prog + [0x00])).final_state


class TestBranches:
    """All 8 branch conditions, taken and not-taken."""

    def test_beq_taken(self) -> None:
        # LDA #0 (Z=1); BEQ +3; LDA #$FF (skip); BRK
        # BEQ target: offset=3, jumps over 2-byte LDA+1-byte BRK = lands on 0x00
        #  0x00: A9 00   LDA #0
        #  0x02: F0 02   BEQ +2 (skip LDA #$FF)
        #  0x04: A9 FF   LDA #$FF   ← skipped
        #  0x06: 00      BRK
        s = run([0xA9, 0x00, 0xF0, 0x02, 0xA9, 0xFF])
        assert s.a == 0

    def test_beq_not_taken(self) -> None:
        # LDA #1 (Z=0); BEQ +2 (not taken); LDA #$AA; BRK
        s = run([0xA9, 0x01, 0xF0, 0x02, 0xA9, 0xAA])
        assert s.a == 0xAA

    def test_bne_taken(self) -> None:
        # LDA #1 (Z=0); BNE +2; LDA #$FF (skip); BRK
        s = run([0xA9, 0x01, 0xD0, 0x02, 0xA9, 0xFF])
        assert s.a == 1

    def test_bne_not_taken(self) -> None:
        s = run([0xA9, 0x00, 0xD0, 0x02, 0xA9, 0xAA])
        assert s.a == 0xAA

    def test_bcc_taken(self) -> None:
        # CLC (C=0); BCC +2; LDA #$FF; BRK
        s = run([0x18, 0x90, 0x02, 0xA9, 0xFF])
        assert s.a == 0

    def test_bcc_not_taken(self) -> None:
        # SEC (C=1); BCC +2 (not taken); LDA #$AA; BRK
        s = run([0x38, 0x90, 0x02, 0xA9, 0xAA])
        assert s.a == 0xAA

    def test_bcs_taken(self) -> None:
        # SEC; BCS +2; LDA #$FF; BRK
        s = run([0x38, 0xB0, 0x02, 0xA9, 0xFF])
        assert s.a == 0

    def test_bcs_not_taken(self) -> None:
        s = run([0x18, 0xB0, 0x02, 0xA9, 0xAA])
        assert s.a == 0xAA

    def test_bpl_taken(self) -> None:
        # LDA #1 (N=0); BPL +2; LDA #$FF; BRK
        s = run([0xA9, 0x01, 0x10, 0x02, 0xA9, 0xFF])
        assert s.a == 1

    def test_bpl_not_taken(self) -> None:
        # LDA #$80 (N=1); BPL +2 (not taken)
        s = run([0xA9, 0x80, 0x10, 0x02, 0xA9, 0x01])
        assert s.a == 0x01

    def test_bmi_taken(self) -> None:
        # LDA #$80 (N=1); BMI +2; LDA #$FF; BRK
        s = run([0xA9, 0x80, 0x30, 0x02, 0xA9, 0xFF])
        assert s.a == 0x80

    def test_bmi_not_taken(self) -> None:
        s = run([0xA9, 0x01, 0x30, 0x02, 0xA9, 0x55])
        assert s.a == 0x55

    def test_bvc_taken(self) -> None:
        # CLV; BVC +2; LDA #$FF; BRK
        s = run([0xB8, 0x50, 0x02, 0xA9, 0xFF])
        assert s.a == 0

    def test_bvs_taken(self) -> None:
        # Cause overflow: LDA #$7F; ADC #$01 → V=1; BVS +2; LDA #$FF; BRK
        s = run([0xA9, 0x7F, 0x69, 0x01, 0x70, 0x02, 0xA9, 0xFF])
        assert s.a == 0x80   # $7F+1=$80, BVS taken, $FF not loaded

    def test_branch_backward(self) -> None:
        # Loop: LDX #3; DEX; BNE -3 (back to DEX); BRK
        # Byte layout:
        #  0: A2 03  LDX #3
        #  2: CA     DEX
        #  3: D0 FD  BNE -3  (offset = -3 → PC after BNE = 5, target = 5-3 = 2)
        #  5: 00     BRK
        s = run([0xA2, 0x03, 0xCA, 0xD0, 0xFD])
        assert s.x == 0


class TestJMP:
    def test_jmp_absolute(self) -> None:
        # JMP $0006; LDA #$FF (skip); BRK at $0006
        # Layout:
        #  0: 4C 06 00  JMP $0006
        #  3: A9 FF     LDA #$FF (skipped)
        #  5: 00        BRK (would be reached if JMP not taken)
        #  6: 00        BRK (actual target)
        prog = bytes([0x4C, 0x06, 0x00, 0xA9, 0xFF, 0xEA, 0x00])
        result = MOS6502Simulator().execute(prog)
        assert result.final_state.a == 0   # $FF not loaded

    def test_jmp_indirect_bug(self) -> None:
        # JMP ($01FF) — page-wrap bug: hi byte read from $0100, not $0200
        # Set up: mem[$01FF]=0x00, mem[$0100]=0x03 → target=$0300
        # (Without bug: would read hi from $0200)
        sim = MOS6502Simulator()
        sim.reset()
        sim._memory[0x01FF] = 0x00   # lo byte
        sim._memory[0x0100] = 0x03   # hi byte (bug: from same page)
        sim._memory[0x0300] = 0x00   # BRK at target
        sim.load(bytes([0x6C, 0xFF, 0x01]))  # JMP ($01FF)
        while not sim._halted:
            sim.step()
        assert sim.get_state().pc == 0x0301   # landed at $0300, advanced to $0301


class TestJSRRTS:
    def test_jsr_rts(self) -> None:
        # Subroutine at $0009 sets A=$42 and returns
        prog = bytes([
            0xA9, 0x00,        # 0x0000: LDA #0
            0x20, 0x09, 0x00,  # 0x0002: JSR $0009
            0x00,              # 0x0005: BRK (return lands here + PC advance = 0x0006)
            0x00, 0x00, 0x00,  # 0x0006-08: padding
            0xA9, 0x42,        # 0x0009: LDA #$42
            0x60,              # 0x000B: RTS
        ])
        result = MOS6502Simulator().execute(prog)
        assert result.final_state.a == 0x42

    def test_jsr_pushes_pc_minus_1(self) -> None:
        # JSR pushes PC-1 (address of last byte of JSR), not PC
        # After JSR at addr=0, JSR is 3 bytes, so next PC=3
        # JSR pushes 2 (0x0003 - 1 = 0x0002)
        # RTS pops 2 and adds 1 → returns to PC=3
        prog = bytes([
            0x20, 0x06, 0x00,  # 0x0000: JSR $0006
            0xA9, 0x01,        # 0x0003: LDA #1 (return here)
            0x00,              # 0x0005: BRK
            0x60,              # 0x0006: RTS
        ])
        result = MOS6502Simulator().execute(prog)
        assert result.final_state.a == 1  # Returned to LDA #1


class TestRTI:
    def test_rti_restores_flags_and_pc(self) -> None:
        # Build an interrupt-return sequence manually:
        # Push PC hi, PC lo, P; then RTI
        # Simpler: check that RTI does not add 1 to PC (unlike RTS)
        # Set up the stack manually, then run RTI from a known address.
        sim = MOS6502Simulator()
        sim.reset()
        # Manually set stack with return address $0010 and P=0x24
        sim._s = 0xFC
        sim._memory[0x01FF] = 0x00   # hi of return PC ($0010)
        sim._memory[0x01FE] = 0x10   # lo of return PC
        sim._memory[0x01FD] = 0x24   # P
        sim._memory[0x0000] = 0x40   # RTI at $0000
        sim._memory[0x0010] = 0x00   # BRK at return address
        while not sim._halted:
            sim.step()
        s = sim.get_state()
        assert s.pc == 0x0011   # $0010 + 1 step (BRK advances PC)
        assert s.flag_i is True
