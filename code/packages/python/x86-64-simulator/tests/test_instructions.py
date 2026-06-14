"""Tests for individual x86-64 instructions.

Test organisation:
  1. MOV — immediate, register-to-register, memory
  2. PUSH / POP
  3. Arithmetic — ADD, SUB, IMUL, MUL, DIV, IDIV, NEG, INC, DEC
  4. ADC / SBB (carry-chain arithmetic)
  5. Logical — AND, OR, XOR, NOT
  6. CMP / TEST (flag-only ops)
  7. Shifts — SHL, SHR, SAR, ROL, ROR
  8. MOVSX / MOVZX / MOVSXD
  9. LEA
 10. XCHG
 11. CMOVcc / SETcc
 12. BSF / BSR / BT / BSWAP
 13. REP STOSQ
"""

from x86_64_simulator import X86_64Simulator, X86_64State

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def run(prog: list[int]) -> X86_64State:
    sim = X86_64Simulator()
    return sim.execute(bytes(prog + [0xF4]))  # append HLT


def rex(W=0, R=0, X=0, B=0) -> int:
    """Build a REX prefix byte."""
    return 0x40 | (W << 3) | (R << 2) | (X << 1) | B


REXW = rex(W=1)


def imm64(v: int) -> list[int]:
    """Little-endian 8-byte immediate."""
    return [(v >> (8 * i)) & 0xFF for i in range(8)]


def imm32(v: int) -> list[int]:
    """Little-endian 4-byte immediate (unsigned)."""
    v &= 0xFFFF_FFFF
    return [(v >> (8 * i)) & 0xFF for i in range(4)]


def imm8s(v: int) -> int:
    """Sign-extended 8-bit immediate."""
    return v & 0xFF


# ---------------------------------------------------------------------------
# 1. MOV
# ---------------------------------------------------------------------------

class TestMov:
    def test_mov_rax_imm64(self):
        # MOV RAX, 0x0102030405060708
        val = 0x0102030405060708
        prog = [REXW, 0xB8] + imm64(val)
        state = run(prog)
        assert state.rax == val

    def test_mov_rcx_imm64(self):
        # MOV RCX, 999
        prog = [REXW, 0xB9] + imm64(999)  # B8+1 = B9 for RCX (index 1)
        state = run(prog)
        assert state.rcx == 999

    def test_mov_r8_imm64(self):
        # REX.B + (B8+0) → MOV R8, 12345
        prog = [rex(W=1, B=1), 0xB8] + imm64(12345)
        state = run(prog)
        assert state.r8 == 12345

    def test_mov_eax_imm32_zero_extends(self):
        # First set RAX to all-ones, then MOV EAX, 1 (should zero upper 32 bits)
        prog = (
            [REXW, 0xB8] + imm64(0xFFFF_FFFF_FFFF_FFFF)  # MOV RAX, -1
            + [0xB8] + imm32(1)                            # MOV EAX, 1
        )
        state = run(prog)
        assert state.rax == 1  # upper 32 bits zeroed

    def test_mov_reg_to_reg(self):
        # MOV RAX, 5; MOV RCX, RAX
        # MOV RAX, 5: REX.W B8 05 00 00 00 00 00 00 00
        # MOV RCX, RAX: REX.W 8B /r (mod=11, reg=RCX=1, rm=RAX=0 → C8)
        prog = (
            [REXW, 0xB8] + imm64(5)       # MOV RAX, 5
            + [REXW, 0x8B, 0xC8]           # MOV RCX, RAX  (8B /r, mod=11 reg=1 rm=0)
        )
        state = run(prog)
        assert state.rcx == 5

    def test_mov_mem_to_reg(self):
        # Store 42 at address 64, then load it into RAX via [RDI+0].
        # We load the program first so reset() does not wipe our data, then
        # manually set memory[64] and step until halted.
        prog = bytes(
            [REXW, 0xBF] + imm64(64)       # MOV RDI, 64  (B8+7=BF)
            + [REXW, 0x8B, 0x07]            # MOV RAX, [RDI]  (mod=00 reg=0 rm=7)
            + [0xF4]
        )
        sim = X86_64Simulator()
        sim.load(prog)                      # reset + copy program to memory[0..]
        sim._cpu.memory[64] = 42           # set data AFTER load so it isn't wiped
        while not sim._cpu.halted:
            sim.step()
        state = sim.get_state()
        assert state.rax == 42

    def test_mov_reg_to_mem(self):
        # MOV RAX, 77; MOV [RDI], RAX (RDI=0x100)
        sim = X86_64Simulator()
        prog = (
            [REXW, 0xB8] + imm64(77)       # MOV RAX, 77
            + [REXW, 0xBF] + imm64(0x100)  # MOV RDI, 0x100
            + [REXW, 0x89, 0x07]            # MOV [RDI], RAX
            + [0xF4]
        )
        state = sim.execute(bytes(prog))
        addr = 0x100
        stored = sum(state.memory[addr + i] << (8 * i) for i in range(8))
        assert stored == 77

    def test_mov_rm64_imm32_sign_extended(self):
        # C7 /0 with REX.W: MOV RAX, -1 (imm32 = 0xFFFFFFFF sign-extended to 64)
        prog = [REXW, 0xC7, 0xC0] + imm32(0xFFFFFFFF)  # mod=11 reg=0 rm=0 → 0xC0
        state = run(prog)
        assert state.rax == 0xFFFF_FFFF_FFFF_FFFF  # -1 as u64


# ---------------------------------------------------------------------------
# 2. PUSH / POP
# ---------------------------------------------------------------------------

