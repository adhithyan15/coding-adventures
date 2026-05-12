"""End-to-end program tests for the CDC 6600 simulator.

Each test encodes a small hand-assembled program and verifies the result.
Programs are described in pseudocode alongside the encoding for readability.

Parcel address arithmetic
--------------------------
- Short (15-bit) instruction = 1 parcel → advances P by 1
- Long (30-bit) instruction  = 2 parcels → advances P by 2
- Branch targets are parcel addresses (NOT byte offsets)

Helper function ``assemble()`` concatenates bytes and returns the program.
"""

from cdc6600_simulator import HALT, CDC6600Simulator, long_instr, short_instr
from cdc6600_simulator.simulator import (
    F_BXXR,
    F_CMPLT,
    F_IAAP,
    F_IBBM,
    F_IXMUL,
    F_IXXP,
    F_JEQ,
    F_JMP,
    F_JNE,
    F_JSR,
    F_LDAI,
    F_LDBI,
    F_LDX,
    F_LDXI,
    F_RET,
    F_STX,
    F_TXB,
)


def assemble(*pieces: bytes) -> bytes:
    return b"".join(pieces)


def run(prog: bytes, max_steps: int = 10_000):
    sim = CDC6600Simulator()
    result = sim.execute(prog, max_steps=max_steps)
    assert result.ok, f"Program error: {result.error}"
    return result.final_state


# ── Program 1: Sum 1 + 2 + … + 10 = 55 ──────────────────────────────────────
#
# Pseudocode:
#   X1 = 0         (accumulator)
#   B1 = 10        (counter: counts down from 10 to 0)
#   B2 = 1         (constant 1 for decrement)
#   loop:
#     TXB X3, B1   (X3 = B1 as 60-bit value)
#     IXXP X1, X1, X3  (X1 += X3)
#     IBBM B1, B1, B2  (B1 -= 1)
#     JNE B1!=0, loop  (if B1 != 0 goto loop)
#   HALT
#
# Parcel layout:
#   P= 0,1:  LDXI X1, 0       (2 parcels)
#   P= 2,3:  LDBI B1, 10      (2 parcels)
#   P= 4,5:  LDBI B2, 1       (2 parcels)
#   P= 6:    TXB  X3, B1      (1 parcel) ← loop top
#   P= 7:    IXXP X1, X1, X3 (1 parcel)
#   P= 8:    IBBM B1, B1, B2 (1 parcel)
#   P= 9,10: JNE  B1!=0, 6   (2 parcels → next P=11)
#   P=11:    HALT

def test_sum_1_to_10():
    prog = assemble(
        long_instr(F_LDXI, 1, 0, 0),        # P=0: X1=0
        long_instr(F_LDBI, 1, 0, 10),       # P=2: B1=10
        long_instr(F_LDBI, 2, 0, 1),        # P=4: B2=1
        # loop top at P=6:
        short_instr(F_TXB, 3, 1, 0),        # P=6: X3=B1
        short_instr(F_IXXP, 1, 1, 3),       # P=7: X1+=X3
        short_instr(F_IBBM, 1, 1, 2),       # P=8: B1-=B2
        long_instr(F_JNE, 0, 1, 6),         # P=9: if B1!=0 goto P=6
        HALT,                                # P=11
    )
    s = run(prog)
    assert s.x1 == 55


# ── Program 2: Factorial 5! = 120 ─────────────────────────────────────────────
#
# Pseudocode:
#   X1 = 1         (accumulator, starts at 1)
#   B1 = 5         (counter: 5, 4, 3, 2, 1)
#   B2 = 1
#   loop:
#     TXB X2, B1   (X2 = B1 as 60-bit)
#     IXMUL X1, X1, X2  (X1 *= X2)
#     IBBM B1, B1, B2   (B1 -= 1)
#     JNE B1!=0, loop
#   HALT

