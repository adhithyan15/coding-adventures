"""
Instruction-level tests for the RV64I + M extension simulator.

Each test encodes a short program, runs it to halt, and asserts register/memory
state.  All instruction encoders live in conftest.py.
"""

from __future__ import annotations

from conftest import (
    add,
    addi,
    addiw,
    addw,
    and_,
    andi,
    auipc,
    beq,
    bge,
    bgeu,
    blt,
    bltu,
    bne,
    div_,
    divu,
    divuw,
    divw,
    jal,
    jalr,
    lb,
    lbu,
    ld,
    lh,
    lhu,
    lui,
    lw,
    lwu,
    mul,
    mulh,
    mulhsu,
    mulhu,
    mulw,
    or_,
    ori,
    rem,
    remu,
    remuw,
    remw,
    run,
    sb,
    sd,
    sh,
    sll,
    slli,
    slliw,
    sllw,
    slt,
    slti,
    sltiu,
    sltu,
    sra,
    srai,
    sraiw,
    sraw,
    srl,
    srli,
    srliw,
    srlw,
    sub,
    subw,
    sw,
    xor,
    xori,
)

from riscv_rv64i_simulator import RV64ISimulator

# ── ALU Immediate ──────────────────────────────────────────────────────────────


class TestALUImmediate:
    def test_addi_positive(self):
        """ADDI x10, x0, 42 → x10 = 42."""
        state = run([addi(10, 0, 42)])
        assert state.a0 == 42

    def test_addi_negative(self):
        """ADDI x10, x0, -1 → x10 = 0xFFFF_FFFF_FFFF_FFFF (−1 sign-extended)."""
        state = run([addi(10, 0, -1)])
        assert state.a0 == 0xFFFF_FFFF_FFFF_FFFF

    def test_addi_accumulate(self):
        """Two ADDIs: x10 = 0 + 10 + 5 = 15."""
        state = run([addi(10, 0, 10), addi(10, 10, 5)])
        assert state.a0 == 15

    def test_xori_flip_bits(self):
        """XORI x10, x0, 0xFF → x10 = 0xFF."""
        state = run([xori(10, 0, 0xFF)])
        assert state.a0 == 0xFF

    def test_ori_set_bits(self):
        """Set bits via ORI."""
        state = run([addi(10, 0, 0xF0), ori(10, 10, 0x0F)])
        assert state.a0 == 0xFF

    def test_andi_clear_bits(self):
        """ANDI x10, x10, 0x0F clears high nibble."""
        state = run([addi(10, 0, 0xFF), andi(10, 10, 0x0F)])
        assert state.a0 == 0x0F

    def test_slti_true(self):
        """SLTI: -1 < 1 → 1."""
        state = run([addi(10, 0, -1), slti(11, 10, 1)])
        assert state.a1 == 1

    def test_slti_false(self):
        """SLTI: 5 < 3 → 0."""
        state = run([addi(10, 0, 5), slti(11, 10, 3)])
        assert state.a1 == 0

    def test_sltiu_unsigned(self):
        """SLTIU: 1 < 0xFFF (unsigned) → 1."""
        state = run([addi(10, 0, 1), sltiu(11, 10, 0xFFF)])
        assert state.a1 == 1

    def test_slli(self):
        """SLLI x10, x10, 3 → 1 << 3 = 8."""
        state = run([addi(10, 0, 1), slli(10, 10, 3)])
        assert state.a0 == 8

    def test_srli(self):
        """SRLI x10, x10, 2 → 16 >> 2 = 4 (logical, no sign extension)."""
        state = run([addi(10, 0, 16), srli(10, 10, 2)])
        assert state.a0 == 4

    def test_srai_positive(self):
        """SRAI on positive value: 16 >> 2 = 4."""
        state = run([addi(10, 0, 16), srai(10, 10, 2)])
        assert state.a0 == 4

    def test_srai_negative(self):
        """SRAI on negative value propagates sign bit."""
        # x10 = -8 (0xFFFF_FFFF_FFFF_FFF8)
        state = run([addi(10, 0, -8), srai(10, 10, 1)])
        # -8 >> 1 = -4 (0xFFFF_FFFF_FFFF_FFFC)
        assert state.a0 == 0xFFFF_FFFF_FFFF_FFFC


