"""Per-instruction correctness tests for the CDC 6600 simulator."""


from cdc6600_simulator import HALT, CDC6600Simulator, long_instr, short_instr
from cdc6600_simulator.simulator import (
    F_BXMR,
    F_BXND,
    F_BXOR,
    F_BXXR,
    F_CMPEQ,
    F_CMPGT,
    F_CMPLT,
    F_IAAM,
    F_IAAP,
    F_IBBM,
    F_IBBP,
    F_IXMB,
    F_IXMUL,
    F_IXPB,
    F_IXXM,
    F_IXXP,
    F_JEQ,
    F_JMP,
    F_JNE,
    F_JSR,
    F_JXN,
    F_JXZ,
    F_LDAI,
    F_LDB,
    F_LDBI,
    F_LDX,
    F_LDXI,
    F_LSHL,
    F_LSHR,
    F_RET,
    F_STB,
    F_STX,
    F_TAX,
    F_TBX,
    F_TXA,
    F_TXB,
)
from cdc6600_simulator.state import MASK18, MASK60

# ── Helpers ────────────────────────────────────────────────────────────────────

def run(prog: bytes) -> "CDC6600State":  # noqa: F821
    sim = CDC6600Simulator()
    result = sim.execute(prog)
    assert result.ok, f"Program failed: {result.error}"
    return result.final_state


def preset(sim: CDC6600Simulator, *, x=None, a=None, b=None) -> None:
    """Manually set register values in a loaded simulator before stepping."""
    s = sim.get_state()
    nx = list(s.x)
    na = list(s.a)
    nb = list(s.b)
    if x:
        for idx, val in x.items():
            nx[idx] = val & MASK60
    if a:
        for idx, val in a.items():
            na[idx] = val & MASK18
    if b:
        for idx, val in b.items():
            if idx != 0:  # B0 stays 0
                nb[idx] = val & MASK18
    from cdc6600_simulator.state import CDC6600State
    sim._state = CDC6600State(
        p=s.p, x=tuple(nx), a=tuple(na), b=tuple(nb),
        memory=s.memory, halted=s.halted,
    )


def run_from_current(sim: CDC6600Simulator, max_steps: int = 1000):
    """
    Run the simulator from its current state WITHOUT calling load() again.

    execute() calls load() internally which would wipe preset register values.
    Use this after preset() to run from the manually-configured state.
    """
    for _ in range(max_steps):
        if sim.get_state().halted:
            break
        trace = sim.step()
        if sim.get_state().halted:
            break
        if trace.mnemonic.startswith("ERROR:"):
            return sim.get_state(), trace.mnemonic   # (state, error)
    return sim.get_state(), None   # (state, error=None)


# ── TXB: Xi = Bj ──────────────────────────────────────────────────────────────

def test_txb_basic():
    # Load B1=99, then TXB X2,B1 → X2 should be 99
    prog = long_instr(F_LDBI, 1, 0, 99) + short_instr(F_TXB, 2, 1, 0) + HALT
    s = run(prog)
    assert s.x2 == 99


def test_txb_zero_extends():
    # B registers are 18-bit; verify zero-extension into 60-bit X
    prog = long_instr(F_LDBI, 1, 0, 0x3FFFF) + short_instr(F_TXB, 3, 1, 0) + HALT
    s = run(prog)
    assert s.x3 == 0x3FFFF
    # MSB of 60-bit word should NOT be set (zero extension, not sign extension)
    assert s.x3 & (1 << 59) == 0


# ── TBX: Bi = Xj[17:0] ────────────────────────────────────────────────────────

def test_tbx_basic():
    # Load X1=42, then TBX B2,X1 → B2=42
    prog = long_instr(F_LDXI, 1, 0, 42) + short_instr(F_TBX, 2, 1, 0) + HALT
    s = run(prog)
    assert s.b2 == 42


def test_tbx_masks_to_18bit():
    # X register has 60-bit value; only lower 18 bits go into B
    sim = CDC6600Simulator()
    # Set X1 = 0xFFF_FFFF_FFFF_FFFF (max 60-bit)
    prog = short_instr(F_TBX, 2, 1, 0) + HALT
    sim.load(prog)
    preset(sim, x={1: MASK60})
    sim.step()  # TBX B2, X1
    s = sim.get_state()
    assert s.b2 == MASK18


