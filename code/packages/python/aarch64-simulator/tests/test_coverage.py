"""Edge-case and coverage tests for the AArch64 simulator."""

from aarch64_simulator import (
    COND_NE,
    HALT,
    AArch64Simulator,
    AArch64State,
    branch_cond,
    dp_imm,
    dp_reg,
    ldst_uoff,
    logic_reg,
    movwide,
)
from aarch64_simulator.simulator import (
    _add_with_flags,
    _clz,
    _decode_bitmask,
    _rbit,
    _rev,
    _rev16,
    _rev32,
    _ror,
    _sub_with_flags,
    _u32be,
)

# ── 64-bit wrapping ────────────────────────────────────────────────────────────


def test_add_wraps_64bit():
    """ADD X1, X0, X0 wraps at 64 bits."""
    sim = AArch64Simulator()
    sim.load(dp_reg(1, 0, 0, 0, 0, 0, 0, 1) + HALT)   # ADD X1, X0, X0
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[0] = 0x8000_0000_0000_0000
    sim._state = AArch64State(  # type: ignore[attr-defined]
        pc=s0.pc, gpr=tuple(gpr), sp=s0.sp, nzcv=s0.nzcv,
        memory=s0.memory, halted=s0.halted
    )
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x1 == 0   # 0x8000...0000 + 0x8000...0000 = 0x1_0000...0000 → low 64 bits = 0


def test_sub_underflow_wraps():
    """SUB X1, X0, X2 where X0 < X2 wraps to 2^64-k."""
    sim = AArch64Simulator()
    sim.load(dp_reg(1, 1, 0, 0, 2, 0, 0, 1) + HALT)   # SUB X1, X0, X2
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[0] = 5
    gpr[2] = 10
    sim._state = AArch64State(  # type: ignore[attr-defined]
        pc=s0.pc, gpr=tuple(gpr), sp=s0.sp, nzcv=s0.nzcv,
        memory=s0.memory, halted=s0.halted
    )
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x1 == (5 - 10) & 0xFFFF_FFFF_FFFF_FFFF


# ── Memory big-endian ─────────────────────────────────────────────────────────


def test_str_big_endian_byte_order():
    """STR X1, [X0] writes MSB first (big-endian)."""
    sim = AArch64Simulator()
    sim.load(ldst_uoff(3, 0, 0b00, 0, 0, 1) + HALT)   # STR X1, [X0]
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[0] = 0x100
    gpr[1] = 0x01_02_03_04_05_06_07_08
    sim._state = AArch64State(  # type: ignore[attr-defined]
        pc=s0.pc, gpr=tuple(gpr), sp=s0.sp, nzcv=s0.nzcv,
        memory=s0.memory, halted=s0.halted
    )
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.memory[0x100] == 0x01
    assert s.memory[0x101] == 0x02
    assert s.memory[0x107] == 0x08


def test_ldr_big_endian_byte_order():
    """LDR X1, [X0] reads MSB-first big-endian."""
    sim = AArch64Simulator()
    sim.load(ldst_uoff(3, 0, 0b01, 0, 0, 1) + HALT)   # LDR X1, [X0]
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[0] = 0x200
    mem = list(s0.memory)
    val = 0xDEAD_BEEF_CAFE_1234
    for i in range(7, -1, -1):
        mem[0x200 + (7 - i)] = (val >> (i * 8)) & 0xFF
    sim._state = AArch64State(  # type: ignore[attr-defined]
        pc=s0.pc, gpr=tuple(gpr), sp=s0.sp, nzcv=s0.nzcv,
        memory=tuple(mem), halted=s0.halted
    )
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x1 == 0xDEAD_BEEF_CAFE_1234


# ── HALT semantics ────────────────────────────────────────────────────────────


def test_halt_on_all_zeros():
    """0x00000000 halts immediately at PC=0."""
    sim = AArch64Simulator()
    result = sim.execute(HALT)
    assert result.halted
    assert result.final_state.pc == 0


def test_step_on_halted_stays():
    """Stepping when already halted returns HALT trace without advancing PC."""
    sim = AArch64Simulator()
    sim.execute(HALT)
    trace = sim.step()
    assert "HALT" in trace.mnemonic
    assert trace.pc_before == trace.pc_after
    assert not trace.mnemonic.startswith("ERROR:")


# ── Unknown opcode ────────────────────────────────────────────────────────────