# ── ALU Register ──────────────────────────────────────────────────────────────


class TestALURegister:
    def test_add(self):
        state = run([addi(10, 0, 20), addi(11, 0, 22), add(12, 10, 11)])
        assert state.a2 == 42

    def test_sub(self):
        state = run([addi(10, 0, 10), addi(11, 0, 7), sub(12, 10, 11)])
        assert state.a2 == 3

    def test_sub_to_negative(self):
        """5 - 10 = -5 (wraps to 0xFFFF...FFFB)."""
        state = run([addi(10, 0, 5), addi(11, 0, 10), sub(12, 10, 11)])
        assert state.a2 == 0xFFFF_FFFF_FFFF_FFFB

    def test_and(self):
        state = run([addi(10, 0, 0xFF), addi(11, 0, 0x0F), and_(12, 10, 11)])
        assert state.a2 == 0x0F

    def test_or(self):
        state = run([addi(10, 0, 0xF0), addi(11, 0, 0x0F), or_(12, 10, 11)])
        assert state.a2 == 0xFF

    def test_xor(self):
        state = run([addi(10, 0, 0xFF), addi(11, 0, 0xF0), xor(12, 10, 11)])
        assert state.a2 == 0x0F

    def test_sll(self):
        state = run([addi(10, 0, 1), addi(11, 0, 4), sll(12, 10, 11)])
        assert state.a2 == 16

    def test_srl(self):
        state = run([addi(10, 0, 16), addi(11, 0, 2), srl(12, 10, 11)])
        assert state.a2 == 4

    def test_sra(self):
        """SRA: -8 >> 2 = -2."""
        state = run([addi(10, 0, -8), addi(11, 0, 2), sra(12, 10, 11)])
        assert state.a2 == 0xFFFF_FFFF_FFFF_FFFE

    def test_slt_signed(self):
        """-1 < 0 → 1."""
        state = run([addi(10, 0, -1), addi(11, 0, 0), slt(12, 10, 11)])
        assert state.a2 == 1

    def test_sltu_unsigned(self):
        """1 <u 0xFFFF...FFFF → 1."""
        state = run([addi(10, 0, 1), addi(11, 0, -1), sltu(12, 10, 11)])
        assert state.a2 == 1

    def test_x0_write_ignored(self):
        """Writing to x0 must be silently discarded."""
        state = run([addi(0, 0, 99)])
        assert state.zero == 0
        assert state.gpr[0] == 0


# ── RV64I Word Ops ─────────────────────────────────────────────────────────────


class TestWordOps:
    def test_addiw_truncates_to_32_and_sext(self):
        """
        ADDIW operates on the lower 32 bits and sign-extends to 64.

        x10 = -1 (0xFFFF_FFFF_FFFF_FFFF); ADDIW x10, x10, -1:
          - takes lower 32 bits: 0xFFFF_FFFF (= -1 in 32-bit)
          - adds -1: -1 + (-1) = -2 in 32-bit = 0xFFFF_FFFE
          - sign-extends to 64: 0xFFFF_FFFF_FFFF_FFFE
        """
        state = run([addi(10, 0, -1), addiw(10, 10, -1)])
        assert state.a0 == 0xFFFF_FFFF_FFFF_FFFE

    def test_addw_result_sext(self):
        """ADDW: 32-bit add with sign extension."""
        state = run([addi(10, 0, -1), addi(11, 0, 1), addw(12, 10, 11)])
        assert state.a2 == 0   # -1 + 1 = 0

    def test_subw_result_sext(self):
        """SUBW: 32-bit sub with sign-extension of result.

        x10 = 0, x11 = 1; SUBW x12, x10, x11:
          - 0 - 1 = -1 in 32-bit (0xFFFF_FFFF)
          - sign-extended to 64 → 0xFFFF_FFFF_FFFF_FFFF
        """
        state = run([addi(10, 0, 0), addi(11, 0, 1), subw(12, 10, 11)])
        assert state.a2 == 0xFFFF_FFFF_FFFF_FFFF

    def test_slliw(self):
        """SLLIW: shift 1 left by 4 within 32 bits, sign-extend."""
        state = run([addi(10, 0, 1), slliw(10, 10, 4)])
        assert state.a0 == 16

    def test_srliw_no_sign_extension(self):
        """SRLIW: logical shift fills with 0, result sign-extended to 64."""
        state = run([addi(10, 0, -1), srliw(10, 10, 1)])
        # 0xFFFF_FFFF >> 1 = 0x7FFF_FFFF; positive → sext is same
        assert state.a0 == 0x7FFF_FFFF

    def test_sraiw_propagates_sign(self):
        """SRAIW: arithmetic right shift preserves sign in 32-bit, then sext to 64."""
        state = run([addi(10, 0, -4), sraiw(10, 10, 1)])
        # -4 in 32-bit is 0xFFFF_FFFC; >>1 = 0xFFFF_FFFE = -2; sext → 0xFFFF_FFFF_FFFF_FFFE
        assert state.a0 == 0xFFFF_FFFF_FFFF_FFFE

    def test_sllw(self):
        state = run([addi(10, 0, 1), addi(11, 0, 3), sllw(12, 10, 11)])
        assert state.a2 == 8

    def test_srlw(self):
        state = run([addi(10, 0, 16), addi(11, 0, 2), srlw(12, 10, 11)])
        assert state.a2 == 4

    def test_sraw_negative(self):
        """SRAW on negative 32-bit value sign-extends the shift result."""
        state = run([addi(10, 0, -8), addi(11, 0, 2), sraw(12, 10, 11)])
        # -8 >> 2 = -2 in 32-bit; sext to 64 → 0xFFFF_FFFF_FFFF_FFFE
        assert state.a2 == 0xFFFF_FFFF_FFFF_FFFE