# ── TAX: Xi = Aj ──────────────────────────────────────────────────────────────

def test_tax_basic():
    # Load A3=100, then TAX X4,A3 → X4=100
    prog = long_instr(F_LDAI, 3, 0, 100) + short_instr(F_TAX, 4, 3, 0) + HALT
    s = run(prog)
    assert s.x4 == 100


# ── TXA: Ai = Xj[17:0] ────────────────────────────────────────────────────────

def test_txa_basic():
    # Load X2=200, then TXA A5,X2 → A5=200
    prog = long_instr(F_LDXI, 2, 0, 200) + short_instr(F_TXA, 5, 2, 0) + HALT
    s = run(prog)
    assert s.a5 == 200


# ── IXPB: Xi = Xj + Bk ───────────────────────────────────────────────────────

def test_ixpb_basic():
    # X1=10, B2=5 → IXPB X3,X1,B2 → X3=15
    sim = CDC6600Simulator()
    prog = short_instr(F_IXPB, 3, 1, 2) + HALT
    sim.load(prog)
    preset(sim, x={1: 10}, b={2: 5})
    state, _ = run_from_current(sim)
    assert state.x3 == 15


def test_ixpb_wraps_60bit():
    sim = CDC6600Simulator()
    prog = short_instr(F_IXPB, 1, 1, 2) + HALT
    sim.load(prog)
    preset(sim, x={1: MASK60}, b={2: 1})
    state, _ = run_from_current(sim)
    assert state.x1 == 0   # wraps around at 60 bits


# ── IXMB: Xi = Xj - Bk ───────────────────────────────────────────────────────

def test_ixmb_basic():
    sim = CDC6600Simulator()
    prog = short_instr(F_IXMB, 3, 1, 2) + HALT
    sim.load(prog)
    preset(sim, x={1: 20}, b={2: 7})
    state, _ = run_from_current(sim)
    assert state.x3 == 13


# ── IXXP: Xi = Xj + Xk ───────────────────────────────────────────────────────

def test_ixxp_basic():
    sim = CDC6600Simulator()
    prog = short_instr(F_IXXP, 3, 1, 2) + HALT
    sim.load(prog)
    preset(sim, x={1: 100, 2: 200})
    state, _ = run_from_current(sim)
    assert state.x3 == 300


def test_ixxp_wraps_60bit():
    sim = CDC6600Simulator()
    prog = short_instr(F_IXXP, 1, 1, 2) + HALT
    sim.load(prog)
    preset(sim, x={1: MASK60, 2: 1})
    state, _ = run_from_current(sim)
    assert state.x1 == 0


# ── IXXM: Xi = Xj - Xk ───────────────────────────────────────────────────────

def test_ixxm_basic():
    sim = CDC6600Simulator()
    prog = short_instr(F_IXXM, 3, 1, 2) + HALT
    sim.load(prog)
    preset(sim, x={1: 50, 2: 13})
    state, _ = run_from_current(sim)
    assert state.x3 == 37


def test_ixxm_underflow_wraps():
    sim = CDC6600Simulator()
    prog = short_instr(F_IXXM, 3, 1, 2) + HALT
    sim.load(prog)
    preset(sim, x={1: 0, 2: 1})
    state, _ = run_from_current(sim)
    assert state.x3 == MASK60   # 0 - 1 = all-ones in 60-bit


# ── BXND: Xi = Xj & Xk ───────────────────────────────────────────────────────

def test_bxnd_basic():
    sim = CDC6600Simulator()
    prog = short_instr(F_BXND, 3, 1, 2) + HALT
    sim.load(prog)
    preset(sim, x={1: 0xFF0F, 2: 0x0FFF})
    state, _ = run_from_current(sim)
    assert state.x3 == 0x0F0F


# ── BXOR: Xi = Xj | Xk ───────────────────────────────────────────────────────

def test_bxor_basic():
    sim = CDC6600Simulator()
    prog = short_instr(F_BXOR, 3, 1, 2) + HALT
    sim.load(prog)
    preset(sim, x={1: 0xF0F0, 2: 0x0F0F})
    state, _ = run_from_current(sim)
    assert state.x3 == 0xFFFF