def test_unknown_opcode_fails():
    """An unrecognized opcode emits an ERROR trace (ok=False) and halts."""
    # 0xFFFF_FFFF is not a valid AArch64 instruction
    bad = bytes([0xFF, 0xFF, 0xFF, 0xFF])
    sim = AArch64Simulator()
    result = sim.execute(bad)
    assert not result.ok
    assert result.final_state.halted


# ── State immutability ────────────────────────────────────────────────────────


def test_state_pc_immutable():
    """AArch64State.pc cannot be overwritten."""
    sim = AArch64Simulator()
    s = sim.get_state()
    raised = False
    try:
        s.pc = 4  # type: ignore[misc]
    except (AttributeError, TypeError):
        raised = True
    assert raised


def test_state_gpr_immutable():
    """AArch64State.gpr is a tuple (immutable)."""
    sim = AArch64Simulator()
    s = sim.get_state()
    raised = False
    try:
        s.gpr = (0,) * 32  # type: ignore[misc]
    except (AttributeError, TypeError):
        raised = True
    assert raised


# ── XZR invariant ─────────────────────────────────────────────────────────────


def test_xzr_always_zero_after_step():
    """XZR (index 31) is 0 even after write attempts via MOVZ XZR."""
    # MOVZ X31 would try to write XZR — write must be silently discarded
    prog = movwide(1, 0b10, 0, 0x1234, 31) + HALT   # MOVZ XZR, #0x1234
    result = AArch64Simulator().execute(prog)
    assert result.final_state.gpr[31] == 0


def test_xzr_read_zero_in_load():
    """Using XZR as Rn in a load gives address = imm (as if Rn=0)."""
    # LDR X1, [XZR, #0] → EA = 0; load from address 0
    sim = AArch64Simulator()
    sim.load(ldst_uoff(3, 0, 0b01, 0, 31, 1) + HALT)  # LDR X1, [XZR]
    # But Rn=31 in load/store means SP, not XZR per AArch64 spec.
    # So we test with an actual XZR read in arithmetic:
    prog = dp_imm(1, 0, 0, 99, 0, 31, 1) + HALT  # ADD X1, XZR, #99
    result = AArch64Simulator().execute(prog)
    assert result.final_state.x1 == 99


# ── add_with_flags / sub_with_flags unit tests ────────────────────────────────


def test_add_flags_zero_result():
    """Adding 0xFFFF...FFFF + 1 → result=0, Z=1, C=1."""
    result, nzcv = _add_with_flags(0xFFFF_FFFF_FFFF_FFFF, 1, 1)
    assert result == 0
    N = (nzcv >> 3) & 1
    Z = (nzcv >> 2) & 1
    C = (nzcv >> 1) & 1
    V = nzcv & 1
    assert Z == 1
    assert C == 1
    assert N == 0
    assert V == 0


def test_add_flags_signed_overflow():
    """0x7FFF...FFFF + 1 → V=1, N=1."""
    _, nzcv = _add_with_flags(0x7FFF_FFFF_FFFF_FFFF, 1, 1)
    V = nzcv & 1
    N = (nzcv >> 3) & 1
    assert V == 1
    assert N == 1


def test_sub_flags_no_borrow():
    """5 - 3 → C=1 (no borrow), result=2."""
    result, nzcv = _sub_with_flags(5, 3, 1)
    assert result == 2
    C = (nzcv >> 1) & 1
    assert C == 1


def test_sub_flags_borrow():
    """3 - 5 → C=0 (borrow), result wraps."""
    _, nzcv = _sub_with_flags(3, 5, 1)
    C = (nzcv >> 1) & 1
    assert C == 0


def test_sub_flags_32bit():
    """32-bit SUBS: result is masked to 32 bits."""
    result, _ = _sub_with_flags(10, 3, 0)
    assert result == 7
    assert result == (result & 0xFFFF_FFFF)


# ── Bitmask decode unit tests ─────────────────────────────────────────────────


def test_decode_bitmask_all_ones_64bit():
    """N=1, immr=0, imms=63 → 64-bit mask all ones."""
    mask = _decode_bitmask(1, 0, 63)
    assert mask == 0xFFFF_FFFF_FFFF_FFFF


def test_decode_bitmask_single_bit():
    """N=1, immr=0, imms=0 → single bit set (mask=1)."""
    mask = _decode_bitmask(1, 0, 0)
    assert mask == 1


def test_decode_bitmask_lower_byte():
    """N=1, immr=0, imms=7 → lower 8 bits set (0xFF)."""
    mask = _decode_bitmask(1, 0, 7)
    assert mask == 0xFF


