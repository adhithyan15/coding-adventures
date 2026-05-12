"""End-to-end program tests for the PowerPC 601 simulator.

Each test runs a complete program that solves a real algorithmic task,
verifying that the instruction set works correctly in combination.
"""

from powerpc601_simulator import (
    BI_GT,
    BO_ALWAYS,
    BO_BDNZ,
    BO_TRUE,
    HALT,
    SPR_CTR,
    PowerPC601Simulator,
    PowerPC601State,
    b_form,
    d_form,
    i_form,
    x_form,
    xfx_form,
    xl_form,
    xo_form,
)
from powerpc601_simulator.simulator import (
    PO_ADDI,
    PO_B,
    PO_BC,
    PO_BX,
    PO_LWZ,
    PO_STW,
    PO_X31,
    XO_ADD,
    XO_BCLR,
    XO_CMP,
    XO_MTSPR,
    XO_MULLW,
    XO_OR,
    XO_XOR,
)

# ── Helper ────────────────────────────────────────────────────────────────────


def run(prog: bytes) -> PowerPC601State:
    sim = PowerPC601Simulator()
    result = sim.execute(prog)
    assert result.ok, f"program failed: {result.error}"
    return result.final_state


# ── Program 1: Sum 1 through 10 = 55 ─────────────────────────────────────────
#
# r3 = accumulator (starts at 0)
# r4 = counter (1 to 10)
# CTR = loop count = 10
#
# loop:
#   r3 += r4       (add r3, r3, r4)
#   r4 += 1        (addi r4, r4, 1)
#   bdnz loop      (decrement CTR; branch if CTR != 0)
# HALT


def test_sum_1_to_10():
    """Sum 1+2+…+10 = 55."""
    # Layout (each instruction is 4 bytes):
    # [0]  addi r5, 0, 10     r5 = 10
    # [4]  mtspr CTR, r5      CTR = 10
    # [8]  addi r4, 0, 1      r4 = 1 (addend, increments each iteration)
    # [12] addi r3, 0, 0      r3 = 0 (accumulator)
    # [16] add r3, r3, r4     r3 += r4        ← loop start
    # [20] addi r4, r4, 1     r4++
    # [24] bdnz -8             back to [16]
    # [28] HALT
    prog = (
        d_form(PO_ADDI, 5, 0, 10)                # [0]  r5 = 10
        + xfx_form(PO_X31, 5, SPR_CTR, XO_MTSPR) # [4]  CTR = r5 = 10
        + d_form(PO_ADDI, 4, 0, 1)               # [8]  r4 = 1
        + d_form(PO_ADDI, 3, 0, 0)               # [12] r3 = 0
        + xo_form(PO_X31, 3, 3, 4, 0, XO_ADD)   # [16] loop: r3 += r4
        + d_form(PO_ADDI, 4, 4, 1)               # [20] r4++
        + b_form(PO_BC, BO_BDNZ, 0, -8)          # [24] bdnz to [16]
        + HALT                                     # [28]
    )
    s = run(prog)
    assert s.r3 == 55


# ── Program 2: Factorial 5! = 120 ────────────────────────────────────────────
#
# r3 = result (starts at 1)
# r4 = counter (starts at 5, decrements to 1)
# CTR = 4 (we multiply r3 by 5, 4, 3, 2)
#
# loop:
#   r3 = r3 * r4   (mullw r3, r3, r4)
#   r4 -= 1        (addi r4, r4, -1)
#   bdnz loop
# HALT


def test_factorial_5():
    """5! = 120."""
    prog = (
        d_form(PO_ADDI, 3, 0, 1)                  # r3 = 1
        + d_form(PO_ADDI, 4, 0, 5)                # r4 = 5
        + d_form(PO_ADDI, 5, 0, 4)                # r5 = 4 (loop count)
        + xfx_form(PO_X31, 5, SPR_CTR, XO_MTSPR)  # CTR = 4
        # loop @ [16]:
        + xo_form(PO_X31, 3, 3, 4, 0, XO_MULLW)  # r3 *= r4
        + d_form(PO_ADDI, 4, 4, -1)               # r4--
        + b_form(PO_BC, BO_BDNZ, 0, -8)           # bdnz to [16]
        + HALT
    )
    s = run(prog)
    assert s.r3 == 120