# ── BXXR: Xi = Xj ^ Xk ───────────────────────────────────────────────────────

def test_bxxr_basic():
    sim = CDC6600Simulator()
    prog = short_instr(F_BXXR, 3, 1, 2) + HALT
    sim.load(prog)
    preset(sim, x={1: 0xAAAA, 2: 0xAAAA})
    state, _ = run_from_current(sim)
    assert state.x3 == 0   # XOR with itself = 0


def test_bxxr_nonzero():
    sim = CDC6600Simulator()
    prog = short_instr(F_BXXR, 3, 1, 2) + HALT
    sim.load(prog)
    preset(sim, x={1: 0b1010, 2: 0b1100})
    state, _ = run_from_current(sim)
    assert state.x3 == 0b0110


# ── BXMR: Xi = ~Xj ───────────────────────────────────────────────────────────

def test_bxmr_all_zeros():
    sim = CDC6600Simulator()
    prog = short_instr(F_BXMR, 1, 2, 0) + HALT
    sim.load(prog)
    preset(sim, x={2: 0})
    state, _ = run_from_current(sim)
    assert state.x1 == MASK60   # ~0 = all-ones (60 bits)


def test_bxmr_all_ones():
    sim = CDC6600Simulator()
    prog = short_instr(F_BXMR, 1, 2, 0) + HALT
    sim.load(prog)
    preset(sim, x={2: MASK60})
    state, _ = run_from_current(sim)
    assert state.x1 == 0


# ── LSHL / LSHR ───────────────────────────────────────────────────────────────

def test_lshl_basic():
    # X1=1, B2=4 → LSHL X3,X1,B2 → X3=16
    sim = CDC6600Simulator()
    prog = short_instr(F_LSHL, 3, 1, 2) + HALT
    sim.load(prog)
    preset(sim, x={1: 1}, b={2: 4})
    state, _ = run_from_current(sim)
    assert state.x3 == 16


def test_lshr_basic():
    # X1=256, B2=4 → LSHR X3,X1,B2 → X3=16
    sim = CDC6600Simulator()
    prog = short_instr(F_LSHR, 3, 1, 2) + HALT
    sim.load(prog)
    preset(sim, x={1: 256}, b={2: 4})
    state, _ = run_from_current(sim)
    assert state.x3 == 16


def test_lshr_zero_fills():
    # Shift MSB of 60-bit word right — must not sign-extend
    sim = CDC6600Simulator()
    prog = short_instr(F_LSHR, 3, 1, 2) + HALT
    sim.load(prog)
    # X1 = only bit 59 set
    preset(sim, x={1: 1 << 59}, b={2: 1})
    state, _ = run_from_current(sim)
    # After logical right-shift by 1, bit 59 becomes 0 (zero-fill)
    assert state.x3 == (1 << 58)
    assert state.x3 & (1 << 59) == 0


# ── IBBP / IBBM ───────────────────────────────────────────────────────────────

def test_ibbp_basic():
    # B1=10, B2=7 → IBBP B3,B1,B2 → B3=17
    sim = CDC6600Simulator()
    prog = short_instr(F_IBBP, 3, 1, 2) + HALT
    sim.load(prog)
    preset(sim, b={1: 10, 2: 7})
    state, _ = run_from_current(sim)
    assert state.b3 == 17


def test_ibbm_basic():
    sim = CDC6600Simulator()
    prog = short_instr(F_IBBM, 3, 1, 2) + HALT
    sim.load(prog)
    preset(sim, b={1: 20, 2: 5})
    state, _ = run_from_current(sim)
    assert state.b3 == 15


def test_ibbp_wraps_18bit():
    sim = CDC6600Simulator()
    prog = short_instr(F_IBBP, 3, 1, 2) + HALT
    sim.load(prog)
    preset(sim, b={1: MASK18, 2: 1})
    state, _ = run_from_current(sim)
    assert state.b3 == 0   # wraps at 18 bits


# ── IAAP / IAAM ───────────────────────────────────────────────────────────────