# ── LUI / AUIPC ───────────────────────────────────────────────────────────────


class TestUpperImmediate:
    def test_lui_loads_upper(self):
        """LUI x10, 1 → x10 = 0x1000."""
        state = run([lui(10, 1)])
        assert state.a0 == 0x1000

    def test_lui_large(self):
        """LUI x10, 0xDEADB → x10 = 0xDEADB000."""
        state = run([lui(10, 0xDEADB)])
        assert state.a0 == 0xFFFF_FFFF_DEAD_B000

    def test_auipc_adds_pc(self):
        """
        AUIPC x10, 1 at PC=0 → x10 = 0 + (1 << 12) = 0x1000.
        """
        state = run([auipc(10, 1)])
        assert state.a0 == 0x1000

    def test_lui_then_addi(self):
        """
        Standard two-instruction load of a 32-bit constant:
          LUI x10, 0xABCDE  →  x10 = 0xABCDE000
          ADDI x10, x10, 0x123  →  x10 = 0xABCDE123
        """
        state = run([lui(10, 0xABCDE), addi(10, 10, 0x123)])
        assert state.a0 == 0xFFFF_FFFF_ABCDE123


# ── Load / Store ──────────────────────────────────────────────────────────────


class TestLoadStore:
    def test_sw_lw_roundtrip(self):
        """Store a word to memory and load it back."""
        sim = RV64ISimulator()
        prog = (
            bytes(addi(5, 0, 0x100))   # x5 = 0x100 (base address)
            + bytes(addi(10, 0, 0x5A)) # x10 = 0x5A (value)
            + bytes(sw(5, 10, 0))      # mem[0x100] = 0x5A
            + bytes(lw(11, 5, 0))      # x11 = mem[0x100]
            + b"\x00\x00\x00\x00"
        )
        state = sim.execute(prog)
        assert state.a1 == 0x5A

    def test_sb_lb_sign_extends(self):
        """LB sign-extends the loaded byte."""
        sim = RV64ISimulator()
        prog = (
            bytes(addi(5, 0, 0x200))
            + bytes(addi(10, 0, -1))   # x10 = 0xFF (truncated to byte)
            + bytes(sb(5, 10, 0))
            + bytes(lb(11, 5, 0))      # x11 = sign_extend(0xFF, 8) = -1
            + b"\x00\x00\x00\x00"
        )
        state = sim.execute(prog)
        assert state.a1 == 0xFFFF_FFFF_FFFF_FFFF

    def test_lbu_zero_extends(self):
        """LBU zero-extends the loaded byte."""
        sim = RV64ISimulator()
        prog = (
            bytes(addi(5, 0, 0x200))
            + bytes(addi(10, 0, -1))
            + bytes(sb(5, 10, 0))
            + bytes(lbu(11, 5, 0))   # x11 = 0xFF (zero-extended)
            + b"\x00\x00\x00\x00"
        )
        state = sim.execute(prog)
        assert state.a1 == 0xFF

    def test_sh_lh_lhu(self):
        """Store halfword; LH sign-extends; LHU zero-extends."""
        sim = RV64ISimulator()
        prog = (
            bytes(addi(5, 0, 0x300))
            + bytes(addi(10, 0, -1))
            + bytes(sh(5, 10, 0))
            + bytes(lh(11, 5, 0))    # sign-extended: -1
            + bytes(lhu(12, 5, 0))   # zero-extended: 0xFFFF
            + b"\x00\x00\x00\x00"
        )
        state = sim.execute(prog)
        assert state.a1 == 0xFFFF_FFFF_FFFF_FFFF
        assert state.a2 == 0xFFFF

    def test_sd_ld_64bit(self):
        """Store and load a 64-bit double word."""
        sim = RV64ISimulator()
        # Build a 64-bit value: 0x0000_0001_0000_0002
        prog = (
            bytes(addi(5, 0, 0x400))          # base address
            + bytes(addi(10, 0, 1))            # x10 = 1
            + bytes(slli(10, 10, 32))          # x10 = 1 << 32
            + bytes(addi(11, 0, 2))            # x11 = 2
            + bytes(or_(10, 10, 11))           # x10 = 0x1_0000_0002
            + bytes(sd(5, 10, 0))              # mem[0x400] = x10
            + bytes(ld(12, 5, 0))              # x12 = mem[0x400]
            + b"\x00\x00\x00\x00"
        )
        state = sim.execute(prog)
        assert state.a2 == 0x0000_0001_0000_0002

    def test_lw_sign_extends_to_64(self):
        """LW sign-extends a 32-bit value to 64 bits."""
        sim = RV64ISimulator()
        prog = (
            bytes(addi(5, 0, 0x500))
            + bytes(addi(10, 0, -1))    # x10 = 0xFFFF_FFFF_FFFF_FFFF
            + bytes(sw(5, 10, 0))       # stores lower 32 bits: 0xFFFF_FFFF
            + bytes(lw(11, 5, 0))       # sign-extended → 0xFFFF_FFFF_FFFF_FFFF
            + b"\x00\x00\x00\x00"
        )
        state = sim.execute(prog)
        assert state.a1 == 0xFFFF_FFFF_FFFF_FFFF

    def test_lwu_zero_extends(self):
        """LWU zero-extends a 32-bit value to 64 bits."""
        sim = RV64ISimulator()
        prog = (
            bytes(addi(5, 0, 0x500))
            + bytes(addi(10, 0, -1))
            + bytes(sw(5, 10, 0))
            + bytes(lwu(11, 5, 0))   # zero-extended → 0x0000_0000_FFFF_FFFF
            + b"\x00\x00\x00\x00"
        )
        state = sim.execute(prog)
        assert state.a1 == 0x0000_0000_FFFF_FFFF

    def test_store_load_with_offset(self):
        """Load/store with positive immediate offset."""
        sim = RV64ISimulator()
        prog = (
            bytes(addi(5, 0, 0x100))
            + bytes(addi(10, 0, 0x99))
            + bytes(sw(5, 10, 8))       # mem[0x108] = 0x99
            + bytes(lw(11, 5, 8))       # x11 = mem[0x108]
            + b"\x00\x00\x00\x00"
        )
        state = sim.execute(prog)
        assert state.a1 == 0x99