class TestPushPop:
    def test_push_pop_r64(self):
        # MOV RAX, 1234; PUSH RAX; MOV RAX, 0; POP RCX
        prog = (
            [REXW, 0xB8] + imm64(1234)   # MOV RAX, 1234
            + [0x50]                        # PUSH RAX (50+0)
            + [REXW, 0xB8] + imm64(0)     # MOV RAX, 0
            + [0x59]                        # POP RCX  (58+1)
            + [0xF4]
        )
        sim = X86_64Simulator()
        state = sim.execute(bytes(prog))
        assert state.rcx == 1234
        assert state.rax == 0

    def test_push_imm8(self):
        # PUSH 42 (6A 2A); POP RAX
        prog = [0x6A, 42, 0x58, 0xF4]
        state = run(prog[:-1])  # run() appends HLT; strip last
        # Actually, just run it directly:
        sim = X86_64Simulator()
        state = sim.execute(bytes([0x6A, 42, 0x58, 0xF4]))
        assert state.rax == 42

    def test_push_imm32(self):
        # PUSH 0x12345 (68); POP RAX
        sim = X86_64Simulator()
        state = sim.execute(bytes([0x68] + imm32(0x12345) + [0x58, 0xF4]))
        assert state.rax == 0x12345

    def test_push_sign_extends_imm8(self):
        # PUSH -1 as imm8 → should extend to 0xFFFF..FF on stack
        sim = X86_64Simulator()
        state = sim.execute(bytes([0x6A, 0xFF, 0x58, 0xF4]))
        assert state.rax == 0xFFFF_FFFF_FFFF_FFFF

    def test_rsp_adjusted_by_push_pop(self):
        # RSP is 0 before reset.  After reset (which execute→load→reset triggers)
        # RSP = 0xFFF8.  PUSH decrements by 8 → 0xFFF0.  We must read initial_rsp
        # after a reset so both sides of the comparison agree.
        sim = X86_64Simulator()
        sim.reset()
        initial_rsp = sim._cpu.gpr[4]   # 0xFFF8 after reset
        state_after_push = sim.execute(bytes([0x50, 0xF4]))   # PUSH RAX
        assert state_after_push.rsp == initial_rsp - 8


# ---------------------------------------------------------------------------
# 3. Arithmetic
# ---------------------------------------------------------------------------

class TestArithmetic:
    def test_add_rax_imm8(self):
        # MOV RAX, 10; ADD RAX, 5
        prog = (
            [REXW, 0xB8] + imm64(10)
            + [REXW, 0x83, 0xC0, 5]   # ADD RAX, 5 (83 /0 ib; mod=11 reg=0 rm=0 → C0)
        )
        state = run(prog)
        assert state.rax == 15

    def test_add_sets_zf_and_cf(self):
        # ADD 0xFFFFFFFFFFFFFFFF + 1 → 0, CF=1 ZF=1
        prog = (
            [REXW, 0xB8] + imm64(0xFFFF_FFFF_FFFF_FFFF)
            + [REXW, 0x83, 0xC0, 1]   # ADD RAX, 1
        )
        state = run(prog)
        assert state.rax == 0
        assert state.cf
        assert state.zf

    def test_add_sets_of_signed(self):
        # 0x7FFFFFFFFFFFFFFF + 1 overflows signed 64-bit
        prog = (
            [REXW, 0xB8] + imm64(0x7FFF_FFFF_FFFF_FFFF)
            + [REXW, 0x83, 0xC0, 1]
        )
        state = run(prog)
        assert state.of

    def test_sub_basic(self):
        prog = (
            [REXW, 0xB8] + imm64(100)
            + [REXW, 0x83, 0xE8, 30]   # SUB RAX, 30 (83 /5 ib; mod=11 reg=5 rm=0 → E8)
        )
        state = run(prog)
        assert state.rax == 70

    def test_sub_sets_cf_on_borrow(self):
        # 5 - 10 → CF=1
        prog = (
            [REXW, 0xB8] + imm64(5)
            + [REXW, 0x83, 0xE8, 10]
        )
        state = run(prog)
        assert state.cf
        assert state.rax == (5 - 10) & 0xFFFF_FFFF_FFFF_FFFF

    def test_neg(self):
        # MOV RAX, 7; NEG RAX
        prog = (
            [REXW, 0xB8] + imm64(7)
            + [REXW, 0xF7, 0xD8]   # NEG RAX (F7 /3; mod=11 reg=3 rm=0 → D8)
        )
        state = run(prog)
        assert state.rax == (-7) & 0xFFFF_FFFF_FFFF_FFFF

    def test_imul_two_operand(self):
        # MOV RAX, 6; MOV RCX, 7; IMUL RCX, RAX (0F AF /r)
        prog = (
            [REXW, 0xB8] + imm64(6)
            + [REXW, 0xB9] + imm64(7)
            + [REXW, 0x0F, 0xAF, 0xC8]  # IMUL RCX, RAX (mod=11 reg=1 rm=0 → C8)
        )
        state = run(prog)
        assert state.rcx == 42

    def test_imul_three_operand_imm8(self):
        # MOV RAX, 5; IMUL RCX, RAX, 4  (6B /r ib)
        # 6B mod=11 reg=RCX=1 rm=RAX=0 → CB, imm=4
        prog = (
            [REXW, 0xB8] + imm64(5)
            + [REXW, 0x6B, 0xC8, 4]   # IMUL RCX, RAX, 4
        )
        state = run(prog)
        assert state.rcx == 20

    def test_mul_unsigned(self):
        # 64×64 → 128-bit unsigned multiply via MUL.
        # RAX = 2^63 = 0x8000_0000_0000_0000; RCX = 2
        # product = 2^64 → low 64 bits = 0, high 64 bits = 1
        # So RDX = 1, RAX = 0
        prog = (
            [REXW, 0xB8] + imm64(0x8000_0000_0000_0000)
            + [REXW, 0xB9] + imm64(2)
            + [REXW, 0xF7, 0xE1]   # MUL RCX (F7 /4; mod=11 reg=4 rm=1 → E1)
        )
        state = run(prog)
        assert state.rdx == 1
        assert state.rax == 0

    def test_div_unsigned(self):
        # RAX=100, RDX=0; DIV RCX (RCX=7) → RAX=14, RDX=2
        prog = (
            [REXW, 0xB8] + imm64(100)
            + [REXW, 0xB9] + imm64(7)
            + [REXW, 0x31, 0xD2]         # XOR RDX, RDX  (clear RDX)
            + [REXW, 0xF7, 0xF1]         # DIV RCX (F7 /6; mod=11 reg=6 rm=1 → F1)
        )
        state = run(prog)
        assert state.rax == 14
        assert state.rdx == 2

    def test_inc_dec(self):
        prog = (
            [REXW, 0xB8] + imm64(10)
            + [REXW, 0xFF, 0xC0]         # INC RAX  (FF /0; mod=11 reg=0 rm=0 → C0)
            + [REXW, 0xFF, 0xC8]         # DEC RAX  (FF /1; mod=11 reg=1 rm=0 → C8)
            + [REXW, 0xFF, 0xC8]         # DEC RAX
        )
        state = run(prog)
        assert state.rax == 9

    def test_inc_does_not_affect_cf(self):
        # Set CF via SUB; INC should not clear it
        prog = (
            [REXW, 0xB8] + imm64(0)
            + [REXW, 0x83, 0xE8, 1]      # SUB RAX, 1  → CF=1
            + [REXW, 0xFF, 0xC0]         # INC RAX
        )
        state = run(prog)
        assert state.cf  # CF preserved by INC


