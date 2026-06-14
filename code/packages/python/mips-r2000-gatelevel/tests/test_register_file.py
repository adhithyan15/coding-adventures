"""Tests for register_file.py — MIPS R2000 gate-level register file."""


from mips_r2000_gatelevel.register_file import RegisterFile32


class TestRegisterFile32:
    def setup_method(self):
        self.rf = RegisterFile32()

    # ── GPR read/write ─────────────────────────────────────────────────────────

    def test_initial_state_all_zero(self):
        for i in range(32):
            assert self.rf.read_reg(i) == 0

    def test_write_read_all_gprs(self):
        for i in range(1, 32):  # skip R0
            self.rf.write_reg(i, i * 0x12345)
            assert self.rf.read_reg(i) == i * 0x12345

    def test_r0_always_returns_zero(self):
        # Write anything to R0 — it should be ignored
        self.rf.write_reg(0, 0xDEAD_BEEF)
        assert self.rf.read_reg(0) == 0

    def test_r0_write_silently_discarded(self):
        self.rf.write_reg(0, 0xFFFF_FFFF)
        self.rf.write_reg(0, 42)
        assert self.rf.read_reg(0) == 0

    def test_write_max_value(self):
        self.rf.write_reg(1, 0xFFFF_FFFF)
        assert self.rf.read_reg(1) == 0xFFFF_FFFF

    def test_write_masks_to_32_bits(self):
        self.rf.write_reg(5, 0x1_0000_0001)  # exceeds 32 bits
        assert self.rf.read_reg(5) == 1

    def test_overwrite_register(self):
        self.rf.write_reg(10, 100)
        self.rf.write_reg(10, 200)
        assert self.rf.read_reg(10) == 200

    def test_registers_are_independent(self):
        for i in range(1, 32):
            self.rf.write_reg(i, i)
        for i in range(1, 32):
            assert self.rf.read_reg(i) == i

    def test_r31_is_regular_register(self):
        self.rf.write_reg(31, 0xABCD_1234)
        assert self.rf.read_reg(31) == 0xABCD_1234

    # ── HI register ───────────────────────────────────────────────────────────

    def test_hi_initial(self):
        assert self.rf.read_hi() == 0

    def test_hi_write_read(self):
        self.rf.write_hi(0xDEAD_BEEF)
        assert self.rf.read_hi() == 0xDEAD_BEEF

    def test_hi_max(self):
        self.rf.write_hi(0xFFFF_FFFF)
        assert self.rf.read_hi() == 0xFFFF_FFFF

    def test_hi_masked(self):
        self.rf.write_hi(0x1_0000_0001)
        assert self.rf.read_hi() == 1

    # ── LO register ───────────────────────────────────────────────────────────

    def test_lo_initial(self):
        assert self.rf.read_lo() == 0

    def test_lo_write_read(self):
        self.rf.write_lo(0x1234_5678)
        assert self.rf.read_lo() == 0x1234_5678

    def test_lo_independent_from_hi(self):
        self.rf.write_hi(0xAAAA_AAAA)
        self.rf.write_lo(0x5555_5555)
        assert self.rf.read_hi() == 0xAAAA_AAAA
        assert self.rf.read_lo() == 0x5555_5555

    # ── PC register ───────────────────────────────────────────────────────────

    def test_pc_initial(self):
        assert self.rf.read_pc() == 0

    def test_pc_write_read(self):
        self.rf.write_pc(0x1000)
        assert self.rf.read_pc() == 0x1000

    def test_pc_masked(self):
        self.rf.write_pc(0x1_0000_0004)
        assert self.rf.read_pc() == 4

    def test_increment_pc_by_4(self):
        self.rf.write_pc(0)
        self.rf.increment_pc(4)
        assert self.rf.read_pc() == 4

    def test_increment_pc_multiple(self):
        self.rf.write_pc(0)
        for _i in range(10):
            self.rf.increment_pc(4)
        assert self.rf.read_pc() == 40

    def test_increment_pc_wraps(self):
        self.rf.write_pc(0xFFFF_FFFC)
        self.rf.increment_pc(4)
        # Wraps around: 0xFFFFFFFC + 4 = 0x100000000 & 0xFFFFFFFF = 0
        assert self.rf.read_pc() == 0

    def test_increment_pc_custom_amount(self):
        self.rf.write_pc(100)
        self.rf.increment_pc(8)
        assert self.rf.read_pc() == 108

    # ── Bit storage integrity ──────────────────────────────────────────────────

    def test_bit_patterns_preserved(self):
        # Test alternating patterns
        self.rf.write_reg(7, 0xAAAA_AAAA)
        assert self.rf.read_reg(7) == 0xAAAA_AAAA

        self.rf.write_reg(7, 0x5555_5555)
        assert self.rf.read_reg(7) == 0x5555_5555

    def test_msb_stored_correctly(self):
        self.rf.write_reg(3, 0x8000_0000)
        assert self.rf.read_reg(3) == 0x8000_0000