def test_iaap_basic():
    # A1=100, B2=50 → IAAP A3,A1,B2 → A3=150
    sim = CDC6600Simulator()
    prog = short_instr(F_IAAP, 3, 1, 2) + HALT
    sim.load(prog)
    preset(sim, a={1: 100}, b={2: 50})
    state, _ = run_from_current(sim)
    assert state.a3 == 150


def test_iaam_basic():
    sim = CDC6600Simulator()
    prog = short_instr(F_IAAM, 3, 1, 2) + HALT
    sim.load(prog)
    preset(sim, a={1: 100}, b={2: 30})
    state, _ = run_from_current(sim)
    assert state.a3 == 70


# ── CMPEQ / CMPLT / CMPGT ────────────────────────────────────────────────────

def test_cmpeq_true():
    sim = CDC6600Simulator()
    prog = short_instr(F_CMPEQ, 1, 2, 3) + HALT
    sim.load(prog)
    preset(sim, x={2: 42, 3: 42})
    state, _ = run_from_current(sim)
    assert state.b1 == 1


def test_cmpeq_false():
    sim = CDC6600Simulator()
    prog = short_instr(F_CMPEQ, 1, 2, 3) + HALT
    sim.load(prog)
    preset(sim, x={2: 42, 3: 43})
    state, _ = run_from_current(sim)
    assert state.b1 == 0


def test_cmplt_true():
    sim = CDC6600Simulator()
    prog = short_instr(F_CMPLT, 1, 2, 3) + HALT
    sim.load(prog)
    # 5 < 10 → B1=1
    preset(sim, x={2: 5, 3: 10})
    state, _ = run_from_current(sim)
    assert state.b1 == 1


def test_cmplt_false():
    sim = CDC6600Simulator()
    prog = short_instr(F_CMPLT, 1, 2, 3) + HALT
    sim.load(prog)
    # 10 < 5 → false → B1=0
    preset(sim, x={2: 10, 3: 5})
    state, _ = run_from_current(sim)
    assert state.b1 == 0


def test_cmpgt_true():
    sim = CDC6600Simulator()
    prog = short_instr(F_CMPGT, 1, 2, 3) + HALT
    sim.load(prog)
    preset(sim, x={2: 99, 3: 1})
    state, _ = run_from_current(sim)
    assert state.b1 == 1


def test_cmpgt_false():
    sim = CDC6600Simulator()
    prog = short_instr(F_CMPGT, 1, 2, 3) + HALT
    sim.load(prog)
    preset(sim, x={2: 1, 3: 99})
    state, _ = run_from_current(sim)
    assert state.b1 == 0


# ── IXMUL ─────────────────────────────────────────────────────────────────────

def test_ixmul_basic():
    sim = CDC6600Simulator()
    prog = short_instr(F_IXMUL, 3, 1, 2) + HALT
    sim.load(prog)
    preset(sim, x={1: 12, 2: 11})
    state, _ = run_from_current(sim)
    assert state.x3 == 132


def test_ixmul_large():
    # Verify lower-60-bit truncation
    sim = CDC6600Simulator()
    prog = short_instr(F_IXMUL, 3, 1, 2) + HALT
    sim.load(prog)
    # 2^30 * 2^30 = 2^60, which truncates to 0 in a 60-bit mask
    preset(sim, x={1: 1 << 30, 2: 1 << 30})
    state, _ = run_from_current(sim)
    # (2^30)*(2^30) = 2^60 → masked to 60 bits → 0
    assert state.x3 == 0


# ── LDXI / LDBI / LDAI ────────────────────────────────────────────────────────

def test_ldxi_basic():
    s = run(long_instr(F_LDXI, 1, 0, 255) + HALT)
    assert s.x1 == 255


def test_ldxi_max():
    s = run(long_instr(F_LDXI, 2, 0, 0x3FFFF) + HALT)
    assert s.x2 == 0x3FFFF


def test_ldbi_basic():
    s = run(long_instr(F_LDBI, 1, 0, 77) + HALT)
    assert s.b1 == 77


def test_ldai_basic():
    s = run(long_instr(F_LDAI, 2, 0, 512) + HALT)
    assert s.a2 == 512


# ── LDX / STX (memory) ────────────────────────────────────────────────────────