# ---------------------------------------------------------------------------
# 4. ADC / SBB
# ---------------------------------------------------------------------------

class TestAdcSbb:
    def test_adc_adds_carry(self):
        # Set CF=1 via SUB 0-1; ADC RAX, 0 → RAX = 0 + 0 + 1 = 1
        prog = (
            [REXW, 0xB8] + imm64(0)         # MOV RAX, 0
            + [REXW, 0xB9] + imm64(0)       # MOV RCX, 0
            + [REXW, 0x83, 0xE9, 1]          # SUB RCX, 1 → CF=1
            + [REXW, 0x83, 0xD0, 0]          # ADC RAX, 0  (83 /2; mod=11 reg=2 rm=0 → D0)
        )
        state = run(prog)
        assert state.rax == 1
        assert not state.cf  # carry consumed

    def test_sbb_subtracts_carry(self):
        # SBB RAX, 0 when CF=1 → RAX decremented by 1
        prog = (
            [REXW, 0xB8] + imm64(5)
            + [REXW, 0xB9] + imm64(0)
            + [REXW, 0x83, 0xE9, 1]          # SUB RCX, 1 → CF=1
            + [REXW, 0x83, 0xD8, 0]          # SBB RAX, 0  (83 /3; mod=11 reg=3 rm=0 → D8)
        )
        state = run(prog)
        assert state.rax == 4


# ---------------------------------------------------------------------------
# 5. Logical
# ---------------------------------------------------------------------------

class TestLogical:
    def test_and(self):
        prog = (
            [REXW, 0xB8] + imm64(0xFF00)
            + [REXW, 0x83, 0xE0, 0x0F]   # AND RAX, 0x0F (83 /4; mod=11 reg=4 rm=0 → E0)
        )
        state = run(prog)
        assert state.rax == 0

    def test_or(self):
        prog = (
            [REXW, 0xB8] + imm64(0xF0)
            + [REXW, 0x83, 0xC8, 0x0F]   # OR RAX, 0x0F (83 /1; mod=11 reg=1 rm=0 → C8)
        )
        state = run(prog)
        assert state.rax == 0xFF

    def test_xor_self_zeroes(self):
        prog = (
            [REXW, 0xB8] + imm64(0xDEAD)
            + [REXW, 0x33, 0xC0]          # XOR RAX, RAX (33 /r mod=11 reg=0 rm=0)
        )
        state = run(prog)
        assert state.rax == 0
        assert state.zf

    def test_not(self):
        prog = (
            [REXW, 0xB8] + imm64(0)
            + [REXW, 0xF7, 0xD0]          # NOT RAX (F7 /2; mod=11 reg=2 rm=0 → D0)
        )
        state = run(prog)
        assert state.rax == 0xFFFF_FFFF_FFFF_FFFF

    def test_and_sets_flags(self):
        prog = (
            [REXW, 0xB8] + imm64(0)
            + [REXW, 0x83, 0xE0, 0]       # AND RAX, 0
        )
        state = run(prog)
        assert state.zf
        assert not state.cf
        assert not state.of


# ---------------------------------------------------------------------------
# 6. CMP / TEST
# ---------------------------------------------------------------------------

class TestCmpTest:
    def test_cmp_equal_sets_zf(self):
        prog = (
            [REXW, 0xB8] + imm64(42)
            + [REXW, 0x83, 0xF8, 42]    # CMP RAX, 42 (83 /7; mod=11 reg=7 rm=0 → F8)
        )
        state = run(prog)
        assert state.zf
        assert not state.cf

    def test_cmp_less_than_sets_cf(self):
        prog = (
            [REXW, 0xB8] + imm64(3)
            + [REXW, 0x83, 0xF8, 10]
        )
        state = run(prog)
        assert state.cf  # 3 < 10 unsigned → borrow

    def test_cmp_does_not_modify_rax(self):
        prog = (
            [REXW, 0xB8] + imm64(99)
            + [REXW, 0x83, 0xF8, 1]
        )
        state = run(prog)
        assert state.rax == 99

    def test_test_sets_zf_for_zero_result(self):
        prog = (
            [REXW, 0xB8] + imm64(0)
            + [REXW, 0x85, 0xC0]   # TEST RAX, RAX (85 /r mod=11 reg=0 rm=0)
        )
        state = run(prog)
        assert state.zf


