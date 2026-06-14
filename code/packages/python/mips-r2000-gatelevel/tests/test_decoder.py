"""Tests for decoder.py — MIPS R2000 instruction field extraction."""


from mips_r2000_gatelevel.decoder import decode_instruction


def r_type(rs: int, rt: int, rd: int, shamt: int, funct: int) -> int:
    """Encode an R-type instruction word."""
    return (0 << 26) | (rs << 21) | (rt << 16) | (rd << 11) | (shamt << 6) | funct


def i_type(op: int, rs: int, rt: int, imm16: int) -> int:
    """Encode an I-type instruction word."""
    return (op << 26) | (rs << 21) | (rt << 16) | (imm16 & 0xFFFF)


def j_type(op: int, target26: int) -> int:
    """Encode a J-type instruction word."""
    return (op << 26) | (target26 & 0x3FF_FFFF)


# ── R-type decoding ────────────────────────────────────────────────────────────


class TestDecodeRType:
    def test_add(self):
        # ADD $t0, $t1, $t2  — rs=9, rt=10, rd=8, funct=0x20
        word = r_type(rs=9, rt=10, rd=8, shamt=0, funct=0x20)
        d = decode_instruction(word)
        assert d["format"] == "R"
        assert d["op"] == 0
        assert d["rs"] == 9
        assert d["rt"] == 10
        assert d["rd"] == 8
        assert d["shamt"] == 0
        assert d["funct"] == 0x20
        assert d["mnemonic"] == "ADD"

    def test_sll(self):
        # SLL $t0, $t1, 5  — rs=0, rt=9, rd=8, shamt=5, funct=0
        word = r_type(rs=0, rt=9, rd=8, shamt=5, funct=0x00)
        d = decode_instruction(word)
        assert d["format"] == "R"
        assert d["rt"] == 9
        assert d["rd"] == 8
        assert d["shamt"] == 5
        assert d["funct"] == 0
        assert d["mnemonic"] == "SLL"

    def test_srl(self):
        word = r_type(rs=0, rt=5, rd=6, shamt=3, funct=0x02)
        d = decode_instruction(word)
        assert d["funct"] == 0x02
        assert d["mnemonic"] == "SRL"
        assert d["shamt"] == 3

    def test_sra(self):
        word = r_type(rs=0, rt=3, rd=4, shamt=2, funct=0x03)
        d = decode_instruction(word)
        assert d["mnemonic"] == "SRA"
        assert d["shamt"] == 2

    def test_jr(self):
        # JR $ra — rs=31
        word = r_type(rs=31, rt=0, rd=0, shamt=0, funct=0x08)
        d = decode_instruction(word)
        assert d["rs"] == 31
        assert d["funct"] == 0x08
        assert d["mnemonic"] == "JR"

    def test_mult(self):
        word = r_type(rs=5, rt=6, rd=0, shamt=0, funct=0x18)
        d = decode_instruction(word)
        assert d["mnemonic"] == "MULT"
        assert d["rs"] == 5
        assert d["rt"] == 6

    def test_divu(self):
        word = r_type(rs=3, rt=4, rd=0, shamt=0, funct=0x1B)
        d = decode_instruction(word)
        assert d["mnemonic"] == "DIVU"

    def test_syscall(self):
        word = r_type(rs=0, rt=0, rd=0, shamt=0, funct=0x0C)
        d = decode_instruction(word)
        assert d["mnemonic"] == "SYSCALL"

    def test_subu(self):
        word = r_type(rs=1, rt=2, rd=3, shamt=0, funct=0x23)
        d = decode_instruction(word)
        assert d["mnemonic"] == "SUBU"

    def test_and(self):
        word = r_type(rs=10, rt=11, rd=12, shamt=0, funct=0x24)
        d = decode_instruction(word)
        assert d["mnemonic"] == "AND"
        assert d["rs"] == 10

    def test_sltu(self):
        word = r_type(rs=7, rt=8, rd=9, shamt=0, funct=0x2B)
        d = decode_instruction(word)
        assert d["mnemonic"] == "SLTU"

    def test_sllv(self):
        word = r_type(rs=2, rt=3, rd=4, shamt=0, funct=0x04)
        d = decode_instruction(word)
        assert d["mnemonic"] == "SLLV"
        assert d["rs"] == 2
        assert d["rt"] == 3

    def test_mfhi(self):
        word = r_type(rs=0, rt=0, rd=8, shamt=0, funct=0x10)
        d = decode_instruction(word)
        assert d["mnemonic"] == "MFHI"
        assert d["rd"] == 8

    def test_unknown_funct(self):
        word = r_type(rs=0, rt=0, rd=0, shamt=0, funct=0x3F)
        d = decode_instruction(word)
        assert d["mnemonic"] == "UNKNOWN"


# ── I-type decoding ────────────────────────────────────────────────────────────


