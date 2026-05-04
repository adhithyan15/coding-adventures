"""Tests for the Decoder8080 combinational instruction decoder."""

from __future__ import annotations

from intel8080_gatelevel.decoder import Decoder8080


class TestDecoder8080:
    def setup_method(self) -> None:
        self.dec = Decoder8080()

    # ── Group detection ────────────────────────────────────────────────────

    def test_group00_nop(self) -> None:
        d = self.dec.decode(0x00)
        assert d.op_group == 0

    def test_group01_mov(self) -> None:
        d = self.dec.decode(0x41)   # MOV B,C
        assert d.op_group == 1
        assert d.is_halt is False

    def test_group10_add(self) -> None:
        d = self.dec.decode(0x80)   # ADD B
        assert d.op_group == 2
        assert d.alu_op == 0   # ADD

    def test_group11_jmp(self) -> None:
        d = self.dec.decode(0xC3)   # JMP
        assert d.op_group == 3

    # ── HLT detection ─────────────────────────────────────────────────────

    def test_hlt_is_halt(self) -> None:
        d = self.dec.decode(0x76)
        assert d.is_halt is True

    def test_mov_not_halt(self) -> None:
        d = self.dec.decode(0x41)
        assert d.is_halt is False

    def test_mov_bb_not_halt(self) -> None:
        # MOV B,B = 0x40 (not 0x76)
        d = self.dec.decode(0x40)
        assert d.is_halt is False

    # ── Register field extraction ─────────────────────────────────────────

    def test_add_b_src(self) -> None:
        d = self.dec.decode(0x80)   # ADD B: alu_op=000, src=000
        assert d.src == 0   # B

    def test_add_a_src(self) -> None:
        d = self.dec.decode(0x87)   # ADD A: src=111
        assert d.src == 7   # A

    def test_mov_dst_src(self) -> None:
        d = self.dec.decode(0x47)   # MOV B,A: dst=000, src=111
        assert d.dst == 0   # B
        assert d.src == 7   # A

    def test_mov_ae_dst_src(self) -> None:
        d = self.dec.decode(0x7B)   # MOV A,E: dst=111, src=011
        assert d.dst == 7   # A
        assert d.src == 3   # E

    # ── ALU op codes ──────────────────────────────────────────────────────

    def test_alu_op_add(self) -> None:
        assert self.dec.decode(0x80).alu_op == 0   # ADD

    def test_alu_op_adc(self) -> None:
        assert self.dec.decode(0x88).alu_op == 1   # ADC

    def test_alu_op_sub(self) -> None:
        assert self.dec.decode(0x90).alu_op == 2   # SUB

    def test_alu_op_ana(self) -> None:
        assert self.dec.decode(0xA0).alu_op == 4   # ANA

    def test_alu_op_cmp(self) -> None:
        assert self.dec.decode(0xB8).alu_op == 7   # CMP

    # ── Memory operand detection ──────────────────────────────────────────

    def test_add_m_memory_src(self) -> None:
        d = self.dec.decode(0x86)   # ADD M: src=6
        assert d.is_memory_src is True

    def test_mov_m_dst_memory_dst(self) -> None:
        d = self.dec.decode(0x70)   # MOV M,B: dst=6
        assert d.is_memory_dst is True

    def test_add_b_no_memory(self) -> None:
        d = self.dec.decode(0x80)
        assert d.is_memory_src is False
        assert d.is_memory_dst is False

    def test_hlt_no_memory(self) -> None:
        d = self.dec.decode(0x76)
        assert d.is_memory_src is False
        assert d.is_memory_dst is False

    # ── Extra bytes (instruction length) ──────────────────────────────────

    def test_mov_1byte(self) -> None:
        assert self.dec.decode(0x41).extra_bytes == 0

    def test_mvi_2byte(self) -> None:
        assert self.dec.decode(0x06).extra_bytes == 1   # MVI B,d8

    def test_lxi_3byte(self) -> None:
        assert self.dec.decode(0x01).extra_bytes == 2   # LXI B,d16

    def test_jmp_3byte(self) -> None:
        assert self.dec.decode(0xC3).extra_bytes == 2   # JMP addr16

    def test_call_3byte(self) -> None:
        assert self.dec.decode(0xCD).extra_bytes == 2   # CALL addr16

    def test_adi_2byte(self) -> None:
        assert self.dec.decode(0xC6).extra_bytes == 1   # ADI d8

    def test_in_2byte(self) -> None:
        assert self.dec.decode(0xDB).extra_bytes == 1   # IN port

    def test_out_2byte(self) -> None:
        assert self.dec.decode(0xD3).extra_bytes == 1   # OUT port

    def test_add_1byte(self) -> None:
        assert self.dec.decode(0x80).extra_bytes == 0

    # ── Register pair field ───────────────────────────────────────────────

    def test_lxi_b_pair(self) -> None:
        d = self.dec.decode(0x01)   # LXI B: reg_pair=00
        assert d.reg_pair == 0

    def test_lxi_sp_pair(self) -> None:
        d = self.dec.decode(0x31)   # LXI SP: reg_pair=11
        assert d.reg_pair == 3

    def test_dad_b_pair(self) -> None:
        d = self.dec.decode(0x09)   # DAD B: reg_pair=00
        assert d.reg_pair == 0

    def test_dad_h_pair(self) -> None:
        d = self.dec.decode(0x29)   # DAD H: reg_pair=10
        assert d.reg_pair == 2