# ---------------------------------------------------------------------------
# 7. Shifts
# ---------------------------------------------------------------------------

class TestShifts:
    def test_shl_by_imm(self):
        # MOV RAX, 1; SHL RAX, 10 (C1 /4 0A)
        prog = (
            [REXW, 0xB8] + imm64(1)
            + [REXW, 0xC1, 0xE0, 10]   # SHL RAX, 10 (C1 /4; mod=11 reg=4 rm=0 → E0)
        )
        state = run(prog)
        assert state.rax == 1 << 10

    def test_shr_by_imm(self):
        prog = (
            [REXW, 0xB8] + imm64(1024)
            + [REXW, 0xC1, 0xE8, 3]    # SHR RAX, 3 (C1 /5; mod=11 reg=5 rm=0 → E8)
        )
        state = run(prog)
        assert state.rax == 128

    def test_sar_preserves_sign(self):
        # SAR -8, 2 → -2 (arithmetic right shift)
        prog = (
            [REXW, 0xB8] + imm64((-8) & 0xFFFF_FFFF_FFFF_FFFF)
            + [REXW, 0xC1, 0xF8, 2]    # SAR RAX, 2 (C1 /7 → F8)
        )
        state = run(prog)
        assert state.rax == (-2) & 0xFFFF_FFFF_FFFF_FFFF

    def test_shl_by_cl(self):
        # MOV RAX, 1; MOV RCX, 5; SHL RAX, CL (D3 /4 E0)
        prog = (
            [REXW, 0xB8] + imm64(1)
            + [REXW, 0xB9] + imm64(5)
            + [REXW, 0xD3, 0xE0]   # SHL RAX, CL (D3 /4; mod=11 reg=4 rm=0 → E0)
        )
        state = run(prog)
        assert state.rax == 32

    def test_rol(self):
        # MOV RAX, 1; ROL RAX, 1  → 2; ROL RAX, 63 → 1
        prog = (
            [REXW, 0xB8] + imm64(1)
            + [REXW, 0xC1, 0xC0, 1]   # ROL RAX, 1 (C1 /0; mod=11 reg=0 rm=0 → C0)
        )
        state = run(prog)
        assert state.rax == 2

    def test_ror(self):
        prog = (
            [REXW, 0xB8] + imm64(2)
            + [REXW, 0xC1, 0xC8, 1]   # ROR RAX, 1 (C1 /1; mod=11 reg=1 rm=0 → C8)
        )
        state = run(prog)
        assert state.rax == 1

    def test_shl_sets_cf(self):
        # SHL 0x8000000000000000, 1 → CF=1 (MSB shifted out)
        prog = (
            [REXW, 0xB8] + imm64(0x8000_0000_0000_0000)
            + [REXW, 0xC1, 0xE0, 1]   # SHL RAX, 1
        )
        state = run(prog)
        assert state.cf
        assert state.rax == 0

    def test_shr_zero_count_leaves_flags(self):
        # SHR by 0 should not modify flags
        prog = (
            [REXW, 0xB8] + imm64(5)
            + [REXW, 0x83, 0xF8, 5]    # CMP RAX, 5 → ZF=1
            + [REXW, 0xC1, 0xE8, 0]    # SHR RAX, 0 → no flag change
        )
        state = run(prog)
        assert state.zf   # ZF from CMP preserved


# ---------------------------------------------------------------------------
# 8. MOVSX / MOVZX / MOVSXD
# ---------------------------------------------------------------------------

class TestMovExtend:
    def test_movzx_r64_r8(self):
        # MOVZX RAX, CL  (0F B6; REX.W mod=11 reg=RAX rm=CL)
        # CL = 0xFF → RAX = 0xFF (zero-extended)
        prog = (
            [REXW, 0xB9] + imm64(0xFF)   # MOV RCX, 0xFF (RCX = 0xFF)
            + [REXW, 0x0F, 0xB6, 0xC1]   # MOVZX RAX, CL (mod=11 reg=0 rm=1 → C1)
        )
        state = run(prog)
        assert state.rax == 0xFF

    def test_movsx_r64_r8(self):
        # MOVSX RAX, CL where CL=0xFF → -1 sign-extended
        prog = (
            [REXW, 0xB9] + imm64(0xFF)
            + [REXW, 0x0F, 0xBE, 0xC1]   # MOVSX RAX, CL
        )
        state = run(prog)
        assert state.rax == 0xFFFF_FFFF_FFFF_FFFF

    def test_movsxd_r64_r32(self):
        # MOVSXD RAX, ECX where ECX = 0x80000000 → -0x80000000 sign-extended to 64
        prog = (
            [0xB9] + imm32(0x8000_0000)   # MOV ECX, 0x80000000 (32-bit)
            + [REXW, 0x63, 0xC1]           # MOVSXD RAX, ECX (mod=11 reg=0 rm=1 → C1)
        )
        state = run(prog)
        assert state.rax == 0xFFFF_FFFF_8000_0000


# ---------------------------------------------------------------------------
# 9. LEA
# ---------------------------------------------------------------------------

class TestLea:
    def test_lea_reg_plus_disp8(self):
        # MOV RDI, 100; LEA RAX, [RDI+16]
        prog = (
            [REXW, 0xBF] + imm64(100)
            + [REXW, 0x8D, 0x47, 16]   # LEA RAX, [RDI+16] (8D mod=01 reg=0 rm=7 → 47, disp=16)
        )
        state = run(prog)
        assert state.rax == 116

    def test_lea_sib(self):
        # LEA RAX, [RDI + RSI*2 + 8]
        # MOV RDI, 10; MOV RSI, 5
        # Expected: 10 + 5*2 + 8 = 28
        prog = (
            [REXW, 0xBF] + imm64(10)        # MOV RDI, 10
            + [REXW, 0xBE] + imm64(5)       # MOV RSI, 5
            # LEA RAX, [RDI + RSI*2 + 8]:
            # mod=01 reg=RAX=0 rm=4(SIB) → 0x44
            # SIB: scale=01 index=RSI=6 base=RDI=7 → 0x77
            # disp8 = 8
            + [REXW, 0x8D, 0x44, 0x77, 8]
        )
        state = run(prog)
        assert state.rax == 28