def test_factorial_5():
    prog = assemble(
        long_instr(F_LDXI, 1, 0, 1),        # P=0: X1=1
        long_instr(F_LDBI, 1, 0, 5),        # P=2: B1=5
        long_instr(F_LDBI, 2, 0, 1),        # P=4: B2=1
        # loop top at P=6:
        short_instr(F_TXB, 2, 1, 0),        # P=6: X2=B1
        short_instr(F_IXMUL, 1, 1, 2),      # P=7: X1*=X2
        short_instr(F_IBBM, 1, 1, 2),       # P=8: B1-=1
        long_instr(F_JNE, 0, 1, 6),         # P=9: if B1!=0 goto P=6
        HALT,                                # P=11
    )
    s = run(prog)
    assert s.x1 == 120


# ── Program 3: Fibonacci F(0)..F(9) written to memory, verify F(9)=34 ─────────
#
# F(0)=0, F(1)=1, F(2)=1, F(3)=2, F(4)=3, F(5)=5, F(6)=8, F(7)=13, F(8)=21, F(9)=34
#
# Pseudocode:
#   mem[200] = 0   (F0)
#   mem[201] = 1   (F1)
#   A1 = 200       (base address)
#   B1 = 2         (index into mem, starts at 2)
#   B5 = 10        (stop when B1 == 10)
#   B2 = 1         (constant 1)
#   loop:
#     if B1 == B5: done
#     X3 = mem[A1 + B1 - 2]  (prev-prev)
#     X4 = mem[A1 + B1 - 1]  (prev)
#     X5 = X3 + X4
#     mem[A1 + B1] = X5
#     B1 += 1
#   done:
#   X1 = mem[A1 + 9]   (load F(9))
#   HALT

def test_fibonacci_f9():
    # Store F(0)=0 and F(1)=1 before the loop
    # We'll use A2 for relative addressing tricks
    #
    # Simpler approach: pre-store F0,F1 and iterate 8 more steps
    # using indirect addressing: A1=200, compute each F(n) = F(n-2) + F(n-1)
    #
    # Encoding note: LDX Xi, Aj+K uses static offset K; we can't use a dynamic
    # B-register offset for LDX directly.  Instead we move A1 forward each step.
    #
    # Strategy: use B1 as loop counter (8 iterations), A1 = base=200
    #   each iter: load from A1+0 (F(n-2)) and A1+1 (F(n-1)), store to A1+2,
    #   then increment A1 by 1 (IAAP A1, A1, B2) where B2=1.

    prog = assemble(
        # Store F(0)=0 and F(1)=1 at words 200 and 201
        long_instr(F_LDXI, 1, 0, 0),        # P=0:  X1 = 0
        long_instr(F_LDAI, 1, 0, 200),      # P=2:  A1 = 200
        long_instr(F_STX, 1, 1, 0),         # P=4:  mem[A1+0=200] = X1 (F0=0)
        long_instr(F_LDXI, 1, 0, 1),        # P=6:  X1 = 1
        long_instr(F_STX, 1, 1, 1),         # P=8:  mem[A1+1=201] = X1 (F1=1)
        # Setup loop: A1=200, B1=8 iterations, B2=1
        long_instr(F_LDAI, 1, 0, 200),      # P=10: A1=200
        long_instr(F_LDBI, 1, 0, 8),        # P=12: B1=8 (loop count)
        long_instr(F_LDBI, 2, 0, 1),        # P=14: B2=1 (stride)
        # loop top at P=16:
        #   X3 = mem[A1+0]   (F(n-2))
        #   X4 = mem[A1+1]   (F(n-1))
        #   X5 = X3 + X4
        #   mem[A1+2] = X5
        #   A1 += 1  (IAAP A1, A1, B2)
        #   B1 -= 1  (IBBM B1, B1, B2)
        #   JNE B1!=0, P=16
        long_instr(F_LDX, 3, 1, 0),         # P=16: X3=mem[A1+0]
        long_instr(F_LDX, 4, 1, 1),         # P=18: X4=mem[A1+1]
        short_instr(F_IXXP, 5, 3, 4),       # P=20: X5=X3+X4
        long_instr(F_STX, 1, 5, 2),         # P=21: mem[A1+2]=X5
        short_instr(F_IAAP, 1, 1, 2),       # P=23: A1+=1
        short_instr(F_IBBM, 1, 1, 2),       # P=24: B1-=1
        long_instr(F_JNE, 0, 1, 16),        # P=25: if B1!=0 goto P=16
        # After loop: A1=208, F(9) is at address 200+9=209 but we advanced A1
        # to 200+8=208; F(9)=mem[208+1]=mem[209]? No: A1 starts at 200 and
        # we do +1 each iter, so after 8 iters A1=208. F(9) is stored to
        # mem[A1+2] when A1=207 (last iter). Let me verify:
        #   iter1: A1=200 → store F(2) to mem[202], then A1→201
        #   iter2: A1=201 → store F(3) to mem[203], then A1→202
        #   ...
        #   iter8: A1=207 → store F(9) to mem[209], then A1→208
        # So F(9) is at address 209. To load it we need A1=209 or use A1=208, offset=1.
        # After the loop A1=208.
        long_instr(F_LDX, 1, 1, 1),         # P=27: X1=mem[A1+1=209]=F(9)
        HALT,                                # P=29
    )
    s = run(prog)
    assert s.x1 == 34