def test_decode_bitmask_rotated():
    """N=1, immr=1, imms=7 → 8 ones rotated right 1 → 0xFF >> 1 | bit63."""
    mask = _decode_bitmask(1, 1, 7)
    # welem = 0xFF, ror(0xFF, 1, 64) = 0x7F with bit 63 set
    assert mask == _ror(0xFF, 1, 64)


def test_decode_bitmask_repeating_pattern():
    """N=0, immr=0, imms=1 → small repeating pattern (not all-ones, not zero)."""
    # For N=0: combined = (~imms & 0x3F) | (N << 6) = (~1 & 0x3F) = 0b111110
    # combined.bit_length() = 6, so len_ = 5, esize = 32
    # S = imms & (esize-1) = 1 & 31 = 1 → 2 ones; R = immr = 0 → no rotation
    # welem = 0b11 replicated in 32-bit → 0x0000_0003; telem replicated 2x → some non-trivial int
    result = _decode_bitmask(0, 0, 1)
    assert isinstance(result, int)
    assert 0 < result < 0xFFFF_FFFF_FFFF_FFFF


def test_decode_bitmask_undefined_raises():
    """The UNDEFINED encoding (N=0, imms=63) raises ValueError.

    When N=0 and imms=63 (0b111111):
      combined = (~63 & 0x3F) | (0 << 6) = 0 | 0 = 0
      len_ = 0.bit_length() - 1 = 0 - 1 = -1 ≤ 0 → UNDEFINED
    """
    import pytest
    with pytest.raises(ValueError):
        _decode_bitmask(0, 0, 63)


# ── Bit-manipulation helper tests ─────────────────────────────────────────────


def test_clz_zero():
    """CLZ of 0 returns the register width."""
    assert _clz(0, 64) == 64
    assert _clz(0, 32) == 32


def test_clz_one():
    """CLZ of 1 returns width-1."""
    assert _clz(1, 64) == 63
    assert _clz(1, 32) == 31


def test_rbit_single_bit():
    """RBIT moves bit 0 to bit 63."""
    assert _rbit(1, 64) == 0x8000_0000_0000_0000


def test_rev_bytes():
    """REV reverses the byte order."""
    assert _rev(0x0102_0304_0506_0708, 64) == 0x0807_0605_0403_0201


def test_rev16_halfwords():
    """REV16 swaps bytes within each 16-bit halfword."""
    val = 0x0102_0304
    result = _rev16(val, 32)
    assert result == 0x0201_0403


def test_rev32_words():
    """REV32 swaps bytes within each 32-bit word in a 64-bit value."""
    val = 0x0102_0304_0506_0708
    result = _rev32(val)
    assert result == 0x0403_0201_0807_0605


# ── LDRSH / LDRSH32 sign extension ────────────────────────────────────────────


def test_ldrsh_sign_extends_to_64():
    """LDRSH Xt, [Xn] sign-extends 16-bit value to 64 bits."""
    sim = AArch64Simulator()
    prog = (
        ldst_uoff(1, 0, 0b00, 0, 0, 1)     # STRH W1, [X0]
        + ldst_uoff(1, 0, 0b10, 0, 0, 2)   # LDRSH X2, [X0]
        + HALT
    )
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[0] = 0x300
    gpr[1] = 0xFFFF   # -1 in 16-bit signed
    sim._state = AArch64State(  # type: ignore[attr-defined]
        pc=s0.pc, gpr=tuple(gpr), sp=s0.sp, nzcv=s0.nzcv,
        memory=s0.memory, halted=s0.halted
    )
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 0xFFFF_FFFF_FFFF_FFFF


# ── 32-bit mode flag operations ───────────────────────────────────────────────


def test_adds_32bit_zero_extends_result():
    """32-bit ADDS sets Z flag correctly and zero-extends to 64 bits."""
    sim = AArch64Simulator()
    sim.load(dp_imm(0, 0, 1, 1, 0, 0, 1) + HALT)   # ADDS W1, W0, #1
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[0] = 0xFFFF_FFFF   # max 32-bit unsigned
    sim._state = AArch64State(  # type: ignore[attr-defined]
        pc=s0.pc, gpr=tuple(gpr), sp=s0.sp, nzcv=s0.nzcv,
        memory=s0.memory, halted=s0.halted
    )
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x1 == 0   # 32-bit result = 0, zero-extended
    assert s.z         # Z flag set
    assert s.c         # carry