# ---------------------------------------------------------------------------
# 10. XCHG
# ---------------------------------------------------------------------------

class TestXchg:
    def test_xchg_registers(self):
        prog = (
            [REXW, 0xB8] + imm64(1)
            + [REXW, 0xB9] + imm64(2)
            + [REXW, 0x87, 0xC1]   # XCHG RAX, RCX (87 /r mod=11 reg=0 rm=1 → C1)
        )
        state = run(prog)
        assert state.rax == 2
        assert state.rcx == 1


# ---------------------------------------------------------------------------
# 11. CMOVcc / SETcc
# ---------------------------------------------------------------------------

class TestCmovSetcc:
    def test_cmove_taken(self):
        # CMP RAX, RAX → ZF=1; CMOVE RCX, RAX  (RCX should become RAX)
        prog = (
            [REXW, 0xB8] + imm64(99)
            + [REXW, 0xB9] + imm64(0)
            + [REXW, 0x39, 0xC0]            # CMP RAX, RAX (mod=11 reg=0 rm=0)
            + [REXW, 0x0F, 0x44, 0xC8]      # CMOVE RCX, RAX (0F 44; mod=11 reg=1 rm=0 → C8)
        )
        state = run(prog)
        assert state.rcx == 99  # taken

    def test_cmovne_not_taken(self):
        # ZF=1; CMOVNE should NOT copy
        prog = (
            [REXW, 0xB8] + imm64(99)
            + [REXW, 0xB9] + imm64(50)
            + [REXW, 0x39, 0xC0]            # CMP RAX, RAX → ZF=1
            + [REXW, 0x0F, 0x45, 0xC8]      # CMOVNE RCX, RAX (not taken)
        )
        state = run(prog)
        assert state.rcx == 50  # not modified

    def test_setcc_sets_byte(self):
        # CMP RAX, RAX → ZF=1; SETE CL
        prog = (
            [REXW, 0xB8] + imm64(5)
            + [REXW, 0x83, 0xF8, 5]         # CMP RAX, 5 → ZF=1
            + [0x0F, 0x94, 0xC1]             # SETE CL (0F 94; mod=11 rm=1 → C1)
        )
        state = run(prog)
        assert state.rcx & 0xFF == 1

    def test_setne_zero_when_equal(self):
        prog = (
            [REXW, 0xB9] + imm64(0xFF)      # RCX starts non-zero
            + [REXW, 0xB8] + imm64(7)
            + [REXW, 0x83, 0xF8, 7]          # CMP RAX, 7 → ZF=1
            + [0x0F, 0x95, 0xC1]              # SETNE CL
        )
        state = run(prog)
        assert state.rcx & 0xFF == 0


# ---------------------------------------------------------------------------
# 12. BSF / BSR / BT / BSWAP
# ---------------------------------------------------------------------------

class TestBitOps:
    def test_bsf(self):
        # BSF RAX, RCX — RCX=0b1000 → first set bit at index 3
        prog = (
            [REXW, 0xB9] + imm64(0b1000)
            + [REXW, 0x0F, 0xBC, 0xC1]   # BSF RAX, RCX (mod=11 reg=0 rm=1 → C1)
        )
        state = run(prog)
        assert state.rax == 3
        assert not state.zf

    def test_bsf_zero_sets_zf(self):
        prog = (
            [REXW, 0xB9] + imm64(0)
            + [REXW, 0x0F, 0xBC, 0xC1]
        )
        state = run(prog)
        assert state.zf

    def test_bsr(self):
        # BSR RAX, RCX — RCX=0b1010 → highest set bit at index 3
        prog = (
            [REXW, 0xB9] + imm64(0b1010)
            + [REXW, 0x0F, 0xBD, 0xC1]   # BSR RAX, RCX
        )
        state = run(prog)
        assert state.rax == 3

    def test_bt_register(self):
        # BT RAX, RCX — RAX=0b1010, RCX=1 → CF = bit1 of RAX = 1
        prog = (
            [REXW, 0xB8] + imm64(0b1010)
            + [REXW, 0xB9] + imm64(1)
            + [REXW, 0x0F, 0xA3, 0xC8]   # BT RAX, RCX (mod=11 reg=1 rm=0 → C8)
        )
        state = run(prog)
        assert state.cf

    def test_bswap(self):
        # BSWAP RAX where RAX = 0x0102030405060708 → 0x0807060504030201
        prog = [REXW, 0xB8] + imm64(0x0102030405060708) + [REXW, 0x0F, 0xC8]
        # 0F C8 = BSWAP RAX (C8+0=C8, REX.B=0)
        state = run(prog)
        assert state.rax == 0x0807060504030201


# ---------------------------------------------------------------------------
# 13. REP STOSQ
# ---------------------------------------------------------------------------

class TestRepStosq:
    def test_rep_stosq(self):
        # Fill 4 qwords starting at address 0x200 with 0xABCD
        sim = X86_64Simulator()
        prog = (
            [REXW, 0xB8] + imm64(0xABCD)    # MOV RAX, 0xABCD (value to store)
            + [REXW, 0xBF] + imm64(0x200)   # MOV RDI, 0x200  (destination)
            + [REXW, 0xB9] + imm64(4)        # MOV RCX, 4      (repeat count)
            + [0xF3, REXW, 0xAB]             # REP STOSQ
            + [0xF4]
        )
        state = sim.execute(bytes(prog))
        # Each of the 4 qwords at 0x200, 0x208, 0x210, 0x218 should be 0xABCD
        for i in range(4):
            addr = 0x200 + i * 8
            stored = sum(state.memory[addr + j] << (8 * j) for j in range(8))
            assert stored == 0xABCD, f"qword {i} at 0x{addr:X} wrong: {stored:#x}"
        # RCX should be 0 and RDI should be 0x220
        assert state.rcx == 0
        assert state.rdi == 0x220