class TestDecodeIType:
    def test_addiu(self):
        # ADDIU $t0, $t1, 100  — op=9, rs=9, rt=8, imm=100
        word = i_type(op=0x09, rs=9, rt=8, imm16=100)
        d = decode_instruction(word)
        assert d["format"] == "I"
        assert d["op"] == 0x09
        assert d["rs"] == 9
        assert d["rt"] == 8
        assert d["imm16"] == 100
        assert d["mnemonic"] == "ADDIU"

    def test_addi(self):
        word = i_type(op=0x08, rs=5, rt=6, imm16=42)
        d = decode_instruction(word)
        assert d["mnemonic"] == "ADDI"
        assert d["imm16"] == 42

    def test_sign_extension_positive(self):
        # imm16 = 0x7FFF (32767, positive) — should not sign-extend
        word = i_type(op=0x09, rs=0, rt=1, imm16=0x7FFF)
        d = decode_instruction(word)
        assert d["imm16"] == 0x7FFF

    def test_sign_extension_negative(self):
        # imm16 = 0xFFFF (-1) — should sign-extend to -1
        word = i_type(op=0x09, rs=0, rt=1, imm16=0xFFFF)
        d = decode_instruction(word)
        assert d["imm16"] == -1

    def test_sign_extension_0x8000(self):
        # 0x8000 = -32768
        word = i_type(op=0x09, rs=0, rt=1, imm16=0x8000)
        d = decode_instruction(word)
        assert d["imm16"] == -32768

    def test_beq(self):
        word = i_type(op=0x04, rs=8, rt=9, imm16=5)
        d = decode_instruction(word)
        assert d["mnemonic"] == "BEQ"
        assert d["rs"] == 8
        assert d["rt"] == 9
        assert d["imm16"] == 5

    def test_bne(self):
        word = i_type(op=0x05, rs=3, rt=4, imm16=0xFFFC)
        d = decode_instruction(word)
        assert d["mnemonic"] == "BNE"
        assert d["imm16"] == -4  # sign extended

    def test_lw(self):
        word = i_type(op=0x23, rs=29, rt=4, imm16=8)
        d = decode_instruction(word)
        assert d["mnemonic"] == "LW"
        assert d["rs"] == 29
        assert d["rt"] == 4
        assert d["imm16"] == 8

    def test_sw(self):
        word = i_type(op=0x2B, rs=29, rt=4, imm16=0)
        d = decode_instruction(word)
        assert d["mnemonic"] == "SW"

    def test_lui(self):
        word = i_type(op=0x0F, rs=0, rt=8, imm16=0x1234)
        d = decode_instruction(word)
        assert d["mnemonic"] == "LUI"
        assert d["imm16"] == 0x1234

    def test_ori(self):
        word = i_type(op=0x0D, rs=8, rt=8, imm16=0x00FF)
        d = decode_instruction(word)
        assert d["mnemonic"] == "ORI"

    def test_andi(self):
        word = i_type(op=0x0C, rs=10, rt=10, imm16=0x00FF)
        d = decode_instruction(word)
        assert d["mnemonic"] == "ANDI"

    def test_slti(self):
        word = i_type(op=0x0A, rs=5, rt=6, imm16=10)
        d = decode_instruction(word)
        assert d["mnemonic"] == "SLTI"

    def test_unknown_op(self):
        word = i_type(op=0x3F, rs=0, rt=0, imm16=0)
        d = decode_instruction(word)
        assert d["mnemonic"] == "UNKNOWN"


# ── J-type decoding ────────────────────────────────────────────────────────────


class TestDecodeJType:
    def test_j(self):
        # J 0x1000 — op=2, target26=0x1000
        word = j_type(op=0x02, target26=0x1000)
        d = decode_instruction(word)
        assert d["format"] == "J"
        assert d["op"] == 0x02
        assert d["target26"] == 0x1000
        assert d["mnemonic"] == "J"

    def test_jal(self):
        word = j_type(op=0x03, target26=0x2000)
        d = decode_instruction(word)
        assert d["format"] == "J"
        assert d["mnemonic"] == "JAL"
        assert d["target26"] == 0x2000

    def test_j_large_target(self):
        # Max target26 = 0x3FFFFFF
        word = j_type(op=0x02, target26=0x3FF_FFFF)
        d = decode_instruction(word)
        assert d["target26"] == 0x3FF_FFFF

    def test_j_zero_target(self):
        word = j_type(op=0x02, target26=0)
        d = decode_instruction(word)
        assert d["target26"] == 0


# ── REGIMM decoding ───────────────────────────────────────────────────────────


class TestDecodeRegimm:
    def test_bltz(self):
        # BLTZ $t0, offset — op=1, rs=8, rt=0
        word = i_type(op=0x01, rs=8, rt=0x00, imm16=5)
        d = decode_instruction(word)
        assert d["format"] == "I"
        assert d["mnemonic"] == "BLTZ"
        assert d["rs"] == 8

    def test_bgez(self):
        word = i_type(op=0x01, rs=5, rt=0x01, imm16=3)
        d = decode_instruction(word)
        assert d["mnemonic"] == "BGEZ"

    def test_bltzal(self):
        word = i_type(op=0x01, rs=3, rt=0x10, imm16=2)
        d = decode_instruction(word)
        assert d["mnemonic"] == "BLTZAL"

    def test_bgezal(self):
        word = i_type(op=0x01, rs=1, rt=0x11, imm16=10)
        d = decode_instruction(word)
        assert d["mnemonic"] == "BGEZAL"


# ── Field boundary tests ───────────────────────────────────────────────────────


class TestFieldBoundaries:
    def test_all_register_fields(self):
        # Encode with distinct register numbers to verify each field is extracted correctly
        word = r_type(rs=1, rt=2, rd=3, shamt=4, funct=0x20)
        d = decode_instruction(word)
        assert d["rs"] == 1
        assert d["rt"] == 2
        assert d["rd"] == 3
        assert d["shamt"] == 4

    def test_max_register_numbers(self):
        word = r_type(rs=31, rt=31, rd=31, shamt=31, funct=0x20)
        d = decode_instruction(word)
        assert d["rs"] == 31
        assert d["rt"] == 31
        assert d["rd"] == 31
        assert d["shamt"] == 31

    def test_nop_word(self):
        # 0x00000000 = SLL $zero, $zero, 0
        d = decode_instruction(0)
        assert d["format"] == "R"
        assert d["op"] == 0
        assert d["rs"] == 0
        assert d["rt"] == 0
        assert d["rd"] == 0
        assert d["shamt"] == 0
        assert d["funct"] == 0