def test_stx_ldx_roundtrip():
    # Store X1=12345 at address 100, then load it back into X2
    prog = (
        long_instr(F_LDXI, 1, 0, 12345) +   # X1 = 12345
        long_instr(F_LDAI, 2, 0, 100) +      # A2 = 100  (address)
        long_instr(F_STX, 2, 1, 0) +         # mem[A2+0] = X1
        long_instr(F_LDAI, 3, 0, 100) +      # A3 = 100
        long_instr(F_LDX, 4, 3, 0) +         # X4 = mem[A3+0]
        HALT
    )
    s = run(prog)
    assert s.x4 == 12345


def test_stx_offset():
    # Store X1=99 at address A2+5 = 105
    prog = (
        long_instr(F_LDXI, 1, 0, 99) +
        long_instr(F_LDAI, 2, 0, 100) +
        long_instr(F_STX, 2, 1, 5) +         # mem[A2+5 = 105] = X1
        long_instr(F_LDX, 3, 2, 5) +         # X3 = mem[A2+5]
        HALT
    )
    s = run(prog)
    assert s.x3 == 99


def test_ldx_out_of_bounds():
    # Accessing address >= 4096 should raise ValueError
    from cdc6600_simulator.state import MEMORY_WORDS
    sim = CDC6600Simulator()
    prog = (
        long_instr(F_LDAI, 1, 0, MEMORY_WORDS) +  # A1 = 4096 (out of bounds)
        long_instr(F_LDX, 2, 1, 0) +               # LDX X2, A1+0 → error
        HALT
    )
    result = sim.execute(prog)
    assert not result.ok


# ── LDB / STB ─────────────────────────────────────────────────────────────────

def test_stb_ldb_roundtrip():
    # Store B1=777 to memory, then load into B2
    prog = (
        long_instr(F_LDBI, 1, 0, 777) +      # B1 = 777
        long_instr(F_LDAI, 3, 0, 200) +      # A3 = 200
        long_instr(F_STB, 3, 1, 0) +         # mem[A3+0][17:0] = B1
        long_instr(F_LDB, 2, 3, 0) +         # B2 = mem[A3+0][17:0]
        HALT
    )
    s = run(prog)
    assert s.b2 == 777


# ── JEQ / JNE ─────────────────────────────────────────────────────────────────

def test_jeq_taken():
    # B1=0 → JEQ should branch to skip the LDXI X1,99 and land at LDXI X2,1
    # Layout: (parcel 0-1: JEQ B1==0, target=4) (2-3: LDXI X1,99) (4-5: LDXI X2,1) (6: HALT)
    # Parcels: 0=JEQ hi, 1=JEQ lo, 2=LDXI hi, 3=LDXI lo, 4=LDXI hi, 5=LDXI lo, 6=HALT
    prog = (
        long_instr(F_JEQ, 0, 1, 4) +      # if B1==0: P=4  (skip next long instr)
        long_instr(F_LDXI, 1, 0, 99) +    # skipped: X1=99
        long_instr(F_LDXI, 2, 0, 1) +     # X2=1
        HALT
    )
    s = run(prog)
    assert s.x1 == 0    # X1 was never set (branch taken over LDXI X1,99)
    assert s.x2 == 1


def test_jeq_not_taken():
    # B1=5 (non-zero) → JEQ not taken
    sim = CDC6600Simulator()
    prog = (
        long_instr(F_JEQ, 0, 1, 4) +      # if B1==0: P=4 (NOT taken since B1=5)
        long_instr(F_LDXI, 1, 0, 99) +    # X1=99 (should execute)
        long_instr(F_LDXI, 2, 0, 1) +
        HALT
    )
    sim.load(prog)
    preset(sim, b={1: 5})
    state, _ = run_from_current(sim)
    assert state.x1 == 99


def test_jne_taken():
    # B1=3 (non-zero) → JNE taken, skip LDXI X1,99
    sim = CDC6600Simulator()
    prog = (
        long_instr(F_JNE, 0, 1, 4) +      # if B1!=0: P=4
        long_instr(F_LDXI, 1, 0, 99) +    # skipped
        long_instr(F_LDXI, 2, 0, 7) +     # X2=7
        HALT
    )
    sim.load(prog)
    preset(sim, b={1: 3})
    state, _ = run_from_current(sim)
    assert state.x1 == 0
    assert state.x2 == 7