# ── Program 3: Fibonacci F(9) = 34 ───────────────────────────────────────────
#
# Uses registers for state:
# r3 = F(n-2), r4 = F(n-1), r5 = F(n)
# r6 = loop counter (0 to 7 = 8 iterations to go from F(1)=1 to F(9))


def test_fibonacci_f9():
    """F(9) = 34, starting from F(1)=1, F(2)=1."""
    # Layout:
    # [0]  addi r3, 0, 1      r3 = F(1) = 1
    # [4]  addi r4, 0, 1      r4 = F(2) = 1
    # [8]  addi r6, 0, 7      r6 = 7 (need 7 more additions: F(3)…F(9))
    # [12] mtspr CTR, r6      CTR = 7
    # [16] add r5, r3, r4     r5 = r3 + r4   ← loop start
    # [20] or r3, r4, r4      r3 = r4 (mr: move r4 → r3)
    # [24] or r4, r5, r5      r4 = r5 (mr: move r5 → r4)
    # [28] bdnz -12            back to [16]
    # [32] HALT
    prog = (
        d_form(PO_ADDI, 3, 0, 1)                  # [0]  r3 = F(1) = 1
        + d_form(PO_ADDI, 4, 0, 1)                # [4]  r4 = F(2) = 1
        + d_form(PO_ADDI, 6, 0, 7)                # [8]  r6 = 7
        + xfx_form(PO_X31, 6, SPR_CTR, XO_MTSPR)  # [12] CTR = 7
        + xo_form(PO_X31, 5, 3, 4, 0, XO_ADD)    # [16] r5 = r3 + r4
        + x_form(PO_X31, 4, 3, 4, XO_OR)          # [20] r3 = r4  (mr r3, r4)
        + x_form(PO_X31, 5, 4, 5, XO_OR)          # [24] r4 = r5  (mr r4, r5)
        + b_form(PO_BC, BO_BDNZ, 0, -12)           # [28] bdnz to [16]
        + HALT                                      # [32]
    )
    s = run(prog)
    assert s.r4 == 34   # F(9) ends up in r4


# ── Program 4: Subroutine call and return ─────────────────────────────────────
#
# Calls a subroutine that doubles r3, returns via blr.


def test_subroutine_call_return():
    """bl / blr subroutine call and return."""
    # Layout:
    # [0]  addi r3, 0, 21    r3 = 21
    # [4]  bl +12            call subroutine at [16]   (LR = 8)
    # [8]  HALT
    # [12] HALT (padding)
    # [16] add r3, r3, r3   double r3  ← subroutine
    # [20] blr               return to LR
    prog = (
        d_form(PO_ADDI, 3, 0, 21)               # [0]  r3 = 21
        + i_form(PO_B, 12, LK=1)                # [4]  bl +12 → [16]
        + HALT                                   # [8]
        + HALT                                   # [12] padding
        + xo_form(PO_X31, 3, 3, 3, 0, XO_ADD)  # [16] r3 = r3 + r3 = 42
        + xl_form(PO_BX, BO_ALWAYS, 0, 0, XO_BCLR)  # [20] blr
    )
    s = run(prog)
    assert s.r3 == 42   # doubled
    assert s.lr == 8    # LR saved by bl at [4]: CIA+4 = 8


# ── Program 5: Array sum ──────────────────────────────────────────────────────
#
# Sum of array [10, 20, 30, 40, 50] = 150, stored in memory.
# r3 = base address of array
# r4 = accumulator
# r5 = loop count
# Load each word and accumulate.