# ---------------------------------------------------------------------------
# 14. Control flow — CALL / RET / Jcc / LOOP / JRCXZ / JMP
# ---------------------------------------------------------------------------

class TestControlFlow:
    def test_call_and_ret(self):
        """CALL pushes return address and jumps; RET pops and returns."""
        # Layout:
        #   0: E8 0B 00 00 00   CALL +11 → target = 0+5+11 = 16
        #   5-14: MOV RCX, 42  (executed after RET from function)
        #   15: F4              HLT
        #   16-25: MOV RAX, 42 (the "function" body)
        #   26: C3              RET → pops 5, PC ← 5
        prog = bytes([
            0xE8, 0x0B, 0x00, 0x00, 0x00,           # CALL +11 → addr 16
            REXW, 0xB9, 42, 0, 0, 0, 0, 0, 0, 0,   # MOV RCX, 42 (after RET)
            0xF4,                                     # HLT
            REXW, 0xB8, 42, 0, 0, 0, 0, 0, 0, 0,   # MOV RAX, 42 (func body)
            0xC3,                                     # RET → jumps to 5
        ])
        sim = X86_64Simulator()
        state = sim.execute(prog)
        assert state.rax == 42
        assert state.rcx == 42

    def test_ret_imm16(self):
        """RET imm16 pops RIP and adds imm16 to RSP (for callee-cleaned frames)."""
        # CALL → push ret_addr; then at function: RET 0 (C2 00 00)
        # Same as C3 when imm16=0; RSP = RSP_after_pop + 0 = RSP_after_pop
        prog = bytes([
            0xE8, 0x0B, 0x00, 0x00, 0x00,           # CALL +11 → addr 16
            REXW, 0xB9, 99, 0, 0, 0, 0, 0, 0, 0,   # MOV RCX, 99 (after return)
            0xF4,                                     # HLT
            REXW, 0xB8, 55, 0, 0, 0, 0, 0, 0, 0,   # MOV RAX, 55
            0xC2, 0x00, 0x00,                        # RET 0 → pop + add 0 to RSP
        ])
        sim = X86_64Simulator()
        state = sim.execute(prog)
        assert state.rax == 55
        assert state.rcx == 99

    def test_jz_taken_skips_instruction(self):
        """JZ jumps when ZF=1; the skipped MOV RCX,99 leaves RCX=0."""
        # Bytes: 0-9 MOV RAX,10 | 10-13 CMP RAX,10 | 14-15 JZ+10 | 16-25 MOV RCX,99 | 26 HLT
        prog = bytes([
            REXW, 0xB8, 10, 0, 0, 0, 0, 0, 0, 0,   # MOV RAX, 10
            REXW, 0x83, 0xF8, 10,                    # CMP RAX, 10  → ZF=1
            0x74, 10,                                 # JZ +10 → target=26
            REXW, 0xB9, 99, 0, 0, 0, 0, 0, 0, 0,   # MOV RCX, 99 (SKIPPED)
            0xF4,                                     # HLT
        ])
        sim = X86_64Simulator()
        state = sim.execute(prog)
        assert state.rcx == 0   # JZ was taken; MOV RCX never ran

    def test_jnz_not_taken_falls_through(self):
        """JNZ is NOT taken when ZF=1; falls through to MOV RCX, 77."""
        prog = bytes([
            REXW, 0xB8, 5, 0, 0, 0, 0, 0, 0, 0,    # MOV RAX, 5
            REXW, 0x83, 0xF8, 5,                     # CMP RAX, 5 → ZF=1
            0x75, 10,                                 # JNZ +10 → NOT taken
            REXW, 0xB9, 77, 0, 0, 0, 0, 0, 0, 0,   # MOV RCX, 77 (executed)
            0xF4,                                     # HLT
        ])
        sim = X86_64Simulator()
        state = sim.execute(prog)
        assert state.rcx == 77   # JNZ not taken, fell through

    def test_jb_taken_on_carry(self):
        """JB (72 / condition 2 = CF=1) taken after borrow in SUB."""
        # RAX=0; SUB RAX,1 → CF=1; JB taken → skip MOV RCX,99
        prog = bytes([
            REXW, 0xB8, 0, 0, 0, 0, 0, 0, 0, 0,    # MOV RAX, 0
            REXW, 0x83, 0xE8, 1,                     # SUB RAX, 1 → CF=1
            0x72, 10,                                 # JB +10 → taken (skip)
            REXW, 0xB9, 99, 0, 0, 0, 0, 0, 0, 0,   # MOV RCX, 99 (SKIPPED)
            0xF4,                                     # HLT
        ])
        sim = X86_64Simulator()
        state = sim.execute(prog)
        assert state.rcx == 0   # JB was taken

    def test_js_taken_on_negative(self):
        """JS (78 / condition 8 = SF=1) taken when result is negative."""
        # RAX = -1 (0xFFFF…); CMP sets SF=1; JS taken
        prog = bytes([
            REXW, 0xB8, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,  # MOV RAX,-1
            REXW, 0x83, 0xF8, 0,                     # CMP RAX, 0 → SF=1
            0x78, 10,                                 # JS +10 → taken
            REXW, 0xB9, 99, 0, 0, 0, 0, 0, 0, 0,   # MOV RCX, 99 (SKIPPED)
            0xF4,
        ])
        sim = X86_64Simulator()
        state = sim.execute(prog)
        assert state.rcx == 0

    def test_jbe_taken_when_cf_set(self):
        """JBE (76 / condition 6 = CF=1 or ZF=1) taken when CF=1."""
        # RAX=0; SUB RAX,1 → CF=1; JBE taken
        prog = bytes([
            REXW, 0xB8, 0, 0, 0, 0, 0, 0, 0, 0,
            REXW, 0x83, 0xE8, 1,                     # SUB RAX, 1 → CF=1
            0x76, 10,                                 # JBE +10 → taken
            REXW, 0xB9, 99, 0, 0, 0, 0, 0, 0, 0,
            0xF4,
        ])
        sim = X86_64Simulator()
        state = sim.execute(prog)
        assert state.rcx == 0

    def test_jnp_not_taken_on_even_parity(self):
        """JNP (7B / condition 11 = PF=0) not taken when PF=1."""
        # MOV RAX, 0 (PF=1 since popcount(0)=0 which is even)
        # ... actually PF is set on arithmetic ops. Let me use XOR RAX, RAX → ZF=1, PF=1
        prog = bytes([
            REXW, 0xB8, 0, 0, 0, 0, 0, 0, 0, 0,
            REXW, 0x31, 0xC0,                        # XOR RAX, RAX → PF=1
            0x7B, 10,                                 # JNP +10 → NOT taken (PF=1)
            REXW, 0xB9, 55, 0, 0, 0, 0, 0, 0, 0,
            0xF4,
        ])
        sim = X86_64Simulator()
        state = sim.execute(prog)
        assert state.rcx == 55   # JNP not taken

    def test_jl_taken_when_sf_ne_of(self):
        """JL (7C / condition 12 = SF≠OF) taken for signed less-than."""
        # IMUL overflow: RAX=0x7FFF..FF; ADD RAX,1 → OF=1, SF=1 initially, after: SF=1,OF=1
        # Actually easier: just set up SUB that produces SF=1,OF=0 or SF=0,OF=1
        # SUB 0-1: SUB 0,1 → result=MAXU64, SF=1, OF=0 → SF≠OF → JL taken
        prog = bytes([
            REXW, 0xB8, 0, 0, 0, 0, 0, 0, 0, 0,
            REXW, 0x83, 0xE8, 1,                     # SUB RAX, 1 → SF=1, OF=0
            0x7C, 10,                                 # JL +10 → taken
            REXW, 0xB9, 99, 0, 0, 0, 0, 0, 0, 0,
            0xF4,
        ])
        sim = X86_64Simulator()
        state = sim.execute(prog)
        assert state.rcx == 0   # JL taken

    def test_jmp_rel32(self):
        """JMP rel32 (E9) jumps forward past a block."""
        # 0-4: E9 disp32  (5 bytes)
        # 5-14: MOV RCX,99 (skipped)
        # 15: HLT
        disp = 10  # skip MOV RCX (10 bytes): target = 0+5+10 = 15
        prog = bytes([
            0xE9, disp, 0, 0, 0,                     # JMP +10
            REXW, 0xB9, 99, 0, 0, 0, 0, 0, 0, 0,   # MOV RCX, 99 (SKIPPED)
            0xF4,                                     # HLT
        ])
        sim = X86_64Simulator()
        state = sim.execute(prog)
        assert state.rcx == 0

    def test_loop_counts_down(self):
        """LOOP decrements RCX and branches back while RCX ≠ 0."""
        # Bytes: 0-9 MOV RCX,5 | 10-13 ADD RAX,1 | 14-15 LOOP -6 | 16 HLT
        # LOOP at addr 14: disp = 10-(14+2) = -6 = 0xFA
        prog = bytes([
            REXW, 0xB9, 5, 0, 0, 0, 0, 0, 0, 0,    # MOV RCX, 5
            REXW, 0x83, 0xC0, 1,                     # ADD RAX, 1
            0xE2, 0xFA,                               # LOOP -6 → back to ADD
            0xF4,                                     # HLT
        ])
        sim = X86_64Simulator()
        state = sim.execute(prog)
        assert state.rax == 5
        assert state.rcx == 0

    def test_jrcxz_taken_when_rcx_zero(self):
        """JRCXZ (E3) jumps when RCX = 0."""
        # 0-9: MOV RCX,0 | 10-11: JRCXZ+10 | 12-21: MOV RAX,99 (SKIPPED) | 22: HLT
        prog = bytes([
            REXW, 0xB9, 0, 0, 0, 0, 0, 0, 0, 0,    # MOV RCX, 0
            0xE3, 10,                                 # JRCXZ +10 → skip MOV RAX,99
            REXW, 0xB8, 99, 0, 0, 0, 0, 0, 0, 0,   # MOV RAX, 99 (SKIPPED)
            0xF4,                                     # HLT
        ])
        sim = X86_64Simulator()
        state = sim.execute(prog)
        assert state.rax == 0   # JRCXZ was taken