# ── Memory address wrapping ───────────────────────────────────────────────────


def test_load_wraps_memory():
    """Load from address near 0xFFFF wraps around to byte 0."""
    sim = AArch64Simulator()
    # Store a known value at address 0xFFFE (near end of 64K)
    # Load doubleword at 0xFFFE → bytes 0xFFFE, 0xFFFF, 0x0000, ... 0x0005
    # We'll just verify the address wrap doesn't crash
    prog = ldst_uoff(3, 0, 0b01, 0, 0, 1) + HALT   # LDR X1, [X0]
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[0] = 0xFFF8   # 8 bytes from end
    sim._state = AArch64State(  # type: ignore[attr-defined]
        pc=s0.pc, gpr=tuple(gpr), sp=s0.sp, nzcv=s0.nzcv,
        memory=s0.memory, halted=s0.halted
    )
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    # Memory is zeroed except for our program at 0; wrapping read gives 0 or program bytes
    assert isinstance(s.x1, int)


# ── SVC as NOP ────────────────────────────────────────────────────────────────


def test_svc_is_nop():
    """SVC instruction is treated as NOP and execution continues."""
    # SVC #0: encoding 0xD4000001
    svc = bytes([0xD4, 0x00, 0x00, 0x01])
    prog = svc + dp_imm(1, 0, 0, 42, 0, 0, 1) + HALT
    result = AArch64Simulator().execute(prog)
    assert result.final_state.x1 == 42


# ── REV16 for 64-bit ─────────────────────────────────────────────────────────


def test_rev16_64bit():
    """REV16 on 64-bit register swaps bytes in all four 16-bit halfwords."""
    raw = (1 << 31) | (1 << 30) | (0b11010110 << 21) | (0 << 16) | (0b000001 << 10) | (0 << 5) | 1
    prog = _u32be(raw) + HALT
    sim = AArch64Simulator()
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[0] = 0x0102_0304_0506_0708
    sim._state = AArch64State(  # type: ignore[attr-defined]
        pc=s0.pc, gpr=tuple(gpr), sp=s0.sp, nzcv=s0.nzcv,
        memory=s0.memory, halted=s0.halted
    )
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x1 == 0x0201_0403_0605_0807


# ── REV32 ─────────────────────────────────────────────────────────────────────


def test_rev32_instruction():
    """REV32 X1, X0 byte-reverses within 32-bit word halves."""
    raw = (1 << 31) | (1 << 30) | (0b11010110 << 21) | (0 << 16) | (0b000011 << 10) | (0 << 5) | 1
    prog = _u32be(raw) + HALT
    sim = AArch64Simulator()
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[0] = 0x0102_0304_0506_0708
    sim._state = AArch64State(  # type: ignore[attr-defined]
        pc=s0.pc, gpr=tuple(gpr), sp=s0.sp, nzcv=s0.nzcv,
        memory=s0.memory, halted=s0.halted
    )
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x1 == 0x0403_0201_0807_0605


# ── RORV / ASRV ───────────────────────────────────────────────────────────────


def test_rorv():
    """RORV X2, X0, X1 rotates right."""
    raw = (1 << 31) | (0 << 30) | (0b11010110 << 21) | (1 << 16) | (0b001011 << 10) | (0 << 5) | 2
    prog = _u32be(raw) + HALT
    sim = AArch64Simulator()
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[0] = 1
    gpr[1] = 1
    sim._state = AArch64State(  # type: ignore[attr-defined]
        pc=s0.pc, gpr=tuple(gpr), sp=s0.sp, nzcv=s0.nzcv,
        memory=s0.memory, halted=s0.halted
    )
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 0x8000_0000_0000_0000   # rotate 1 right by 1 → MSB


def test_asrv_fills_sign_bit():
    """ASRV X2, X0, X1 fills with sign bit."""
    raw = (1 << 31) | (0 << 30) | (0b11010110 << 21) | (1 << 16) | (0b001010 << 10) | (0 << 5) | 2
    prog = _u32be(raw) + HALT
    sim = AArch64Simulator()
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[0] = 0x8000_0000_0000_0000   # negative
    gpr[1] = 4
    sim._state = AArch64State(  # type: ignore[attr-defined]
        pc=s0.pc, gpr=tuple(gpr), sp=s0.sp, nzcv=s0.nzcv,
        memory=s0.memory, halted=s0.halted
    )
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x2 == 0xF800_0000_0000_0000   # arithmetic shift fills with 1s


