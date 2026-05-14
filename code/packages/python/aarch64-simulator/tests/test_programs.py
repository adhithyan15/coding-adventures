"""Integration tests: small programs that exercise multiple instructions."""

from aarch64_simulator import (
    COND_EQ,
    COND_NE,
    HALT,
    AArch64Simulator,
    AArch64State,
    branch_cond,
    branch_imm,
    branch_reg,
    cbz_cbnz,
    csel_enc,
    dp_imm,
    dp_reg,
    ldst_uoff,
    logic_reg,
    madd_msub,
    movwide,
)


def run(prog: bytes, x_regs: dict[int, int] | None = None) -> AArch64State:
    """
    Execute a program with optional register preset and return final state.

    x_regs: maps register index → initial value (e.g. {0: 10, 1: 5}).
    """
    sim = AArch64Simulator()
    sim.load(prog)
    if x_regs:
        s = sim.get_state()
        gpr = list(s.gpr)
        for idx, val in x_regs.items():
            gpr[idx] = val & 0xFFFF_FFFF_FFFF_FFFF
        sim._state = AArch64State(  # type: ignore[attr-defined]
            pc=s.pc, gpr=tuple(gpr), sp=s.sp, nzcv=s.nzcv,
            memory=s.memory, halted=s.halted
        )
    from conftest import run_from_current
    state, _ = run_from_current(sim)
    return state


# ── Sum 1 to N ────────────────────────────────────────────────────────────────


def test_sum_1_to_10():
    """
    Sum integers 1..10 using a CBNZ countdown loop.

    Loop structure:
        X0 = 10  (counter, counts down)
        X1 = 0   (accumulator)
        X2 = 0   (current value being added, increments up)

    Loop body:
        ADD X2, X2, #1   X2 increments from 1 to 10
        ADD X1, X1, X2   accumulate
        SUBS X0, X0, #1  decrement counter, set flags
        B.NE loop        continue if not zero

    Expected: X1 = 1+2+3+...+10 = 55
    """
    prog = (
        movwide(1, 0b10, 0, 10, 0)             # [0]  X0 = 10  (counter)
        + movwide(1, 0b10, 0, 0, 1)            # [4]  X1 = 0   (sum)
        + movwide(1, 0b10, 0, 0, 2)            # [8]  X2 = 0   (current value)
        # loop starts at [12]
        + dp_imm(1, 0, 0, 1, 0, 2, 2)         # [12] ADD X2, X2, #1
        + dp_reg(1, 0, 0, 0, 2, 0, 1, 1)      # [16] ADD X1, X1, X2
        + dp_imm(1, 1, 1, 1, 0, 0, 0)         # [20] SUBS X0, X0, #1
        + branch_cond(-3, COND_NE)             # [24] B.NE #-12 → [12]
        + HALT
    )
    s = run(prog)
    assert s.x1 == 55


# ── Fibonacci ─────────────────────────────────────────────────────────────────


def test_fibonacci_f10():
    """
    Compute the 10th Fibonacci number (F(10)=55, 0-indexed: F(0)=0, F(1)=1, ...).

    Uses the standard two-register iteration:
        a, b = 0, 1
        for _ in range(n):
            a, b = b, a+b
        return a

    Registers:
        X0 = counter (n = 10 iterations)
        X1 = a
        X2 = b
        X3 = temp (a+b)

    Expected: X1 = 55 (F(10))
    """
    prog = (
        movwide(1, 0b10, 0, 10, 0)             # [0]  X0 = 10
        + movwide(1, 0b10, 0, 0, 1)            # [4]  X1 = 0 (a)
        + movwide(1, 0b10, 0, 1, 2)            # [8]  X2 = 1 (b)
        # loop at [12]
        + dp_reg(1, 0, 0, 0, 2, 0, 1, 3)      # [12] ADD X3, X1, X2 (temp = a+b)
        + logic_reg(1, 0b01, 0, 0, 2, 0, 31, 1)  # [16] MOV X1, X2  (a = b)
        + logic_reg(1, 0b01, 0, 0, 3, 0, 31, 2)  # [20] MOV X2, X3  (b = temp)
        + dp_imm(1, 1, 1, 1, 0, 0, 0)         # [24] SUBS X0, X0, #1
        + branch_cond(-4, COND_NE)             # [28] B.NE #-16 → [12]
        + HALT
    )
    s = run(prog)
    assert s.x1 == 55


# ── XOR swap ──────────────────────────────────────────────────────────────────


def test_xor_swap():
    """
    XOR swap: exchange two registers without a temporary.

    The XOR swap algorithm:
        a ^= b   →  a = a XOR b
        b ^= a   →  b = b XOR (a XOR b) = original a
        a ^= b   →  a = (a XOR b) XOR a = original b

    Starting: X0=42, X1=99
    Expected: X0=99, X1=42
    """
    prog = (
        logic_reg(1, 0b10, 0, 0, 1, 0, 0, 0)   # X0 = X0 ^ X1
        + logic_reg(1, 0b10, 0, 0, 0, 0, 1, 1)  # X1 = X1 ^ X0
        + logic_reg(1, 0b10, 0, 0, 1, 0, 0, 0)  # X0 = X0 ^ X1
        + HALT
    )
    s = run(prog, {0: 42, 1: 99})
    assert s.x0 == 99
    assert s.x1 == 42