def test_array_sum():
    """Sum array [10, 20, 30, 40, 50] = 150."""
    base = 0x400   # array starts at address 0x400
    # Build array in memory via stw instructions at the start
    # Array: [10, 20, 30, 40, 50] at 0x400..0x413
    prog = (
        d_form(PO_ADDI, 3, 0, base)               # r3 = base address
        + d_form(PO_ADDI, 4, 0, 0)               # r4 = 0 (accumulator)
        + d_form(PO_ADDI, 5, 0, 5)               # r5 = 5 (loop count)
        + xfx_form(PO_X31, 5, SPR_CTR, XO_MTSPR) # CTR = 5
        # loop @ [16]:
        + d_form(PO_LWZ, 6, 3, 0)               # r6 = MEM[r3]
        + xo_form(PO_X31, 4, 4, 6, 0, XO_ADD)   # r4 += r6
        + d_form(PO_ADDI, 3, 3, 4)              # r3 += 4 (next word)
        + b_form(PO_BC, BO_BDNZ, 0, -12)        # bdnz to [16]
        + HALT
    )
    sim = PowerPC601Simulator()
    sim.load(prog)
    # Write the array into memory at base
    s0 = sim.get_state()
    mem = list(s0.memory)
    for i, v in enumerate([10, 20, 30, 40, 50]):
        addr = base + i * 4
        mem[addr]     = (v >> 24) & 0xFF
        mem[addr + 1] = (v >> 16) & 0xFF
        mem[addr + 2] = (v >> 8)  & 0xFF
        mem[addr + 3] =  v        & 0xFF
    sim._state = PowerPC601State(**{**s0.__dict__, "memory": tuple(mem)})  # type: ignore[arg-type]

    from conftest import run_from_current
    s, err = run_from_current(sim)
    assert err is None
    assert s.r4 == 150


# ── Program 6: Maximum of two values ─────────────────────────────────────────
#
# r3 = max(r3, r4) using cmpw and conditional branch.


def test_max_two_values():
    """Compute max(15, 27) = 27."""
    # [0]  cmpw cr0, r3, r4   compare r3 and r4
    # [4]  bgt  +4             if r3 > r4 skip the move
    # [8]  or r3, r4, r4       r3 = r4  (mr r3, r4)
    # [12] HALT
    prog = (
        x_form(PO_X31, 0, 3, 4, XO_CMP)           # cmpw cr0, r3, r4
        + b_form(PO_BC, BO_TRUE, BI_GT, 8)         # bgt +8 (skip move)
        + x_form(PO_X31, 4, 3, 4, XO_OR)           # r3 = r4
        + HALT
    )
    sim = PowerPC601Simulator()
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[3] = 15; gpr[4] = 27
    sim._state = PowerPC601State(**{**s0.__dict__, "gpr": tuple(gpr)})  # type: ignore[arg-type]
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.r3 == 27


def test_max_two_values_first_wins():
    prog = (
        x_form(PO_X31, 0, 3, 4, XO_CMP)
        + b_form(PO_BC, BO_TRUE, BI_GT, 8)
        + x_form(PO_X31, 4, 3, 4, XO_OR)
        + HALT
    )
    sim = PowerPC601Simulator()
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[3] = 100; gpr[4] = 50
    sim._state = PowerPC601State(**{**s0.__dict__, "gpr": tuple(gpr)})  # type: ignore[arg-type]
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.r3 == 100


# ── Program 7: Memory copy ────────────────────────────────────────────────────
#
# Copy 4 words from source region to destination region.


def test_memory_copy():
    """Copy 4 words from 0x500 to 0x600."""
    src = 0x500; dst = 0x600
    prog = (
        d_form(PO_ADDI, 3, 0, src)               # r3 = src
        + d_form(PO_ADDI, 4, 0, dst)             # r4 = dst
        + d_form(PO_ADDI, 5, 0, 4)               # r5 = 4 (word count)
        + xfx_form(PO_X31, 5, SPR_CTR, XO_MTSPR) # CTR = 4
        # loop @ [16]:
        + d_form(PO_LWZ, 6, 3, 0)               # r6 = MEM[r3]
        + d_form(PO_STW, 6, 4, 0)               # MEM[r4] = r6
        + d_form(PO_ADDI, 3, 3, 4)              # r3 += 4
        + d_form(PO_ADDI, 4, 4, 4)              # r4 += 4
        + b_form(PO_BC, BO_BDNZ, 0, -16)        # bdnz to [16]
        + HALT
    )
    sim = PowerPC601Simulator()
    sim.load(prog)
    s0 = sim.get_state()
    mem = list(s0.memory)
    src_data = [0x1111_1111, 0x2222_2222, 0x3333_3333, 0x4444_4444]
    for i, v in enumerate(src_data):
        addr = src + i * 4
        mem[addr]     = (v >> 24) & 0xFF
        mem[addr + 1] = (v >> 16) & 0xFF
        mem[addr + 2] = (v >> 8)  & 0xFF
        mem[addr + 3] =  v        & 0xFF
    sim._state = PowerPC601State(**{**s0.__dict__, "memory": tuple(mem)})  # type: ignore[arg-type]
    from conftest import run_from_current
    s, err = run_from_current(sim)
    assert err is None
    for i, expected in enumerate(src_data):
        addr = dst + i * 4
        word = (s.memory[addr] << 24 | s.memory[addr + 1] << 16
                | s.memory[addr + 2] << 8 | s.memory[addr + 3])
        assert word == expected, f"word {i}: expected 0x{expected:08X}, got 0x{word:08X}"