# ── JXZ / JXN ─────────────────────────────────────────────────────────────────

def test_jxz_taken():
    # X1=0 → JXZ taken
    prog = (
        long_instr(F_JXZ, 0, 1, 4) +      # if X1==0: P=4
        long_instr(F_LDXI, 2, 0, 55) +    # skipped: X2=55
        long_instr(F_LDXI, 3, 0, 11) +    # X3=11
        HALT
    )
    s = run(prog)
    assert s.x2 == 0    # skipped
    assert s.x3 == 11


def test_jxn_taken():
    sim = CDC6600Simulator()
    prog = (
        long_instr(F_JXN, 0, 1, 4) +      # if X1!=0: P=4
        long_instr(F_LDXI, 2, 0, 55) +    # skipped
        long_instr(F_LDXI, 3, 0, 11) +
        HALT
    )
    sim.load(prog)
    preset(sim, x={1: 42})
    state, _ = run_from_current(sim)
    assert state.x2 == 0    # skipped
    assert state.x3 == 11


# ── JMP (unconditional) ───────────────────────────────────────────────────────

def test_jmp_basic():
    # JMP to parcel 4, skipping LDXI X1,99 at parcels 2-3
    prog = (
        long_instr(F_JMP, 0, 0, 4) +      # JMP P=4
        long_instr(F_LDXI, 1, 0, 99) +    # skipped
        long_instr(F_LDXI, 2, 0, 7) +     # X2=7
        HALT
    )
    s = run(prog)
    assert s.x1 == 0
    assert s.x2 == 7


# ── JSR / RET ─────────────────────────────────────────────────────────────────

def test_jsr_saves_return_address():
    # JSR to parcel 4 should save return parcel address (2) in B7.
    # Layout:
    #   P=0,1: JSR target=4  (2 parcels; saves B7=2)
    #   P=2:   HALT           (not reached by JSR)
    #   P=3:   HALT           (padding to align subroutine at P=4)
    #   P=4,5: LDXI X1,42    (subroutine body)
    #   P=6:   HALT
    prog = (
        long_instr(F_JSR, 0, 0, 4) +      # P=0,1: B7=2; P=4
        HALT +                             # P=2: not reached
        HALT +                             # P=3: padding
        long_instr(F_LDXI, 1, 0, 42) +    # P=4,5: X1=42
        HALT                               # P=6
    )
    s = run(prog)
    assert s.x1 == 42
    assert s.b7 == 2   # return address = parcel after JSR (P=2)


def test_ret_returns_to_caller():
    # Simple call/return: JSR to subroutine, RET back, check result
    # Layout:
    #   P=0,1: JSR to P=6 (subroutine start)   [saves B7=2]
    #   P=2,3: LDXI X2,100  (after return)
    #   P=4:   HALT
    #   P=5:   padding (HALT)
    #   P=6,7: LDXI X1,42   (subroutine body)
    #   P=8,9: RET B7        (return)
    prog = (
        long_instr(F_JSR, 0, 0, 6) +      # P=0: call subroutine at P=6
        long_instr(F_LDXI, 2, 0, 100) +   # P=2: X2=100 (runs after return)
        HALT +                             # P=4
        HALT +                             # P=5 (padding)
        long_instr(F_LDXI, 1, 0, 42) +    # P=6: subroutine: X1=42
        long_instr(F_RET, 0, 7, 0) +      # P=8: RET P=B7
        HALT
    )
    s = run(prog)
    assert s.x1 == 42    # subroutine ran
    assert s.x2 == 100   # caller continued after return


# ── Unknown opcode ────────────────────────────────────────────────────────────

def test_unknown_short_opcode():
    # Opcode 63 (0x3F) is not defined — should error
    prog = short_instr(63, 0, 0, 0) + HALT
    sim = CDC6600Simulator()
    result = sim.execute(prog)
    assert not result.ok


def test_unknown_long_opcode():
    # Opcode 63 (in long range >= 32, but 63 is not defined)
    prog = long_instr(63, 0, 0, 0) + HALT
    sim = CDC6600Simulator()
    result = sim.execute(prog)
    assert not result.ok