# ---------------------------------------------------------------------------
# 15. Register–register two-operand forms (01/03 ADD, 09/0B OR, 21/23 AND,
#     29/2B SUB, 31/33 XOR, 11/13 ADC, 19/1B SBB)
# These opcodes encode "r, r/m" and "r/m, r" and use a single ModRM byte.
# ---------------------------------------------------------------------------

class TestRegRegOpcodes:
    def test_add_r64_rm64(self):
        """03 /r — ADD r64, r/m64 (source=rm, dest=reg)."""
        prog = (
            [REXW, 0xB8] + imm64(10)
            + [REXW, 0xB9] + imm64(5)
            + [REXW, 0x03, 0xC1]   # ADD RAX, RCX  (03; mod=11 reg=0 rm=1 → C1)
        )
        assert run(prog).rax == 15

    def test_add_rm64_r64(self):
        """01 /r — ADD r/m64, r64 (source=reg, dest=rm)."""
        prog = (
            [REXW, 0xB8] + imm64(10)
            + [REXW, 0xB9] + imm64(5)
            + [REXW, 0x01, 0xC8]   # ADD RAX, RCX  (01; mod=11 reg=1 rm=0 → C8)
        )
        assert run(prog).rax == 15

    def test_sub_r64_rm64(self):
        """2B /r — SUB r64, r/m64."""
        prog = (
            [REXW, 0xB8] + imm64(10)
            + [REXW, 0xB9] + imm64(3)
            + [REXW, 0x2B, 0xC1]   # SUB RAX, RCX
        )
        assert run(prog).rax == 7

    def test_sub_rm64_r64(self):
        """29 /r — SUB r/m64, r64."""
        prog = (
            [REXW, 0xB8] + imm64(10)
            + [REXW, 0xB9] + imm64(3)
            + [REXW, 0x29, 0xC8]   # SUB RAX, RCX (rm=RAX, reg=RCX)
        )
        assert run(prog).rax == 7

    def test_and_r64_rm64(self):
        """23 /r — AND r64, r/m64."""
        prog = (
            [REXW, 0xB8] + imm64(0xFF0F)
            + [REXW, 0xB9] + imm64(0x00FF)
            + [REXW, 0x23, 0xC1]   # AND RAX, RCX
        )
        assert run(prog).rax == 0x000F

    def test_and_rm64_r64(self):
        """21 /r — AND r/m64, r64."""
        prog = (
            [REXW, 0xB8] + imm64(0xFF0F)
            + [REXW, 0xB9] + imm64(0x00FF)
            + [REXW, 0x21, 0xC8]   # AND RAX, RCX (rm=RAX, reg=RCX)
        )
        assert run(prog).rax == 0x000F

    def test_or_r64_rm64(self):
        """0B /r — OR r64, r/m64."""
        prog = (
            [REXW, 0xB8] + imm64(0xF0)
            + [REXW, 0xB9] + imm64(0x0F)
            + [REXW, 0x0B, 0xC1]   # OR RAX, RCX
        )
        assert run(prog).rax == 0xFF

    def test_or_rm64_r64(self):
        """09 /r — OR r/m64, r64."""
        prog = (
            [REXW, 0xB8] + imm64(0xF0)
            + [REXW, 0xB9] + imm64(0x0F)
            + [REXW, 0x09, 0xC8]   # OR RAX, RCX (rm=RAX)
        )
        assert run(prog).rax == 0xFF

    def test_xor_r64_rm64(self):
        """33 /r — XOR r64, r/m64."""
        prog = (
            [REXW, 0xB8] + imm64(0xFF)
            + [REXW, 0xB9] + imm64(0x0F)
            + [REXW, 0x33, 0xC1]   # XOR RAX, RCX
        )
        assert run(prog).rax == 0xF0

    def test_xor_rm64_r64(self):
        """31 /r — XOR r/m64, r64."""
        prog = (
            [REXW, 0xB8] + imm64(0xFF)
            + [REXW, 0xB9] + imm64(0x0F)
            + [REXW, 0x31, 0xC8]   # XOR RAX, RCX (rm=RAX)
        )
        assert run(prog).rax == 0xF0

    def test_adc_r64_rm64(self):
        """13 /r — ADC r64, r/m64 (adds carry flag)."""
        prog = (
            [REXW, 0xB8] + imm64(0)
            + [REXW, 0xB9] + imm64(0)
            + [REXW, 0x83, 0xE9, 1]          # SUB RCX, 1 → CF=1
            + [REXW, 0x13, 0xC1]             # ADC RAX, RCX  (RAX = 0 + MAXU64 + 1)
        )
        # 0 + 0xFFFF…FF + CF=1 = 0x10000…00, lo = 0, CF=1
        state = run(prog)
        assert state.cf   # carry out

    def test_sbb_r64_rm64(self):
        """1B /r — SBB r64, r/m64 (subtracts carry flag)."""
        prog = (
            [REXW, 0xB8] + imm64(10)
            + [REXW, 0xB9] + imm64(0)
            + [REXW, 0x83, 0xE9, 1]          # SUB RCX, 1 → CF=1
            + [REXW, 0x1B, 0xC1]             # SBB RAX, RCX (RAX = 10 - MAXU64 - 1)
        )
        state = run(prog)
        # 10 - MAXU64 - 1 → borrow; CF=1, result wraps
        assert state.cf

    def test_cmp_rm64_r64(self):
        """39 /r — CMP r/m64, r64 (sets flags, discards result)."""
        prog = (
            [REXW, 0xB8] + imm64(7)
            + [REXW, 0xB9] + imm64(7)
            + [REXW, 0x39, 0xC8]   # CMP RAX, RCX (rm=RAX, reg=RCX) → ZF=1
        )
        assert run(prog).zf


# ---------------------------------------------------------------------------
# 16. REP STOSD (32-bit store without REX.W)
# ---------------------------------------------------------------------------

class TestRepStosd:
    def test_rep_stosd(self):
        """REP STOSD fills memory with 32-bit DWORD copies of EAX."""
        sim = X86_64Simulator()
        prog = bytes(
            [REXW, 0xB8] + imm64(0xCAFE)    # MOV RAX, 0xCAFE
            + [REXW, 0xBF] + imm64(0x300)   # MOV RDI, 0x300
            + [REXW, 0xB9] + imm64(3)        # MOV RCX, 3
            + [0xF3, 0xAB]                    # REP STOSD (no REX.W → 32-bit)
            + [0xF4]
        )
        state = sim.execute(prog)
        for i in range(3):
            addr = 0x300 + i * 4
            stored = sum(state.memory[addr + j] << (8 * j) for j in range(4))
            assert stored == 0xCAFE, f"dword {i}: {stored:#x}"
        assert state.rcx == 0
        assert state.rdi == 0x30C  # 0x300 + 3*4
