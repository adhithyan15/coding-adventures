"""Tests for decoder.py — Z80 instruction decoder."""

from z80_gatelevel.decoder import DecoderZ80


class TestDecodeUnprefixed:
    def setup_method(self):
        self.dec = DecoderZ80()

    def test_nop(self):
        d = self.dec.decode_unprefixed(0x00)
        assert d.op_group == 0
        assert d.is_halt is False
        assert d.extra_bytes == 0

    def test_halt(self):
        d = self.dec.decode_unprefixed(0x76)
        assert d.is_halt is True

    def test_ld_b_n(self):
        # LD B, n = 0x06
        d = self.dec.decode_unprefixed(0x06)
        assert d.extra_bytes == 1

    def test_ld_a_n(self):
        # LD A, n = 0x3E
        d = self.dec.decode_unprefixed(0x3E)
        assert d.extra_bytes == 1

    def test_ld_rr_nn(self):
        # LD BC, nn = 0x01
        d = self.dec.decode_unprefixed(0x01)
        assert d.extra_bytes == 2
        assert d.reg_pair == 0  # BC

    def test_add_a_b(self):
        # ADD A, B = 0x80 (group 10, ALU op 0, src B)
        d = self.dec.decode_unprefixed(0x80)
        assert d.op_group == 2
        assert d.alu_op == 0   # ADD
        assert d.src == 0      # B

    def test_add_a_a(self):
        # ADD A, A = 0x87 (group 10, src A=7)
        d = self.dec.decode_unprefixed(0x87)
        assert d.op_group == 2
        assert d.alu_op == 0
        assert d.src == 7  # A

    def test_sub_c(self):
        # SUB C = 0x91 (group 10, ALU op 2, src C=1)
        d = self.dec.decode_unprefixed(0x91)
        assert d.op_group == 2
        assert d.alu_op == 2   # SUB
        assert d.src == 1      # C

    def test_and_h(self):
        # AND H = 0xA4 (group 10, ALU op 4, src H=4)
        d = self.dec.decode_unprefixed(0xA4)
        assert d.op_group == 2
        assert d.alu_op == 4   # AND
        assert d.src == 4      # H

    def test_cp_n(self):
        # CP n = 0xFE (group 11, ALU op 7, immediate)
        d = self.dec.decode_unprefixed(0xFE)
        assert d.op_group == 3
        assert d.extra_bytes == 1

    def test_jp_nn(self):
        # JP nn = 0xC3
        d = self.dec.decode_unprefixed(0xC3)
        assert d.op_group == 3
        assert d.extra_bytes == 2

    def test_call_nn(self):
        # CALL nn = 0xCD
        d = self.dec.decode_unprefixed(0xCD)
        assert d.extra_bytes == 2

    def test_jr_e(self):
        # JR e = 0x18
        d = self.dec.decode_unprefixed(0x18)
        assert d.extra_bytes == 1

    def test_jr_nz_e(self):
        # JR NZ, e = 0x20
        d = self.dec.decode_unprefixed(0x20)
        assert d.extra_bytes == 1

    def test_ld_r_r_group01(self):
        # LD B, C = 0x41 (group 01)
        d = self.dec.decode_unprefixed(0x41)
        assert d.op_group == 1
        assert d.dst == 0   # B
        assert d.src == 1   # C

    def test_memory_src(self):
        # ADD A, (HL) = 0x86 (group 10, src=6)
        d = self.dec.decode_unprefixed(0x86)
        assert d.is_memory_src is True
        assert d.is_halt is False

    def test_memory_dst(self):
        # LD (HL), A = 0x77 (group 01, dst=6)
        d = self.dec.decode_unprefixed(0x77)
        assert d.is_memory_dst is True
        assert d.is_halt is False

    def test_ld_hl_nn(self):
        # LD HL, nn = 0x21
        d = self.dec.decode_unprefixed(0x21)
        assert d.extra_bytes == 2

    def test_group_groups(self):
        # Verify one-hot encoding
        for op in (0x00, 0x01, 0x06, 0x21):
            d = self.dec.decode_unprefixed(op)
            assert d.op_group == 0
        for op in (0x40, 0x41, 0x7E):
            d = self.dec.decode_unprefixed(op)
            assert d.op_group == 1
        for op in (0x80, 0x87, 0xBF):
            d = self.dec.decode_unprefixed(op)
            assert d.op_group == 2
        for op in (0xC3, 0xCD, 0xFF):
            d = self.dec.decode_unprefixed(op)
            assert d.op_group == 3


class TestDecodeCB:
    def setup_method(self):
        self.dec = DecoderZ80()

    def test_rlc_b(self):
        # CB 0x00 = RLC B
        d = self.dec.decode_cb(0x00)
        assert d.prefix == 0xCB
        assert d.op_group == 0
        assert d.src == 0   # B

    def test_rlc_a(self):
        # CB 0x07 = RLC A
        d = self.dec.decode_cb(0x07)
        assert d.src == 7   # A

    def test_bit_3_h(self):
        # CB 0x5C = BIT 3, H
        d = self.dec.decode_cb(0x5C)
        assert d.op_group == 1  # BIT group
        assert d.dst == 3       # bit number
        assert d.src == 4       # H

    def test_res_0_a(self):
        # CB 0x87 = RES 0, A
        d = self.dec.decode_cb(0x87)
        assert d.op_group == 2  # RES group

    def test_set_7_l(self):
        # CB 0xFD = SET 7, L
        d = self.dec.decode_cb(0xFD)
        assert d.op_group == 3  # SET group
        assert d.src == 5       # L

    def test_memory_src_cb(self):
        # CB 0x06 = RLC (HL) — r_code == 6
        d = self.dec.decode_cb(0x06)
        assert d.is_memory_src is True


class TestDecodeED:
    def setup_method(self):
        self.dec = DecoderZ80()

    def test_adc_hl_bc(self):
        d = self.dec.decode_ed(0x4A)
        assert d.prefix == 0xED
        assert d.extra_bytes == 0

    def test_sbc_hl_bc(self):
        d = self.dec.decode_ed(0x42)
        assert d.prefix == 0xED

    def test_ld_rp_nn(self):
        # ED 0x4B = LD BC, (nn)
        d = self.dec.decode_ed(0x4B)
        assert d.extra_bytes == 2

    def test_neg(self):
        d = self.dec.decode_ed(0x44)
        assert d.prefix == 0xED


class TestDecodeDDFD:
    def setup_method(self):
        self.dec = DecoderZ80()

    def test_ld_ix_nn(self):
        d = self.dec.decode_dd_fd(0xDD, 0x21)
        assert d.prefix == 0xDD
        assert d.extra_bytes == 2

    def test_add_ix_bc(self):
        d = self.dec.decode_dd_fd(0xDD, 0x09)
        assert d.prefix == 0xDD

    def test_iy_prefix(self):
        d = self.dec.decode_dd_fd(0xFD, 0x21)
        assert d.prefix == 0xFD