# ── Program 8: XOR-based register swap ───────────────────────────────────────
#
# Swap r3 and r4 using three XOR operations (no temp register).


def test_xor_swap():
    """Swap r3 and r4 without a temporary register using XOR."""
    # x_form(opcode, rS, rA, rB, xo): rA = rS ^ rB
    prog = (
        x_form(PO_X31, 3, 3, 4, XO_XOR)   # r3 = r3 ^ r4  (rS=3, rA=3, rB=4)
        + x_form(PO_X31, 4, 4, 3, XO_XOR)  # r4 = r4 ^ r3  (rS=4, rA=4, rB=3) = original r3
        + x_form(PO_X31, 3, 3, 4, XO_XOR)  # r3 = r3 ^ r4  (rS=3, rA=3, rB=4) = original r4
        + HALT
    )
    sim = PowerPC601Simulator()
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[3] = 0xAAAA_AAAA; gpr[4] = 0x5555_5555
    sim._state = PowerPC601State(**{**s0.__dict__, "gpr": tuple(gpr)})  # type: ignore[arg-type]
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.r3 == 0x5555_5555
    assert s.r4 == 0xAAAA_AAAA


# ── Program 9: Count-down with bdz ───────────────────────────────────────────


def test_countdown_with_bdz():
    """Count CTR from 5 down to 0, accumulate total iterations."""
    # r3 = number of times the body executed
    # [0]  addi r3, r3, 1      r3++
    # [4]  bdz +4               if CTR==0 after dec, jump to [8]
    # [8]  b -8                 else loop back
    # [12] HALT
    prog = (
        d_form(PO_ADDI, 3, 3, 1)         # [0] r3++
        + b_form(PO_BC, 12, 0, 8)         # [4] bdz +8 → skip b, go to HALT
        + i_form(PO_B, -8)                # [8] b -8 → back to [0]
        + HALT                             # [12]
    )
    sim = PowerPC601Simulator()
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[3] = 0
    sim._state = PowerPC601State(**{**s0.__dict__, "gpr": tuple(gpr), "ctr": 5})  # type: ignore[arg-type]
    from conftest import run_from_current
    s, err = run_from_current(sim)
    assert err is None
    assert s.r3 == 5


# ── Program 10: 64-bit add (two 32-bit halves) ───────────────────────────────
#
# Demonstrates addc / adde for multi-word arithmetic.
# Computes (r3:r4) + (r5:r6) → (r7:r8)
# where r3/r5 are low words and r4/r6 are high words.


def test_64bit_add():
    """Add two 64-bit numbers using addc/adde."""
    from powerpc601_simulator.simulator import XO_ADDC, XO_ADDE
    # 0x0000_0001_FFFF_FFFE + 0x0000_0001_0000_0002 = 0x0000_0003_0000_0000
    # Low:  0xFFFF_FFFE + 0x0000_0002 = 0x1_0000_0000 → low=0x0000_0000, CA=1
    # High: 0x0000_0001 + 0x0000_0001 + CA=1 = 0x0000_0003
    prog = (
        xo_form(PO_X31, 7, 3, 5, 0, XO_ADDC)   # r7 = r3 + r5 (low), set CA
        + xo_form(PO_X31, 8, 4, 6, 0, XO_ADDE) # r8 = r4 + r6 + CA (high)
        + HALT
    )
    sim = PowerPC601Simulator()
    sim.load(prog)
    s0 = sim.get_state()
    gpr = list(s0.gpr)
    gpr[3] = 0xFFFF_FFFE  # low of first
    gpr[4] = 0x0000_0001  # high of first
    gpr[5] = 0x0000_0002  # low of second
    gpr[6] = 0x0000_0001  # high of second
    sim._state = PowerPC601State(**{**s0.__dict__, "gpr": tuple(gpr)})  # type: ignore[arg-type]
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.r7 == 0x0000_0000  # low result
    assert s.r8 == 0x0000_0003  # high result