# ── Branches ──────────────────────────────────────────────────────────────────


class TestBranches:
    def test_beq_taken(self):
        """BEQ: equal values → branch taken."""
        # x10 = x11 = 5; BEQ → skip MOV x12, 1; x12 stays 0
        state = run([
            addi(10, 0, 5),
            addi(11, 0, 5),
            beq(10, 11, 8),      # skip next 2 instructions (8 bytes)
            addi(12, 0, 1),      # skipped
            addi(12, 12, 0),     # skipped
        ])
        assert state.a2 == 0

    def test_beq_not_taken(self):
        """BEQ: unequal values → fall through."""
        state = run([
            addi(10, 0, 5),
            addi(11, 0, 6),
            beq(10, 11, 8),
            addi(12, 0, 1),
        ])
        assert state.a2 == 1

    def test_bne_taken(self):
        state = run([
            addi(10, 0, 1),
            addi(11, 0, 2),
            bne(10, 11, 8),
            addi(12, 0, 99),   # skipped
            addi(12, 12, 0),   # skipped
        ])
        assert state.a2 == 0

    def test_blt_signed(self):
        """-1 < 1 → BLT taken."""
        state = run([
            addi(10, 0, -1),
            addi(11, 0, 1),
            blt(10, 11, 8),
            addi(12, 0, 99),   # skipped
            addi(12, 12, 0),   # skipped
        ])
        assert state.a2 == 0

    def test_blt_not_taken_for_larger(self):
        """5 < 3 → BLT not taken."""
        state = run([
            addi(10, 0, 5),
            addi(11, 0, 3),
            blt(10, 11, 8),
            addi(12, 0, 1),
        ])
        assert state.a2 == 1

    def test_bge_taken(self):
        """3 >= 3 → BGE taken."""
        state = run([
            addi(10, 0, 3),
            addi(11, 0, 3),
            bge(10, 11, 8),
            addi(12, 0, 99),   # skipped
            addi(12, 12, 0),   # skipped
        ])
        assert state.a2 == 0

    def test_bltu_treats_as_unsigned(self):
        """BLTU: 1 <u 0xFFFF...FFFF (= -1 signed) → taken."""
        state = run([
            addi(10, 0, 1),
            addi(11, 0, -1),   # x11 = 0xFFFF...FFFF (large unsigned)
            bltu(10, 11, 8),
            addi(12, 0, 99),   # skipped
            addi(12, 12, 0),   # skipped
        ])
        assert state.a2 == 0

    def test_bgeu_taken(self):
        """-1 >=u 1 → BGEU taken (−1 is max unsigned)."""
        state = run([
            addi(10, 0, -1),
            addi(11, 0, 1),
            bgeu(10, 11, 8),
            addi(12, 0, 99),   # skipped
            addi(12, 12, 0),   # skipped
        ])
        assert state.a2 == 0

    def test_backward_branch_loop(self):
        """Simple countdown loop: x10 = 3, count down to 0."""
        sim = RV64ISimulator()
        # Addr 0: addi x10, x0, 3       (x10 = 3)
        # Addr 4: addi x10, x10, -1     (x10--)
        # Addr 8: bne  x10, x0, -4     (jump back to addr 4 while x10 != 0)
        # Addr 12: halt
        prog = (
            bytes(addi(10, 0, 3))
            + bytes(addi(10, 10, -1))
            + bytes(bne(10, 0, -4))   # back 4 bytes → addr 4
            + b"\x00\x00\x00\x00"
        )
        state = sim.execute(prog)
        assert state.a0 == 0