# ── GCD (Euclidean algorithm) ─────────────────────────────────────────────────


def test_gcd():
    """
    Greatest Common Divisor via the Euclidean algorithm.

    Algorithm (iterative):
        while b != 0:
            a, b = b, a mod b
        return a

    We use UDIV + MSUB to compute a mod b:
        q = a / b
        r = a - q * b  ← this is MSUB: r = a - q*b

    Registers:
        X0 = a
        X1 = b
        X2 = quotient
        X3 = remainder
    """
    from aarch64_simulator.simulator import _u32be
    # UDIV X2, X0, X1
    udiv = _u32be(
        (1 << 31) | (0b11010110 << 21) | (1 << 16) | (0b000010 << 10) | (0 << 5) | 2
    )
    prog = (
        # loop at [0]
        udiv                                    # [0]  X2 = X0 / X1
        + madd_msub(1, 0, 1, 1, 0, 2, 3)      # [4]  X3 = X0 - X2*X1 (MSUB X3, X2, X1, X0)
        + logic_reg(1, 0b01, 0, 0, 1, 0, 31, 0)  # [8]  MOV X0, X1 (a = b)
        + logic_reg(1, 0b01, 0, 0, 3, 0, 31, 1)  # [12] MOV X1, X3 (b = r)
        + cbz_cbnz(1, 0, 2, 1)                 # [16] CBZ X1, #8 → [24] when b=0
        + branch_imm(0, -5)                    # [20] B #-20 → [0] loop
        + HALT                                  # [24]
    )
    s = run(prog, {0: 48, 1: 18})
    assert s.x0 == 6   # GCD(48, 18) = 6


# ── Absolute value ────────────────────────────────────────────────────────────


def test_abs_value():
    """
    Absolute value using conditional select.

    abs(x) = (x >= 0) ? x : -x

    Use SUBS to set flags, then CSNEG (false path negates):
        SUBS XZR, X0, #0   → sets N flag if X0 is negative
        CSNEG X1, X0, X0, PL → X0 if positive (N=0), else -X0
    """
    prog = (
        dp_imm(1, 1, 1, 0, 0, 0, 31)       # SUBS XZR, X0, #0 (sets flags)
        + csel_enc(1, 1, 0, 0, COND_EQ ^ 1, 0b01, 0, 1)  # CSNEG X1, X0, X0, PL(N=0)
        + HALT
    )
    # COND_PL = 0b0101; CSNEG: true=X0, false=-X0
    # When N=0 (positive), COND_PL is true → result = X0
    # When N=1 (negative), COND_PL is false → result = -X0
    from aarch64_simulator import COND_PL
    prog = (
        dp_imm(1, 1, 1, 0, 0, 0, 31)           # SUBS XZR, X0, #0
        + csel_enc(1, 1, 0, 0, COND_PL, 0b01, 0, 1)  # CSNEG X1, X0, X0, PL
        + HALT
    )
    # Test with positive value
    s = run(prog, {0: 42})
    assert s.x1 == 42

    # Test with negative value
    s = run(prog, {0: ((-7) & 0xFFFF_FFFF_FFFF_FFFF)})
    assert s.x1 == 7


# ── Power of 2 check ──────────────────────────────────────────────────────────


def test_power_of_two_check():
    """
    Check if X0 is a power of 2 using the classic bitmask test.

    A number n is a power of 2 if (n & (n-1)) == 0 (and n != 0).
    Steps:
        SUB X1, X0, #1    X1 = X0 - 1
        AND X2, X0, X1    X2 = X0 & (X0 - 1)
        CBZ X2, #8        if zero: it's a power of 2, skip X3=0 and set X3=1
    """
    prog = (
        dp_imm(1, 1, 0, 1, 0, 0, 1)            # [0]  SUB X1, X0, #1
        + logic_reg(1, 0b00, 0, 0, 1, 0, 0, 2)  # [4]  AND X2, X0, X1
        + cbz_cbnz(1, 0, 2, 2)                  # [8]  CBZ X2, #+8 → [16]
        + movwide(1, 0b10, 0, 0, 3)              # [12] X3=0 (not power of 2)
        + branch_imm(0, 2)                       # [16]... wait, we need to skip [16]
        + HALT
    )
    # Actually let's do simpler: X3=1 if power of 2, X3=0 if not
    prog = (
        dp_imm(1, 1, 0, 1, 0, 0, 1)            # [0]  SUB X1, X0, #1
        + logic_reg(1, 0b00, 0, 0, 1, 0, 0, 2)  # [4]  AND X2, X0, X1
        + cbz_cbnz(1, 1, 2, 2)                  # [8]  CBNZ X2, #+8 → [16] if NOT power of 2
        + movwide(1, 0b10, 0, 1, 3)              # [12] X3=1 (power of 2)
        + branch_imm(0, 2)                       # [16] B #+8 → [24]  skip X3=0
        + movwide(1, 0b10, 0, 0, 3)              # [20] X3=0 (not power of 2)
        + HALT                                   # [24]
    )
    s = run(prog, {0: 16})
    assert s.x3 == 1, f"16 should be power of 2, got X3={s.x3}"

    s = run(prog, {0: 12})
    assert s.x3 == 0, f"12 should not be power of 2, got X3={s.x3}"