# ── Program 4: Subroutine call/return ─────────────────────────────────────────
#
# Subroutine "double": X1 = X1 + X1 (doubles X1)
# Main: X1=21; call double; verify X1=42

def test_subroutine_call_return():
    # Layout:
    #   P= 0,1: LDXI X1, 21
    #   P= 2,3: JSR to P=8 (double subroutine)
    #   P= 4,5: LDXI X2, 99  (verify this runs after return)
    #   P= 6:   HALT
    #   P= 7:   HALT  (padding to align subroutine)
    #   P= 8:   IXXP X1,X1,X1  (X1 = X1+X1)
    #   P= 9,10: RET B7
    #   P=11:   HALT

    prog = assemble(
        long_instr(F_LDXI, 1, 0, 21),       # P=0:  X1=21
        long_instr(F_JSR, 0, 0, 8),         # P=2:  call P=8; B7=4
        long_instr(F_LDXI, 2, 0, 99),       # P=4:  X2=99 (after return)
        HALT,                                # P=6
        HALT,                                # P=7  (padding)
        short_instr(F_IXXP, 1, 1, 1),       # P=8:  X1=X1+X1 (double)
        long_instr(F_RET, 0, 7, 0),         # P=9:  RET (P=B7=4)
        HALT,                                # P=11
    )
    s = run(prog)
    assert s.x1 == 42
    assert s.x2 == 99


# ── Program 5: Array sum using LDX + IAAP loop ────────────────────────────────
#
# Sum an array of 5 values stored at words 500–504: [3, 7, 11, 2, 6] = 29

def test_array_sum():
    # First we need to store the array values, then sum them.
    # Array: mem[500]=3, mem[501]=7, mem[502]=11, mem[503]=2, mem[504]=6

    prog = assemble(
        # Store array values at addresses 500–504
        long_instr(F_LDAI, 1, 0, 500),       # P=0:  A1=500
        long_instr(F_LDXI, 2, 0, 3),         # P=2:  X2=3
        long_instr(F_STX, 1, 2, 0),          # P=4:  mem[500]=3
        long_instr(F_LDXI, 2, 0, 7),         # P=6:  X2=7
        long_instr(F_STX, 1, 2, 1),          # P=8:  mem[501]=7
        long_instr(F_LDXI, 2, 0, 11),        # P=10: X2=11
        long_instr(F_STX, 1, 2, 2),          # P=12: mem[502]=11
        long_instr(F_LDXI, 2, 0, 2),         # P=14: X2=2
        long_instr(F_STX, 1, 2, 3),          # P=16: mem[503]=2
        long_instr(F_LDXI, 2, 0, 6),         # P=18: X2=6
        long_instr(F_STX, 1, 2, 4),          # P=20: mem[504]=6
        # Now sum: A1=500, X1=0 (sum), B1=5 (count), B2=1 (stride)
        long_instr(F_LDAI, 1, 0, 500),       # P=22: A1=500
        long_instr(F_LDXI, 1, 0, 0),         # P=24: X1=0 (sum)
        long_instr(F_LDBI, 1, 0, 5),         # P=26: B1=5
        long_instr(F_LDBI, 2, 0, 1),         # P=28: B2=1
        # loop top at P=30:
        #   X3 = mem[A1+0]
        #   X1 += X3
        #   A1 += 1
        #   B1 -= 1
        #   JNE B1!=0, P=30
        long_instr(F_LDX, 3, 1, 0),          # P=30: X3=mem[A1]
        short_instr(F_IXXP, 1, 1, 3),        # P=32: X1+=X3
        short_instr(F_IAAP, 1, 1, 2),        # P=33: A1+=B2
        short_instr(F_IBBM, 1, 1, 2),        # P=34: B1-=B2
        long_instr(F_JNE, 0, 1, 30),         # P=35: if B1!=0 goto P=30
        HALT,                                 # P=37
    )
    s = run(prog)
    assert s.x1 == 29