# ── JAL / JALR ────────────────────────────────────────────────────────────────


class TestJumpAndLink:
    def test_jal_forward(self):
        """JAL x1, +8: saves return address and jumps forward."""
        sim = RV64ISimulator()
        # Addr 0: JAL x1, +8  → jumps to addr 8, x1 = 4
        # Addr 4: addi x10, x0, 99  (skipped)
        # Addr 8: addi x10, x0, 42
        # Addr 12: halt
        prog = (
            bytes(jal(1, 8))
            + bytes(addi(10, 0, 99))   # skipped
            + bytes(addi(10, 0, 42))
            + b"\x00\x00\x00\x00"
        )
        state = sim.execute(prog)
        assert state.a0 == 42
        assert state.ra == 4   # return address

    def test_jalr_returns(self):
        """JALR returns to the saved RA."""
        sim = RV64ISimulator()
        # Addr 0: JAL x1, +8  → jumps to addr 8, x1 = 4
        # Addr 4: halt (return target)
        # Addr 8: addi x10, x0, 42
        # Addr 12: JALR x0, x1, 0  → return to addr 4 (halt)
        prog = (
            bytes(jal(1, 8))
            + b"\x00\x00\x00\x00"   # halt at addr 4
            + bytes(addi(10, 0, 42))
            + bytes(jalr(0, 1, 0))   # return via x1
        )
        state = sim.execute(prog)
        assert state.a0 == 42
        assert state.halted

    def test_jal_x0_plain_jump(self):
        """JAL x0, offset is a plain jump (return address discarded)."""
        state = run([
            jal(0, 8),          # jump over next instruction
            addi(10, 0, 99),    # skipped
            addi(10, 0, 42),    # executed
        ])
        assert state.a0 == 42
        assert state.gpr[0] == 0   # x0 always 0