# ── Array copy ────────────────────────────────────────────────────────────────


def test_array_copy():
    """
    Copy 4 bytes from one memory region to another.

    Source at address 0x200, destination at 0x300.
    Reads 4 bytes one at a time using LDRB/STRB.
    """
    sim = AArch64Simulator()
    prog = (
        movwide(1, 0b10, 0, 0x200, 0)           # [0]  X0 = src=0x200
        + movwide(1, 0b10, 0, 0x300, 1)         # [4]  X1 = dst=0x300
        + movwide(1, 0b10, 0, 4, 2)             # [8]  X2 = count=4
        # loop at [12]
        + ldst_uoff(0, 0, 0b01, 0, 0, 3)        # [12] LDRB W3, [X0]
        + ldst_uoff(0, 0, 0b00, 0, 1, 3)        # [16] STRB W3, [X1]
        + dp_imm(1, 0, 0, 1, 0, 0, 0)           # [20] ADD X0, X0, #1
        + dp_imm(1, 0, 0, 1, 0, 1, 1)           # [24] ADD X1, X1, #1
        + dp_imm(1, 1, 1, 1, 0, 2, 2)           # [28] SUBS X2, X2, #1
        + branch_cond(-5, COND_NE)               # [32] B.NE #-20 → [12]
        + HALT
    )
    sim.load(prog)
    s0 = sim.get_state()
    mem = list(s0.memory)
    mem[0x200] = 0xDE
    mem[0x201] = 0xAD
    mem[0x202] = 0xBE
    mem[0x203] = 0xEF
    sim._state = AArch64State(  # type: ignore[attr-defined]
        pc=s0.pc, gpr=s0.gpr, sp=s0.sp, nzcv=s0.nzcv,
        memory=tuple(mem), halted=s0.halted
    )
    from conftest import run_from_current
    s, _ = run_from_current(sim)
    assert s.memory[0x300] == 0xDE
    assert s.memory[0x301] == 0xAD
    assert s.memory[0x302] == 0xBE
    assert s.memory[0x303] == 0xEF


# ── Factorial (recursive via BL/RET) ──────────────────────────────────────────


def test_factorial_iterative():
    """
    Compute 5! = 120 using an iterative multiply loop.

    X0 = n (counts down from 5 to 1)
    X1 = accumulator (product), starts at 1
    """
    prog = (
        movwide(1, 0b10, 0, 5, 0)              # [0]  X0 = 5
        + movwide(1, 0b10, 0, 1, 1)            # [4]  X1 = 1
        # loop at [8]
        + madd_msub(1, 0, 0, 0, 31, 1, 1)     # [8]  MUL X1, X1, X0 (MADD X1,X1,X0,XZR)
        + dp_imm(1, 1, 1, 1, 0, 0, 0)         # [12] SUBS X0, X0, #1
        + branch_cond(-2, COND_NE)             # [16] B.NE #-8 → [8]
        + HALT
    )
    s = run(prog)
    assert s.x1 == 120   # 5! = 120


# ── Max of two values ─────────────────────────────────────────────────────────


def test_max_of_two():
    """
    Return max(X0, X1) in X2 using CSEL after CMP.

    CMP X0, X1 (SUBS XZR, X0, X1) → sets flags
    CSEL X2, X0, X1, GE → X0 if X0 >= X1, else X1
    """
    prog = (
        dp_reg(1, 1, 1, 0, 1, 0, 0, 31)       # CMP X0, X1 (SUBS XZR, X0, X1)
        + csel_enc(1, 0, 0, 1, 0b1010, 0b00, 0, 2)  # CSEL X2, X0, X1, GE
        + HALT
    )
    s = run(prog, {0: 42, 1: 17})
    assert s.x2 == 42

    s = run(prog, {0: 5, 1: 100})
    assert s.x2 == 100

    s = run(prog, {0: 7, 1: 7})
    assert s.x2 == 7   # equal → GE is true → X0


# ── BL / RET subroutine call ──────────────────────────────────────────────────


def test_bl_ret_subroutine():
    """
    Call a subroutine that doubles X0, then return.

    Layout:
        [0]  BL #double     → X30=4; PC=double
        [4]  HALT
        [8] double:
             ADD X0, X0, X0  → X0 *= 2
        [12] RET             → PC=X30=4 → HALT
    """
    prog = (
        branch_imm(1, 2)                       # [0] BL #+8 → X30=4; PC=8
        + HALT                                  # [4]
        + dp_reg(1, 0, 0, 0, 0, 0, 0, 0)      # [8] ADD X0, X0, X0
        + branch_reg(0b010, 30)                 # [12] RET
    )
    s = run(prog, {0: 21})
    assert s.x0 == 42