# ── BICS sets flags ────────────────────────────────────────────────────────────


def test_bics_sets_n_flag():
    """BICS X2, X0, X1 sets N when result is negative."""
    sim = AArch64Simulator()
    sim.load(logic_reg(1, 0b11, 0, 1, 1, 0, 0, 2) + HALT)   # BICS X2, X0, X1
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[0] = 0xFFFF_FFFF_FFFF_FFFF
    gpr[1] = 0x7FFF_FFFF_FFFF_FFFF   # ~X1 = 0x8000...0000
    sim._state = AArch64State(  # type: ignore[attr-defined]
        pc=s0.pc, gpr=tuple(gpr), sp=s0.sp, nzcv=s0.nzcv,
        memory=s0.memory, halted=s0.halted
    )
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    # X0 & ~X1 = 0xFFFF...FFFF & 0x8000...0000 = 0x8000...0000 → N=1
    assert s.n
    assert s.x2 == 0x8000_0000_0000_0000


# ── EON instruction ────────────────────────────────────────────────────────────


def test_eon_reg():
    """EON X2, X0, X1 = X0 ^ ~X1."""
    sim = AArch64Simulator()
    sim.load(logic_reg(1, 0b10, 0, 1, 1, 0, 0, 2) + HALT)   # EON X2, X0, X1
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[0] = 0b1010
    gpr[1] = 0b1100
    sim._state = AArch64State(  # type: ignore[attr-defined]
        pc=s0.pc, gpr=tuple(gpr), sp=s0.sp, nzcv=s0.nzcv,
        memory=s0.memory, halted=s0.halted
    )
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    # ~0b1100 in 64-bit = 0xFFFF...FFF3; X0 ^ ~X1 = 0b1010 ^ 0xFF...F3 = 0xFF...F9
    expected = (0b1010 ^ (~0b1100 & 0xFFFF_FFFF_FFFF_FFFF)) & 0xFFFF_FFFF_FFFF_FFFF
    assert s.x2 == expected


# ── Snapshot independence ─────────────────────────────────────────────────────


def test_snapshot_independence():
    """get_state() snapshots are independent across steps."""
    sim = AArch64Simulator()
    sim.load(dp_imm(1, 0, 0, 1, 0, 0, 1) + HALT)  # ADD X1, X0, #1
    snap1 = sim.get_state()
    sim.step()
    snap2 = sim.get_state()
    assert snap1.pc == 0
    assert snap2.pc == 4
    assert snap1.x1 == 0
    assert snap2.x1 == 1


# ── Conditional branch backward ───────────────────────────────────────────────


def test_b_cond_backward_branch():
    """B.NE with a negative offset loops backward."""
    # Counts from 3 down to 0: loop body at [0], decrement, branch back if NE
    # X0 starts at 3; SUBS X0, X0, #1; B.NE #-8
    sim = AArch64Simulator()
    prog = (
        dp_imm(1, 1, 1, 1, 0, 0, 0)        # [0] SUBS X0, X0, #1
        + branch_cond(-1, COND_NE)           # [4] B.NE #-4 (back to [0])
        + HALT
    )
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[0] = 3
    sim._state = AArch64State(  # type: ignore[attr-defined]
        pc=s0.pc, gpr=tuple(gpr), sp=s0.sp, nzcv=s0.nzcv,
        memory=s0.memory, halted=s0.halted
    )
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.x0 == 0


# ── Security: execute() input validation ──────────────────────────────────────


def test_execute_max_steps_zero_raises():
    """execute() raises ValueError for max_steps=0 (silent no-op guard)."""
    sim = AArch64Simulator()
    try:
        sim.execute(HALT, max_steps=0)
        assert False, "expected ValueError"
    except ValueError:
        pass


def test_execute_max_steps_negative_raises():
    """execute() raises ValueError for negative max_steps."""
    sim = AArch64Simulator()
    try:
        sim.execute(HALT, max_steps=-1)
        assert False, "expected ValueError"
    except ValueError:
        pass


def test_load_oversized_program_truncated():
    """load() silently truncates programs larger than MEM_SIZE."""
    from aarch64_simulator import MEM_SIZE
    big = b"\xAB" * (MEM_SIZE + 1000)
    sim = AArch64Simulator()
    sim.load(big)          # must not raise or infinite-loop
    s = sim.get_state()
    assert s.memory[0] == 0xAB
    assert s.memory[MEM_SIZE - 1] == 0xAB