# ── M Extension — Multiply ────────────────────────────────────────────────────


class TestMultiply:
    def test_mul_basic(self):
        """MUL: 6 × 7 = 42."""
        state = run([addi(10, 0, 6), addi(11, 0, 7), mul(12, 10, 11)])
        assert state.a2 == 42

    def test_mul_overflow_wraps(self):
        """MUL takes lower 64 bits of the 128-bit product."""
        # 2^62 * 4 = 2^64 → lower 64 bits = 0
        state = run([addi(10, 0, 4), slli(10, 10, 60), addi(11, 0, 4), mul(12, 10, 11)])
        assert state.a2 == 0

    def test_mulh_signed(self):
        """MULH: upper 64 bits of signed × signed."""
        # -1 × -1 = +1 (128-bit); upper 64 bits = 0
        state = run([addi(10, 0, -1), addi(11, 0, -1), mulh(12, 10, 11)])
        assert state.a2 == 0

    def test_mulhu_unsigned(self):
        """MULHU: upper 64 bits of unsigned × unsigned."""
        # 0xFFFF...FFFF × 0xFFFF...FFFF = large; upper 64 = 0xFFFF...FFFE
        state = run([addi(10, 0, -1), addi(11, 0, -1), mulhu(12, 10, 11)])
        assert state.a2 == 0xFFFF_FFFF_FFFF_FFFE

    def test_mulhsu(self):
        """MULHSU: upper 64 of signed × unsigned."""
        # 1 × (2^64-1) = 2^64-1; upper 64 = 0
        state = run([addi(10, 0, 1), addi(11, 0, -1), mulhsu(12, 10, 11)])
        assert state.a2 == 0

    def test_mulw(self):
        """MULW: 32-bit multiply, result sign-extended to 64."""
        state = run([addi(10, 0, 3), addi(11, 0, 4), mulw(12, 10, 11)])
        assert state.a2 == 12


# ── M Extension — Divide ──────────────────────────────────────────────────────


class TestDivide:
    def test_div_basic(self):
        """DIV: 42 / 7 = 6."""
        state = run([addi(10, 0, 42), addi(11, 0, 7), div_(12, 10, 11)])
        assert state.a2 == 6

    def test_div_truncates_toward_zero(self):
        """-7 / 2 = -3 (truncated toward zero, not -4)."""
        state = run([addi(10, 0, -7), addi(11, 0, 2), div_(12, 10, 11)])
        assert state.a2 == 0xFFFF_FFFF_FFFF_FFFD   # -3

    def test_div_by_zero_returns_neg_one(self):
        """DIV by zero → -1 (= 0xFFFF...FFFF)."""
        state = run([addi(10, 0, 5), addi(11, 0, 0), div_(12, 10, 11)])
        assert state.a2 == 0xFFFF_FFFF_FFFF_FFFF

    def test_divu_basic(self):
        """DIVU: 10 / 3 = 3 (unsigned)."""
        state = run([addi(10, 0, 10), addi(11, 0, 3), divu(12, 10, 11)])
        assert state.a2 == 3

    def test_divu_by_zero(self):
        """DIVU by zero → MAXUINT."""
        state = run([addi(10, 0, 1), addi(11, 0, 0), divu(12, 10, 11)])
        assert state.a2 == 0xFFFF_FFFF_FFFF_FFFF

    def test_rem_basic(self):
        """REM: 10 % 3 = 1."""
        state = run([addi(10, 0, 10), addi(11, 0, 3), rem(12, 10, 11)])
        assert state.a2 == 1

    def test_rem_negative_dividend(self):
        """-7 % 2 = -1 (sign matches dividend)."""
        state = run([addi(10, 0, -7), addi(11, 0, 2), rem(12, 10, 11)])
        assert state.a2 == 0xFFFF_FFFF_FFFF_FFFF   # -1

    def test_rem_by_zero_returns_dividend(self):
        """REM by zero → dividend."""
        state = run([addi(10, 0, 5), addi(11, 0, 0), rem(12, 10, 11)])
        assert state.a2 == 5

    def test_remu_basic(self):
        """REMU: 10 % 3 = 1."""
        state = run([addi(10, 0, 10), addi(11, 0, 3), remu(12, 10, 11)])
        assert state.a2 == 1

    def test_divw_truncates_to_32(self):
        """DIVW: operates on lower 32 bits."""
        state = run([addi(10, 0, 12), addi(11, 0, 4), divw(12, 10, 11)])
        assert state.a2 == 3

    def test_divuw(self):
        state = run([addi(10, 0, 12), addi(11, 0, 4), divuw(12, 10, 11)])
        assert state.a2 == 3

    def test_remw(self):
        state = run([addi(10, 0, 10), addi(11, 0, 3), remw(12, 10, 11)])
        assert state.a2 == 1

    def test_remuw(self):
        state = run([addi(10, 0, 10), addi(11, 0, 3), remuw(12, 10, 11)])
        assert state.a2 == 1