# ── Program 6: XOR-based swap ─────────────────────────────────────────────────
#
# Swap X1 and X2 using XOR without a temporary:
#   X1 ^= X2
#   X2 ^= X1
#   X1 ^= X2

def test_xor_swap():
    from cdc6600_simulator.state import CDC6600State

    sim = CDC6600Simulator()
    prog = assemble(
        short_instr(F_BXXR, 1, 1, 2),   # X1 = X1 ^ X2
        short_instr(F_BXXR, 2, 2, 1),   # X2 = X2 ^ X1 (= X2 ^ (X1_orig ^ X2) = X1_orig)
        short_instr(F_BXXR, 1, 1, 2),   # X1 = X1 ^ X2 (= (X1_orig^X2) ^ X1_orig = X2_orig)
        HALT,
    )
    sim.load(prog)
    # Set X1=111, X2=222  (X0 is index 0, X1 is index 1, X2 is index 2)
    s = sim.get_state()
    sim._state = CDC6600State(
        p=0, x=(0, 111, 222, 0, 0, 0, 0, 0),
        a=s.a, b=s.b, memory=s.memory, halted=False,
    )
    # Run from current state (don't call execute() — it would re-load)
    for _ in range(100):
        if sim.get_state().halted:
            break
        sim.step()
    final = sim.get_state()
    assert final.x1 == 222
    assert final.x2 == 111


# ── Program 7: Count-down loop using JEQ ──────────────────────────────────────
#
# Use JEQ (branch if B == 0) for a count-down:
#   B1 = 5
#   X1 = 0
#   loop:
#     B1 -= 1
#     X1 = X1 + 1  (via TXB + IXXP)
#     JEQ B1==0, done
#     JMP loop
#   done:
#     HALT

def test_countdown_loop():
    # P= 0,1: LDBI B1, 5
    # P= 2,3: LDBI B2, 1   (decrement constant)
    # P= 4,5: LDXI X1, 0   (counter)
    # P= 6:   IBBM B1, B1, B2   ← loop top
    # P= 7:   TXB  X2, B2   (X2 = 1)
    # P= 8:   IXXP X1, X1, X2  (X1 += 1)
    # P= 9,10: JEQ B1==0, 14   (branch to HALT if done)
    # P=11,12: JMP 6
    # P=13:   HALT  (padding)
    # P=14:   HALT
    prog = assemble(
        long_instr(F_LDBI, 1, 0, 5),        # P=0:  B1=5
        long_instr(F_LDBI, 2, 0, 1),        # P=2:  B2=1
        long_instr(F_LDXI, 1, 0, 0),        # P=4:  X1=0
        # loop at P=6:
        short_instr(F_IBBM, 1, 1, 2),       # P=6:  B1-=1
        short_instr(F_TXB, 2, 2, 0),        # P=7:  X2=B2=1
        short_instr(F_IXXP, 1, 1, 2),       # P=8:  X1+=1
        long_instr(F_JEQ, 0, 1, 14),        # P=9:  if B1==0 goto P=14 (HALT)
        long_instr(F_JMP, 0, 0, 6),         # P=11: goto P=6
        HALT,                                # P=13
        HALT,                                # P=14 (landing pad)
    )
    s = run(prog)
    assert s.x1 == 5   # loop ran 5 times, incrementing X1 each time