# ── ECALL / EBREAK halt ───────────────────────────────────────────────────────


class TestSystem:
    def test_ecall_halts(self):
        """ECALL (opcode 0x73, imm=0) causes halt."""
        ecall = [0x73, 0x00, 0x00, 0x00]
        sim = RV64ISimulator()
        sim.load(bytes(ecall))
        state = sim.execute(bytes(ecall))
        assert state.halted

    def test_ebreak_halts(self):
        """EBREAK (opcode 0x73, imm=1) causes halt."""
        ebreak = [0x73, 0x00, 0x10, 0x00]
        sim = RV64ISimulator()
        state = sim.execute(bytes(ebreak))
        assert state.halted


# ── Programs ──────────────────────────────────────────────────────────────────


class TestPrograms:
    def test_fibonacci_6(self):
        """
        Compute fib(6) = 8 using a simple iterative loop.

        Register usage:
          x10 = n (countdown)
          x11 = a (fib(n-2))
          x12 = b (fib(n-1))
          x13 = temp
        """
        sim = RV64ISimulator()
        # x10 = 6 (number of iterations)
        # x11 = 0 (a = fib(0))
        # x12 = 1 (b = fib(1))
        # loop: temp = a + b; a = b; b = temp; n--; bne n, 0, loop
        prog = (
            bytes(addi(10, 0, 6))    # n = 6
            + bytes(addi(11, 0, 0))  # a = 0
            + bytes(addi(12, 0, 1))  # b = 1
            # loop (addr 12):
            + bytes(add(13, 11, 12)) # temp = a + b
            + bytes(addi(11, 12, 0)) # a = b  (ADDI x11, x12, 0)
            + bytes(addi(12, 13, 0)) # b = temp
            + bytes(addi(10, 10, -1))# n--
            + bytes(bne(10, 0, -16)) # if n != 0: back to addr 12
            + b"\x00\x00\x00\x00"
        )
        state = sim.execute(prog)
        # After 6 iterations: a (x11) = fib(6) = 8, b (x12) = fib(7) = 13.
        # The loop body assigns b←(a+b) and a←(old b), so after n steps:
        # x11 = fib(n), x12 = fib(n+1).
        assert state.a1 == 8   # a = fib(6) = 8

    def test_sum_1_to_10(self):
        """Sum integers 1..10 = 55."""
        sim = RV64ISimulator()
        # x10 = 10 (counter), x11 = 0 (sum)
        # loop: sum += counter; counter--; bne counter, 0, loop
        prog = (
            bytes(addi(10, 0, 10))   # counter = 10
            + bytes(addi(11, 0, 0))  # sum = 0
            # loop (addr 8):
            + bytes(add(11, 11, 10)) # sum += counter
            + bytes(addi(10, 10, -1))# counter--
            + bytes(bne(10, 0, -8))  # back to addr 8
            + b"\x00\x00\x00\x00"
        )
        state = sim.execute(prog)
        assert state.a1 == 55