# ── Program 8: Maximum of two values using CMPGT ──────────────────────────────
#
# Compute max(X1, X2) → X3
#   CMPGT B1, X1, X2   (B1=1 if X1 > X2)
#   JNE B1 != 0, take_x1   (if B1=1, X3=X1)
#   X3 = X2  (X1 <= X2)
#   JMP done
#   take_x1: X3 = X1
#   done: HALT

def test_max_two_values():
    from cdc6600_simulator.state import CDC6600State

    sim = CDC6600Simulator()
    # Correct layout:
    prog2 = assemble(
        # P=0: CMPLT B1, X1, X2  (B1=1 if X1<X2, meaning X2 is max)
        short_instr(F_CMPLT, 1, 1, 2),
        # P=1,2: JNE B1!=0, P=7  (if X1<X2, jump to "X3=X2" at P=7)
        long_instr(F_JNE, 0, 1, 7),
        # P=3: X1>=X2 path: X3=X1
        short_instr(F_IXXP, 3, 1, 0),      # X3 = X1+0 = X1
        # P=4,5: JMP P=9
        long_instr(F_JMP, 0, 0, 9),
        # P=6: padding
        HALT,
        # P=7: X1<X2 path: X3=X2
        short_instr(F_IXXP, 3, 2, 0),      # X3 = X2+0 = X2
        # P=8: padding (so P=9 is next)
        HALT,
        # P=9: HALT
        HALT,
    )

    def run_with_x(x1, x2):
        sim.load(prog2)
        s = sim.get_state()
        sim._state = CDC6600State(
            p=0, x=(0, x1, x2, 0, 0, 0, 0, 0),
            a=s.a, b=s.b, memory=s.memory, halted=False,
        )
        for _ in range(100):
            if sim.get_state().halted:
                break
            sim.step()
        return sim.get_state()

    # Test with X1=10, X2=30 → max=30
    assert run_with_x(10, 30).x3 == 30

    # Test with X1=50, X2=20 → max=50
    assert run_with_x(50, 20).x3 == 50


# ── Program 9: Memory copy ─────────────────────────────────────────────────────
#
# Copy 4 words from addresses [300..303] to [400..403]

def test_memory_copy():
    # Store source data [10,20,30,40] at 300-303, then copy to 400-403.
    # Parcel counts: each long_instr=2, each short_instr=1.
    # Setup: 9 long = 18 + 4 long = 8 → 26 parcels; loop at P=26.

    prog2 = assemble(
        # Store [10, 20, 30, 40] at addresses 300–303
        long_instr(F_LDAI, 1, 0, 300),       # P=0
        long_instr(F_LDXI, 2, 0, 10),        # P=2
        long_instr(F_STX, 1, 2, 0),          # P=4
        long_instr(F_LDXI, 2, 0, 20),        # P=6
        long_instr(F_STX, 1, 2, 1),          # P=8
        long_instr(F_LDXI, 2, 0, 30),        # P=10
        long_instr(F_STX, 1, 2, 2),          # P=12
        long_instr(F_LDXI, 2, 0, 40),        # P=14
        long_instr(F_STX, 1, 2, 3),          # P=16
        # Setup
        long_instr(F_LDAI, 1, 0, 300),       # P=18
        long_instr(F_LDAI, 2, 0, 400),       # P=20
        long_instr(F_LDBI, 1, 0, 4),         # P=22
        long_instr(F_LDBI, 2, 0, 1),         # P=24
        # loop at P=26:
        long_instr(F_LDX, 3, 1, 0),          # P=26: X3=mem[A1]
        long_instr(F_STX, 2, 3, 0),          # P=28: mem[A2]=X3
        short_instr(F_IAAP, 1, 1, 2),        # P=30: A1+=1
        short_instr(F_IAAP, 2, 2, 2),        # P=31: A2+=1
        short_instr(F_IBBM, 1, 1, 2),        # P=32: B1-=1
        long_instr(F_JNE, 0, 1, 26),         # P=33: if B1!=0 goto P=26
        HALT,                                 # P=35
    )

    s = run(prog2)
    # Verify destination addresses 400–403 hold [10, 20, 30, 40]
    assert s.memory[400] == 10
    assert s.memory[401] == 20
    assert s.memory[402] == 30
    assert s.memory[403] == 40
